# Chương 94: Tiền huấn luyện (AdamW, Muon, WSD) & Động cơ Phục vụ Suy luận (PagedAttention, RadixAttention, Giải mã Suy đoán & Int8) (Pretraining Optimizers & LLM Serving Engine)

---

## 1. Giới thiệu & Mục tiêu

Chào mừng bạn đến với **Chương 94** của giáo trình `learn-rs` — một trong những điểm nút hạ tầng công nghệ quan trọng nhất thuộc **Chủ đề 27: Kiến trúc, Suy luận & Huấn luyện Hậu kỳ LLM Từ Số Không Bằng Rust Thuần `std`**.

Sau khi đã nắm vững nền tảng biểu diễn từ vựng, đại số tuyến tính có bước nhảy (`Strided Tensor`) và động cơ vi phân ngược (`AutogradTape`) ở Chương 92, cũng như kiến trúc khối giải mã biến áp hiện đại (`RMSNorm`, `RoPE`, `SwiGLU`, `GQA`, `DeepSeek MLA`, `GatedDeltaNet` và `SparseMoE`) ở Chương 93, chúng ta đứng trước hai bài toán sống còn quyết định toàn bộ tính khả thi của một hệ thống trí tuệ nhân tạo quy mô lớn:

1. **Làm thế nào để huấn luyện mạng nơ-ron hội tụ nhanh, ổn định và không bùng nổ gradient?**
   - Sự trỗi dậy của các mô hình ngôn ngữ biên giới (Frontier LLMs 2024–2026) cho thấy bộ tối ưu hóa kinh điển `AdamW` dù rất mạnh mẽ nhưng đang bộc lộ điểm nghẽn nghiêm trọng khi cập nhật các ma trận trọng số 2D ẩn: hiện tượng các giá trị kỳ dị (singular values) bị lệch pha, dẫn đến tốc độ học không đồng đều giữa các hướng không gian.
   - Thuật toán **`Muon` (Momentum Orthogonalized by Newton–Schulz)** ra đời như một cuộc cách mạng: thay vì chỉ chuẩn hóa từng phần tử vô hướng như AdamW, Muon trực giao hóa toàn bộ ma trận hướng cập nhật bằng chuỗi lặp Newton–Schulz 5 bước, ép các giá trị kỳ dị về mặt cầu phổ đơn vị $O(1)$.
   - Đi kèm với Muon là sự chuyển dịch từ lịch học suy giảm Cosine truyền thống (`CosineSchedule`) sang **`WsdSchedule` (Warmup - Stable - Decay)** — cho phép mô hình duy trì trạng thái học ổn định trên hàng nghìn tỷ token trước khi hạ cánh mượt mà ở pha phân rã, mở ra khả năng tiếp tục huấn luyện (continual pretraining) linh hoạt mà không cần lập lịch lại từ đầu.

2. **Làm thế nào để phục vụ suy luận (serving engine) hàng nghìn người dùng đồng thời với độ trễ tối thiểu và bộ nhớ hữu hạn?**
   - Trong giai đoạn sinh tự hồi quy (autoregressive generation), việc tính toán lại toàn bộ ngữ cảnh quá khứ tại mỗi bước sinh token mới là một thảm họa tính toán $O(N^2)$. Giải pháp lưu đệm Khóa–Giá trị (**KV-cache**) giải quyết bài toán tính toán nhưng lại tạo ra **cuộc khủng hoảng bộ nhớ vật lý**.
   - Bộ nhớ KV-cache tăng tuyến tính theo độ dài chuỗi và số lượng phiên đồng thời. Nếu cấp phát các mảng liên tục tĩnh trong RAM/VRAM, hệ thống sẽ gánh chịu tới **60%–80% lãng phí bộ nhớ do phân mảnh ngoài (external fragmentation)** và dự phòng quá mức (over-reservation).
   - Kiến trúc **`PagedAttention`** (được tiên phong bởi vLLM) giải quyết triệt để vấn đề này bằng cách mượn tư tưởng phân trang bộ nhớ ảo của hệ điều hành: chia KV-cache thành các khối vật lý cố định (`PhysicalBlock`), quản lý qua bảng khối logic (`PagedKvBlockTable`), và hỗ trợ **Copy-on-Write (CoW)** không tốn chi phí khi rẽ nhánh hội thoại.
   - Để tối ưu hóa sâu hơn nữa chi phí tác tử đa lượt (như đã phân tích tại Chương 86 và Chương 91), cấu trúc **`RadixPrefixTree`** (kiểu SGLang) lưu vết cây tiền tố các khối KV đã sinh, cho phép tái sử dụng ngay lập tức các đoạn prompt hệ thống dùng chung (`cache hit 100%`) mà không phải tính lại dù chỉ một phép nhân ma trận.
   - Cuối cùng, để tăng tốc độ sinh token lên gấp 2–3 lần mà không làm thay đổi phân phối xác suất đầu ra, kỹ thuật **Giải mã suy đoán chính xác (`speculative_generate`)** sử dụng một mô hình nháp nhỏ gọn để đề xuất $K$ token trước khi mô hình lớn thẩm định trong một lần quét duy nhất. Toàn bộ trọng số của mô hình phục vụ được nén bằng **`Int8Linear`** lượng tử hóa đối xứng theo kênh, giảm một nửa dung lượng bộ nhớ mà vẫn bảo toàn độ tương đồng cosine $> 0,999$.

Tất cả các cơ chế trên sẽ được chúng ta kiến tạo hoàn toàn bằng **Rust 2024 thuần `std`**, không sử dụng bất kỳ thư viện bên ngoài nào, tuân thủ nghiêm ngặt nguyên tắc named fields, không cảnh báo biên dịch (`-D warnings`) và cấm tuyệt đối định danh phi ASCII (`-F non-ascii-idents`).

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│                        KIẾN TRÚC TỔNG THỂ HẠ TẦNG CHƯƠNG 94                             │
├───────────────────────────────────────────┬────────────────────────────────────────────┤
│      1. ĐỘNG CƠ TIỀN HUẤN LUYỆN           │      2. ĐỘNG CƠ PHỤC VỤ SUY LUẬN           │
│         (Pretraining Pipeline)            │         (Inference Serving Engine)         │
├───────────────────────────────────────────┼────────────────────────────────────────────┤
│ • CausalDataLoader (mini-batch, shift)    │ • KvCache liên tục + RoPE incremental      │
│ • clip_grad_norm_l2 (chống nổ gradient)   │ • PagedKvBlockTable (vLLM Copy-on-Write)   │
│ • AdamW (Decoupled Weight Decay, 1D params)│ • RadixPrefixTree (SGLang LRU Cache)      │
│ • MuonOptimizer (Newton–Schulz 5 bước, 2D)│ • TokenSampler (temp, top-k, top-p, min-p) │
│ • Lịch học: CosineSchedule vs WsdSchedule │ • speculative_generate (chấp nhận chuẩn)   │
│ • Kiểm chứng sai phân hữu hạn < 1e-8      │ • Int8Linear (Lượng tử hóa đối xứng i8)    │
└───────────────────────────────────────────┴────────────────────────────────────────────┘
```

---

### Bảng Mục tiêu Học tập & Kỹ năng Thực chiến

| Thành phần kỹ thuật | Vấn đề cốt lõi giải quyết | Tiêu chuẩn kiểm chứng toán học & hệ thống |
|---|---|---|
| **`CausalDataLoader` & `clip_grad_norm_l2`** | Đóng gói chuỗi token thành các batch huấn luyện nhân quả (mục tiêu dịch chuyển 1 bước); chặn đứng hiện tượng nổ gradient trong mạng sâu | Inputs và targets khớp $y_t = x_{t+1}$; chuẩn gradient sau kẹp không bao giờ vượt quá `max_norm`, bảo toàn hướng vector |
| **`AdamW`** | Tách rời suy giảm trọng số khỏi bước tính đạo hàm thích nghi, tránh làm méo mó độ lớn gradient ở các bước đầu qua hiệu chỉnh chệch | Suy giảm trọng số diễn ra độc lập với gradient; ở bước 1, $\hat{m}_1 / \sqrt{\hat{v}_1} \approx \text{sgn}(g)$ |
| **`MuonOptimizer` (Newton–Schulz)** | Khắc phục hiện tượng suy biến phổ ma trận 2D; trực giao hóa hướng cập nhật bằng thuật toán Newton–Schulz 5 bước với hệ số $(3.4445, -4.7750, 2.0315)$ | Đối xứng chuyển vị cao/rộng sai khác $< 10^{-12}$; đường chéo ma trận Gram $X^T X$ nằm gọn trong dải hội tụ $[0.45, 1.35]$; định tuyến nghiêm ngặt 2D sang Muon và 1D sang AdamW |
| **Lịch học `Cosine` & `WsdSchedule`** | Điều tiết tốc độ học qua các pha Warmup, Stable và Decay; WSD cho phép duy trì tri thức tối đa và linh hoạt thời điểm dừng huấn luyện | Tính liên tục tuyệt đối tại các biên pha; pha Stable giữ nguyên $lr_{\text{base}}$; pha Decay hạ mượt mà về $min\_lr$ |
| **`PagedKvBlockTable` (CoW)** | Loại bỏ phân mảnh bộ nhớ ngoài của KV-cache; cho phép rẽ nhánh luồng đối thoại (beam search, parallel requests) với chi phí sao chép bằng 0 | Phân bổ khối cố định qua `PhysicalBlockPool`; khi rẽ nhánh chỉ tăng `ref_count`; chỉ cấp phát khối mới khi thực sự ghi đè (Copy-on-Write) |
| **`RadixPrefixTree` (SGLang)** | Tái sử dụng vùng đệm KV của các đoạn văn bản tiền tố giống nhau (system prompt, few-shot examples) giữa các yêu cầu khác nhau | Tự động tách cạnh (edge splitting) khi khớp một phần; thu hồi khối theo chính sách LRU có hỗ trợ ghim (`pin`); cache hit $100\%$ khối tiền tố |
| **`TokenSampler` (Đặc biệt `min-p`)** | Điều khiển phân phối từ vựng; `min-p` tự động co dãn ngưỡng lọc theo xác suất của token cao nhất ($p \ge \text{min\_p} \times p_{\max}$), loại bỏ nhiễu đuôi | Greedy cho argmax chính xác; min-p loại bỏ hoàn toàn các token rác khi mô hình tự tin cao và mở rộng đa dạng khi mô hình phân vân |
| **`speculative_generate`** | Tăng tốc độ giải mã suy đoán: mô hình nhỏ sinh nhanh $K$ token, mô hình lớn thẩm định trong một lượt xuôi duy nhất | Phân phối thực nghiệm khớp 100% phân phối mô hình đích qua $2.000$ mẫu; tốc độ sinh đạt $> 1.0$ token trên mỗi bước thẩm định |
| **`Int8Linear`** | Nén trọng số ma trận xuống số nguyên 8-bit đối xứng theo từng hàng/kênh, tiết kiệm 50% bộ nhớ lưu trữ | Độ tương đồng Cosine giữa trọng số giải lượng tử hóa và trọng số thực gốc đạt $> 0,999$ |
| **Mô hình vi mô `MicroCausalModel`** | Kiểm chứng tính đúng đắn của toàn bộ đường ống huấn luyện từ đầu tới cuối | Đạo hàm giải tích khớp sai phân hữu hạn trung tâm sai số $< 10^{-6}$; tiền huấn luyện làm giảm tổn thất $\mathcal{L}_{\text{final}} < 0,35 \times \mathcal{L}_{\text{init}}$ |

---

## 2. Hình tượng hóa đời sống: Xưởng luyện kim vũ trụ và trạm phục vụ cao tốc

Để thấu hiểu sâu sắc bản chất của các thuật toán phức tạp này trước khi chạm vào từng dòng mã nguồn, chúng ta hãy cùng quan sát hai hình tượng ẩn dụ thực tế.

---

### Hình tượng 1: Xưởng luyện kim vũ trụ (Muon, AdamW & Lịch WSD)

Hãy hình dung quá trình tiền huấn luyện một mô hình ngôn ngữ lớn giống như việc rèn đúc một thanh bảo kiếm bằng hợp kim siêu vật liệu trong không gian không trọng lực:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                    HÌNH TƯỢNG: XƯỞNG LUYỆN KIM VŨ TRỤ                        │
│                                                                              │
│  1. ĐỊNH TUYẾN KIM LOẠI (PARAMETER ROUTING):                                 │
│     • Khối thân kiếm (Ma trận 2D):                                           │
│       Diện tích lớn, chịu lực đa chiều ──> Đưa vào Buồng Nén Từ Tính [MUON]   │
│     • Chi tiết khắc rãnh & chuôi kiếm (Vector 1D, Biases, RMSNorm, Embed):   │
│       Từng điểm tinh tế riêng lẻ ──> Đưa vào Tia Laser Tinh Chỉnh [ADAMW]     │
│                                                                              │
│  2. BUỒNG NÉN TỪ TÍNH NEWTON–SCHULZ (MUON ORTHOGONALIZATION):                │
│     Một khối kim loại thô bị méo mó (Gradient có giá trị kỳ dị lệch lạc):    │
│      [ Chiều dài: 1000m | Chiều rộng: 1m ] (Suy biến hình học nghiêm trọng)  │
│     5 chu kỳ nén từ tính với hệ số bậc 3 (3.4445, -4.7750, 2.0315):          │
│      Vòng 1: Ép phẳng các góc nhọn                                           │
│      Vòng 2: Nắn các trục lệch                                               │
│      Vòng 3-5: Hoàn thiện mặt cầu trực giao hoàn hảo (Gram matrix X^T X ≈ I)  │
│                                                                              │
│  3. LÒ NHIỆT LUYỆN THEO LỊCH WSD (WARMUP - STABLE - DECAY):                  │
│     Nhiệt độ (Tốc độ học lr)                                                 │
│       ▲                                                                      │
│  base │       ┌────────────────────────┐ (Pha STABLE: Rèn đúc liên tục,      │
│       │      /                          \             lưu phôi bất cứ lúc nào)│
│       │     / (Pha WARMUP:               \ (Pha DECAY: Làm nguội tinh thể,   │
│       │    /   Làm nóng đều kim loại)     \            khóa cứng vân thép)   │
│   min └────┴──────────────────────────────┴────────► Thời gian (Bước học)   │
│            0   T_warmup                T_stable    T_total                   │
└──────────────────────────────────────────────────────────────────────────────┘
```

1. **Phân loại cấu trúc (Routing)**:
   - Các ma trận ẩn 2D của tầng Attention và MLP quyết định năng lực tư duy cốt lõi của mạng. Nếu dùng AdamW đơn thuần, các phần tử bị chia cho căn bậc hai của bình phương gradient riêng rẽ, khiến cấu trúc hình học tổng thể của ma trận bị méo mó. Muon xem ma trận 2D như một toán tử tuyến tính nguyên khối, áp dụng phép chiếu trực giao để bảo toàn phổ kỳ dị.
   - Ngược lại, các vector 1D (như độ lệch bias, trọng số chuẩn hóa `RMSNorm`, vector nhúng từ vựng) hoạt động như các tham số điều chỉnh độ nhạy cục bộ độc lập, được định tuyến sang AdamW để học với tốc độ riêng cho từng kênh.

2. **Chu kỳ nén Newton–Schulz**:
   - Khi áp lực uốn nắn (gradient) có xu hướng kéo dãn ma trận theo một hướng duy nhất (trị kỳ dị cực lớn) và bỏ quên các hướng khác, thuật toán Newton–Schulz bậc 3 liên tục tính toán đa thức bậc cao của ma trận Gram $X X^T$.
   - Qua đúng 5 lần lặp, mọi trục phân tán năng lượng đều được nắn lại về độ dài chuẩn tắc. Hướng cập nhật trở thành một ma trận trực giao đẳng cự: mô hình bước đi những bước đều đặn và vững chắc trên mặt cầu tham số, không bao giờ rơi vào hố sâu phân kỳ.

3. **Lò nhiệt luyện WSD**:
   - Khác với lịch Cosine truyền thống buộc bạn phải quyết định trước chính xác tổng số bước học (nếu dừng giữa chừng khi tốc độ học còn cao thì mô hình dở dang, nếu đổi ý muốn học thêm thì phải khởi động lại từ đầu), lịch **WSD** mở ra một kỷ nguyên mới.
   - Pha Warmup làm nóng nhẹ nhàng tránh sốc nhiệt. Pha Stable duy trì nhiệt độ tối đa ổn định xuyên suốt 80% thời gian: tri thức mới liên tục được hấp thụ với tốc độ cao nhất. Bất kỳ lúc nào bạn quyết định đóng gói mô hình để phát hành, bạn chỉ cần kích hoạt pha Decay trong 10%–20% bước cuối cùng để làm nguội từ từ, kết tinh toàn bộ tri thức vào các điểm cực tiểu mịn.

---

### Hình tượng 2: Trạm phục vụ cao tốc & Khách sạn thông minh (PagedAttention, RadixTree, Speculative & Int8)

Khi mô hình đã được rèn đúc xong và bước vào trạm phục vụ hàng triệu người dùng, một bài toán giao thông khổng lồ xuất hiện:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│            HÌNH TƯỢNG: TRẠM DỊCH VỤ PHỤC VỤ SUY LUẬN CAO TỐC                 │
│                                                                              │
│  1. BÃI ĐỖ XE PHÂN TRANG (PAGEDATTENTION):                                   │
│     • Cách truyền thống: Mỗi xe (request) đăng ký trước một làn đường dài    │
│       thênh thang 2048 mét. Dù xe mới đi được 10 mét, không ai được vào làn  │
│       đó ──> 80% bãi đỗ bị bỏ hoang (Phân mảnh ngoài)!                       │
│     • PagedAttention: Bãi đỗ chia thành các ô tiêu chuẩn 4 chỗ (Block).      │
│       Xe cần bao nhiêu chỗ, trạm phát bấy nhiêu thẻ từ. Các ô nằm rải rác     │
│       nhưng được kết nối bằng bản đồ logic trong máy tính điều hành.         │
│     • Copy-on-Write: Hai xe chạy chung hành trình đầu? Chia sẻ chung ô đỗ!   │
│       Chỉ khi một xe rẽ hướng khác mới cần cấp thêm ô mới.                   │
│                                                                              │
│  2. LÀN ĐƯỜNG ƯU TIÊN VÉ ĐIỆN TỬ (RADIX PREFIX TREE):                        │
│     Hàng nghìn người dùng gửi cùng một nội quy hệ thống dài 1000 từ:        │
│      Khách 1: [ Nội quy 1000 từ ] + "Hãy làm thơ"                            │
│      Khách 2: [ Nội quy 1000 từ ] + "Hãy viết code"                          │
│     Cây Radix nhận diện ngay thẻ vé [ Nội quy 1000 từ ] đã đỗ sẵn ở Block 1..250!│
│     Không tính toán lại một giây nào! Tái sử dụng thẳng bộ nhớ đệm KV!       │
│                                                                              │
│  3. CẶP ĐÔI CÔNG CHỨNG VIÊN (SPECULATIVE DECODING):                          │
│     • Trợ lý thực tập trẻ (Draft Model nhỏ): Đánh máy siêu tốc 4 từ dự thảo. │
│     • Chuyên gia trưởng (Target Model lớn): Đọc lướt qua một lượt duy nhất. │
│       - Nếu trợ lý viết đúng: Ký duyệt ngay cả 4 từ + Viết thêm từ thứ 5!    │
│       - Nếu trợ lý viết sai ở từ thứ 2: Sửa lại từ thứ 2 bằng mực đỏ,        │
│         xé bỏ từ 3 & 4.                                                      │
│       Hiệu quả: Chuyên gia chỉ đọc 1 lần nhưng duyệt được nhiều từ cùng lúc! │
│                                                                              │
│  4. THÙNG HÀNG CONTAINER TIÊU CHUẨN (INT8 QUANTIZATION):                     │
│     Hàng hóa cồng kềnh (Số thực f64/f32 chiếm 4-8 byte) được nén gọn gàng    │
│     thành các hộp kim loại chuẩn 1 byte (i8 từ -127 đến +127).               │
│     Mỗi thùng dán một nhãn tỉ lệ co dãn (Scale Factor). Khi mở thùng,        │
│     nhân lại với tỉ lệ: sai số hầu như không đáng kể (Cosine sim > 0.999).   │
└──────────────────────────────────────────────────────────────────────────────┘
```

Sự kết hợp của 4 giải pháp này tạo nên một cỗ máy phục vụ suy luận hiện đại:
- **PagedAttention** giúp tiết kiệm bộ nhớ, tăng dung lượng phục vụ đồng thời lên gấp 2–4 lần trên cùng một phần cứng.
- **RadixPrefixTree** biến chi phí xử lý prompt dài lặp lại thành con số 0.
- **Speculative Decoding** bẻ gãy giới hạn băng thông bộ nhớ, tăng tốc độ phản hồi cho người dùng cuối.
- **Int8Linear** nén mô hình để chạy vừa vặn trên các hệ thống có ngân sách phần cứng hạn hẹp.

---

## 3. Khái niệm & Cơ chế kỹ thuật chuyên sâu

Chúng ta sẽ phân tích chi tiết bản chất toán học và cấu trúc dữ liệu của từng thành phần kỹ thuật.

---

### 3.1 Dữ liệu nhân quả & Kẹp chuẩn L2 Gradient (`clip_grad_norm_l2`)

Trong bài toán mô hình hóa ngôn ngữ tự hồi quy (Causal Language Modeling), mô hình nhận vào một chuỗi token đầu vào $x = (x_0, x_1, \dots, x_{T-1})$ và phải dự đoán token tiếp theo tại mỗi vị trí. Do đó, chuỗi mục tiêu $y$ chính là chuỗi đầu vào được dịch chuyển sang trái một vị trí:
$$y_t = x_{t+1}, \quad \forall t \in \{0, 1, \dots, T-2\}$$

Cấu trúc `CausalDataLoader` chịu trách nhiệm phân cắt kho ngữ liệu phẳng thành các lô mini-batch có kích thước $\text{batch\_size} \times \text{seq\_len}$. Mỗi mẫu trong batch là một cặp $(x, y)$ thỏa mãn bất biến dịch chuyển nhân quả.

Khi huấn luyện các mạng nơ-ron sâu qua thời gian, hiện tượng tích lũy gradient qua nhiều tầng có thể khiến độ dài vector gradient $\|\mathbf{g}\|_2$ tăng vọt đột biến tại các bước học gặp dữ liệu dị biệt (outliers). Nếu áp dụng trực tiếp gradient khổng lồ này, bước nhảy tham số sẽ làm nổ tung ma trận trọng số, đẩy mô hình vào trạng thái tràn số `NaN` hoặc phá hủy hoàn toàn tri thức đã học.

Thuật toán kẹp chuẩn gradient L2 toàn cục được định nghĩa như sau:
1. Tính tổng bình phương toàn bộ các phần tử gradient trong toàn bộ mô hình:
   $$\|\mathbf{g}\|_2 = \sqrt{\sum_{i=1}^{P} g_i^2}$$
2. Nếu $\|\mathbf{g}\|_2 > \gamma_{\max}$ (với $\gamma_{\max}$ là ngưỡng `max_norm`), tính hệ số co dãn:
   $$s = \frac{\gamma_{\max}}{\|\mathbf{g}\|_2 + \epsilon}$$
3. Nhân co dãn từng phần tử gradient:
   $$g_i \leftarrow g_i \times s, \quad \forall i \in \{1, \dots, P\}$$

**Bất biến toán học quan trọng**:
Phép kẹp chuẩn L2 **bảo toàn 100% hướng không gian** của vector gradient $\mathbf{g}$, chỉ giới hạn độ dài cực đại của bước nhảy không vượt quá $\gamma_{\max}$. Nếu chuẩn ban đầu nhỏ hơn hoặc bằng $\gamma_{\max}$, vector gradient được giữ nguyên vẹn tuyệt đối.

---

### 3.2 Bộ tối ưu hóa AdamW (Decoupled Weight Decay)

Thuật toán Adam cổ điển (Kingma & Ba, 2014) tích hợp suy giảm trọng số $L_2$ bằng cách cộng trực tiếp đạo hàm $\lambda \theta$ vào gradient: $g_t \leftarrow g_t + \lambda \theta_t$. Tuy nhiên, Loshchilov & Hutter (2017) đã chứng minh rằng điều này khiến các trọng số có gradient lớn bị suy giảm ít hơn so với các trọng số có gradient nhỏ, làm mất đi ý nghĩa nguyên bản của kỹ thuật Weight Decay.

**AdamW** tách rời hoàn toàn phép suy giảm trọng số ra khỏi việc tính toán các thống kê mô-men bậc một và bậc hai:

$$\begin{aligned}
m_t &= \beta_1 m_{t-1} + (1 - \beta_1) g_t \\
v_t &= \beta_2 v_{t-1} + (1 - \beta_2) g_t^2 \\
\hat{m}_t &= \frac{m_t}{1 - \beta_1^t} \quad \text{(Hiệu chỉnh chệch mô-men bậc một)} \\
\hat{v}_t &= \frac{v_t}{1 - \beta_2^t} \quad \text{(Hiệu chỉnh chệch mô-men bậc hai)} \\
\theta_t &\leftarrow \theta_{t-1} - \eta_t \cdot \lambda \cdot \theta_{t-1} - \eta_t \cdot \frac{\hat{m}_t}{\sqrt{\hat{v}_t} + \epsilon}
\end{aligned}$$

Trong đó:
- $\eta_t$: Tốc độ học tại bước $t$.
- $\lambda$: Hệ số suy giảm trọng số (`weight_decay`).
- $\beta_1, \beta_2$: Các hệ số phân rã động lượng (thường là $0.9$ và $0.999$ hoặc $0.95$).
- $\epsilon$: Hằng số ổn định số học (thường là $10^{-8}$).

Tại bước $t=1$, nếu không có hiệu chỉnh chệch $1 - \beta_1^t$, mô-men $m_1 = (1 - \beta_1) g_1 = 0.1 g_1$ sẽ bị thu nhỏ nhân tạo gấp 10 lần do giá trị khởi tạo $m_0 = 0$. Nhờ việc chia cho $1 - \beta_1^1 = 0.1$, ta thu được $\hat{m}_1 = g_1$, đảm bảo bước cập nhật ban đầu có quy mô chuẩn xác.

---

### 3.3 Bộ tối ưu hóa Muon: Trực giao hóa Newton–Schulz ma trận 2D

Bộ tối ưu hóa **Muon** (Keller Jordan et al., 2024; được áp dụng rộng rãi trong các dự án huấn luyện micro-LLM tốc độ kỷ lục như Moonshine và nanogpt) xuất phát từ một quan sát sâu sắc:

> Đối với ma trận trọng số 2D $W \in \mathbb{R}^{R \times C}$ trong mạng nơ-ron, mục tiêu tối ưu hóa lý tưởng không phải là thay đổi từng phần tử độc lập, mà là cập nhật ma trận sao cho nó bảo toàn tính trực giao của các hướng biểu diễn ẩn.

Xét ma trận động lượng $M \in \mathbb{R}^{R \times C}$. Phép trực giao hóa lý tưởng đưa $M$ về ma trận trực giao gần nhất theo chuẩn Frobenius chính là thành phần $U V^T$ trong phép phân tích giá trị kỳ dị SVD: $M = U \Sigma V^T$.
Tuy nhiên, tính toán SVD chính xác đòi hỏi độ phức tạp $O(\min(R, C)^3)$, hoàn toàn bất khả thi để chạy tại mỗi bước huấn luyện.

Thuật toán Newton–Schulz bậc 3 xấp xỉ ma trận trực giao $(X X^T)^{-1/2} X$ một cách cực kỳ nhanh chóng chỉ bằng các phép nhân ma trận thuần túy (`matmul`), tận dụng tối đa năng lực phần cứng:

#### Thuật toán Newton–Schulz 5 bước của Muon

1. **Chuẩn hóa ban đầu theo chuẩn Frobenius**:
   $$X_0 = \frac{M}{\|M\|_F + 10^{-7}}$$
2. **Xử lý đối xứng hình học**:
   Nếu ma trận cao ($R > C$), ta chuyển vị sang ma trận rộng $X_0 \leftarrow X_0^T \in \mathbb{R}^{C \times R}$ để đảm bảo ma trận Gram $A = X X^T$ luôn có kích thước nhỏ nhất: $r = \min(R, C)$.
3. **Chuỗi lặp Newton–Schulz 5 bước** với các hệ số tối ưu xấp xỉ bậc 3:
   $$a = 3.4445, \quad b = -4.7750, \quad c = 2.0315$$
   Tại mỗi bước lặp $k \in \{0, 1, 2, 3, 4\}$:
   $$\begin{aligned}
   A &= X_k X_k^T \quad &(r \times r) \\
   B &= b \cdot A + c \cdot A^2 \quad &(r \times r) \\
   X_{k+1} &= a \cdot X_k + B \cdot X_k \quad &(r \times c)
   \end{aligned}$$
4. **Phục hồi kích thước**:
   Nếu ở bước 2 đã chuyển vị, ta chuyển vị ngược lại: $X_5 \leftarrow X_5^T \in \mathbb{R}^{R \times C}$.
5. **Hệ số co dãn RMS đặc trưng của Muon**:
   $$s_{\text{RMS}} = 0.2 \cdot \eta \cdot \sqrt{\max(R, C)}$$
6. **Cập nhật trọng số kết hợp phân rã tách rời**:
   $$W \leftarrow W \cdot (1 - \eta \cdot \lambda) - s_{\text{RMS}} \cdot X_5$$

#### Định tuyến tham số nghiêm ngặt (Strict Parameter Routing)

Muon không thể áp dụng cho các tham số dạng vector 1D (vì không có khái niệm trực giao hóa ma trận 2D). Do đó, kiến trúc bắt buộc phải có cơ chế định tuyến:
- **`ParamKind::Matrix2D { rows, cols }`**: Toàn bộ ma trận chiếu Attention ($W_q, W_k, W_v, W_o$) và ma trận tầng ẩn MLP ($W_{\text{gate}}, W_{\text{up}}, W_{\text{down}}$) được đưa vào **Muon**.
- **`ParamKind::Vector1D { size }`**: Bảng nhúng từ vựng (`embed`), đầu dự đoán từ vựng (`lm_head`), và các vector tỉ lệ chuẩn hóa (`RMSNorm`, `LayerNorm`) được đưa vào **AdamW**.

#### Phân tích toán học sâu: Tại sao Newton–Schulz 5 bước lại hội tụ trực giao?

Để hiểu tại sao bộ ba hệ số $(a, b, c) = (3.4445, -4.7750, 2.0315)$ lại có sức mạnh thần kỳ, chúng ta hãy xem xét hàm ánh xạ giá trị kỳ dị của phép lặp:
Giả sử ma trận $X$ có phân tích SVD là $X = U \Sigma V^T$, với các giá trị kỳ dị $\sigma_i \in \text{diag}(\Sigma)$.
Khi áp dụng công thức cập nhật:
$$X_{\text{new}} = a X + b (X X^T) X + c (X X^T)^2 X$$
Vì $X X^T = U \Sigma^2 U^T$, ta có:
$$X_{\text{new}} = U \left( a \Sigma + b \Sigma^3 + c \Sigma^5 \right) V^T$$
Điều này có nghĩa là mỗi giá trị kỳ dị $\sigma$ được cập nhật độc lập thông qua đa thức vô hướng bậc 5:
$$f(\sigma) = 3.4445 \sigma - 4.7750 \sigma^3 + 2.0315 \sigma^5$$

```
    f(σ) ▲
         │                    . - ~ ~ - .  (Điểm cố định σ* = 1.0)
     1.0 ┼ - - - - - - - - -/ - - - - - - █ - - - - - - - - -
         │                 /              │\
         │                /               │ \
         │               /                │  \
         │              /                 │   \
         │             /                  │    \
         │            /                   │     \
     0.0 ┼───────────┴────────────────────┴──────┴────────► σ
         0.0        0.5                  1.0    1.15
```

Đặc tính toán học của đa thức này:
1. **Điểm cố định bậc cao tại $\sigma = 1$**:
   Ta có $f(1) = 3.4445 - 4.7750 + 2.0315 = 1.001 \approx 1.0$.
   Hơn nữa, đạo hàm tại điểm cố định triệt tiêu:
   $$f'(1) = 3.4445 - 3 \times 4.7750 + 5 \times 2.0315 = 3.4445 - 14.3250 + 10.1575 = -0.723$$
   Giá trị $|f'(1)| < 1$ đảm bảo điểm cố định $\sigma^* = 1.0$ là một điểm hút (attractor) cực kỳ mạnh mẽ.
2. **Miền hội tụ rộng lớn**:
   Với bất kỳ giá trị kỳ dị ban đầu nào nằm trong khoảng $\sigma \in (0, 1.15)$, qua 5 bước lặp $f(f(f(f(f(\sigma)))))$, giá trị sẽ hội tụ về $1.0 \pm 0.05$.
3. **Vai trò của chuẩn hóa Frobenius**:
   Phép chia ban đầu $X_0 = M / (\|M\|_F + 10^{-7})$ đảm bảo rằng:
   $$\|X_0\|_F^2 = \sum_{i} \sigma_i^2 = 1.0 \implies \sigma_{\max} \le 1.0$$
   Do đó, mọi giá trị kỳ dị của $X_0$ đều nằm gọn trong khoảng $(0, 1.0]$, hoàn toàn nằm trong lưu vực hội tụ (basin of attraction) của đa thức Newton–Schulz!
4. **Kiểm chứng ma trận Gram thực nghiệm**:
   Sau 5 bước lặp, ma trận Gram $G = X_5^T X_5$ (hoặc $X_5 X_5^T$) trở nên tiệm cận ma trận đơn vị $I$. Đường chéo của $G$ đại diện cho bình phương các độ dài cột/hàng, luôn thỏa mãn bất biến:
   $$0.45 \le G_{d, d} \le 1.35, \quad \forall d$$
   Điều này ngăn chặn triệt để tình trạng một số trọng số bùng nổ trong khi các trọng số khác bị triệt tiêu, giữ cho toàn bộ không gian biểu diễn ẩn của LLM luôn ở trạng thái kích hoạt tối ưu.

---

### 3.4 Lịch học: So sánh Cosine và WSD (Warmup - Stable - Decay)

```
       Tốc độ học (lr)
         ▲
base_lr  │       ┌───────────────────────────────────┐
         │      /                                     \
         │     /                 WSD                   \
         │    /                                         \
         │   /   - - - - - - - - - - - - - - - -         \
         │  /   /                               \         \
         │ /   /             Cosine              \         \
         │/   /                                   \         \
min_lr   └┴──┴─────────────────────────────────────┴─────────┴──► Bước học
         0  Warmup                               Stable     Decay
```

#### Lịch Cosine kinh điển
Tốc độ học tăng tuyến tính trong pha Warmup, sau đó giảm dần liên tục theo hàm cosin trong toàn bộ thời gian còn lại:
$$\eta(t) = \eta_{\min} + \frac{1}{2}(\eta_{\text{base}} - \eta_{\min})\left(1 + \cos\left(\pi \frac{t - T_{\text{warmup}}}{T_{\text{total}} - T_{\text{warmup}}}\right)\right)$$
*Hạn chế*: Đường cong dốc xuống liên tục khiến mô hình giảm khả năng tiếp thu tri thức mới ở nửa sau chu kỳ huấn luyện. Nếu bạn muốn mở rộng thêm dữ liệu, việc kéo dài lịch Cosine sẽ gây sốc phân phối tham số.

#### Lịch WSD (Warmup - Stable - Decay)
Lịch WSD chia quá trình học thành 3 pha rõ rệt:
1. **Pha Khởi động (`Warmup`, $0 \le t < T_w$)**: Tăng tuyến tính từ $\eta_{\min}$ đến $\eta_{\text{base}}$.
2. **Pha Ổn định (`Stable`, $T_w \le t < T_w + T_s$)**: Giữ nguyên tuyệt đối ở mức trần $\eta_{\text{base}}$. Mô hình duy trì khả năng khám phá và tích lũy tri thức tối đa. Mọi điểm kiểm tra (checkpoint) trong pha này đều có thể dùng làm mốc khởi đầu để tiếp tục huấn luyện trên dữ liệu mới mà không bị suy giảm tốc độ học.
3. **Pha Phân rã (`Decay`, $T_w + T_s \le t \le T_{\text{total}}$)**: Giảm dần theo hàm cosin từ $\eta_{\text{base}}$ về $\eta_{\min}$ để tinh chỉnh hội tụ vào đáy cực tiểu địa phương.

---

### 3.5 Bộ nhớ đệm KV và Chú ý nhóm truy vấn (GQA)

Trong mô hình Transformer tự hồi quy, khi sinh token thứ $t$, vector truy vấn $q_t$ cần chấm tích vô hướng với tất cả các vector chìa khóa $k_0, \dots, k_t$ và tổng hợp thông tin từ các vector giá trị $v_0, \dots, v_t$.

Nếu không lưu đệm, tại mỗi bước ta phải tính lại toàn bộ $K, V$ từ vị trí $0$ đến $t-1$, dẫn đến tổng độ phức tạp thời gian cho chuỗi độ dài $N$ là $\sum_{t=1}^N O(t) = O(N^2)$.
**`KvCache`** lưu trữ các vector $K$ và $V$ của các bước trước đó. Ở mỗi bước sinh mới, ta chỉ cần:
1. Chiếu token mới thành một vector $q_{\text{new}}$, $k_{\text{new}}$, $v_{\text{new}}$.
2. Áp dụng nhúng vị trí quay RoPE cho $q_{\text{new}}$ và $k_{\text{new}}$ tại vị trí hiện tại `pos`.
3. Nối $k_{\text{new}}$ và $v_{\text{new}}$ vào đuôi bộ đệm `KvCache`.
4. Tính tích vô hướng giữa $q_{\text{new}}$ với toàn bộ lịch sử $K$ trong bộ đệm để sinh vector đầu ra của tầng Attention.

Độ phức tạp tính toán tại mỗi bước giảm xuống còn $O(t)$, tổng thời gian giải mã cho $N$ token trở thành tuyến tính $O(N)$ đối với mỗi token sinh ra.

---

### 3.6 PagedAttention: Quản lý phân trang bộ nhớ ảo kiểu vLLM

Mặc dù `KvCache` giải quyết triệt để bài toán tính toán, nó đẩy hệ thống vào bế tắc về bộ nhớ:
- Độ dài câu trả lời của người dùng là không biết trước (có thể 10 token hoặc 2048 token).
- Nếu cấp phát trước một mảng tĩnh kích thước tối đa cho mỗi kết nối, phần lớn bộ nhớ sẽ bị bỏ trống nhưng không tiến trình nào khác được dùng.
- Khi nhiều chuỗi kết thúc và chuỗi mới được tạo, bộ nhớ bị phân mảnh nghiêm trọng: tổng dung lượng trống thì đủ, nhưng không có một khối liên tục nào đủ dài.

**`PagedAttention`** mô phỏng hoàn hảo cơ chế phân trang ảo của hệ điều hành:
1. **Khối vật lý (`PhysicalBlock`)**: Bộ nhớ đệm KV được chia nhỏ thành các khối có kích thước cố định (ví dụ `block_size = 4` hoặc `16` token). Các khối này nằm rải rác trong bể bộ nhớ `PhysicalBlockPool`.
2. **Bảng khối logic (`PagedKvBlockTable`)**: Mỗi chuỗi yêu cầu chỉ lưu một danh sách các chỉ số khối vật lý `block_ids: Vec<usize>`. Khi chuỗi sinh thêm token, nếu khối hiện tại còn chỗ thì ghi tiếp; nếu đầy thì xin cấp phát một khối mới từ danh sách trống (`free_list`).
3. **Cơ chế Copy-on-Write (CoW)**:
   - Khi thực hiện tìm kiếm chùm (Beam Search) hoặc phân nhánh tác tử đa luồng, chuỗi con kế thừa toàn bộ lịch sử của chuỗi cha.
   - Thay vì sao chép toàn bộ bộ đệm KV (tốn megabytes bộ nhớ và độ trễ copy), hàm `fork_sequence` chỉ đơn giản sao chép danh sách `block_ids` và tăng biến đếm tham chiếu `ref_counts` của từng khối vật lý lên 1.
   - Khi một trong hai chuỗi sinh token mới và ghi vào khối cuối cùng: nếu khối đó đang có `ref_count > 1`, hệ thống mới thực sự cấp phát một khối riêng biệt, sao chép dữ liệu của khối đó sang, giảm `ref_count` của khối cũ, rồi mới ghi đè. **Chi phí sao chép giảm về 0 cho đến khi có sự phân kỳ dữ liệu thực sự.**

#### Bảng đối chiếu kiến trúc: Hệ điều hành (Virtual Memory) vs PagedAttention (vLLM)

| Thành phần trong Linux OS | Thành phần tương ứng trong PagedAttention | Bản chất chức năng |
|---|---|---|
| **Không gian địa chỉ ảo (Virtual Address Space)** | **Chuỗi token logic (Logical Token Sequence)** | Chuỗi token liên tục từ $0$ đến $T-1$ theo góc nhìn của giải thuật Attention |
| **Trang bộ nhớ ảo (Virtual Page)** | **Khối logic (Logical Block)** | Đơn vị gộp $B$ token liên tiếp trong chuỗi logic |
| **Khung trang vật lý (Page Frame)** | **Khối vật lý (`PhysicalBlock`)** | Vùng nhớ RAM thực sự chứa $B \times \text{kv\_dim}$ số thực |
| **Bảng trang (Page Table / MMU)** | **Bảng khối (`PagedKvBlockTable`)** | Ánh xạ từ chỉ số khối logic sang ID khối vật lý trong `PhysicalBlockPool` |
| **Lỗi trang (Page Fault / On-Demand Paging)** | **Cấp phát khối mới (`allocate_block`)** | Khi chuỗi sinh token vượt quá dung lượng khối hiện tại, tự động xin khối mới từ `free_list` |
| **Sao chép khi ghi (Copy-on-Write - fork())** | **Rẽ nhánh chuỗi (`fork_sequence`)** | Chia sẻ chung khối vật lý qua biến đếm `ref_counts`; chỉ nhân bản khi có thao tác ghi đè |
| **Giải phóng trang (Page Reclaim / free())** | **Thu hồi khối (`release`)** | Giảm `ref_counts`, đẩy ID khối về `free_list` khi không còn chuỗi nào tham chiếu |

#### Sơ đồ so sánh bố cục bộ nhớ: Phân bổ liên tục vs PagedAttention

```
CÁCH TRUYỀN THỐNG (Contiguous Allocation):
 VRAM: [ Seq 1: 10/2048 token ] [ Lãng phí 99% ] [ Seq 2: 200/2048 token ] [ Lãng phí 90% ]
  ===> Phân mảnh ngoài nghiêm trọng: Không thể nhồi thêm Seq 3 dù tổng dung lượng trống rất lớn!

PAGEDATTENTION (Block Paging):
 Physical Pool:
  [Block 0: Seq 1] [Block 1: Seq 2] [Block 2: Seq 1] [Block 3: TRỐNG] [Block 4: Seq 2]
 Bảng khối Seq 1: [ Block 0, Block 2 ]
 Bảng khối Seq 2: [ Block 1, Block 4 ]
  ===> 0% PHÂN MẢNH NGOÀI! Bất kỳ ô trống nào cũng có thể cấp ngay cho bất kỳ chuỗi nào!
```

#### Phân tích toán học: Phân mảnh trong (Internal Fragmentation)

Trong PagedAttention, hiện tượng phân mảnh ngoài bị triệt tiêu 100%, nhưng hệ thống vẫn còn một lượng **phân mảnh trong** ở khối cuối cùng của mỗi chuỗi.
Nếu kích thước khối là $B$ token (`block_size`):
- Một chuỗi có độ dài ngẫu nhiên $T$ sẽ cần $K = \lceil T / B \rceil$ khối.
- Khối cuối cùng chứa $T \pmod B$ token (nếu $T$ không chia hết cho $B$).
- Giả định độ dài câu $T$ phân phối đều theo modulo $B$, số lượng token bị lãng phí trung bình ở khối cuối là:
  $$\mathbb{E}[\text{lãng phí}] = \frac{B - 1}{2} \text{ token/chuỗi}$$
- **Đánh đổi kiến trúc**:
  - Nếu chọn $B = 4$: Lãng phí trung bình chỉ là $1.5$ token/chuỗi (rất tiết kiệm bộ nhớ), nhưng bảng khối phải lưu nhiều con trỏ, tăng phụ phí tra cứu bảng khối.
  - Nếu chọn $B = 16$ hoặc $B = 32$: Tối ưu hóa việc nạp dữ liệu theo khối lớn (coalesced memory access trên GPU/SIMD CPU), nhưng lãng phí trung bình tăng lên $7.5$ đến $15.5$ token/chuỗi.
  Trong thực tế sản xuất, giá trị $B = 16$ được coi là điểm cân bằng vàng giữa thông lượng phần cứng và hiệu quả sử dụng bộ nhớ.

---

### 3.7 RadixPrefixTree: Cây tiền tố LRU kiểu SGLang

Trong các ứng dụng tác tử lập trình hoặc hội thoại đa lượt (như đã xây dựng ở Chương 86 và Chương 91), các yêu cầu gửi đến mô hình thường xuyên lặp lại một đoạn mở đầu rất dài:
- Lời nhắc hệ thống (System Prompt) quy định vai trò, nguyên tắc an toàn, danh mục công cụ.
- Lịch sử các lượt trao đổi trước đó giữa người dùng và tác tử.

Nếu dùng `PagedAttention` đơn thuần, mỗi phiên làm việc mới vẫn phải tính toán lại KV-cache cho toàn bộ System Prompt này.
**`RadixPrefixTree`** giải quyết triệt để vấn đề này bằng cách cấu trúc hóa toàn bộ các chuỗi token trong bộ đệm thành một cây tiền tố Radix nén:

```
                      [ Root ]
                      /      \
            token [1, 2, 3]   token [9, 10]
             (Block #10)       (Block #20)
                /     \
    token [4, 5]       token [6, 7]
     (Block #11)        (Block #12)
       (Req A)            (Req B)
```

1. **Khớp tiền tố (`match_prefix`)**: Khi một yêu cầu mới đến với danh sách token $T$, cây Radix duyệt từ gốc để tìm nhánh khớp dài nhất. Toàn bộ các `block_id` đã được tính toán và lưu đệm trên đường đi này được tái sử dụng ngay lập tức! Mô hình chỉ cần tính prefill cho phần đuôi mới chưa từng xuất hiện.
2. **Tách cạnh tự động (Edge Splitting)**: Khi chèn một chuỗi token mới có phần đầu trùng với một cạnh hiện có nhưng rẽ nhánh ở giữa, nút cạnh sẽ tự động được chẻ đôi thành một nút trung gian (chứa phần chung) và hai nhánh con.
3. **Thu hồi theo thời gian truy cập gần nhất (LRU Eviction)**: Khi bể khối vật lý sắp hết dung lượng, cây Radix sẽ tìm kiếm các nút lá không bị ghim (`pinned == false`) có nhãn thời gian `last_access` cũ nhất để giải phóng, hoàn trả các khối vật lý về bể nhớ. Các prompt quan trọng được ghim (`pin_node`) sẽ được bảo vệ vĩnh viễn không bao giờ bị dọn dẹp.

#### Mối liên hệ hữu cơ với Chương 86: Kinh tế học KV-Cache và Bất biến CLM

Tại **Chương 86: Gương Ngữ Cảnh Sống (LiveContextMirror - CLM) & Kinh Tế Học Token**, chúng ta đã thiết lập một bất biến cốt lõi: **`PrefixStablePrompt`**.
Bây giờ, khi đứng ở tầng hạ tầng phục vụ suy luận của Chương 94, bạn có thể nhìn thấy lý do toán học và kinh tế học đằng sau bất biến đó:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│        SỰ KẾT HỢP GIỮA LIVE CONTEXT MIRROR (CH 86) VÀ RADIX TREE (CH 94)    │
│                                                                              │
│  YÊU CẦU 1 (Lượt 1):                                                         │
│  [ System Prompt ] [ Lịch sử Chat Lượt 1 ] [ Câu hỏi mới 1 ]                │
│  ├── Khớp Radix: ──┤ ── Khớp Radix: ─────┤ ── Prefill mới: ─┤               │
│  │  (Block 0..50)  │ │   (Block 51..80)   │ │  (Cấp Block 81)│               │
│                                                                              │
│  YÊU CẦU 2 (Lượt 2) - NẾU BẢO TOÀN THỨ TỰ (Prefix Stable):                  │
│  [ System Prompt ] [ Lịch sử Chat Lượt 1 ] [ Lời đáp 1 ] [ Câu hỏi mới 2 ]  │
│  ├── CACHE HIT 100%! ────────────────────┤ ── Prefill ngắn: ──────────────┤  │
│  │  Tái sử dụng ngay Block 0..81!         │ │  (Chỉ tính 20 token mới)     │  │
│  │  Thời gian TTFT: 15 mili-giây          │ │                              │  │
│                                                                              │
│  YÊU CẦU 2 (Lượt 2) - NẾU BỊ ĐẢO THỨ TỰ HOẶC TIÊM LỆNH VÀO GIỮA:             │
│  [ System Prompt ] [ CHÈN RÁC ] [ Lịch sử Chat Lượt 1 ] [ Câu hỏi mới 2 ]   │
│  ├── CACHE MISS! ────────────────────────────────────────────────────────┤  │
│  │  Toàn bộ cây Radix bị trượt (Cache Miss 100%) từ điểm chèn rác!          │  │
│  │  Phải tính toán lại toàn bộ KV-cache từ đầu! TTFT vọt lên 2.500 mili-giây│  │
│  │  Chi phí token API đội lên gấp 2.0x!                                      │  │
└──────────────────────────────────────────────────────────────────────────────┘
```

- **Quy tắc vàng của tác tử tự hành**:
  Mọi tác tử (Coding Agent) phải giữ cho phần đầu của ngữ cảnh **bất biến tuyệt đối**. Mọi thông tin mới (kết quả chạy lệnh, nội dung tệp vừa đọc) phải được nối thêm vào đuôi (`append-only`). Bất kỳ thao tác xáo trộn nào (như đảo vị trí tài liệu RAG giữa các lượt) sẽ biến cây Radix thành vô dụng, làm sụp đổ hoàn toàn thông lượng của hệ thống serving.

---

### 3.8 Bộ lấy mẫu Token (`TokenSampler` & Kỹ thuật `min-p`)

Từ các giá trị logit thô $z \in \mathbb{R}^V$ tại bước cuối cùng, bộ lấy mẫu quyết định token tiếp theo được chọn:

1. **Greedy Argmax**: Chọn token có logit lớn nhất: $t^* = \arg\max_i z_i$. Thích hợp cho các bài toán suy luận logic, toán học và lập trình cần tính tất định cao nhất.
2. **Nhiệt độ (`temperature`)**: Co dãn logit: $z_i' = z_i / T$. Nhiệt độ thấp ($T < 1.0$) làm phân phối trở nên nhọn hơn, tập trung vào các từ có xác suất cao; nhiệt độ cao ($T > 1.0$) làm phẳng phân phối, tăng tính sáng tạo nhưng dễ sinh ảo giác.
3. **Lọc Top-k**: Chỉ giữ lại $k$ token có xác suất cao nhất, gán xác suất các token còn lại bằng 0.
4. **Lọc Top-p (Nucleus Sampling)**: Sắp xếp xác suất giảm dần, giữ lại tập hợp nhỏ nhất các token có tổng xác suất tích lũy $\sum p_i \ge p_{\text{nucleus}}$.
5. **Kỹ thuật Min-p (Frontier 2024–2025)**:
   Điểm yếu của Top-p là khi mô hình rất tự tin (token đứng đầu có xác suất $95\%$), Top-p vẫn có thể giữ lại một vài token rác ở đuôi nếu ngưỡng là $0.99$. Ngược lại, khi mô hình phân vân giữa 50 lựa chọn ngang nhau, Top-k hay Top-p cố định có thể cắt cụt các lựa chọn hợp lệ.
   **Min-p** giải quyết triệt để điều này bằng cách đặt ngưỡng động theo token dẫn đầu:
   $$p_{\text{threshold}} = \text{min\_p} \times p_{\max}$$
   Mọi token có xác suất $p_i < p_{\text{threshold}}$ đều bị loại bỏ ngay lập tức trước khi chuẩn hóa lại. Khi mô hình tự tin ($p_{\max} \approx 0.99$), chỉ những token có xác suất cực cao mới sống sót; khi mô hình không chắc chắn ($p_{\max} \approx 0.05$), ngưỡng tự động hạ thấp cho phép mở rộng không gian tìm kiếm.

---

### 3.9 Giải mã suy đoán chính xác (`speculative_generate`)

Trong giải mã tự hồi quy thông thường, mỗi bước sinh 1 token đòi hỏi phải nạp toàn bộ hàng tỷ tham số của mô hình lớn từ VRAM sang chip xử lý (Memory Bandwidth Bound). Tốc độ tính toán của GPU/CPU bị lãng phí vì phải chờ đợi nạp dữ liệu.

**Giải mã suy đoán** (Leviathan et al., 2023; Chen et al., 2023) tận dụng một mô hình nháp nhỏ $M_q$ (ví dụ kích thước bằng 1/10 mô hình đích $M_p$):
1. **Pha sinh nháp**: Mô hình nhỏ tự hồi quy sinh nhanh $K$ token dự thảo: $\tilde{x}_1, \tilde{x}_2, \dots, \tilde{x}_K$ với phân phối xác suất dự thảo $q(\tilde{x}_i)$.
2. **Pha thẩm định song song**: Mô hình lớn nhận toàn bộ chuỗi $K$ token này và chạy đúng **một lượt forward duy nhất**, tính toán phân phối xác suất đích $p(\cdot)$ tại tất cả $K$ vị trí cùng lúc.
3. **Quy tắc chấp nhận không chệch (Unbiased Acceptance Criterion)**:
   Tại mỗi bước $i \in \{1, \dots, K\}$:
   - Bốc thăm ngẫu nhiên $u \sim U(0, 1)$.
   - Nếu $u < \min\left(1, \frac{p(\tilde{x}_i)}{q(\tilde{x}_i)}\right)$: **Chấp nhận token dự thảo $\tilde{x}_i$**!
   - Nếu bị từ chối: **Dừng lại ngay lập tức**. Bốc thăm token thay thế từ phân phối phần dư chuẩn hóa:
     $$p_{\text{residual}}(x) = \frac{\max(0, p(x) - q(x))}{\sum_{y} \max(0, p(y) - q(y))}$$
     Hủy bỏ toàn bộ các token dự thảo từ vị trí $i+1$ đến $K$.
4. **Token thưởng (Bonus Token)**: Nếu toàn bộ $K$ token dự thảo đều được chấp nhận, mô hình lớn sẽ hào phóng sinh thêm một token thứ $K+1$ hoàn toàn miễn phí từ phân phối tại vị trí cuối cùng!

**Bất biến toán học vĩ đại**:
Bất kể mô hình nháp $M_q$ có chất lượng ra sao, **phân phối xác suất của chuỗi token xuất xưởng luôn luôn trùng khít 100% với phân phối của mô hình lớn $M_p$**! Hoàn toàn không có hiện tượng suy giảm chất lượng văn bản, trong khi số lượng token sinh ra trên mỗi lần gọi mô hình lớn tăng lên từ $1.5\times$ đến $3.5\times$.

#### Chứng minh toán học: Định lý bảo toàn phân phối của Giải mã suy đoán

Rất nhiều kỹ sư mới tiếp cận giải mã suy đoán đều băn khoăn: *"Nếu mô hình nháp sinh sai nhiều thì văn bản đầu ra có bị sai lệch theo không?"*
Câu trả lời là **KHÔNG**. Dưới đây là chứng minh giải tích chặt chẽ:

Xét một bước sinh token tại một vị trí bất kỳ. Gọi $x$ là một token bất kỳ trong từ vựng $\mathcal{V}$.
Xác suất để token $x$ được xuất ra từ thuật toán giải mã suy đoán là tổng của hai biến cố xung khắc:
1. **Biến cố $A$**: $x$ được sinh ra từ mô hình nháp (xác suất $q(x)$) và **được chấp nhận** (xác suất $\min(1, p(x)/q(x))$):
   $$\mathbb{P}(A) = q(x) \cdot \min\left(1, \frac{p(x)}{q(x)}\right) = \min(q(x), p(x))$$
2. **Biến cố $B$**: Mô hình nháp sinh ra một token $\tilde{x}$ bất kỳ nhưng bị từ chối, và token thay thế được lấy mẫu từ phân phối phần dư $p_{\text{residual}}(x)$.
   - Tổng xác suất bị từ chối của mô hình nháp qua tất cả các token là:
     $$1 - \alpha = 1 - \sum_{y \in \mathcal{V}} \min(q(y), p(y)) = \sum_{y \in \mathcal{V}} (p(y) - \min(q(y), p(y))) = \sum_{y \in \mathcal{V}} \max(0, p(y) - q(y))$$
   - Khi bị từ chối, xác suất chọn token $x$ từ phân phối phần dư là:
     $$p_{\text{residual}}(x) = \frac{\max(0, p(x) - q(x))}{\sum_{y} \max(0, p(y) - q(y))} = \frac{\max(0, p(x) - q(x))}{1 - \alpha}$$
   - Do đó, xác suất của biến cố $B$ là tích của xác suất bị từ chối với xác suất lấy mẫu được $x$:
     $$\mathbb{P}(B) = (1 - \alpha) \cdot p_{\text{residual}}(x) = \max(0, p(x) - q(x))$$

Tổng hợp xác suất để token $x$ được xuất xưởng:
$$\mathbb{P}(\text{xuất } x) = \mathbb{P}(A) + \mathbb{P}(B) = \min(q(x), p(x)) + \max(0, p(x) - q(x))$$
Vì với hai số thực bất kỳ $a, b$, ta luôn có đẳng thức $\min(a, b) + \max(0, a - b) = a$ (hoặc với $p, q$: $\min(q, p) + \max(0, p - q) = p$):
- Nếu $p(x) \ge q(x)$: $\min(q(x), p(x)) = q(x)$ và $\max(0, p(x) - q(x)) = p(x) - q(x) \implies q(x) + p(x) - q(x) = p(x)$.
- Nếu $p(x) < q(x)$: $\min(q(x), p(x)) = p(x)$ và $\max(0, p(x) - q(x)) = 0 \implies p(x) + 0 = p(x)$.

Như vậy:
$$\mathbb{P}(\text{xuất } x) = p(x), \quad \forall x \in \mathcal{V}$$
$$\tag*{$\blacksquare$}$$

*Hệ quả*: Tỷ lệ chấp nhận $\alpha = \sum \min(p(x), q(x))$ càng cao thì tốc độ sinh càng lớn, nhưng phân phối của chuỗi đầu ra luôn luôn là phân phối thuần khiết của mô hình lớn $M_p$.

---

### 3.10 Lượng tử hóa số nguyên 8-bit đối xứng (`Int8Linear`)

Để đưa mô hình vào môi trường phục vụ thực tế, ta lượng tử hóa ma trận trọng số thực $W \in \mathbb{R}^{R \times C}$ sang số nguyên có dấu 8-bit $W_{q} \in [-127, 127]^{R \times C}$.

Ta áp dụng lượng tử hóa đối xứng theo từng hàng (Per-Channel Symmetric Quantization):
1. Tìm biên độ tuyệt đối lớn nhất của từng hàng $r \in \{0, \dots, R-1\}$:
   $$\alpha_r = \max_{c} |W_{r, c}|$$
2. Tính tỉ lệ co dãn của hàng:
   $$\text{scale}_r = \frac{\alpha_r}{127.0}$$
3. Lượng tử hóa và kẹp giá trị nguyên:
   $$W_{q}[r, c] = \text{clamp}\left(\text{round}\left(\frac{W[r, c]}{\text{scale}_r}\right), -127, 127\right)$$
4. Khi lan truyền xuôi với vector đầu vào thực $x \in \mathbb{R}^C$:
   $$y_r = \text{scale}_r \sum_{c=0}^{C-1} (W_{q}[r, c] \cdot x_c) + \text{bias}_r$$

Phép nhân tích vô hướng có thể được tính toán bằng các tập lệnh số nguyên siêu tốc (SIMD INT8), tiết kiệm 50% băng thông bộ nhớ trong khi độ tương đồng cosine giữa trọng số xấp xỉ và trọng số thực gốc luôn vượt ngưỡng $0,999$.

---

## 4. Mã nguồn minh họa thực chiến

Toàn bộ mã nguồn thực chiến đã được cài đặt và kiểm chứng hoàn chỉnh tại `code/ch94/src/main.rs`. Dưới đây là phân tích chi tiết các khối mã đại diện cốt lõi cùng các quyết định thiết kế tinh vi theo chuẩn Rust 2024.

---

### 4.1 Cấu trúc CausalDataLoader và CausalBatch

```rust
/// Cặp batch gồm chuỗi đầu vào và chuỗi mục tiêu tự hồi quy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CausalBatch {
    pub inputs: Vec<Vec<u32>>,
    pub targets: Vec<Vec<u32>>,
}

/// Bộ nạp dữ liệu nhân quả cho huấn luyện tự hồi quy.
#[derive(Debug, Clone)]
pub struct CausalDataLoader {
    pub tokens: Vec<u32>,
    pub batch_size: usize,
    pub seq_len: usize,
    pub current_idx: usize,
}

impl CausalDataLoader {
    pub fn new(tokens: Vec<u32>, batch_size: usize, seq_len: usize) -> Self {
        assert!(seq_len > 0, "seq_len phải lớn hơn 0");
        assert!(batch_size > 0, "batch_size phải lớn hơn 0");
        assert!(
            tokens.len() > seq_len,
            "tokens length phải lớn hơn seq_len để dịch chuyển mục tiêu"
        );
        Self {
            tokens,
            batch_size,
            seq_len,
            current_idx: 0,
        }
    }

    pub fn next_batch(&mut self) -> Option<CausalBatch> {
        let total_usable = self.tokens.len().saturating_sub(1);
        let tokens_per_batch = self.batch_size * self.seq_len;
        if self.current_idx + tokens_per_batch > total_usable {
            return None;
        }

        let mut inputs = Vec::with_capacity(self.batch_size);
        let mut targets = Vec::with_capacity(self.batch_size);

        for b in 0..self.batch_size {
            let start = self.current_idx + b * self.seq_len;
            let inp = self.tokens[start..start + self.seq_len].to_vec();
            let tgt = self.tokens[start + 1..start + self.seq_len + 1].to_vec();
            inputs.push(inp);
            targets.push(tgt);
        }

        self.current_idx += tokens_per_batch;
        Some(CausalBatch { inputs, targets })
    }
}
```

*Điểm nhấn kỹ thuật*:
- Thay vì trả về kiểu tuple phức tạp lồng nhau `(Vec<Vec<u32>>, Vec<Vec<u32>>)` dễ gây lỗi `clippy::type-complexity`, ta định nghĩa một struct `CausalBatch` tường minh với các trường có tên (`inputs`, `targets`).
- Việc tính `total_usable = tokens.len().saturating_sub(1)` đảm bảo token cuối cùng luôn có mục tiêu tương ứng tại vị trí $t+1$ mà không bao giờ gây lỗi vượt biên (`out-of-bounds index panic`).

---

### 4.2 Thuật toán trực giao hóa Newton–Schulz của Muon

```rust
/// Trực giao hóa ma trận Newton–Schulz 5 bước với hệ số xấp xỉ bậc 3:
/// `(3.4445, -4.7750, 2.0315)`.
pub fn muon_orthogonalize(grad: &[f64], rows: usize, cols: usize, steps: usize) -> Vec<f64> {
    assert_eq!(grad.len(), rows * cols);
    let frob = grad.iter().map(|&v| v * v).sum::<f64>().sqrt() + 1e-7;
    let norm: Vec<f64> = grad.iter().map(|&v| v / frob).collect();
    let (r, c, mut x) = if rows > cols {
        (cols, rows, transpose(&norm, rows, cols))
    } else {
        (rows, cols, norm)
    };

    for _ in 0..steps {
        // a = X * X^T (r x r)
        let xt = transpose(&x, r, c);
        let a = matmul(&x, &xt, r, c, r);
        // aa = a * a (r x r)
        let aa = matmul(&a, &a, r, r, r);
        // b = -4.7750 * a + 2.0315 * aa
        let b: Vec<f64> = a
            .iter()
            .zip(&aa)
            .map(|(&u, &v)| -4.7750 * u + 2.0315 * v)
            .collect();
        // bx = b * X (r x c)
        let bx = matmul(&b, &x, r, r, c);
        // X = 3.4445 * X + bx
        for (xv, &bxv) in x.iter_mut().zip(&bx) {
            *xv = 3.4445 * (*xv) + bxv;
        }
    }

    if rows > cols {
        transpose(&x, r, c)
    } else {
        x
    }
}
```

*Điểm nhấn kỹ thuật*:
- Phép kiểm tra `if rows > cols` tự động chuyển vị ma trận để phép nhân $X X^T$ luôn sinh ra ma trận vuông bậc nhỏ nhất $\min(R, C) \times \min(R, C)$. Nhờ đó, tính đối xứng cao/rộng được bảo toàn tuyệt đối: `muon_orthogonalize(M) == transpose(muon_orthogonalize(M^T))` với sai số nhỏ hơn $10^{-12}$.
- Phép tính nội vi ma trận sử dụng các mảng phẳng 1D liên tục, tối ưu hóa bộ nhớ đệm CPU L1/L2.

---

### 4.3 Định tuyến tối ưu hóa MuonOptimizer

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    Matrix2D { rows: usize, cols: usize },
    Vector1D { size: usize },
}

impl MuonOptimizer {
    pub fn step(
        &mut self,
        param_id: usize,
        kind: ParamKind,
        param: &mut [f64],
        grad: &[f64],
        lr_override: Option<f64>,
    ) {
        assert_eq!(param.len(), grad.len());

        match kind {
            ParamKind::Matrix2D { rows, cols } => {
                assert_eq!(param.len(), rows * cols);
                let lr = lr_override.unwrap_or(self.muon_config.lr);
                let mom = self.muon_config.momentum;
                let wd = self.muon_config.weight_decay;
                let ns_steps = self.muon_config.ns_steps;

                let m_buf = self
                    .muon_momentum
                    .entry(param_id)
                    .or_insert_with(|| vec![0.0; param.len()]);

                // Cập nhật động lượng: M = mom * M + grad
                for (mv, &gv) in m_buf.iter_mut().zip(grad) {
                    *mv = mom * (*mv) + gv;
                }

                // Trực giao hóa hướng cập nhật bằng Newton-Schulz 5 bước
                let ortho = muon_orthogonalize(m_buf, rows, cols, ns_steps);

                // Hệ số co dãn RMS đặc trưng của Muon: 0.2 * lr * sqrt(max(rows, cols))
                let rms_scale = 0.2 * lr * ((rows.max(cols)) as f64).sqrt();

                // Cập nhật tham số với phân rã trọng số tách rời
                for (p, &o) in param.iter_mut().zip(&ortho) {
                    *p = *p * (1.0 - lr * wd) - rms_scale * o;
                }
            }
            ParamKind::Vector1D { size } => {
                assert_eq!(param.len(), size);
                let opt = self
                    .adamw_optimizers
                    .entry(param_id)
                    .or_insert_with(|| AdamW::new(size, self.adamw_config.clone()));
                opt.update(param, grad, lr_override);
            }
        }
    }
}
```

*Điểm nhấn kỹ thuật*:
- Định tuyến nghiêm ngặt dựa trên kiểu phân loại `ParamKind`. Mỗi tham số được nhận diện bằng `param_id` duy nhất, duy trì bộ đệm động lượng riêng biệt trong `HashMap`.
- Tách rời hệ số suy giảm trọng số `* (1.0 - lr * wd)` tương thích hoàn hảo với nguyên lý của AdamW.

---

### 4.4 Phân trang bộ nhớ đệm KV Copy-on-Write (PagedKvBlockTable)

```rust
impl PagedKvBlockTable {
    pub fn append_kv(
        &mut self,
        pool: &mut PhysicalBlockPool,
        k: &[f64],
        v: &[f64],
    ) -> Result<(), String> {
        let need_new_block = if let Some(&last_id) = self.block_ids.last() {
            pool.blocks[last_id].num_tokens == self.block_size
        } else {
            true
        };

        if need_new_block {
            let new_id = pool.allocate_block()?;
            self.block_ids.push(new_id);
        }

        let last_idx = self.block_ids.len() - 1;
        let last_id = self.block_ids[last_idx];

        // Cơ chế Copy-on-Write: Nếu khối có nhiều hơn 1 tham chiếu, nhân bản riêng trước khi sửa
        if pool.ref_counts[last_id] > 1 {
            let fresh_id = pool.allocate_block()?;
            let existing_tokens = pool.blocks[last_id].num_tokens;
            let kv_dim = pool.kv_dim;

            // Tách biệt hoàn toàn việc đọc dữ liệu khối cũ và ghi vào khối mới
            let (src_keys, src_vals) = {
                let src = &pool.blocks[last_id];
                (
                    src.keys[..existing_tokens * kv_dim].to_vec(),
                    src.values[..existing_tokens * kv_dim].to_vec(),
                )
            };

            pool.blocks[fresh_id].keys[..existing_tokens * kv_dim].copy_from_slice(&src_keys);
            pool.blocks[fresh_id].values[..existing_tokens * kv_dim].copy_from_slice(&src_vals);
            pool.blocks[fresh_id].num_tokens = existing_tokens;

            pool.dec_ref(last_id);
            self.block_ids[last_idx] = fresh_id;
        }

        let curr_id = self.block_ids[last_idx];
        let token_pos = pool.blocks[curr_id].num_tokens;
        let kv_dim = pool.kv_dim;
        let offset = token_pos * kv_dim;

        pool.blocks[curr_id].keys[offset..offset + kv_dim].copy_from_slice(k);
        pool.blocks[curr_id].values[offset..offset + kv_dim].copy_from_slice(v);
        pool.blocks[curr_id].num_tokens += 1;
        self.total_tokens += 1;

        Ok(())
    }

    pub fn fork_sequence(&self, new_seq_id: usize, pool: &mut PhysicalBlockPool) -> Self {
        for &id in &self.block_ids {
            pool.inc_ref(id);
        }
        Self {
            sequence_id: new_seq_id,
            block_ids: self.block_ids.clone(),
            total_tokens: self.total_tokens,
            block_size: self.block_size,
        }
    }
}
```

*Điểm nhấn kỹ thuật*:
- Khi sao chép giữa hai khối trong cùng một `pool.blocks`, Rust Borrow Checker ngăn cấm mượn vừa khả biến (`&mut pool.blocks[fresh_id]`) vừa bất biến (`&pool.blocks[last_id]`). Ta giải quyết triệt để bằng một khối cục bộ `{ ... }` trích xuất nhanh dữ liệu của khối cũ sang bộ đệm trung gian trước khi gán sang khối mới, hoàn toàn an toàn và không gây xung đột mượn.
- Phép rẽ nhánh `fork_sequence` chỉ tăng `ref_counts` trên các `block_id`, đạt hiệu năng $O(B)$ với $B$ là số khối (thay vì $O(N \cdot D)$ khi sao chép toàn bộ vector KV).

---

### 4.5 Cây tiền tố RadixPrefixTree và Tách cạnh tự động

```rust
impl RadixPrefixTree {
    pub fn insert(&mut self, tokens: &[u32], block_ids: &[usize]) -> usize {
        self.access_clock += 1;
        let mut curr_id = self.root_id;
        let mut token_offset = 0;
        let mut block_offset = 0;

        self.nodes[curr_id].last_access = self.access_clock;

        while token_offset < tokens.len() {
            let next_tok = tokens[token_offset];
            if let Some(&child_id) = self.nodes[curr_id].children.get(&next_tok) {
                let edge_tokens = self.nodes[child_id].tokens.clone();
                let remaining = &tokens[token_offset..];
                let common = edge_tokens
                    .iter()
                    .zip(remaining)
                    .take_while(|&(a, b)| *a == *b)
                    .count();

                if common == edge_tokens.len() {
                    token_offset += common;
                    block_offset += self.nodes[child_id].cached_block_ids.len();
                    curr_id = child_id;
                    self.nodes[curr_id].last_access = self.access_clock;
                } else {
                    // Tách cạnh (Split Edge): tạo nút trung gian lưu phần chung
                    let intermediate_id = self.nodes.len();
                    let common_tokens = edge_tokens[..common].to_vec();
                    let child_remainder = edge_tokens[common..].to_vec();

                    let (common_blocks, child_blocks) = (
                        Vec::new(),
                        self.nodes[child_id].cached_block_ids.clone(),
                    );

                    let intermediate_node = RadixNode {
                        node_id: intermediate_id,
                        tokens: common_tokens,
                        cached_block_ids: common_blocks,
                        children: BTreeMap::new(),
                        parent_id: Some(curr_id),
                        last_access: self.access_clock,
                        pinned: false,
                    };
                    self.nodes.push(intermediate_node);

                    // Nối curr_id -> intermediate
                    self.nodes[curr_id].children.insert(next_tok, intermediate_id);

                    // Cập nhật child_id thành con của intermediate
                    let child_branch_tok = child_remainder[0];
                    self.nodes[child_id].tokens = child_remainder;
                    self.nodes[child_id].cached_block_ids = child_blocks;
                    self.nodes[child_id].parent_id = Some(intermediate_id);

                    self.nodes[intermediate_id]
                        .children
                        .insert(child_branch_tok, child_id);

                    token_offset += common;
                    curr_id = intermediate_id;
                    break;
                }
            } else {
                break;
            }
        }

        // Tạo nút lá cho phần đuôi còn lại nếu có
        if token_offset < tokens.len() {
            let leaf_id = self.nodes.len();
            let leaf_tokens = tokens[token_offset..].to_vec();
            let leaf_blocks = block_ids[block_offset..].to_vec();

            let leaf_node = RadixNode {
                node_id: leaf_id,
                tokens: leaf_tokens,
                cached_block_ids: leaf_blocks,
                children: BTreeMap::new(),
                parent_id: Some(curr_id),
                last_access: self.access_clock,
                pinned: false,
            };
            self.nodes.push(leaf_node);

            let branch_tok = tokens[token_offset];
            self.nodes[curr_id].children.insert(branch_tok, leaf_id);
            leaf_id
        } else {
            curr_id
        }
    }
}
```

*Điểm nhấn kỹ thuật*:
- Cú pháp Rust 2024: Trong `.take_while(|&(a, b)| *a == *b)`, việc giải tham chiếu tường minh tránh các cảnh báo pattern matching mơ hồ của trình biên dịch mới.
- Bố cục cây sử dụng `BTreeMap<u32, usize>` cho các liên kết con đảm bảo tính tất định khi duyệt và tìm kiếm nhánh rẽ theo token đầu tiên.

---

### 4.6 Giải mã suy đoán không chệch (speculative_decode_step)

```rust
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
    // 1. Mô hình nháp tự hồi quy sinh nhanh K token
    let mut draft_seq = current_tokens.to_vec();
    let mut draft_tokens = Vec::with_capacity(draft_len_k);
    let mut draft_dists = Vec::with_capacity(draft_len_k);

    for _ in 0..draft_len_k {
        let (tok, dist) = draft_fn(&draft_seq);
        draft_seq.push(tok);
        draft_tokens.push(tok);
        draft_dists.push(dist);
    }

    // 2. Mô hình đích thẩm định song song từng vị trí
    let mut eval_seq = current_tokens.to_vec();
    let mut accepted_tokens = Vec::new();
    let mut num_draft_accepted = 0;
    let mut rejected = false;

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

        let u = rng.next_f64();
        if u < accept_prob {
            accepted_tokens.push(draft_tok);
            eval_seq.push(draft_tok);
            num_draft_accepted += 1;
        } else {
            // Lấy mẫu từ phần dư phân phối chuẩn hóa: max(0, p - q)
            let mut residual: Vec<f64> = p_dist
                .iter()
                .zip(&draft_dists[i])
                .map(|(&p, &q)| (p - q).max(0.0))
                .collect();
            let res_sum: f64 = residual.iter().sum();
            let replacement_token = if res_sum > 1e-12 {
                for r in residual.iter_mut() {
                    *r /= res_sum;
                }
                sample_from_distribution(&residual, rng)
            } else {
                sample_from_distribution(&p_dist, rng)
            };

            accepted_tokens.push(replacement_token);
            eval_seq.push(replacement_token);
            rejected = true;
            break;
        }
    }

    let mut bonus_token_generated = false;
    if !rejected {
        let p_bonus = target_fn(&eval_seq);
        let bonus_tok = sample_from_distribution(&p_bonus, rng);
        accepted_tokens.push(bonus_tok);
        bonus_token_generated = true;
    }

    SpeculativeStepResult {
        accepted_tokens,
        draft_tokens,
        num_draft_accepted,
        total_emitted: accepted_tokens.len(),
        bonus_token_generated,
    }
}
```

*Điểm nhấn kỹ thuật*:
- Toàn bộ thuật toán lấy mẫu phần dư $\max(0, p - q)$ được tính toán bằng các phép toán vector ổn định số học. Nếu tổng phần dư tiệm cận 0 (do phân phối $q$ bao phủ hoàn toàn $p$), hàm tự động chuyển sang lấy mẫu trực tiếp từ $p_{\text{dist}}$ để chống lỗi chia cho 0 (`NaN`).

---

### 4.7 Lượng tử hóa Int8Linear theo kênh

```rust
impl Int8Linear {
    pub fn quantize(
        float_weights: &[f64],
        in_features: usize,
        out_features: usize,
        bias: Option<Vec<f64>>,
    ) -> Self {
        assert_eq!(float_weights.len(), in_features * out_features);
        let mut weight_int8 = vec![0i8; float_weights.len()];
        let mut scales = vec![0.0f64; out_features];

        for row in 0..out_features {
            let row_slice = &float_weights[row * in_features..(row + 1) * in_features];
            let max_abs = row_slice
                .iter()
                .map(|v| v.abs())
                .fold(0.0f64, f64::max);
            let scale = if max_abs > 1e-12 {
                max_abs / 127.0
            } else {
                1.0
            };
            scales[row] = scale;

            for col in 0..in_features {
                let val = float_weights[row * in_features + col];
                let q = (val / scale).round().clamp(-127.0, 127.0) as i8;
                weight_int8[row * in_features + col] = q;
            }
        }

        Self {
            in_features,
            out_features,
            weight_int8,
            scales,
            bias,
        }
    }

    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.in_features);
        let mut out = vec![0.0; self.out_features];

        for row in 0..self.out_features {
            let mut dot = 0.0;
            let offset = row * self.in_features;
            for (col, &x_val) in x.iter().enumerate() {
                let w = self.weight_int8[offset + col] as f64 * self.scales[row];
                dot += w * x_val;
            }
            if let Some(ref b) = self.bias {
                dot += b[row];
            }
            out[row] = dot;
        }

        out
    }
}
```

*Điểm nhấn kỹ thuật*:
- `x.iter().enumerate()` tuân thủ nghiêm ngặt quy tắc của Clippy (`clippy::needless_range_loop`), loại bỏ hoàn toàn kiểm tra biên chỉ mục thừa (`bound-checking`) khi duyệt qua mảng đầu vào.

---

### 4.8 Mô hình vi mô MicroCausalModel & Kiểm chứng Đạo hàm Giải tích vs Sai phân Hữu hạn

Để chứng minh toàn bộ các thành phần của Chương 94 hoạt động hoàn hảo và trung thực trong thực tế mà không cần phụ thuộc vào PyTorch hay Candle, chúng ta xây dựng mô hình `MicroCausalModel` tự chứa:

```rust
/// Mô hình Transformer nhân quả siêu nhỏ phục vụ kiểm chứng đạo hàm giải tích vs sai phân hữu hạn.
#[derive(Debug, Clone)]
pub struct MicroCausalModel {
    pub vocab_size: usize,
    pub d_model: usize,
    pub embed: Vec<f64>,
    pub w_q: Vec<f64>,
    pub w_k: Vec<f64>,
    pub w_v: Vec<f64>,
    pub w_o: Vec<f64>,
    pub lm_head: Vec<f64>,
}
```

#### Quy trình tính toán xuôi (Forward Pass):
1. **Tra cứu nhúng từ**:
   $$x_t = \text{embed}[\text{token}_t] \in \mathbb{R}^d$$
2. **Chiếu Query, Key, Value**:
   $$Q = X W_q, \quad K = X W_k, \quad V = X W_v$$
3. **Ma trận chú ý nhân quả (Causal Attention Mask)**:
   $$S_{i, j} = \begin{cases} \frac{Q_i \cdot K_j}{\sqrt{d}} & \text{nếu } j \le i \\ -\infty & \text{nếu } j > i \end{cases}$$
   $$P_{i, j} = \text{softmax}(S_{i, :})_j$$
   $$\text{Context}_i = \sum_{j \le i} P_{i, j} V_j$$
4. **Chiếu đầu ra và nối tắt dư (Residual Skip Connection)**:
   $$O = \text{Context} \cdot W_o, \quad H = X + O$$
5. **Dự đoán từ vựng và tổn thất Cross-Entropy**:
   $$\text{Logits} = H \cdot W_{\text{head}} \in \mathbb{R}^{T \times V}$$
   $$\mathcal{L} = -\frac{1}{T-1} \sum_{t=0}^{T-2} \ln \left( \frac{\exp(\text{Logits}_{t, y_t})}{\sum_v \exp(\text{Logits}_{t, v})} \right)$$

#### Quy trình lan truyền ngược giải tích (Analytical Backward Pass):
Đạo hàm qua hàm mất mát Cross-Entropy và Softmax:
$$\frac{\partial \mathcal{L}}{\partial \text{Logits}_{t, v}} = \frac{1}{T-1} \left( P(v \mid x_{\le t}) - \mathbb{I}(v == y_t) \right)$$
Từ đó, các ma trận gradient được giải tích tuần tự theo quy tắc dây chuyền (Chain Rule):
$$\begin{aligned}
\nabla W_{\text{head}} &= H^T \cdot \nabla \text{Logits} \\
\nabla H &= \nabla \text{Logits} \cdot W_{\text{head}}^T \\
\nabla W_o &= \text{Context}^T \cdot \nabla H, \quad \nabla \text{Context} = \nabla H \cdot W_o^T \\
\nabla V &= P^T \cdot \nabla \text{Context} \\
\nabla S_{i, j} &= \frac{P_{i, j}}{\sqrt{d}} \left( \nabla P_{i, j} - \sum_{k \le i} P_{i, k} \nabla P_{i, k} \right) \\
\nabla Q &= \nabla S \cdot K, \quad \nabla K = (\nabla S)^T \cdot Q \\
\nabla W_q &= X^T \cdot \nabla Q, \quad \nabla W_k = X^T \cdot \nabla K, \quad \nabla W_v = X^T \cdot \nabla V \\
\nabla X &= \nabla H + \nabla Q \cdot W_q^T + \nabla K \cdot W_k^T + \nabla V \cdot W_v^T
\end{aligned}$$

#### Kiểm chứng với Sai phân hữu hạn trung tâm (Central Finite Difference):
Với bất kỳ tham số vô hướng $\theta$ nào trong mô hình, ta kiểm chứng đạo hàm giải tích $g_{\text{analytic}}$ bằng công thức sai phân:
$$g_{\text{approx}} = \frac{\mathcal{L}(\theta + \epsilon) - \mathcal{L}(\theta - \epsilon)}{2\epsilon}, \quad \text{với } \epsilon = 10^{-6}$$
Sai số giữa hai phương pháp:
$$|g_{\text{analytic}} - g_{\text{approx}}| < 10^{-6}$$
Kiểm thử `test_micro_llm_analytical_grad_vs_finite_difference` đã chứng minh toán học của toàn bộ mạng giải tích chính xác tuyệt đối.

---

## 5. Bảng tra cứu lỗi biên dịch thường gặp

Khi xây dựng các hệ thống tối ưu hóa và phục vụ suy luận bằng Rust thuần, bạn sẽ thường xuyên đối mặt với các lỗi biên dịch đặc thù của hệ thống sở hữu bộ nhớ tĩnh và phiên bản Rust 2024. Bảng tra cứu dưới đây tổng hợp các lỗi kinh điển cùng giải pháp triệt để:

| Mã lỗi biên dịch | Nguyên nhân sâu xa trong bài toán | Cách khắc phục triệt chuẩn Rust 2024 |
|---|---|---|
| **`E0502: cannot borrow pool.blocks as immutable because it is also borrowed as mutable`** | Trong cơ chế Copy-on-Write của `PagedKvBlockTable`, khi sao chép dữ liệu từ khối cũ (`last_id`) sang khối mới (`fresh_id`), cả hai khối đều nằm trong cùng một `Vec<PhysicalBlock>`. Mượn `&mut pool.blocks[fresh_id]` đồng thời với `&pool.blocks[last_id]` vi phạm luật mượn độc quyền của Rust. | Sử dụng một khối scope cục bộ `{ let (k, v) = { let src = &pool.blocks[last_id]; (src.keys.to_vec(), src.values.to_vec()) }; ... }` để kết thúc lượt mượn bất biến trước khi thực hiện mượn khả biến, hoặc sử dụng phương thức `split_at_mut`. |
| **`error: cannot explicitly dereference within an implicitly-borrowing pattern`** | Trong Rust 2024 Edition, cú pháp closure lặp `.take_while(|(&a, &b)| a == b)` khi duyệt qua cặp tham chiếu `(&u32, &u32)` gây xung đột mẫu mượn ngầm định (binding modes). | Khắc phục bằng cách khớp trực tiếp trên tham chiếu toàn thể: `.take_while(|&(a, b)| *a == *b)`. |
| **`clippy::type-complexity: very complex type used`** | Định nghĩa hàm trả về tuple lồng nhau nhiều tầng như `Option<(Vec<Vec<u32>>, Vec<Vec<u32>>)>` làm mã nguồn khó đọc và dễ nhầm lẫn giữa inputs và targets. | Định nghĩa struct tường minh với các trường có tên (named fields): `struct CausalBatch { pub inputs: ..., pub targets: ... }`. |
| **`clippy::needless-range-loop: the loop variable is used to index x`** | Dùng chỉ số `for col in 0..in_features { ... x[col] ... }` khiến trình biên dịch phải chèn mã kiểm tra giới hạn biên tại mỗi vòng lặp runtime. | Chuyển sang sử dụng iterator: `for (col, &x_val) in x.iter().enumerate()`. |
| **`E0499: cannot borrow rng as mutable more than once at a time`** | Khi kiểm thử giải mã suy đoán, closure mô phỏng mô hình nháp mượn khả biến `&mut rng` bên trong thân hàm, trong khi hàm giải mã chính cũng nhận `&mut rng` làm đối số thẩm định. | Tách rời trạng thái ngẫu nhiên: sử dụng hai bộ sinh số ngẫu nhiên độc lập `draft_rng` và `verifier_rng` (phản ánh đúng bản chất hai tiến trình phân tán độc lập trong thực tế). |
| **`clippy::useless-vec: useless use of vec!`** | Khởi tạo mảng đếm tạm thời trong test bằng `let mut counts = vec![0usize; 3];` gây cấp phát vùng nhớ heap không cần thiết khi kích thước đã cố định. | Thay thế bằng mảng tĩnh trên stack: `let mut counts = [0usize; 3];`. |
| **`Mất tính trực giao / Bùng nổ Gram Matrix trong Muon`** | Khởi tạo ma trận Newton–Schulz mà không chia chuẩn Frobenius $\|M\|_F$, hoặc hệ số lặp đa thức bậc 3 bị sai dấu làm các trị riêng văng ra ngoài bán kính hội tụ. | Bắt buộc chuẩn hóa $X_0 = M / (\|M\|_F + 10^{-7})$ và sử dụng chính xác bộ ba hệ số $(3.4445, -4.7750, 2.0315)$; kiểm tra đường chéo Gram nằm trong $[0.45, 1.35]$. |
| **`Tràn số 0/0 (NaN) trong Speculative Residual Sampling`** | Khi phân phối của mô hình nháp $q$ bao phủ hoàn toàn phân phối đích $p$, hiệu số $\max(0, p - q)$ bằng 0 trên toàn bộ từ vựng, dẫn đến mẫu số $\sum \text{residual} = 0$. | Bổ sung chốt bảo vệ `if res_sum > 1e-12 { ... } else { sample_from_distribution(&p_dist, rng) }`. |

---

### Nghiên cứu tình huống 1: Lỗi mượn kép trong cơ chế Copy-on-Write (E0502)

#### Mã nguồn gây lỗi:
```rust
// CỐ TÌNH LỖI: Mượn vừa khả biến vừa bất biến trên cùng một Vec<PhysicalBlock>
let fresh_id = pool.allocate_block().unwrap();
let existing_tokens = pool.blocks[last_id].num_tokens;
let kv_dim = pool.kv_dim;

pool.blocks[fresh_id].keys[..existing_tokens * kv_dim]
    .copy_from_slice(&pool.blocks[last_id].keys[..existing_tokens * kv_dim]);
//                   ^^^^^^^^^^^ LỖI E0502: cannot borrow pool.blocks as immutable 
//                                     because it is also borrowed as mutable
```

#### Thông điệp của trình biên dịch `rustc`:
```text
error[E0502]: cannot borrow `pool.blocks` as immutable because it is also borrowed as mutable
   --> src/main.rs:763:35
    |
762 |   pool.blocks[fresh_id].keys[..existing_tokens * kv_dim]
    |   ----------- mutable borrow occurs here
763 |       .copy_from_slice(&pool.blocks[last_id].keys[..existing_tokens * kv_dim]);
    |        ---------------  ^^^^^^^^^^^ immutable borrow occurs here
    |        |
    |        mutable borrow later used by call
```

#### Bản chất và Cách khắc phục chuẩn:
Rust Borrow Checker không thể biết được tại thời điểm biên dịch rằng `fresh_id != last_id`. Nó chỉ thấy toàn bộ mảng `pool.blocks` đang bị mượn khả biến ở vế trái (`pool.blocks[fresh_id]`). Để khắc phục, ta sử dụng một khối phạm vi cục bộ `{ ... }` để sao chép tạm dữ liệu nguồn sang vector ngắn hạn trên stack/heap, kết thúc hoàn toàn lượt mượn bất biến trước khi ghi vào khối đích:
```rust
// GIẢI PHÁP CHUẨN RUST:
let (src_keys, src_vals) = {
    let src = &pool.blocks[last_id];
    (
        src.keys[..existing_tokens * kv_dim].to_vec(),
        src.values[..existing_tokens * kv_dim].to_vec(),
    )
};

pool.blocks[fresh_id].keys[..existing_tokens * kv_dim].copy_from_slice(&src_keys);
pool.blocks[fresh_id].values[..existing_tokens * kv_dim].copy_from_slice(&src_vals);
```

---

### Nghiên cứu tình huống 2: Xung đột mẫu tham chiếu ngầm định trong Rust 2024 Edition

#### Mã nguồn gây lỗi:
```rust
// CỐ TÌNH LỖI trong Rust 2024:
let common = edge_tokens
    .iter()
    .zip(remaining)
    .take_while(|(&a, &b)| a == b) // LỖI: cannot explicitly dereference within an implicitly-borrowing pattern
    .count();
```

#### Thông điệp của trình biên dịch `rustc`:
```text
error: cannot explicitly dereference within an implicitly-borrowing pattern
   --> src/main.rs:893:35
    |
893 |   .take_while(|(&a, &b)| a == b)
    |                 ^   ^ reference pattern not allowed when implicitly borrowing
```

#### Bản chất và Cách khắc phục chuẩn:
Trong Rust 2024, các quy tắc khớp mẫu mượn ngầm định (binding modes) được siết chặt hơn để tránh sự mơ hồ giữa việc khớp theo giá trị hay theo tham chiếu. Khi phương thức `take_while` nhận một tham chiếu đến phần tử của iterator (ở đây phần tử đã là tuple `(&u32, &u32)`), đối số truyền vào closure có kiểu `&(&u32, &u32)`. Việc viết `(&a, &b)` bên trong cố tình bóc một tầng tham chiếu ngầm định mà không khớp tầng tham chiếu ngoài cùng.
Giải pháp chuẩn là khớp tường minh tầng ngoài cùng bằng `|&(a, b)|` và giải tham chiếu bằng toán tử `*`:
```rust
// GIẢI PHÁP CHUẨN RUST 2024:
let common = edge_tokens
    .iter()
    .zip(remaining)
    .take_while(|&(a, b)| *a == *b)
    .count();
```

---

### Nghiên cứu tình huống 3: Mượn khả biến trùng lặp PRNG trong kiểm thử giải mã suy đoán (E0499)

#### Mã nguồn gây lỗi:
```rust
// CỐ TÌNH LỖI:
let mut rng = SplitMix64::new(42);
let res = speculative_decode_step(
    &[],
    1,
    |_seq| {
        (sample_from_distribution(&draft_dist, &mut rng), draft_dist.clone())
        //                                     ^^^^^^^^ Lượt mượn 1 trong closure
    },
    |_seq| target_dist.clone(),
    &mut rng, // <--- Lượt mượn 2 trong đối số hàm ngoài: E0499!
);
```

#### Thông điệp của trình biên dịch `rustc`:
```text
error[E0499]: cannot borrow `rng` as mutable more than once at a time
    --> src/main.rs:2135:17
     |
2128 |   |_seq| {
     |   ------ first mutable borrow occurs here
2130 |       sample_from_distribution(&draft_dist, &mut rng),
     |                                                  --- first borrow occurs due to use of `rng` in closure
2135 |   &mut rng,
     |   ^^^^^^^^ second mutable borrow occurs here
```

#### Bản chất và Cách khắc phục chuẩn:
Closure `draft_fn` là một closure khả biến (`FnMut`), nó bắt giữ tham chiếu khả biến `&mut rng`. Trong khi đó, hàm `speculative_decode_step` lại yêu cầu truyền thêm một tham chiếu `&mut rng` cho quá trình thẩm định của mô hình lớn. Rust ngăn cấm hai tham chiếu khả biến tồn tại cùng lúc tới cùng một đối tượng.
Trong kiến trúc hệ thống thực tế, mô hình nháp và mô hình lớn là hai thực thể độc lập (thậm chí chạy trên hai thiết bị hoặc hai tiến trình khác nhau), mỗi bên có bộ sinh số ngẫu nhiên riêng. Việc phân tách thành `draft_rng` và `verifier_rng` vừa triệt tiêu lỗi biên dịch vừa mô phỏng chính xác kiến trúc phân tán:
```rust
// GIẢI PHÁP CHUẨN RUST:
let mut draft_rng = SplitMix64::new(555);
let mut verifier_rng = SplitMix64::new(999);

let res = speculative_decode_step(
    &[],
    1,
    |_seq| {
        (sample_from_distribution(&draft_dist, &mut draft_rng), draft_dist.clone())
    },
    |_seq| target_dist.clone(),
    &mut verifier_rng,
);
```

---

## 6. Tóm tắt & Bài tập rèn luyện

### Tóm tắt cốt lõi

1. **Bộ tối ưu hóa hiện đại**:
   - `AdamW` giải quyết sự méo mó của suy giảm trọng số bằng cách tách rời $L_2$ decay khỏi bước cập nhật gradient thích nghi.
   - `Muon` đưa đại số tuyến tính ma trận vào trọng tâm của quá trình tối ưu: trực giao hóa Newton–Schulz 5 bước ép ma trận cập nhật về mặt cầu đẳng cự, triệt tiêu sự mất cân bằng giữa các trị kỳ dị.
   - Định tuyến chặt chẽ: Ma trận 2D $\to$ Muon; Vector 1D $\to$ AdamW.
2. **Lịch học WSD**:
   - Thay thế đường cong dốc liên tục của Cosine bằng ba pha: Warmup $\to$ Stable $\to$ Decay.
   - Cho phép mô hình duy trì tốc độ hấp thụ tri thức tối đa xuyên suốt phần lớn vòng đời và linh hoạt đóng gói bất kỳ lúc nào.
3. **Phục vụ suy luận hiệu năng cao**:
   - `PagedAttention` loại bỏ phân mảnh bộ nhớ ngoài của KV-cache, cung cấp cơ chế Copy-on-Write miễn phí cho các tác tử rẽ nhánh.
   - `RadixPrefixTree` biến việc xử lý các prompt hệ thống lặp lại thành con số 0 qua cấu trúc cây tiền tố LRU.
   - `TokenSampler` với kỹ thuật `min-p` tự động thanh lọc các token rác dựa trên độ tin cậy của token dẫn đầu.
   - `speculative_generate` tăng tốc độ sinh từ 1.5 đến 3 lần mà không làm suy giảm dù chỉ một phần triệu chất lượng văn bản.
   - `Int8Linear` nén một nửa dung lượng lưu trữ với độ tương đồng cosine $> 0,999$.

---

### Bài tập rèn luyện

Dưới đây là 3 bài tập nâng cao giúp bạn làm chủ toàn diện các cơ chế này. Mỗi bài tập đều đi kèm lời giải bằng mã nguồn Rust 2024 hoàn chỉnh, biên dịch sạch không phụ thuộc thư viện ngoài.

---

#### Bài tập 1: Bộ điều chỉnh nhiệt độ thích nghi theo Entropy (Entropy-Adaptive Temperature Sampler)

**Đề bài**:
Trong các tác tử suy luận phức tạp, việc cố định nhiệt độ `temperature` là không tối ưu. Khi mô hình có độ bất định cao (entropy của phân phối logit lớn), ta muốn nhiệt độ tự động hạ thấp để ép mô hình suy luận chặt chẽ; khi mô hình tự tin (entropy thấp), ta muốn giữ nguyên nhiệt độ để câu trả lời tự nhiên.
Hãy cài đặt hàm `adaptive_entropy_sample(logits: &[f64], base_temp: f64, max_entropy: f64, rng: &mut SplitMix64) -> usize`.

<details>
<summary>Gợi ý & Lời giải bài tập 1</summary>

```rust
/// Bộ sinh số SplitMix64 tối giản phục vụ bài tập
pub struct SimpleRng(u64);
impl SimpleRng {
    pub fn new(seed: u64) -> Self { Self(seed) }
    pub fn next_f64(&mut self) -> f64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        ((z ^ (z >> 31)) >> 11) as f64 / ((1u64 << 53) as f64)
    }
}

/// Lấy mẫu thích nghi theo Entropy Shannon của phân phối
pub fn adaptive_entropy_sample(
    logits: &[f64],
    base_temp: f64,
    max_entropy_ref: f64,
    rng: &mut SimpleRng,
) -> usize {
    assert!(!logits.is_empty());
    // 1. Tính Softmax chuẩn ở nhiệt độ 1.0 để đo Entropy
    let max_l = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut probs: Vec<f64> = logits.iter().map(|&l| (l - max_l).exp()).collect();
    let sum_p: f64 = probs.iter().sum();
    for p in probs.iter_mut() {
        *p /= sum_p;
    }

    // 2. Tính Entropy Shannon: H = -sum(p * ln(p))
    let mut entropy = 0.0;
    for &p in &probs {
        if p > 1e-12 {
            entropy -= p * p.ln();
        }
    }

    // 3. Điều chỉnh nhiệt độ: Nếu entropy cao hơn tham chiếu, hạ thấp nhiệt độ
    let ratio = (entropy / max_entropy_ref.max(1e-6)).clamp(0.5, 2.0);
    let effective_temp = (base_temp / ratio).max(0.01);

    // 4. Lấy mẫu với nhiệt độ hiệu dụng
    let scaled_logits: Vec<f64> = logits.iter().map(|&l| l / effective_temp).collect();
    let max_scaled = scaled_logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut final_probs: Vec<f64> = scaled_logits.iter().map(|&l| (l - max_scaled).exp()).collect();
    let final_sum: f64 = final_probs.iter().sum();
    for p in final_probs.iter_mut() {
        *p /= final_sum;
    }

    let r = rng.next_f64();
    let mut accum = 0.0;
    for (i, &p) in final_probs.iter().enumerate() {
        accum += p;
        if r <= accum {
            return i;
        }
    }
    final_probs.len() - 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_adaptive_sampling() {
        let mut rng = SimpleRng::new(42);
        // Phân phối tự tin cao: [10.0, 0.0, 0.0] -> entropy thấp
        let confident_logits = vec![10.0, 0.0, 0.0];
        let tok = adaptive_entropy_sample(&confident_logits, 0.8, 1.0, &mut rng);
        assert_eq!(tok, 0);

        // Phân phối phân vân: [2.0, 2.0, 2.0] -> entropy cao
        let uncertain_logits = vec![2.0, 2.0, 2.0];
        let tok_u = adaptive_entropy_sample(&uncertain_logits, 0.8, 0.5, &mut rng);
        assert!(tok_u < 3);
    }
}
```

</details>

---

#### Bài tập 2: Cơ chế thu hồi bộ nhớ PagedAttention theo ngưỡng khẩn cấp (Emergency Memory Compactor)

**Đề bài**:
Trong các hệ thống phục vụ chịu tải cao, khi số khối trống trong bể `PhysicalBlockPool` giảm xuống dưới ngưỡng an toàn (ví dụ còn dưới 10%), hệ thống không thể nhận thêm yêu cầu mới. Hãy xây dựng cấu trúc `ServingMemoryManager` quản lý đồng thời cả `PhysicalBlockPool` và `RadixPrefixTree`, cung cấp phương thức `ensure_free_capacity(&mut self, required_blocks: usize) -> Result<usize, String>` tự động dọn dẹp các nút lá cũ nhất không bị ghim trong cây Radix cho đến khi đạt đủ số khối yêu cầu.

<details>
<summary>Gợi ý & Lời giải bài tập 2</summary>

```rust
use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct MiniBlockPool {
    pub free_count: usize,
    pub ref_counts: Vec<usize>,
}

impl MiniBlockPool {
    pub fn new(capacity: usize) -> Self {
        Self {
            free_count: capacity,
            ref_counts: vec![0; capacity],
        }
    }
    pub fn release_block(&mut self, id: usize) {
        if self.ref_counts[id] > 0 {
            self.ref_counts[id] -= 1;
            if self.ref_counts[id] == 0 {
                self.free_count += 1;
            }
        }
    }
}

pub struct MiniRadixNode {
    pub id: usize,
    pub cached_blocks: Vec<usize>,
    pub children: BTreeMap<u32, usize>,
    pub last_access: u64,
    pub pinned: bool,
}

pub struct ServingMemoryManager {
    pub pool: MiniBlockPool,
    pub nodes: Vec<MiniRadixNode>,
}

impl ServingMemoryManager {
    pub fn new(pool_capacity: usize) -> Self {
        Self {
            pool: MiniBlockPool::new(pool_capacity),
            nodes: Vec::new(),
        }
    }

    /// Thu hồi các khối bộ nhớ từ cây Radix cho đến khi số khối trống >= required_blocks
    pub fn ensure_free_capacity(&mut self, required_blocks: usize) -> Result<usize, String> {
        let mut total_freed = 0;

        while self.pool.free_count < required_blocks {
            // Tìm nút lá không bị ghim có last_access nhỏ nhất
            let mut oldest_leaf: Option<(usize, u64)> = None;
            for node in &self.nodes {
                if node.children.is_empty() && !node.pinned && !node.cached_blocks.is_empty() {
                    match oldest_leaf {
                        None => oldest_leaf = Some((node.id, node.last_access)),
                        Some((_, best_time)) if node.last_access < best_time => {
                            oldest_leaf = Some((node.id, node.last_access));
                        }
                        _ => {}
                    }
                }
            }

            if let Some((target_id, _)) = oldest_leaf {
                let blocks = std::mem::take(&mut self.nodes[target_id].cached_blocks);
                let num_blocks = blocks.len();
                for b in blocks {
                    self.pool.release_block(b);
                }
                total_freed += num_blocks;
            } else {
                // Không còn nút lá nào có thể dọn dẹp
                return Err("Không thể thu hồi đủ bộ nhớ: toàn bộ nút đều bị ghim hoặc rỗng".to_string());
            }
        }

        Ok(total_freed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_compaction() {
        let mut mgr = ServingMemoryManager::new(10);
        mgr.pool.free_count = 1; // Khẩn cấp: chỉ còn 1 khối trống
        mgr.pool.ref_counts[0] = 1;
        mgr.pool.ref_counts[1] = 1;

        mgr.nodes.push(MiniRadixNode {
            id: 0,
            cached_blocks: vec![0, 1],
            children: BTreeMap::new(),
            last_access: 100,
            pinned: false,
        });

        // Yêu cầu tối thiểu 3 khối trống
        let freed = mgr.ensure_free_capacity(3).unwrap();
        assert_eq!(freed, 2);
        assert_eq!(mgr.pool.free_count, 3);
    }
}
```

</details>

---

#### Bài tập 3: Bộ giải mã suy đoán đa tầng (Tree-based Speculative Decoding Verifier)

**Đề bài**:
Trong giải mã suy đoán hiện đại (như Medusa hoặc SpecInfer), mô hình nháp không chỉ đề xuất một chuỗi tuyến tính duy nhất mà đề xuất một **cây phân nhánh các ứng viên** (ví dụ: top 2 token tại bước 1, mỗi nhánh rẽ tiếp 2 token tại bước 2).
Hãy cài đặt hàm `verify_speculative_branch(candidates: &[Vec<usize>], target_fn: impl Fn(&[usize]) -> Vec<f64>) -> Vec<usize>` nhận vào danh sách các nhánh ứng viên và chọn ra nhánh dài nhất được chấp nhận theo tiêu chuẩn tham lam xác suất.

<details>
<summary>Gợi ý & Lời giải bài tập 3</summary>

```rust
/// Thẩm định và chọn nhánh giải mã suy đoán tốt nhất
pub fn verify_speculative_branch<F>(
    prefix: &[usize],
    candidates: &[Vec<usize>],
    target_fn: F,
) -> Vec<usize>
where
    F: Fn(&[usize]) -> Vec<f64>,
{
    let mut best_branch = Vec::new();

    for branch in candidates {
        let mut current = prefix.to_vec();
        let mut accepted_in_branch = Vec::new();

        for &token in branch {
            let dist = target_fn(&current);
            // Tìm token có xác suất cao nhất của mô hình đích
            let mut argmax_token = 0;
            let mut max_val = f64::NEG_INFINITY;
            for (idx, &p) in dist.iter().enumerate() {
                if p > max_val {
                    max_val = p;
                    argmax_token = idx;
                }
            }

            if argmax_token == token {
                // Khớp chính xác token tối ưu của mô hình đích
                accepted_in_branch.push(token);
                current.push(token);
            } else {
                // Nhánh này bị gãy tại vị trí token này, lấy token sửa sai của mô hình đích
                accepted_in_branch.push(argmax_token);
                break;
            }
        }

        if accepted_in_branch.len() > best_branch.len() {
            best_branch = accepted_in_branch;
        }
    }

    best_branch
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_speculative_verification() {
        let prefix = vec![1, 2];
        let branch_a = vec![10, 20, 30];
        let branch_b = vec![10, 25, 40];

        // Mô hình đích mong muốn: sau [1, 2] là 10; sau [1, 2, 10] là 20; sau [1, 2, 10, 20] là 99
        let target = |seq: &[usize]| -> Vec<f64> {
            let mut dist = vec![0.0; 100];
            if seq == [1, 2] {
                dist[10] = 1.0;
            } else if seq == [1, 2, 10] {
                dist[20] = 1.0;
            } else if seq == [1, 2, 10, 20] {
                dist[99] = 1.0;
            } else {
                dist[0] = 1.0;
            }
            dist
        };

        let result = verify_speculative_branch(&prefix, &[branch_a, branch_b], target);
        // Nhánh A khớp 10, 20 và sửa 30 thành 99 -> độ dài 3
        assert_eq!(result, vec![10, 20, 99]);
    }
}
```

</details>

---

#### Bài tập 4: Bộ tối ưu hóa Muon với Động lượng Nesterov (Nesterov Accelerated Muon)

**Đề bài**:
Kỹ thuật động lượng Nesterov (Nesterov Accelerated Gradient - NAG) thường giúp tăng tốc độ hội tụ và giảm dao động quán tính tại các khúc ngoặt của hàm mất mát so với động lượng tiêu chuẩn. Trong phiên bản kết hợp Muon–Nesterov:
1. Cập nhật vận tốc:
   $$V_t = \beta V_{t-1} + G_t$$
2. Hướng cập nhật nhìn trước (Look-ahead direction):
   $$M_t = G_t + \beta V_t$$
3. Áp dụng trực giao hóa Newton–Schulz 5 bước lên ma trận $M_t$.
4. Cập nhật tham số với hệ số RMS và suy giảm trọng số tách rời.

Hãy cài đặt struct `NesterovMuon` và kiểm chứng việc cập nhật ma trận tham số.

<details>
<summary>Gợi ý & Lời giải bài tập 4</summary>

```rust
/// Trực giao hóa Newton-Schulz 5 bước tối giản cho bài tập
fn simple_ns5(mat: &[f64], rows: usize, cols: usize) -> Vec<f64> {
    let frob = mat.iter().map(|&v| v * v).sum::<f64>().sqrt() + 1e-7;
    let mut x: Vec<f64> = mat.iter().map(|&v| v / frob).collect();
    
    // Giả sử rows <= cols để minh họa trực quan
    for _ in 0..5 {
        // a = X * X^T (rows x rows)
        let mut a = vec![0.0; rows * rows];
        for i in 0..rows {
            for j in 0..rows {
                let dot: f64 = (0..cols).map(|c| x[i * cols + c] * x[j * cols + c]).sum();
                a[i * rows + j] = dot;
            }
        }
        // aa = a * a
        let mut aa = vec![0.0; rows * rows];
        for i in 0..rows {
            for j in 0..rows {
                let dot: f64 = (0..rows).map(|k| a[i * rows + k] * a[k * rows + j]).sum();
                aa[i * rows + j] = dot;
            }
        }
        // B = -4.7750 * a + 2.0315 * aa
        let mut b = vec![0.0; rows * rows];
        for k in 0..rows * rows {
            b[k] = -4.7750 * a[k] + 2.0315 * aa[k];
        }
        // X = 3.4445 * X + B * X
        let mut new_x = vec![0.0; rows * cols];
        for i in 0..rows {
            for c in 0..cols {
                let bx: f64 = (0..rows).map(|k| b[i * rows + k] * x[k * cols + c]).sum();
                new_x[i * cols + c] = 3.4445 * x[i * cols + c] + bx;
            }
        }
        x = new_x;
    }
    x
}

/// Bộ tối ưu hóa Muon với động lượng Nesterov
pub struct NesterovMuon {
    pub lr: f64,
    pub momentum: f64,
    pub weight_decay: f64,
    pub velocity: Vec<f64>,
}

impl NesterovMuon {
    pub fn new(len: usize, lr: f64, momentum: f64, weight_decay: f64) -> Self {
        Self {
            lr,
            momentum,
            weight_decay,
            velocity: vec![0.0; len],
        }
    }

    pub fn step(&mut self, param: &mut [f64], grad: &[f64], rows: usize, cols: usize) {
        assert_eq!(param.len(), rows * cols);
        assert_eq!(grad.len(), rows * cols);

        let mut nesterov_dir = vec![0.0; param.len()];
        for i in 0..param.len() {
            // V_t = beta * V_{t-1} + G_t
            self.velocity[i] = self.momentum * self.velocity[i] + grad[i];
            // M_t = G_t + beta * V_t (Nesterov Look-ahead)
            nesterov_dir[i] = grad[i] + self.momentum * self.velocity[i];
        }

        let ortho = simple_ns5(&nesterov_dir, rows, cols);
        let rms_scale = 0.2 * self.lr * ((rows.max(cols)) as f64).sqrt();

        for i in 0..param.len() {
            param[i] = param[i] * (1.0 - self.lr * self.weight_decay) - rms_scale * ortho[i];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nesterov_muon_step() {
        let mut opt = NesterovMuon::new(16, 0.05, 0.9, 0.01);
        let mut param = vec![1.0; 16]; // 4 x 4
        let grad = vec![0.25; 16];

        opt.step(&mut param, &grad, 4, 4);
        assert!(param.iter().all(|&v| v < 1.0));
        assert!(opt.velocity.iter().all(|&v| v > 0.0));
    }
}
```

</details>

---

#### Bài tập 5: Lượng tử hóa Int4 theo nhóm và đóng gói Byte (Grouped Int4 Linear with Nibble Packing)

**Đề bài**:
Để nén mô hình phục vụ trên các thiết bị nhúng và tăng gấp 4 lần băng thông bộ nhớ, người ta lượng tử hóa trọng số xuống số nguyên 4-bit có dấu $[-8, 7]$. Mỗi byte `u8` sẽ đóng gói hai giá trị 4-bit (nibble thấp và nibble cao). Mỗi nhóm `group_size = 32` phần tử liên tiếp được chuẩn hóa bởi một hệ số `scale` độc lập.
Hãy cài đặt struct `GroupedInt4Linear` với các phương thức `quantize` và `forward`.

<details>
<summary>Gợi ý & Lời giải bài tập 5</summary>

```rust
/// Tầng tuyến tính lượng tử hóa Int4 theo nhóm với đóng gói 2 số/byte
pub struct GroupedInt4Linear {
    pub in_features: usize,
    pub out_features: usize,
    pub group_size: usize,
    pub packed_weights: Vec<u8>,
    pub scales: Vec<f64>,
}

impl GroupedInt4Linear {
    pub fn quantize(
        weights: &[f64],
        in_features: usize,
        out_features: usize,
        group_size: usize,
    ) -> Self {
        assert_eq!(weights.len(), in_features * out_features);
        assert_eq!(weights.len() % group_size, 0);
        assert_eq!(group_size % 2, 0);

        let num_groups = weights.len() / group_size;
        let mut scales = vec![0.0; num_groups];
        let mut packed_weights = vec![0u8; weights.len() / 2];

        for g in 0..num_groups {
            let start = g * group_size;
            let slice = &weights[start..start + group_size];
            let max_abs = slice.iter().map(|v| v.abs()).fold(0.0f64, f64::max);
            let scale = if max_abs > 1e-12 { max_abs / 7.0 } else { 1.0 };
            scales[g] = scale;

            for pair in 0..(group_size / 2) {
                let idx1 = start + pair * 2;
                let idx2 = idx1 + 1;

                let q1 = (weights[idx1] / scale).round().clamp(-8.0, 7.0) as i8;
                let q2 = (weights[idx2] / scale).round().clamp(-8.0, 7.0) as i8;

                // Đóng gói: q1 vào 4 bit thấp, q2 vào 4 bit cao
                let low = (q1 as u8) & 0x0F;
                let high = ((q2 as u8) & 0x0F) << 4;

                let byte_idx = (start + pair * 2) / 2;
                packed_weights[byte_idx] = low | high;
            }
        }

        Self {
            in_features,
            out_features,
            group_size,
            packed_weights,
            scales,
        }
    }

    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.in_features);
        let mut out = vec![0.0; self.out_features];

        for row in 0..self.out_features {
            let mut dot = 0.0;
            for (col, &x_val) in x.iter().enumerate() {
                let flat_idx = row * self.in_features + col;
                let group_idx = flat_idx / self.group_size;
                let scale = self.scales[group_idx];

                let byte_idx = flat_idx / 2;
                let byte = self.packed_weights[byte_idx];

                // Giải nén 4-bit có dấu bù hai
                let raw_nibble = if flat_idx % 2 == 0 {
                    byte & 0x0F
                } else {
                    (byte >> 4) & 0x0F
                };
                let signed_val = if raw_nibble >= 8 {
                    (raw_nibble as i8) - 16
                } else {
                    raw_nibble as i8
                };

                dot += (signed_val as f64 * scale) * x_val;
            }
            out[row] = dot;
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_grouped_int4_quantization() {
        // Ma trận 2 x 4 = 8 phần tử, group_size = 4
        let weights = vec![0.7, -0.6, 0.1, -0.2, 1.4, -1.0, 0.5, -0.3];
        let layer = GroupedInt4Linear::quantize(&weights, 4, 2, 4);

        assert_eq!(layer.packed_weights.len(), 4); // 8 phần tử đóng vào 4 byte
        assert_eq!(layer.scales.len(), 2);

        let x = vec![1.0, 0.5, -1.0, 2.0];
        let y = layer.forward(&x);
        assert_eq!(y.len(), 2);
    }
}
```

</details>
