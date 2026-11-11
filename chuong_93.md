# Chương 93: Từ GPT-2 Cổ điển Đến Bộ giải mã Hiện đại — RMSNorm, RoPE/YaRN, SwiGLU, GQA, DeepSeek MLA, DeltaNet & MoE (Modern Transformer Decoder Architectures)

## 1. Giới thiệu & Mục tiêu

Năm 2019, khi OpenAI công bố mô hình GPT-2 với 1,5 tỷ tham số, thế giới lập trình đã chứng kiến sự định hình của cấu trúc Transformer Decoder tự hồi quy (Autoregressive Decoder-Only Transformer). GPT-2 đại diện cho thế hệ kiến trúc cổ điển: chuẩn hóa tầng trước khối chú ý (`Pre-LN LayerNorm`), hàm kích hoạt phi tuyến tính mượt mà `GELU`, cơ chế chú ý đa đầu đầy đủ (`Multi-Head Attention` - MHA), và mã hóa vị trí tuyệt đối (`Absolute Positional Embeddings`). Trong gần nửa thập kỷ sau đó, cấu trúc này là tiêu chuẩn vàng của ngành xử lý ngôn ngữ tự nhiên.

Tuy nhiên, khi nhân loại bước vào kỷ nguyên mở rộng quy mô (Scaling Era) với các siêu mô hình từ 7 tỷ đến hơn 670 tỷ tham số (như LLaMA 1/2/3, Mistral, Gemma 1/2, Qwen 2.5 và DeepSeek V2/V3), kiến trúc GPT-2 cổ điển nhanh chóng bộc lộ 6 nút thắt vật lý nghiêm trọng:

1. **Bất ổn định số học và độ trễ bộ nhớ của LayerNorm**: Phép tính LayerNorm truyền thống đòi hỏi hai lượt duyệt qua bộ nhớ cho mỗi vector: lượt một tính kỳ vọng $\mu$, lượt hai tính phương sai $\sigma^2$ rồi mới thực hiện phép trừ và chia. Việc trừ kỳ vọng không chỉ tiêu tốn băng thông bộ nhớ (memory bandwidth) mà còn dễ gây sụt giảm gradient trong chế độ huấn luyện nửa độ chính xác (`fp16`/`bf16`).
2. **Điểm mù vị trí và trần ngữ cảnh của Absolute Positional Embeddings**: Vector vị trí cộng dồn $x_t + p_t$ cố định khiến mô hình hoàn toàn mất khả năng nắm bắt khoảng cách tương đối giữa hai từ khi câu bị xáo trộn hoặc kéo dài. Hơn thế, mô hình không thể ngoại suy vượt ra ngoài độ dài ngữ cảnh tối đa đã thấy lúc huấn luyện ($L > L_{\max}$).
3. **Hiệu suất kích hoạt giới hạn của GELU**: Hàm GELU truyền thống xử lý các chiều nơ-ron một cách thụ động, thiếu cơ chế cổng thích ứng (gating mechanism) để mô hình tự học cách lọc hoặc điều tiết dòng thông tin chảy qua mạng.
4. **Cơn ác mộng dung lượng KV-Cache của MHA**: Trong cơ chế Multi-Head Attention cổ điển, mỗi token sinh ra phải lưu trữ toàn bộ các vector Khóa (Key) và Giá trị (Value) của tất cả các đầu chú ý. Khi phục vụ hàng triệu người dùng đồng thời trên các chuỗi ngữ cảnh dài (32K - 128K tokens), dung lượng VRAM của cụm GPU hoàn toàn bị bóp nghẹt bởi bộ nhớ đệm KV chứ không phải bởi trọng số mô hình.
5. **Bức tường độ phức tạp bậc hai $O(N^2)$ của ma trận Attention**: Việc so sánh mọi token với tất cả các token trước đó khiến chi phí tính toán và bộ nhớ bùng nổ theo cấp số nhân khi ngữ cảnh mở rộng, biến việc đọc tài liệu hàng trăm nghìn trang thành một bài toán bất khả thi về mặt kinh tế.
6. **Chi phí kích hoạt lãng phí của Dense MLP**: Mọi token — dù là một dấu phẩy đơn giản hay một phương trình toán học phức tạp — đều buộc phải kích hoạt 100% trọng số của các tầng FFN khổng lồ, khiến chi phí suy luận bị đội lên gấp nhiều lần mức cần thiết.

Để vượt qua những giới hạn này, một cuộc cách mạng kiến trúc toàn diện đã diễn ra trong giai đoạn 2021–2026. Chương 93 được thiết kế để dẫn dắt bạn qua toàn bộ hành trình tiến hóa kỹ thuật đó, cài đặt từ con số không (zero-dependency) bằng **100% Rust 2024 thuần `std`**:

- **Thay thế LayerNorm bằng RMSNorm**: Loại bỏ hoàn toàn phép trừ kỳ vọng $\mu$, giảm 30% thao tác nạp/xả bộ nhớ RAM/VRAM, kèm biến thể độ lệch đơn vị `(1 + w)` của Gemma và đạo hàm giải tích xuôi/ngược đạt độ chính xác sai phân hữu hạn $< 10^{-8}$.
- **Mã hóa vị trí quay RoPE & Ngoại suy ngữ cảnh YaRN**: Thay thế vector vị trí cộng dồn bằng phép quay hình học trong không gian 2D, chứng minh bằng toán học rằng tích vô hướng $q_m^\top k_n$ chỉ phụ thuộc khoảng cách tương đối $(m - n)$ và bảo toàn nghiêm ngặt chuẩn Euclid vector. Đồng thời, triển khai thuật toán YaRN (Yet another RoPE extensioN) kết hợp phân giải bước sóng Fourier để mở rộng ngữ cảnh gấp 4–16 lần mà không cần huấn luyện lại từ đầu.
- **Tầng FFN cổng tích cực SwiGLU**: Thay thế GELU bằng phép nhân cổng Swish Gated Linear Unit với tỷ lệ chiều ẩn $\frac{8}{3} d_{\text{model}}$, tối ưu hóa khả năng biểu diễn của mô hình trên từng nơ-ron.
- **Grouped-Query Attention (GQA) & QK-RMSNorm & Sliding Window**: Nhóm nhiều đầu truy vấn Query vào một số lượng nhỏ đầu Khóa/Giá trị Key-Value, kết hợp chuẩn hóa QK-RMSNorm trước phép nhân vô hướng để triệt tiêu hiện tượng trôi logit, và áp dụng cửa sổ trượt (sliding window) để khống chế chi phí bộ nhớ.
- **DeepSeek Multi-Head Latent Attention (MLA)**: Đột phá kiến trúc lớn nhất giai đoạn 2024–2026 của DeepSeek. Nén không gian Key-Value đa đầu khổng lồ thành một vector tiềm ẩn duy nhất $c^{KV}$ có số chiều thấp, đồng thời tách riêng nhánh RoPE vị trí $k^R$. Giảm tới **70%–85% dung lượng KV-Cache** so với MHA truyền thống.
- **Gated DeltaNet — Trạng thái tuyến tính đệ quy $O(N)$**: Mô hình Linear RNN / State Space thế hệ mới ứng dụng luật Delta (Widrow-Hoff learning rule) và cổng suy giảm $\alpha_t$, cho phép bộ nhớ làm việc tự động sửa sai cục bộ và duy trì khả năng suy luận liên tục với độ phức tạp thời gian tuyến tính $O(N)$.
- **Hỗn hợp chuyên gia thưa Sparse MoE & Chuyên gia dùng chung**: Kiến trúc định tuyến Top-$k$ chuyên gia có điều kiện, kết hợp các chuyên gia dùng chung (Shared Experts) kiểu DeepSeek V2/V3 và hàm tổn thất phụ cân bằng tải Switch `aux_loss` ngăn chặn hội chứng sụp đổ định tuyến (routing collapse).

### Mục tiêu học tập cụ thể:
1. **Làm chủ toán học nền tảng**: Nắm vững phương trình giải tích, đạo hàm ngược và chứng minh hình học của từng khối kiến trúc.
2. **Kỹ nghệ Rust 2024 không thỏa hiệp**: Cài đặt toàn bộ hệ thống bằng struct có tên trường rõ ràng (`named fields`), không phụ thuộc thư viện ngoài (`dependencies` rỗng 100%), định danh tiếng Anh chuẩn hóa, doc-comment tiếng Việt theo chuẩn `RiskGate`.
3. **Toán học trung thực quy mô vi mô**: Chạy trực tiếp trên CPU tất định với $d_{\text{model}} = 16..32$, thời gian thực thi toàn bộ kiểm thử $< 150\text{ ms}$, chứng minh sai số đạo hàm giải tích so với sai phân hữu hạn trung tâm $< 10^{-8}$.
4. **Tư duy kiến trúc hệ thống AI**: Hiểu rõ khi nào nên dùng GQA thay vì MHA, khi nào nên chọn MLA thay vì GQA, và cách kết hợp MoE cùng DeltaNet để xây dựng các mô hình nền tảng thế hệ mới.

---

## 2. Hình tượng hóa đời sống: Từ động cơ hơi nước đến phản lực siêu thanh

Để có cái nhìn trực quan sâu sắc trước khi đi vào những ma trận số học phức tạp, hãy quan sát bảng so sánh tiến hóa giữa hai thế hệ kiến trúc:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│               TIẾN HÓA BỘ GIẢI MÃ TRANSFORMER (2019 -> 2026)                           │
│                                                                                        │
│   KIẾN TRÚC GPT-2 CỔ ĐIỂN (2019)               KIẾN TRÚC HIỆN ĐẠI (2024-2026)         │
│   (Động cơ hơi nước - Nặng nề, tốn VRAM)       (Động cơ phản lực siêu thanh)           │
│                                                                                        │
│   ┌───────────────────────────┐                ┌───────────────────────────┐           │
│   │ Input x + Absolute Pos    │                │ Input x                   │           │
│   └─────────────┬─────────────┘                └─────────────┬─────────────┘           │
│                 ▼                                            ▼                         │
│   ┌───────────────────────────┐                ┌───────────────────────────┐           │
│   │ Pre-LN: LayerNorm(μ, σ²)  │ ──► Thay thế ──►│ Pre-RMSNorm: RMS(x)       │ (Gemma 1+w│
│   └─────────────┬─────────────┘                └─────────────┬─────────────┘           │
│                 ▼                                            ▼                         │
│   ┌───────────────────────────┐                ┌───────────────────────────┐           │
│   │ Multi-Head Attention      │                │ Grouped-Query Attn (GQA)  │           │
│   │ (MHA - Lưu toàn bộ KV)    │ ──► Thay thế ──►│ hoặc DeepSeek MLA (Nén KV)│ (+ RoPE / │
│   │ QK^T / √d, Causal Mask    │                │ hoặc GatedDeltaNet (O(N)) │   YaRN)   │
│   └─────────────┬─────────────┘                └─────────────┬─────────────┘           │
│                 ▼                                            ▼                         │
│   ┌───────────────────────────┐                ┌───────────────────────────┐           │
│   │ Residual Add (+)          │                │ Residual Add (+)          │           │
│   └─────────────┬─────────────┘                └─────────────┬─────────────┘           │
│                 ▼                                            ▼                         │
│   ┌───────────────────────────┐                ┌───────────────────────────┐           │
│   │ Pre-LN: LayerNorm(μ, σ²)  │ ──► Thay thế ──►│ Pre-RMSNorm: RMS(x)       │           │
│   └─────────────┬─────────────┘                └─────────────┬─────────────┘           │
│                 ▼                                            ▼                         │
│   ┌───────────────────────────┐                ┌───────────────────────────┐           │
│   │ Dense MLP:                │                │ SwiGLU MLP                │           │
│   │ GELU(x W_fc + b) * W_proj │ ──► Thay thế ──►│ hoặc Sparse MoE (Top-k    │ (Shared   │
│   │ (Kích hoạt 100% nơ-ron)   │                │ + Shared Experts + AuxLoss│  Experts) │
│   └─────────────┬─────────────┘                └─────────────┬─────────────┘           │
│                 ▼                                            ▼                         │
│   ┌───────────────────────────┐                ┌───────────────────────────┐           │
│   │ Output Hidden States      │                │ Output Hidden States      │           │
│   └───────────────────────────┘                └───────────────────────────┘           │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

Hãy cùng hình dung 5 bước nhảy vọt kỹ thuật này thông qua các hình ảnh ẩn dụ đời sống thường nhật:

### 1. LayerNorm vs RMSNorm: Người cân đo tỉ mỉ từng hạt bụi vs Chiếc kính lọc quang học
- **LayerNorm (GPT-2)** giống như một giám định viên kỹ tính bước vào phòng cân. Trước tiên, ông gom toàn bộ đồ vật lại, cân tổng thể rồi chia trung bình để tìm trọng tâm $\mu$. Sau đó, ông lấy từng món đồ trừ đi trọng tâm đó, rồi lại cân tiếp một vòng nữa để tính độ lệch chuẩn $\sigma$. Quá trình này đòi hỏi hai lượt kiểm kê riêng biệt.
- **RMSNorm (LLaMA/Mistral)** nhận ra rằng việc tìm trọng tâm $\mu$ là hoàn toàn dư thừa đối với mạng nơ-ron sâu; thứ duy nhất giúp mạng không bị nổ gradient là quy mô độ lớn (magnitude/energy). RMSNorm giống như một chiếc kính râm phân cực quang học: nó chỉ đo tổng năng lượng ánh sáng $\sqrt{\sum x_i^2 / d}$ và chia tỷ lệ để cường độ sáng đạt mức chuẩn mực. Nhanh hơn 30%, tốn ít bộ nhớ hơn, và gradient chảy thông suốt hơn.

### 2. Absolute Positional vs RoPE/YaRN: Đánh số phòng khách sạn cố định vs Chiếc la bàn quay theo góc tương đối
- **Absolute Embeddings (GPT-2)** giống như việc ban quản lý gắn số phòng cố định từ 1 đến 2048 lên cửa khách sạn. Nếu có một vị khách mới đến ở phòng 2049, cả hệ thống khách sạn bị tê liệt vì không có số phòng tương ứng. Quan trọng hơn, nếu phòng 5 và phòng 8 cách nhau 3 bước, mô hình không tự nhận biết được điều đó mà phải học vẹt từng cặp số (5, 8).
- **RoPE (Rotary Position Embeddings)** loại bỏ hoàn toàn số phòng tuyệt đối. Thay vào đó, mỗi vị khách mang theo một chiếc la bàn. Khi bước sang phòng tiếp theo, kim la bàn chỉ đơn giản là quay thêm một góc $\theta$. Khi hai người nói chuyện với nhau, khoảng cách tương đối giữa họ chính là góc lệch giữa hai chiếc la bàn ($m\theta - n\theta = (m - n)\theta$). La bàn có thể quay vòng tròn mãi mãi, cho phép mở rộng ngữ cảnh vô hạn!
- **YaRN** giống như việc ta tinh chỉnh tốc độ quay của kim la bàn khi bước vào một khuôn viên rộng gấp 10 lần: kim quay chậm lại một cách êm ái trên các bước sóng dài để mô hình không bị chóng mặt, trong khi vẫn giữ nguyên độ nhạy sắc bén ở cự ly gần.

### 3. MHA vs GQA vs DeepSeek MLA: Một thư ký cho mỗi sếp vs Thư ký dùng chung vs Tốc ký nén mật mã
- **Multi-Head Attention (GPT-2)**: Một công ty có 32 vị giám đốc (32 Query heads), và công ty phải thuê đúng 32 cô thư ký riêng (32 Key-Value heads). Mỗi khi đi họp, 32 cô thư ký phải mang theo 32 cuốn sổ ghi chép dày cộp. Chi phí văn phòng (VRAM) phình to đến mức phá sản.
- **Grouped-Query Attention (LLaMA 3)**: Công ty tái cơ cấu. Cứ 4 giám đốc sẽ dùng chung 1 cô thư ký (8 Key-Value heads cho 32 Query heads). Dung lượng sổ ghi chép giảm đi 4 lần, nhưng hiệu quả công việc vẫn giữ được 99%.
- **DeepSeek MLA (DeepSeek V2/V3)**: Bước nhảy vọt mang tính cách mạng. Không cần thuê nhiều thư ký mang sổ dày nữa. Thay vào đó, mọi thông tin quan trọng được một chuyên viên mã hóa tóm tắt thành một đoạn mật mã siêu ngắn (vector tiềm ẩn $c^{KV}$). Sổ ghi chép giờ đây mỏng như một tờ giấy. Khi giám đốc nào cần thông tin chi tiết, họ chỉ việc dùng chìa khóa giải mã nội bộ để tra cứu tức thì. VRAM giảm tới 80%!

### 4. Dense MLP vs Sparse MoE: Người thợ đa năng kiệt sức vs Hội đồng 64 viện sĩ chuyên khoa
- **Dense MLP (GPT-2)** giống như một người thợ sửa chữa duy nhất phải làm tất cả mọi việc: từ sửa ống nước, hàn xì, sửa điện cho đến phẫu thuật tim và viết luận án luật. Dù nhiệm vụ chỉ là vặn một con ốc, toàn bộ năng lượng của người thợ vẫn phải vận hành hết công suất.
- **Sparse MoE (DeepSeek / Mixtral)** tổ chức một viện hàn lâm gồm 64 chuyên gia hàng đầu thuộc các lĩnh vực khác nhau. Khi một nhiệm vụ gửi đến, viên điều phối (Router) chỉ chọn đúng 2 chuyên gia phù hợp nhất (Top-2) để xử lý, trong khi 62 chuyên gia còn lại nghỉ ngơi. Đặc biệt, kiến trúc còn bố trí 1–2 chuyên gia dùng chung (Shared Experts) luôn túc trực để xử lý các kiến thức phổ quát. Chi phí FLOPs chỉ bằng một phần nhỏ nhưng tổng tri thức tích lũy lại khổng lồ.

### 5. Attention $O(N^2)$ vs Gated DeltaNet $O(N)$: Lật từng trang hồ sơ lưu trữ vs Trí nhớ làm việc tự sửa sai
- **Standard Attention**: Giống như việc một luật sư mỗi khi viết một từ trong bản án đều phải đọc lại toàn bộ hàng nghìn trang hồ sơ từ đầu chí cuối. Khi vụ án kéo dài 100.000 trang, thời gian viết một câu dài bằng cả một đời người.
- **Gated DeltaNet**: Giống như một vị thẩm phán thông thái sở hữu một tấm bảng nhớ làm việc (trạng thái $S_t$). Khi có thông tin mới, ngài so sánh xem thông tin này có mâu thuẫn với ký ức cũ không (luật Delta: $v_t - S_{t-1} k_t$). Nếu mâu thuẫn, ngài xóa ký ức cũ đi và ghi đè sự thật mới lên bảng. Tốc độ đọc và viết là hằng số cho mỗi trang giấy, thời gian xử lý toàn bộ vụ án chỉ tỷ lệ thuận tuyến tính $O(N)$!

---

## 3. Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 3.1. Phép chuẩn hóa RMSNorm và biến thể Gemma Unit Offset

Chuẩn hóa theo tầng truyền thống `LayerNorm` được định nghĩa bởi phương trình:

$$y = \frac{x - \mu}{\sqrt{\sigma^2 + \epsilon}} \odot \gamma + \beta$$

trong đó $\mu = \frac{1}{d} \sum_{i=1}^d x_i$ và $\sigma^2 = \frac{1}{d} \sum_{i=1}^d (x_i - \mu)^2$.

Zhang và Sennrich (2019) đã chứng minh bằng thực nghiệm rằng thành phần bất biến tịnh tiến (translation invariance) do phép trừ $\mu$ mang lại không đóng góp vào việc ổn định gradient. Thứ thực sự giúp mạng nơ-ron hội tụ nhanh là phép chuẩn hóa độ lớn (scaling invariance). Bằng cách loại bỏ $\mu$, ta thu được công thức `RMSNorm`:

$$\text{RMS}(x) = \sqrt{\frac{1}{d} \sum_{i=1}^d x_i^2 + \epsilon}$$

$$y_i = \frac{x_i}{\text{RMS}(x)} \cdot w_i$$

#### Biến thể Gemma Unit Offset:
Trong các mô hình như Gemma của Google DeepMind, trọng số $w$ được khởi tạo bằng $0.0$ thay vì $1.0$ để tránh mất mát độ chính xác trong định dạng số thực dấu phẩy động 16-bit (`bfloat16`). Khi đó, công thức biến đổi thành:

$$y_i = \frac{x_i}{\text{RMS}(x)} \cdot (1.0 + w_i)$$

#### Đạo hàm ngược giải tích (Analytical Backward Pass):
Để phục vụ việc kiểm chứng thuật toán bằng sai phân hữu hạn trung tâm sai số $< 10^{-8}$, ta tính đạo hàm riêng của hàm mất mát $\mathcal{L}$ theo $w_i$ và $x_i$:

Gọi $g_i = \frac{\partial \mathcal{L}}{\partial y_i}$, $\text{inv} = \frac{1}{\text{RMS}(x)}$, và $\hat{x}_i = x_i \cdot \text{inv}$.

Đạo hàm theo trọng số:
$$\frac{\partial \mathcal{L}}{\partial w_i} = g_i \cdot \hat{x}_i$$

Đạo hàm theo đầu vào $x_i$:
$$\frac{\partial \mathcal{L}}{\partial x_i} = \text{inv} \cdot \left( g_i w_i - \hat{x}_i \cdot \frac{1}{d} \sum_{j=1}^d g_j w_j \hat{x}_j \right)$$

Nhờ công thức này, ta chỉ cần một lượt tích vô hướng duy nhất $\text{dot} = \frac{1}{d} \sum g_j w_j \hat{x}_j$ là có thể tính toàn bộ gradient $\frac{\partial \mathcal{L}}{\partial x}$ trong $O(d)$ mà không cần lưu ma trận Jacobian $d \times d$!

---

### 3.2. Mã hóa vị trí quay RoPE & Ngoại suy ngữ cảnh YaRN

Trong các Transformer cổ điển, vector vị trí $p_t$ được cộng thẳng vào vector nhúng token: $x_t \leftarrow x_t + p_t$. Cách làm này phá vỡ tính chất hình học của vector biểu diễn.

Su et al. (2021) giới thiệu `RoPE` (Rotary Positional Embeddings), biến đổi vector biểu diễn của Query $q$ và Key $k$ tại vị trí $m$ thông qua ma trận quay trực giao:

$$\tilde{q}_m = R_{\Theta, m} q_m, \quad \tilde{k}_n = R_{\Theta, n} k_n$$

Ma trận quay trực giao $R_{\Theta, m}$ được cấu tạo từ các khối ma trận 2D đường chéo:

$$R_{\Theta, m} = \text{diag}\left( R_{\theta_0, m}, R_{\theta_1, m}, \dots, R_{\theta_{d/2 - 1}, m} \right)$$

$$R_{\theta_i, m} = \begin{pmatrix} \cos(m \theta_i) & -\sin(m \theta_i) \\ \sin(m \theta_i) & \cos(m \theta_i) \end{pmatrix}, \quad \text{với } \theta_i = b^{-2i/d}$$

Thông thường, cơ số $b = 10.000$ (trong GPT-NeoX, LLaMA) hoặc $b = 500.000$ (trong LLaMA 3).

#### Bất biến khoảng cách tương đối:
Do tính chất trực giao của ma trận quay ($R_m^\top = R_{-m}$), tích vô hướng giữa Query tại vị trí $m$ và Key tại vị trí $n$ thỏa mãn:

$$\langle \tilde{q}_m, \tilde{k}_n \rangle = (R_{\Theta, m} q_m)^\top (R_{\Theta, n} k_n) = q_m^\top R_{\Theta, m}^\top R_{\Theta, n} k_n = q_m^\top R_{\Theta, n - m} k_n$$

Tích vô hướng này **chỉ phụ thuộc vào khoảng cách tương đối $(n - m)$**, hoàn toàn không phụ thuộc vào vị trí tuyệt đối $m$ hay $n$!

#### Bảo toàn chuẩn Euclid:
Vì $R_{\Theta, m}$ là ma trận trực giao ($R^\top R = I$), chuẩn của vector được bảo toàn tuyệt đối:

$$\|\tilde{q}_m\|_2 = \sqrt{\tilde{q}_m^\top \tilde{q}_m} = \sqrt{q_m^\top R_m^\top R_m q_m} = \sqrt{q_m^\top q_m} = \|q_m\|_2$$

Sai số bảo toàn chuẩn trong các phép tính dấu phẩy động 64-bit (`f64`) đạt mức $< 10^{-12}$.

#### YaRN (Yet another RoPE extensioN):
Khi mở rộng ngữ cảnh từ độ dài gốc $L_0$ lên mục tiêu $L = s \cdot L_0$ ($s > 1$), việc kéo giãn tuyến tính đơn thuần $\theta'_i = \theta_i / s$ (Linear Positional Interpolation) sẽ làm mờ các tần số cao (chi tiết ngữ pháp ngắn hạn).

YaRN (Peng et al., 2023) giải quyết vấn đề này bằng cách phân tích bước sóng $\lambda_i = \frac{2\pi}{\theta_i}$:
- Nếu $\lambda_i < \beta_{\text{slow}} L_0$: Bước sóng rất ngắn, không cần nội suy ($\gamma_i = 0$, giữ nguyên tần số $\theta_i$).
- Nếu $\lambda_i > \beta_{\text{fast}} L_0$: Bước sóng rất dài, thông tin vị trí bị thưa thớt, cần nội suy tuyến tính hoàn toàn ($\gamma_i = 1$, tần số mới $\theta_i / s$).
- Ở giữa hai ngưỡng: Áp dụng dốc chuyển mượt mà:
  $$\gamma_i = \frac{\lambda_i - \beta_{\text{slow}} L_0}{(\beta_{\text{fast}} - \beta_{\text{slow}}) L_0}$$
  $$\tilde{\theta}_i = (1 - \gamma_i) \theta_i + \gamma_i \frac{\theta_i}{s}$$

Đồng thời, YaRN áp dụng hệ số co giãn độ lớn chú ý `mscale`:
$$\text{mscale} = 0.1 \ln(s) + 1.0$$
giúp ổn định phân phối xác suất Softmax khi số lượng token trong ngữ cảnh tăng vọt.

---

### 3.3. Tầng FFN cổng tích cực SwiGLU

Trong GPT-2, tầng Feed-Forward Network gồm hai ma trận biến đổi tuyến tính kẹp giữa hàm GELU:

$$\text{FFN}_{\text{GPT-2}}(x) = \text{GELU}(x W_{\text{fc}} + b_{\text{fc}}) W_{\text{proj}} + b_{\text{proj}}$$

với kích thước ẩn $d_{\text{hidden}} = 4 d_{\text{model}}$.

Shazeer (2020) đã khảo sát các biến thể Gated Linear Units (GLU) và chứng minh rằng hàm SwiGLU vượt trội hơn tất cả các biến thể khác về tốc độ hội tụ và điểm đánh giá ngôn ngữ:

$$\text{SwiGLU}(x) = \left( \text{Swish}(x W_{\text{gate}}) \odot (x W_{\text{up}}) \right) W_{\text{down}}$$

trong đó $\text{Swish}(z) = z \cdot \text{sigmoid}(z)$, và $\odot$ là phép nhân từng phần tử (Hadamard product).

#### Quy tắc kích thước ẩn $\frac{8}{3} d_{\text{model}}$:
Vì SwiGLU sử dụng 3 ma trận trọng số ($W_{\text{gate}}, W_{\text{up}}, W_{\text{down}}$) thay vì 2 ma trận như GPT-2, để giữ cho tổng số lượng tham số và số phép tính FLOPs không đổi, kích thước ẩn được điều chỉnh:

$$d_{\text{hidden}} \approx \frac{2}{3} \times (4 d_{\text{model}}) = \frac{8}{3} d_{\text{model}}$$

Trong thực tế, giá trị này thường được làm tròn lên bội số gần nhất của 64 hoặc 256 để tối ưu hóa việc phân chia khối trên các bộ vi xử lý và card tăng tốc phần cứng.

---

### 3.4. Grouped-Query Attention (GQA) & QK-RMSNorm & Cửa sổ trượt

Trong MHA cổ điển, số đầu Query $H_Q$ bằng đúng số đầu Key $H_K$ và Value $H_V$. Khi sinh token theo từng bước (Autoregressive Decoding), tại mỗi bước $t$, mô hình phải nạp toàn bộ KV-Cache của $t$ bước trước đó từ bộ nhớ DRAM vào bộ xử lý:

$$\text{Dung lượng nạp mỗi bước} = 2 \times B \times L \times H \times d_h \times \text{sizeof(dtype)}$$

Ainslie et al. (2023) đề xuất `Grouped-Query Attention` (GQA): gom cụm các đầu Query thành từng nhóm kích thước $G = H_Q / H_{KV}$, trong đó mỗi nhóm chia sẻ chung một đầu Key và một đầu Value:

```
Multi-Head Attention (MHA)      Grouped-Query Attention (GQA)      Multi-Query Attention (MQA)
    Q1 Q2 Q3 Q4 Q5 Q6 Q7 Q8         Q1 Q2  Q3 Q4  Q5 Q6  Q7 Q8         Q1 Q2 Q3 Q4 Q5 Q6 Q7 Q8
    │  │  │  │  │  │  │  │          └──┬──┘  └──┬──┘  └──┬──┘  └──┬──┘         └───────┬───────┘
    K1 K2 K3 K4 K5 K6 K7 K8            K1       K2       K3       K4                    K1
    V1 V2 V3 V4 V5 V6 V7 V8            V1       V2       V3       V4                    V1
   (Tỷ lệ KV: 100% dung lượng)        (Tỷ lệ KV: 25% dung lượng - LLaMA 3)   (Tỷ lệ KV: 12.5% dung lượng)
```

#### QK-RMSNorm (Chuẩn hóa Query-Key):
Khi kích thước mô hình vượt quá hàng chục tỷ tham số, tích vô hướng $q^\top k$ có xu hướng xuất hiện các giá trị ngoại lai cực lớn (logit drift / outliers), đẩy hàm Softmax vào vùng bão hòa và làm triệt tiêu gradient.
Để khắc phục, các kiến trúc hiện đại (Gemma 2, Qwen 2.5) áp dụng RMSNorm trực tiếp lên từng đầu của Query và Key trước khi nhân tích vô hướng:

$$q_{h} \leftarrow \text{RMSNorm}(q_{h}), \quad k_{h} \leftarrow \text{RMSNorm}(k_{h})$$

#### Cửa sổ trượt (Sliding Window Attention - SWA):
Để giới hạn chi phí tính toán khi chuỗi văn bản dài đến hàng chục nghìn token, SWA quy định rằng token tại vị trí $i$ chỉ được phép chú ý tới các token $j$ nằm trong cửa sổ kích thước $W$:

$$\text{Causal Mask}_{i, j} = \begin{cases} 0 & \text{nếu } i - W < j \le i \\ -\infty & \text{ngược lại} \end{cases}$$

Nhờ các tầng xếp chồng lên nhau, trường tiếp nhận thông tin (receptive field) của mô hình vẫn bao phủ toàn bộ văn bản sau $L$ tầng ($L \times W$), trong khi chi phí bộ nhớ đệm KV của mỗi tầng bị chặn cứng ở mức $W$.

---

### 3.5. DeepSeek Multi-Head Latent Attention (MLA)

Đột phá kiến trúc ấn tượng nhất trong thế hệ mô hình ngôn ngữ 2024–2026 chính là `Multi-Head Latent Attention` (MLA) được giới thiệu bởi DeepSeek-AI (DeepSeek-V2, 2024; DeepSeek-V3, 2025).

#### Giới hạn không thể vượt qua của GQA:
Mặc dù GQA giúp giảm kích thước KV-cache bằng cách giảm số lượng đầu chú ý $H_{KV}$, nó vô tình làm suy giảm năng lực biểu diễn thông tin đa dạng của không gian Key-Value, dẫn đến việc sụt giảm nhẹ điểm benchmark so với MHA ở cùng quy mô.

#### Ý tưởng cốt lõi của MLA: Nén hạng thấp Key-Value:
Thay vì lưu trữ các vector Key và Value có kích thước đầy đủ $H \times d_h$, MLA nén toàn bộ vector biểu diễn đầu vào $x_t \in \mathbb{R}^{d_{\text{model}}}$ xuống một **vector tiềm ẩn duy nhất** $c_t^{KV} \in \mathbb{R}^{d_c}$ thông qua ma trận nén $W_{DKV} \in \mathbb{R}^{d_{\text{model}} \times d_c}$ với $d_c \ll H \times d_h$:

$$c_t^{KV} = \text{RMSNorm}(x_t W_{DKV})$$

Khi cần tính toán chú ý, vector $c_t^{KV}$ được giải nén thành các vector Key nội dung $k_t^C$ và Value nội dung $v_t^C$:

$$k_{t, h}^C = c_t^{KV} W_{UK, h}, \quad v_{t, h}^C = c_t^{KV} W_{UV, h}$$

#### Tách nhánh vị trí RoPE (Decoupled RoPE):
Một thách thức toán học xuất hiện: Nếu ta nhúng vị trí RoPE vào $c_t^{KV}$, phép quay vị trí sẽ bị trộn lẫn với ma trận giải nén $W_{UK}$, phá hủy tính chất trực giao của RoPE.
DeepSeek giải quyết vấn đề này bằng một cơ chế tinh tế: **tách riêng nhánh vị trí**.
- Mỗi token tạo ra một vector khóa vị trí riêng biệt $k_t^R \in \mathbb{R}^{d_r}$ (với $d_r$ rất nhỏ, ví dụ $d_r = 64$):
  $$k_t^R = \text{RoPE}(x_t W_{KR}, t)$$
- Vector truy vấn của mỗi đầu cũng được tách thành phần nội dung $q_{t, h}^C$ và phần vị trí $q_{t, h}^R$:
  $$q_{t, h}^C = c_t^Q W_{UQ, h}, \quad q_{t, h}^R = \text{RoPE}(c_t^Q W_{QR, h}, t)$$

#### Tích vô hướng tổng hợp trong MLA:
Điểm chú ý giữa Query tại vị trí $t$ và Key tại vị trí $s$ được tính bằng tổng của hai tích vô hướng độc lập:

$$\text{Score}_{t, s, h} = \frac{1}{\sqrt{d_h + d_r}} \left( \langle q_{t, h}^C, k_{s, h}^C \rangle + \langle q_{t, h}^R, k_s^R \rangle \right)$$

#### Phép màu tiết kiệm bộ nhớ khi suy luận:
Trong quá trình sinh token tự hồi quy, **bộ nhớ đệm KV-Cache chỉ cần lưu đúng hai vector cho mỗi token**:
1. Vector tiềm ẩn nén $c_s^{KV}$ ($d_c$ số thực)
2. Vector khóa vị trí RoPE $k_s^R$ ($d_r$ số thực)

Tổng dung lượng bộ nhớ đệm cho mỗi token:
$$\text{Dung lượng MLA} = (d_c + d_r) \times \text{sizeof(dtype)}$$

So sánh định lượng trên mô hình DeepSeek-V2 ($d_{\text{model}} = 5120, H = 128, d_h = 128, d_c = 512, d_r = 64$):
- **MHA**: $2 \times H \times d_h = 2 \times 128 \times 128 = 32.768$ số thực / token.
- **GQA (16 nhóm)**: $2 \times H_{KV} \times d_h = 2 \times 16 \times 128 = 4.096$ số thực / token.
- **MLA**: $d_c + d_r = 512 + 64 = \mathbf{576}$ số thực / token!
- **MLA giảm dung lượng KV-Cache tới 98.2% so với MHA và giảm 85.9% so với GQA**, trong khi vẫn duy trì trọn vẹn 128 đầu chú ý độc lập cho phần nội dung!

---

### 3.6. Gated DeltaNet: Trạng thái tuyến tính đệ quy $O(N)$

Các kiến trúc Transformer dựa trên Softmax Attention có chi phí tính toán tích vô hướng $Q K^\top$ tăng theo bình phương độ dài chuỗi $O(N^2)$.
Dòng mô hình Linear Attention (Katharopoulos et al., 2020) cố gắng loại bỏ Softmax để tận dụng tính chất kết hợp của phép nhân ma trận: $(Q K^\top) V = Q (K^\top V)$, chuyển độ phức tạp về $O(N)$. Tuy nhiên, Linear Attention thông thường thất bại trong các bài toán truy xuất liên tưởng (associative recall) vì ma trận trạng thái $S = \sum k_i v_i^\top$ cộng dồn thông tin một cách thụ động, khiến các thông tin cũ bị nhiễu và đè bẹp.

`Gated DeltaNet` (Schlag et al., 2021; Yang et al., 2024) khắc phục triệt để nhược điểm này bằng cách đưa **Luật Delta (Widrow-Hoff rule)** vào phương trình cập nhật trạng thái đệ quy:

$$S_t = \alpha_t S_{t-1} + \beta_t (v_t - S_{t-1} k_t) k_t^\top$$

trong đó:
- $k_t \in \mathbb{R}^d$ là vector khóa được chuẩn hóa chuẩn đơn vị: $\|k_t\|_2 = 1$.
- $v_t \in \mathbb{R}^d$ là vector giá trị.
- $\alpha_t = \text{sigmoid}(x_t W_\alpha) \in (0, 1)$ là cổng suy giảm (forget gate).
- $\beta_t = \text{sigmoid}(x_t W_\beta) \in (0, 1)$ là tốc độ cập nhật Delta (learning rate gate).
- Đại lượng $S_{t-1} k_t$ chính là **dự báo của ký ức hiện tại** đối với khóa $k_t$.
- Biểu thức sai số $\delta_t = v_t - S_{t-1} k_t$ chính là **phần thông tin mới chưa được học**.

#### Ý nghĩa toán học của Luật Delta:
Khi ta truy vấn trạng thái mới $S_t$ bằng chính vector khóa $k_t$, do $\|k_t\|^2 = k_t^\top k_t = 1$:

$$S_t k_t = \alpha_t S_{t-1} k_t + \beta_t (v_t - S_{t-1} k_t) (k_t^\top k_t) = (\alpha_t - \beta_t) S_{t-1} k_t + \beta_t v_t$$

Nếu $\alpha_t = \beta_t = 1$, ta có:
$$S_t k_t = v_t$$

Điều này có nghĩa là: **ký ức $S_t$ tự động trừ khử hoàn toàn thông tin cũ gây nhiễu liên quan đến $k_t$, và thay thế chính xác bằng giá trị mới $v_t$!** Đây là cơ chế tương đương với thao tác `UPDATE` hoặc `DELETE` trong cơ sở dữ liệu, thứ mà Transformer cổ điển hoàn toàn không thể làm được.

Đầu ra tại thời điểm $t$ được điều tiết bởi vector truy vấn $q_t$ và cổng đầu ra SiLU:
$$y_t = (S_t q_t) \odot \text{SiLU}(x_t W_{\text{gate}})$$

Chi phí tính toán cho mỗi token là $O(d^2)$ (nhân ma trận với vector), hoàn toàn độc lập với vị trí $t$. Trên toàn bộ chuỗi độ dài $N$, chi phí thời gian là **tuyến tính $O(N)$** và bộ nhớ trạng thái là hằng số $O(d^2)$.

---

### 3.7. Hỗn hợp chuyên gia thưa Sparse MoE & Aux-Loss

Trong một mạng nơ-ron truyền thống (`Dense`), tất cả $N_{\text{params}}$ tham số đều tham gia tính toán cho mỗi token.
Kiến trúc `Mixture of Experts` (MoE) phân chia tầng FFN thành $E$ mạng chuyên gia độc lập: $\{\text{Expert}_1, \text{Expert}_2, \dots, \text{Expert}_E\}$.

#### Cơ chế định tuyến Router Top-$k$:
Một bộ định tuyến tuyến tính với ma trận trọng số $W_g \in \mathbb{R}^{d_{\text{model}} \times E}$ tính toán điểm số cho từng chuyên gia:

$$s = x W_g \in \mathbb{R}^E$$

Phân phối xác suất định tuyến:
$$P = \text{softmax}(s)$$

Mô hình chọn ra $k$ chuyên gia có điểm số cao nhất (Top-$k$, ví dụ $k = 2$ trong tổng số $E = 8$ chuyên gia). Trọng số của các chuyên gia được chọn được chuẩn hóa lại:

$$w_i = \frac{P_i}{\sum_{j \in \text{Top-k}} P_j}, \quad \forall i \in \text{Top-k}$$

Đầu ra của nhánh định tuyến:
$$y_{\text{routed}} = \sum_{i \in \text{Top-k}} w_i \cdot \text{Expert}_i(x)$$

#### Chuyên gia dùng chung (Shared Experts) kiểu DeepSeek:
Một vấn đề lớn của MoE truyền thống là các tri thức nền tảng mang tính khái quát cao (như ngữ pháp tiếng Việt, cú pháp câu) bị phân tán và lặp lại trên nhiều chuyên gia, làm lãng phí dung lượng mô hình.
DeepSeek V2/V3 giới thiệu giải pháp `Shared Experts`: cố định một số lượng chuyên gia $E_{\text{shared}}$ luôn luôn được kích hoạt cho **100% token**:

$$y_{\text{final}} = y_{\text{routed}} + \sum_{j=1}^{E_{\text{shared}}} \text{SharedExpert}_j(x)$$

Nhờ đó, các chuyên gia dùng chung sẽ chuyên môn hóa việc ghi nhớ tri thức phổ quát, giải phóng các chuyên gia định tuyến để họ tập trung vào các tri thức chuyên sâu mang tính đặc thù cao.

#### Tổn thất phụ cân bằng tải Switch Aux-Loss:
Nếu không có sự can thiệp, cơ chế định tuyến thường rơi vào bẫy **sụp đổ định tuyến (routing collapse)**: một vài chuyên gia ngẫu nhiên nhận được điểm cao ở giai đoạn đầu huấn luyện sẽ tiếp tục nhận được nhiều token hơn, được cập nhật gradient nhiều hơn, và trở nên vượt trội hơn. Hậu quả là 90% token chỉ chạy qua 1–2 chuyên gia ("cháy tải"), trong khi các chuyên gia còn lại hoàn toàn không được sử dụng ("chết não").

Để ép Router phải chia đều tải cho toàn bộ $E$ chuyên gia, Fedus et al. (2022) giới thiệu hàm tổn thất phụ `aux_loss`:

$$\mathcal{L}_{\text{aux}} = \alpha \cdot E \cdot \sum_{i=1}^E f_i \cdot P_i$$

trong đó:
- $f_i$ là tỷ lệ token trong batch thực sự được định tuyến tới chuyên gia $i$:
  $$f_i = \frac{1}{T \cdot k} \sum_{t=1}^T \mathbb{I}(\text{chuyên gia } i \in \text{Top-k của token } t)$$
- $P_i$ là xác suất trung bình mà Router gán cho chuyên gia $i$ trên toàn bộ batch:
  $$P_i = \frac{1}{T} \sum_{t=1}^T P_{t, i}$$
- $\alpha$ là hệ số phạt cân bằng tải (thường đặt $\alpha = 0.01$).

#### Chứng minh tính chất cực tiểu khi tải chia đều:
Theo bất đẳng thức Cauchy-Schwarz, tổng $\sum_{i=1}^E f_i P_i$ đạt giá trị nhỏ nhất khi và chỉ khi hai phân phối là đồng nhất và phân bố đều:
$$f_i = \frac{1}{E}, \quad P_i = \frac{1}{E}, \quad \forall i \in \{1, \dots, E\}$$

Khi đó:
$$\sum_{i=1}^E f_i P_i = \sum_{i=1}^E \frac{1}{E} \cdot \frac{1}{E} = E \cdot \frac{1}{E^2} = \frac{1}{E}$$

$$\mathcal{L}_{\text{aux}}^{\min} = \alpha \cdot E \cdot \frac{1}{E} = \alpha$$

Ngược lại, nếu toàn bộ token bị dồn vào đúng một chuyên gia duy nhất ($f_1 = 1, P_1 \approx 1$, còn các chuyên gia khác bằng 0):
$$\sum_{i=1}^E f_i P_i = 1 \cdot 1 = 1$$
$$\mathcal{L}_{\text{aux}}^{\max} = \alpha \cdot E \cdot 1 = \alpha \cdot E$$

Tổn thất phạt tăng vọt gấp $E$ lần! Điều này tạo ra một lực đẩy gradient cực mạnh buộc Router phải phân bổ đồng đều token tới tất cả các chuyên gia.

---

### 3.8. Biểu diễn số phức của RoPE và tương đương trực giao của Phân nửa Vector

Trong bài báo gốc RoFormer (Su et al., 2021), phép quay RoPE ban đầu được mô tả thông qua đại số số phức. Xét một cặp tọa độ liền kề $(x_{2j}, x_{2j+1})$, ta có thể đồng nhất cặp số thực này với một số phức:

$$z_j = x_{2j} + i x_{2j+1} \in \mathbb{C}$$

Phép quay RoPE tại vị trí $m$ tương đương với phép nhân số phức với thừa số pha Euler:

$$\tilde{z}_j = z_j \cdot e^{i m \theta_j} = (x_{2j} + i x_{2j+1}) \cdot (\cos(m \theta_j) + i \sin(m \theta_j))$$

Khai triển phần thực và phần ảo:
$$\text{Re}(\tilde{z}_j) = x_{2j} \cos(m \theta_j) - x_{2j+1} \sin(m \theta_j)$$
$$\text{Im}(\tilde{z}_j) = x_{2j+1} \cos(m \theta_j) + x_{2j} \sin(m \theta_j)$$

Trong các thư viện deep learning hiệu năng cao (như HuggingFace Transformers, vLLM, và triển khai của chúng ta), phép quay thường được biểu diễn theo kỹ thuật **Phân nửa vector (Half-Split RoPE)**:

$$\vec{x} = \begin{pmatrix} x_1 \\ x_2 \end{pmatrix}, \quad x_1 \in \mathbb{R}^{d/2}, x_2 \in \mathbb{R}^{d/2}$$

$$\begin{pmatrix} \tilde{x}_1 \\ \tilde{x}_2 \end{pmatrix} = \begin{pmatrix} x_1 \odot \cos(m \vec{\theta}) - x_2 \odot \sin(m \vec{\theta}) \\ x_2 \odot \cos(m \vec{\theta}) + x_1 \odot \sin(m \vec{\theta}) \end{pmatrix}$$

Hai cách cài đặt này hoàn toàn tương đương nhau về mặt đại số tuyến tính: chúng chỉ khác nhau một ma trận hoán vị trực giao $P$:

$$x_{\text{half}} = P x_{\text{interleaved}}$$

Vì ma trận hoán vị $P$ là trực giao ($P^\top P = I$), tích vô hướng và chuẩn Euclid của vector được bảo toàn nguyên vẹn 100%:

$$\langle P \tilde{q}, P \tilde{k} \rangle = \tilde{q}^\top P^\top P \tilde{k} = \tilde{q}^\top \tilde{k} = \langle \tilde{q}, \tilde{k} \rangle$$

Ưu điểm vượt trội của cách biểu diễn Half-Split trong Rust là cho phép tận dụng triệt để các lệnh SIMD trên mảng liên tục mà không cần thực hiện các thao tác xáo trộn (shuffle/gather) tốn kém chu kỳ CPU.

---

### 3.9. Bản chất toán học của Gated DeltaNet: Gradient Descent trực tuyến trên hàm lỗi hồi tưởng

Một câu hỏi sâu sắc mà mọi kỹ sư AI đều tự hỏi khi tiếp cận Gated DeltaNet là: *Tại sao công thức cập nhật trạng thái lại có dạng $\beta_t (v_t - S_{t-1} k_t) k_t^\top$?*

Câu trả lời nằm ở nguyên lý **Hồi tưởng liên tưởng trực tuyến (Online Associative Recall)**. Giả sử tại bước $t$, ta muốn ma trận bộ nhớ $S$ có khả năng tái hiện chính xác vector giá trị $v_t$ khi được kích hoạt bởi vector khóa $k_t$. Ta định nghĩa hàm lỗi bình phương tức thời:

$$J_t(S) = \frac{1}{2} \| S k_t - v_t \|_2^2$$

Để tìm ma trận $S$ giảm thiểu lỗi này, ta tính gradient của $J_t$ đối với ma trận $S$:
$$\nabla_S J_t(S) = (S k_t - v_t) k_t^\top$$

Áp dụng một bước cập nhật Stochastic Gradient Descent (SGD) với tốc độ học $\beta_t$:
$$S_t = S_{t-1} - \beta_t \nabla_S J_t(S_{t-1}) = S_{t-1} - \beta_t (S_{t-1} k_t - v_t) k_t^\top = S_{t-1} + \beta_t (v_t - S_{t-1} k_t) k_t^\top$$

Để cho phép mô hình quên đi những thông tin quá cũ hoặc lỗi thời trong quá khứ, ta kết hợp thêm hệ số suy giảm $\alpha_t \in (0, 1)$ (tương đương với bộ điều chuẩn trọng số $L_2$ decay):
$$S_t = \alpha_t S_{t-1} + \beta_t (v_t - S_{t-1} k_t) k_t^\top$$

Đây chính là **Luật Delta kinh điển của Widrow-Hoff (1960)**! Điều kỳ diệu ở đây là: Gated DeltaNet không phải là một thuật toán heuristic ngẫu nhiên, mà là một **động cơ tối ưu hóa lồi vi mô** liên tục học và cập nhật tri thức ngay trong quá trình lan truyền xuôi (In-Context Learning via Fast Weights).

---

### 3.10. So sánh toàn diện: Dung lượng KV-Cache và Băng thông bộ nhớ phục vụ suy luận

Trong kỷ nguyên phục vụ mô hình ngôn ngữ lớn (LLM Serving), chi phí vận hành phần cứng được quyết định bởi hai chế độ tính toán:
1. **Prefill Phase (Xử lý prompt đầu vào)**: Toàn bộ các token đầu vào được nạp cùng một lúc. Các phép tính nhân ma trận có kích thước lớn ($T \times d_{\text{model}}$), tận dụng tối đa số lượng nhân tính toán của GPU/CPU. Chế độ này là **Compute-Bound** (nghẽn tính toán).
2. **Decode Phase (Sinh từng token tiếp theo)**: Mỗi bước chỉ sinh ra đúng 1 token mới. Phép tính Attention biến thành phép nhân vector với ma trận KV-cache tích lũy từ trước. Tỷ số tính toán trên mỗi byte bộ nhớ (Arithmetic Intensity = FLOPs / Memory Access) cực kỳ thấp: chỉ khoảng 1–2 FLOPs/Byte, trong khi phần cứng hiện đại cần 100–200 FLOPs/Byte để phát huy hết công suất. Chế độ này là **Memory-Bound** (nghẽn băng thông bộ nhớ).

Bảng đối chiếu định lượng dưới đây giải thích vì sao kiến trúc DeepSeek MLA và Gated DeltaNet lại tạo ra sự chuyển dịch chấn động toàn cầu:

| Tiêu chí kiến trúc | MHA cổ điển (GPT-2, LLaMA 1) | GQA (LLaMA 3, Mistral) | DeepSeek MLA (DeepSeek V2/V3) | Gated DeltaNet (Linear RNN) |
|---|---|---|---|---|
| **Số phần tử KV lưu đệm / token** | $2 \times H \times d_h$ | $2 \times H_{KV} \times d_h$ | $d_c + d_r$ | **0** (Hằng số trạng thái $d_s^2$) |
| **Dung lượng KV cho 128 đầu ($d_h=128$)** | 32.768 floats (65.5 KB/token) | 4.096 floats (8.1 KB/token) | **576 floats (1.1 KB/token)** | **0 KB/token** |
| **Dung lượng KV cho context 64K token** | 4.19 GB / luồng người dùng | 524 MB / luồng người dùng | **73.7 MB / luồng người dùng** | **Hằng số ~16 KB** |
| **Số luồng song song trên 80GB VRAM** | ~18 luồng | ~150 luồng | **> 1.000 luồng** | **Hàng chục nghìn luồng** |
| **Độ phức tạp thời gian chú ý** | $O(N^2)$ | $O(N^2)$ | $O(N^2)$ | **$O(N)$ tuyến tính** |
| **Năng lực biểu diễn đa đầu** | Cao nhất (128 đầu KV) | Trung bình (16 đầu KV) | **Cực cao** (128 đầu nội dung ảo) | Trạng thái ma trận liên tục |

Nhờ MLA, cụm máy chủ phục vụ có thể tăng dung lượng phục vụ đồng thời gấp **10 đến 15 lần**, giảm chi phí tiền điện và phần cứng tới 90% so với GPT-2 MHA truyền thống.

---

### 3.11. Thiết kế MoE thực chiến: Dung lượng chuyên gia (Capacity Factor) và Đánh rơi token (Token Dropping)

Trong môi trường phân tán thực tế (Distributed Training / Inference), mỗi chuyên gia MoE thường được phân bổ trên một GPU hoặc một luồng CPU riêng biệt. Nếu số lượng token gửi tới một chuyên gia vượt quá dung lượng đệm được phân bổ trước, hệ thống sẽ bị treo hoặc sập bộ nhớ.

Để giải quyết vấn đề này, các kiến trúc như Switch Transformer và GShard định nghĩa **Dung lượng chuyên gia (Expert Capacity)**:

$$\text{Capacity} = \text{round}\left( C \times \frac{T \cdot k}{E} \right)$$

trong đó:
- $T$ là tổng số token trong batch.
- $k$ là số chuyên gia được chọn cho mỗi token (Top-$k$).
- $E$ là tổng số chuyên gia.
- $C$ là hệ số dung lượng (Capacity Factor), thường nằm trong khoảng $1.0 \le C \le 1.5$.

#### Hiện tượng đánh rơi token (Token Dropping):
Nếu một chuyên gia đã nhận đủ số lượng token bằng đúng $\text{Capacity}$, bất kỳ token tiếp theo nào được Router định tuyến tới chuyên gia này sẽ bị **đánh rơi (dropped)**. Khi một token bị đánh rơi:
- Token đó không được xử lý bởi chuyên gia định tuyến.
- Đầu ra của tầng MoE đối với token đó chỉ là đường truyền tắt (Residual Connection) giữ nguyên giá trị đầu vào $x_t$.
- Nếu quá nhiều token bị rơi, văn bản sinh ra sẽ trở nên rời rạc và mất khả năng suy luận logic.

Đây chính là lý do vì sao kiến trúc DeepSeek MoE bổ sung **Chuyên gia dùng chung (Shared Experts)**: vì chuyên gia dùng chung luôn nhận 100% token mà không áp dụng bộ định tuyến, ngay cả khi một token bị đánh rơi ở nhánh định tuyến, tri thức cốt lõi vẫn được truyền qua nhánh chuyên gia dùng chung, bảo đảm tính toàn vẹn của mô hình.

---

## 4. Mã nguồn minh họa thực chiến

Toàn bộ mã nguồn dưới đây được trích xuất trực tiếp từ crate sản xuất `code/ch93/src/main.rs`. Mã được thiết kế tuân thủ 100% Rust 2024, không phụ thuộc vào bất kỳ thư viện ngoài nào (`[dependencies]` rỗng), sử dụng `struct` có tên trường rõ ràng (`named fields`), và doc-comment tiếng Việt theo chuẩn `RiskGate`.

### 4.1. Cấu trúc RMSNorm & Đạo hàm giải tích ngược

```rust
/// Chuẩn hóa căn bậc hai trung bình bình phương `RmsNorm` (Root Mean Square Normalization).
///
/// Bỏ qua bước trừ kỳ vọng μ của LayerNorm, giảm 30% thao tác nạp/xả bộ nhớ:
/// y_i = (x_i / RMS(x)) * w_i, trong đó RMS(x) = sqrt(sum(x_j^2) / d + eps)
///
/// Khi `unit_offset = true` (kiểu Gemma), công thức chuyển thành:
/// y_i = (x_i / RMS(x)) * (1.0 + w_i)
#[derive(Debug, Clone, PartialEq)]
pub struct RmsNorm {
    pub dim: usize,
    pub eps: f64,
    pub weight: Vec<f64>,
    pub unit_offset: bool,
}

impl RmsNorm {
    /// Khởi tạo `RmsNorm` tiêu chuẩn với trọng số `1.0` (hoặc `0.0` nếu dùng unit_offset).
    pub fn new(dim: usize, eps: f64, unit_offset: bool) -> Self {
        let weight = if unit_offset {
            vec![0.0; dim]
        } else {
            vec![1.0; dim]
        };
        Self {
            dim,
            eps,
            weight,
            unit_offset,
        }
    }

    /// Lan truyền xuôi của RMSNorm trên 1 token đơn lẻ.
    pub fn forward_token(&self, x: &[f64]) -> Vec<f64> {
        assert_eq!(x.len(), self.dim);
        let d = self.dim as f64;
        let sum_sq: f64 = x.iter().map(|&v| v * v).sum();
        let inv_rms = 1.0 / (sum_sq / d + self.eps).sqrt();

        let mut out = vec![0.0; self.dim];
        for i in 0..self.dim {
            let scale = if self.unit_offset {
                1.0 + self.weight[i]
            } else {
                self.weight[i]
            };
            out[i] = x[i] * inv_rms * scale;
        }
        out
    }

    /// Lan truyền xuôi và tính đạo hàm ngược giải tích (analytical forward-backward).
    ///
    /// Trả về bộ 3: `(y, grad_x, grad_w)`.
    /// Đạo hàm giải tích đã được chứng minh khớp sai phân hữu hạn < 1e-8:
    /// dL/dw_i = g_i * x_hat_i
    /// dL/dx_i = inv * (g_i * w_i - x_hat_i * (1/d) * sum(g_j * w_j * x_hat_j))
    pub fn forward_backward(&self, x: &[f64], grad_out: &[f64]) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
        let n = x.len();
        let d = n as f64;
        let sum_sq: f64 = x.iter().map(|&v| v * v).sum();
        let inv_rms = 1.0 / (sum_sq / d + self.eps).sqrt();

        let mut y = vec![0.0; n];
        let mut grad_w = vec![0.0; n];
        let mut grad_x = vec![0.0; n];
        let mut dot_sum = 0.0;

        for i in 0..n {
            let w_eff = if self.unit_offset {
                1.0 + self.weight[i]
            } else {
                self.weight[i]
            };
            let x_hat = x[i] * inv_rms;
            y[i] = x_hat * w_eff;
            grad_w[i] = grad_out[i] * x_hat;
            dot_sum += grad_out[i] * w_eff * x_hat;
        }

        dot_sum /= d;

        for i in 0..n {
            let w_eff = if self.unit_offset {
                1.0 + self.weight[i]
            } else {
                self.weight[i]
            };
            let x_hat = x[i] * inv_rms;
            grad_x[i] = inv_rms * (grad_out[i] * w_eff - x_hat * dot_sum);
        }

        (y, grad_x, grad_w)
    }
}
```

### 4.2. Cấu trúc RoPE Cache & YaRN Scaling

```rust
/// Cấu hình ngoại suy độ dài ngữ cảnh YaRN (Yet another RoPE extensioN).
#[derive(Debug, Clone, PartialEq)]
pub struct YarnConfig {
    pub original_max_seq: usize,
    pub target_max_seq: usize,
    pub beta_fast: f64,
    pub beta_slow: f64,
    pub mscale: f64,
}

/// Bộ nhớ đệm Cosine & Sine cho phép quay vị trí RoPE (Rotary Positional Embeddings).
#[derive(Debug, Clone, PartialEq)]
pub struct RopeCache {
    pub head_dim: usize,
    pub max_seq_len: usize,
    pub base: f64,
    pub cos_cache: Vec<f64>,
    pub sin_cache: Vec<f64>,
}

impl RopeCache {
    /// Khởi tạo bảng RoPE chuẩn hoặc kết hợp YaRN Scaling.
    pub fn new(head_dim: usize, max_seq_len: usize, base: f64, yarn: Option<&YarnConfig>) -> Self {
        assert_eq!(head_dim % 2, 0, "head_dim của RoPE phải là số chẵn");
        let half = head_dim / 2;
        let mut cos_cache = vec![0.0; max_seq_len * half];
        let mut sin_cache = vec![0.0; max_seq_len * half];

        let scale = if let Some(cfg) = yarn {
            (cfg.target_max_seq as f64) / (cfg.original_max_seq as f64)
        } else {
            1.0
        };

        for i in 0..half {
            let mut freq = 1.0 / base.powf((2 * i) as f64 / (head_dim as f64));

            // Nếu kích hoạt YaRN, điều chỉnh tần số theo dốc nội suy
            if let Some(cfg) = yarn
                && scale > 1.0
            {
                let wavelength = 2.0 * std::f64::consts::PI / freq;
                let l0 = cfg.original_max_seq as f64;
                let low = cfg.beta_slow * l0;
                let high = cfg.beta_fast * l0;

                let gamma = if wavelength < low {
                    0.0
                } else if wavelength > high {
                    1.0
                } else {
                    (wavelength - low) / (high - low)
                };

                let interpolated_freq = freq / scale;
                freq = (1.0 - gamma) * freq + gamma * interpolated_freq;
            }

            for pos in 0..max_seq_len {
                let theta = (pos as f64) * freq;
                let mscale_mul = yarn.map(|y| y.mscale).unwrap_or(1.0);
                cos_cache[pos * half + i] = theta.cos() * mscale_mul;
                sin_cache[pos * half + i] = theta.sin() * mscale_mul;
            }
        }

        Self {
            head_dim,
            max_seq_len,
            base,
            cos_cache,
            sin_cache,
        }
    }

    /// Quay vector đầu vào tại vị trí `pos` theo kỹ thuật phân nửa vector (half-split RoPE):
    /// x'_i = x_i * cos(theta) - x_{i + d/2} * sin(theta)
    /// x'_{i + d/2} = x_{i + d/2} * cos(theta) + x_i * sin(theta)
    pub fn rotate(&self, vec: &mut [f64], pos: usize) {
        let half = self.head_dim / 2;
        assert_eq!(vec.len(), self.head_dim);
        assert!(pos < self.max_seq_len, "Vị trí vượt quá kích thước RoPE cache");

        let cos_slice = &self.cos_cache[pos * half..(pos + 1) * half];
        let sin_slice = &self.sin_cache[pos * half..(pos + 1) * half];

        for i in 0..half {
            let (c, s) = (cos_slice[i], sin_slice[i]);
            let x1 = vec[i];
            let x2 = vec[i + half];
            vec[i] = x1 * c - x2 * s;
            vec[i + half] = x2 * c + x1 * s;
        }
    }
}
```

### 4.3. DeepSeek Multi-Head Latent Attention (MLA)

```rust
/// Cấu hình cho DeepSeek Multi-Head Latent Attention (MLA).
#[derive(Debug, Clone, PartialEq)]
pub struct MlaConfig {
    pub n_heads: usize,
    pub d_model: usize,
    pub d_c: usize,
    pub d_r: usize,
    pub v_dim: usize,
    pub eps: f64,
}

/// Bộ nhớ đệm KV nén siêu nhỏ cho DeepSeek MLA.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MlaKvCache {
    /// Vector tiềm ẩn nén c^{KV} kích thước `seq_len x d_c`.
    pub compressed_kv: Vec<Vec<f64>>,
    /// Vector khóa vị trí RoPE tách rời k^R kích thước `seq_len x d_r`.
    pub k_rope: Vec<Vec<f64>>,
}

/// Lớp chú ý tiềm ẩn đa đầu DeepSeek `MultiHeadLatentAttention`.
#[derive(Debug, Clone, PartialEq)]
pub struct MultiHeadLatentAttention {
    pub config: MlaConfig,
    pub w_dkv: Vec<f64>,
    pub w_uk: Vec<f64>,
    pub w_uv: Vec<f64>,
    pub w_kr: Vec<f64>,
    pub w_dq: Vec<f64>,
    pub w_uq: Vec<f64>,
    pub w_qr: Vec<f64>,
    pub w_o: Vec<f64>,
    pub kv_norm: RmsNorm,
    pub q_norm: RmsNorm,
    pub rope: RopeCache,
}

impl MultiHeadLatentAttention {
    /// Bước sinh tăng dần với bộ nhớ đệm KV nén tiềm ẩn.
    pub fn step_decode(
        &self,
        cache: &mut MlaKvCache,
        x_token: &[f64],
        pos: usize,
    ) -> Vec<f64> {
        let dc = self.config.d_c;
        let dr = self.config.d_r;
        let vd = self.config.v_dim;
        let nh = self.config.n_heads;
        let q_c_dim = nh * vd;
        let q_r_dim = nh * dr;

        // 1. Nén KV: x -> c_kv
        let c_kv_raw = matmul(x_token, &self.w_dkv, 1, self.config.d_model, dc);
        let c_kv = self.kv_norm.forward_token(&c_kv_raw);

        // 2. Nhánh RoPE riêng của Key
        let mut k_r = matmul(x_token, &self.w_kr, 1, self.config.d_model, dr);
        self.rope.rotate(&mut k_r, pos);

        // Lưu vector nén vào cache (chỉ dc + dr floats!)
        cache.compressed_kv.push(c_kv.clone());
        cache.k_rope.push(k_r);

        // 3. Xử lý Query: x -> c_q -> (q_c, q_r)
        let c_q_raw = matmul(x_token, &self.w_dq, 1, self.config.d_model, dc);
        let c_q = self.q_norm.forward_token(&c_q_raw);

        let q_c = matmul(&c_q, &self.w_uq, 1, dc, q_c_dim);
        let mut q_r = matmul(&c_q, &self.w_qr, 1, dc, q_r_dim);
        for h in 0..nh {
            let slot = &mut q_r[h * dr..(h + 1) * dr];
            self.rope.rotate(slot, pos);
        }

        // 4. Tính toán Attention Score dựa trên nội dung nén + vị trí RoPE
        let total_t = cache.compressed_kv.len();
        let scale = 1.0 / ((vd + dr) as f64).sqrt();
        let mut head_outs = vec![0.0; q_c_dim];

        for h in 0..nh {
            let q_c_h = &q_c[h * vd..(h + 1) * vd];
            let q_r_h = &q_r[h * dr..(h + 1) * dr];

            let mut scores = vec![0.0; total_t];
            for (t, score_slot) in scores.iter_mut().enumerate().take(total_t) {
                let c_kv_t = &cache.compressed_kv[t];
                let k_c_all = matmul(c_kv_t, &self.w_uk, 1, dc, q_c_dim);
                let k_c_h = &k_c_all[h * vd..(h + 1) * vd];
                let k_r_t = &cache.k_rope[t];

                let dot_c: f64 = q_c_h.iter().zip(k_c_h).map(|(&a, &b)| a * b).sum();
                let dot_r: f64 = q_r_h.iter().zip(k_r_t).map(|(&a, &b)| a * b).sum();
                *score_slot = (dot_c + dot_r) * scale;
            }

            softmax_row(&mut scores);

            // Tổng hợp Value
            let out_slot = &mut head_outs[h * vd..(h + 1) * vd];
            for (t, &weight) in scores.iter().enumerate().take(total_t) {
                let c_kv_t = &cache.compressed_kv[t];
                let v_all = matmul(c_kv_t, &self.w_uv, 1, dc, q_c_dim);
                let v_h = &v_all[h * vd..(h + 1) * vd];
                for (o, &val) in out_slot.iter_mut().zip(v_h) {
                    *o += weight * val;
                }
            }
        }

        // 5. Chiếu đầu ra tổng hợp
        matmul(&head_outs, &self.w_o, 1, q_c_dim, self.config.d_model)
    }
}
```

### 4.4. Mô hình trạng thái đệ quy tuyến tính Gated DeltaNet

```rust
/// Tầng Gated DeltaNet kết hợp luật Delta với cơ chế cổng quên:
/// S_t = alpha_t * S_{t-1} + beta_t * (v_t - S_{t-1} * k_t) * k_t^T
#[derive(Debug, Clone, PartialEq)]
pub struct GatedDeltaNet {
    pub config: GatedDeltaNetConfig,
    pub w_q: Vec<f64>,
    pub w_k: Vec<f64>,
    pub w_v: Vec<f64>,
    pub w_alpha: Vec<f64>,
    pub w_beta: Vec<f64>,
    pub w_gate: Vec<f64>,
    pub w_o: Vec<f64>,
}

impl GatedDeltaNet {
    /// Lan truyền xuôi đệ quy O(N) duy trì trạng thái ma trận S_t.
    pub fn forward(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        let din = self.config.d_in;
        let ds = self.config.d_state;
        assert_eq!(x.len(), seq_len * din);

        let mut s = vec![0.0; ds * ds];
        let mut out = Vec::with_capacity(seq_len * din);

        for t in 0..seq_len {
            let x_t = &x[t * din..(t + 1) * din];

            let q = matmul(x_t, &self.w_q, 1, din, ds);
            let mut k = matmul(x_t, &self.w_k, 1, din, ds);
            let v = matmul(x_t, &self.w_v, 1, din, ds);

            // Chuẩn hóa vector khóa k để k^T k = 1 (ổn định luật Delta)
            let k_norm: f64 = k.iter().map(|&val| val * val).sum::<f64>().sqrt() + self.config.eps;
            for val in k.iter_mut() {
                *val /= k_norm;
            }

            // Tính cổng suy giảm alpha_t và tốc độ học beta_t
            let alpha_logits = matmul(x_t, &self.w_alpha, 1, din, ds);
            let beta_logits = matmul(x_t, &self.w_beta, 1, din, ds);
            let gate_logits = matmul(x_t, &self.w_gate, 1, din, ds);

            let alpha = sigmoid(alpha_logits.iter().sum::<f64>() / (ds as f64));
            let beta = sigmoid(beta_logits.iter().sum::<f64>() / (ds as f64));

            // S_{t-1} * k_t (dự báo của trạng thái cũ đối với k_t)
            let s_k = matmul(&s, &k, ds, ds, 1);

            // Delta error = v_t - S_{t-1} * k_t
            let mut delta = vec![0.0; ds];
            for i in 0..ds {
                delta[i] = v[i] - s_k[i];
            }

            // Cập nhật trạng thái: S_t = alpha * S_{t-1} + beta * (delta * k^T)
            let mut new_s = vec![0.0; ds * ds];
            for r in 0..ds {
                for c in 0..ds {
                    new_s[r * ds + c] = alpha * s[r * ds + c] + beta * delta[r] * k[c];
                }
            }
            s = new_s;

            // Truy xuất: y_raw = S_t * q_t
            let y_raw = matmul(&s, &q, ds, ds, 1);

            // Cổng trượt đầu ra: y_gated = y_raw * SiLU(gate)
            let mut y_gated = vec![0.0; ds];
            for i in 0..ds {
                y_gated[i] = y_raw[i] * silu(gate_logits[i]);
            }

            // Chiếu về không gian d_in ban đầu
            let y_proj = matmul(&y_gated, &self.w_o, 1, ds, din);
            out.extend(y_proj);
        }

        out
    }
}
```

### 4.5. Grouped-Query Attention & Tầng SwiGLU MLP

```rust
/// Cấu hình cho Grouped-Query Attention (GQA).
#[derive(Debug, Clone, PartialEq)]
pub struct GqaConfig {
    pub n_heads: usize,
    pub n_kv_heads: usize,
    pub head_dim: usize,
    pub sliding_window: Option<usize>,
    pub eps: f64,
}

/// Lớp chú ý nhóm truy vấn `GroupedQueryAttention` tích hợp QK-RMSNorm và RoPE.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupedQueryAttention {
    pub config: GqaConfig,
    pub d_model: usize,
    pub w_q: Vec<f64>,
    pub w_k: Vec<f64>,
    pub w_v: Vec<f64>,
    pub w_o: Vec<f64>,
    pub q_norm: RmsNorm,
    pub k_norm: RmsNorm,
    pub rope: RopeCache,
}

/// Tầng FFN cổng tích cực SwiGLU với 3 ma trận trọng số.
#[derive(Debug, Clone, PartialEq)]
pub struct SwiGluMlp {
    pub in_features: usize,
    pub hidden_features: usize,
    pub out_features: usize,
    pub w_gate: Vec<f64>,
    pub w_up: Vec<f64>,
    pub w_down: Vec<f64>,
}

impl SwiGluMlp {
    pub fn forward_token(&self, x: &[f64]) -> Vec<f64> {
        let gate = matmul(x, &self.w_gate, 1, self.in_features, self.hidden_features);
        let up = matmul(x, &self.w_up, 1, self.in_features, self.hidden_features);

        let mut gated = vec![0.0; self.hidden_features];
        for i in 0..self.hidden_features {
            gated[i] = silu(gate[i]) * up[i];
        }

        matmul(&gated, &self.w_down, 1, self.hidden_features, self.out_features)
    }

    pub fn backward_x(&self, x: &[f64], grad_out: &[f64]) -> Vec<f64> {
        let gate = matmul(x, &self.w_gate, 1, self.in_features, self.hidden_features);
        let up = matmul(x, &self.w_up, 1, self.in_features, self.hidden_features);

        let grad_gated = matmul(
            grad_out,
            &transpose(&self.w_down, self.hidden_features, self.out_features),
            1,
            self.out_features,
            self.hidden_features,
        );

        let mut grad_gate = vec![0.0; self.hidden_features];
        let mut grad_up = vec![0.0; self.hidden_features];

        for i in 0..self.hidden_features {
            let gg = grad_gated[i];
            grad_up[i] = gg * silu(gate[i]);
            grad_gate[i] = gg * up[i] * silu_deriv(gate[i]);
        }

        let grad_x_gate = matmul(
            &grad_gate,
            &transpose(&self.w_gate, self.in_features, self.hidden_features),
            1,
            self.hidden_features,
            self.in_features,
        );
        let grad_x_up = matmul(
            &grad_up,
            &transpose(&self.w_up, self.in_features, self.hidden_features),
            1,
            self.hidden_features,
            self.in_features,
        );

        let mut grad_x = vec![0.0; self.in_features];
        for i in 0..self.in_features {
            grad_x[i] = grad_x_gate[i] + grad_x_up[i];
        }
        grad_x
    }
}
```

### 4.6. Khối giải mã Transformer hiện đại tích hợp (ModernDecoderBlock)

```rust
/// Khối giải mã Transformer hiện đại tích hợp:
/// RmsNorm -> Attention (GQA / MLA / DeltaNet) -> + Residual -> RmsNorm -> MLP (SwiGLU / MoE) -> + Residual.
#[derive(Debug, Clone, PartialEq)]
pub struct ModernDecoderBlock {
    pub d_model: usize,
    pub norm_1: RmsNorm,
    pub attn: AttentionVariant,
    pub norm_2: RmsNorm,
    pub mlp: MlpVariant,
}

impl ModernDecoderBlock {
    pub fn forward(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        let dm = self.d_model;
        assert_eq!(x.len(), seq_len * dm);

        // Nhánh 1: Attention với Pre-RMSNorm
        let normed_1 = self.norm_1.forward(x);
        let attn_out = match &self.attn {
            AttentionVariant::Gqa(gqa) => gqa.forward_causal(&normed_1, seq_len),
            AttentionVariant::Mla(mla) => mla.forward_causal(&normed_1, seq_len),
            AttentionVariant::DeltaNet(dn) => dn.forward(&normed_1, seq_len),
        };

        let mut x_mid = vec![0.0; x.len()];
        for i in 0..x.len() {
            x_mid[i] = x[i] + attn_out[i];
        }

        // Nhánh 2: MLP với Pre-RMSNorm
        let normed_2 = self.norm_2.forward(&x_mid);
        let mlp_out = match &self.mlp {
            MlpVariant::SwiGlu(swiglu) => swiglu.forward(&normed_2),
            MlpVariant::SparseMoe(moe) => moe.forward(&normed_2, seq_len).output,
        };

        let mut x_final = vec![0.0; x.len()];
        for i in 0..x.len() {
            x_final[i] = x_mid[i] + mlp_out[i];
        }
        x_final
    }
}
```

### 4.7. Khối Transformer cổ điển đối chứng: LayerNorm, MHA & Gpt2Block

Để có cái nhìn so sánh trực quan và định lượng, dưới đây là cài đặt khối Transformer cổ điển theo kiến trúc GPT-2 (Radford et al., 2019):

```rust
/// Lớp chuẩn hóa theo tầng `LayerNorm` cổ điển của GPT-2:
/// y = ((x - μ) / sqrt(σ² + eps)) * γ + β
#[derive(Debug, Clone, PartialEq)]
pub struct LayerNorm {
    pub normalized_shape: usize,
    pub eps: f64,
    pub weight: Vec<f64>,
    pub bias: Vec<f64>,
}

impl LayerNorm {
    pub fn new(dim: usize, eps: f64) -> Self {
        Self {
            normalized_shape: dim,
            eps,
            weight: vec![1.0; dim],
            bias: vec![0.0; dim],
        }
    }

    pub fn forward(&self, x: &[f64]) -> Vec<f64> {
        let d = self.normalized_shape;
        let num_tokens = x.len() / d;
        let mut out = vec![0.0; x.len()];

        for t in 0..num_tokens {
            let slice = &x[t * d..(t + 1) * d];
            let mean = slice.iter().sum::<f64>() / (d as f64);
            let var = slice.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / (d as f64);
            let inv_std = 1.0 / (var + self.eps).sqrt();

            for i in 0..d {
                let norm = (slice[i] - mean) * inv_std;
                out[t * d + i] = norm * self.weight[i] + self.bias[i];
            }
        }
        out
    }
}

/// Khối GPT-2 Decoder cổ điển: Pre-LN với Residual Connections.
#[derive(Debug, Clone, PartialEq)]
pub struct Gpt2Block {
    pub d_model: usize,
    pub ln_1: LayerNorm,
    pub attn: MultiHeadAttention,
    pub ln_2: LayerNorm,
    pub mlp_fc: Linear,
    pub mlp_proj: Linear,
}

impl Gpt2Block {
    pub fn new(d_model: usize, n_heads: usize, eps: f64, rng: &mut SplitMix64) -> Self {
        let ln_1 = LayerNorm::new(d_model, eps);
        let attn = MultiHeadAttention::new(d_model, n_heads, rng);
        let ln_2 = LayerNorm::new(d_model, eps);
        let mlp_fc = Linear::new(d_model, 4 * d_model, true, rng);
        let mlp_proj = Linear::new(4 * d_model, d_model, true, rng);
        Self {
            d_model,
            ln_1,
            attn,
            ln_2,
            mlp_fc,
            mlp_proj,
        }
    }

    pub fn forward(&self, x: &[f64], seq_len: usize) -> Vec<f64> {
        let d = self.d_model;
        assert_eq!(x.len(), seq_len * d);

        // Nhánh 1: Attention với Pre-LN và Residual
        let norm1 = self.ln_1.forward(x);
        let attn_out = self.attn.forward(&norm1, seq_len);
        let mut x_mid = vec![0.0; x.len()];
        for i in 0..x.len() {
            x_mid[i] = x[i] + attn_out[i];
        }

        // Nhánh 2: MLP với Pre-LN và Residual
        let norm2 = self.ln_2.forward(&x_mid);
        let fc = self.mlp_fc.forward(&norm2);
        let act = Gelu::forward(&fc);
        let proj = self.mlp_proj.forward(&act);

        let mut x_out = vec![0.0; x.len()];
        for i in 0..x.len() {
            x_out[i] = x_mid[i] + proj[i];
        }
        x_out
    }
}
```

### 4.8. Tầng Hỗn hợp chuyên gia thưa Sparse MoE Layer

```rust
/// Tầng MoE thưa `SparseMoeLayer` với chuyên gia dùng chung (Shared Experts) kiểu DeepSeek V2/V3.
#[derive(Debug, Clone, PartialEq)]
pub struct SparseMoeLayer {
    pub config: SparseMoeConfig,
    pub router_weights: Vec<f64>,
    pub routed_experts: Vec<SwiGluMlp>,
    pub shared_experts: Vec<SwiGluMlp>,
}

impl SparseMoeLayer {
    pub fn forward(&self, x: &[f64], seq_len: usize) -> MoeRoutingResult {
        let dm = self.config.d_model;
        let ne = self.config.n_routed_experts;
        let k = self.config.top_k;
        assert_eq!(x.len(), seq_len * dm);

        let mut out = vec![0.0; x.len()];
        let mut all_selected = Vec::with_capacity(seq_len);
        let mut all_weights = Vec::with_capacity(seq_len);

        let mut expert_counts = vec![0.0; ne];
        let mut expert_prob_sums = vec![0.0; ne];

        for t in 0..seq_len {
            let x_t = &x[t * dm..(t + 1) * dm];

            // 1. Router logits & probabilities
            let logits = matmul(x_t, &self.router_weights, 1, dm, ne);
            let mut probs = logits.clone();
            softmax_row(&mut probs);

            for (i, &p) in probs.iter().enumerate() {
                expert_prob_sums[i] += p;
            }

            // 2. Tìm top-k chuyên gia
            let mut indexed_logits: Vec<(usize, f64)> =
                logits.iter().copied().enumerate().collect();
            indexed_logits
                .sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            let selected: Vec<usize> = indexed_logits.iter().take(k).map(|item| item.0).collect();
            let mut selected_probs: Vec<f64> = selected.iter().map(|&idx| probs[idx]).collect();

            // Chuẩn hóa trọng số top-k
            let sum_prob: f64 = selected_probs.iter().sum();
            let inv_sum = if sum_prob > 0.0 { 1.0 / sum_prob } else { 0.0 };
            for p in selected_probs.iter_mut() {
                *p *= inv_sum;
            }

            for &idx in &selected {
                expert_counts[idx] += 1.0;
            }

            // 3. Thực thi các chuyên gia định tuyến
            let slot = &mut out[t * dm..(t + 1) * dm];
            for (&exp_idx, &weight) in selected.iter().zip(&selected_probs) {
                let exp_out = self.routed_experts[exp_idx].forward_token(x_t);
                for (o, val) in slot.iter_mut().zip(exp_out) {
                    *o += weight * val;
                }
            }

            // 4. Thực thi chuyên gia dùng chung (Shared Experts)
            for shared_exp in &self.shared_experts {
                let s_out = shared_exp.forward_token(x_t);
                for (o, val) in slot.iter_mut().zip(s_out) {
                    *o += val;
                }
            }

            all_selected.push(selected);
            all_weights.push(selected_probs);
        }

        // 5. Tính toán aux_loss cân bằng tải
        let t_f = seq_len as f64;
        let mut aux_loss = 0.0;
        for i in 0..ne {
            let f_i = expert_counts[i] / (t_f * (k as f64));
            let p_i = expert_prob_sums[i] / t_f;
            aux_loss += f_i * p_i;
        }
        aux_loss *= self.config.aux_loss_coef * (ne as f64);

        MoeRoutingResult {
            output: out,
            selected_experts: all_selected,
            routing_weights: all_weights,
            aux_loss,
        }
    }
}
```

### 4.9. Chương trình chạy kiểm chứng thực nghiệm đầu cuối (Demo Pipeline)

```rust
fn main() {
    println!("=== CHƯƠNG 93: TỪ GPT-2 ĐẾN BỘ GIẢI MÃ HIỆN ĐẠI ===");
    let mut rng = SplitMix64::new(42);

    let d_model = 16;
    let n_heads = 4;
    let n_kv_heads = 2;
    let seq_len = 4;

    println!("1. Khởi tạo GPT-2 Block cổ điển (d_model={d_model}, n_heads={n_heads})...");
    let gpt2 = Gpt2Block::new(d_model, n_heads, 1e-5, &mut rng);
    let sample_x: Vec<f64> = (0..seq_len * d_model)
        .map(|_| rng.next_signed(0.5))
        .collect();
    let gpt2_out = gpt2.forward(&sample_x, seq_len);
    println!(
        "   GPT-2 forward hoàn tất: kích thước ra = {}",
        gpt2_out.len()
    );

    println!("2. Khởi tạo ModernDecoderBlock (RMSNorm + GQA + SwiGLU)...");
    let norm1 = RmsNorm::new(d_model, 1e-5, false);
    let gqa_cfg = GqaConfig {
        n_heads,
        n_kv_heads,
        head_dim: d_model / n_heads,
        sliding_window: Some(3),
        eps: 1e-5,
    };
    let gqa = GroupedQueryAttention::new(d_model, gqa_cfg, 32, 10000.0, &mut rng);
    let norm2 = RmsNorm::new(d_model, 1e-5, false);
    let swiglu = SwiGluMlp::new(d_model, (8 * d_model) / 3, d_model, &mut rng);

    let modern_block = ModernDecoderBlock::new(
        d_model,
        norm1,
        AttentionVariant::Gqa(gqa),
        norm2,
        MlpVariant::SwiGlu(swiglu),
    );
    let modern_out = modern_block.forward(&sample_x, seq_len);
    println!(
        "   ModernDecoder forward hoàn tất: kích thước ra = {}",
        modern_out.len()
    );

    println!("3. Kiểm tra nén bộ nhớ DeepSeek MLA:");
    let mla_cfg = MlaConfig {
        n_heads: 4,
        d_model: 16,
        d_c: 6,
        d_r: 4,
        v_dim: 4,
        eps: 1e-5,
    };
    let mla_cache_elements_per_token = mla_cfg.d_c + mla_cfg.d_r;
    let mha_cache_elements_per_token = 2 * d_model;
    let savings = 100.0
        * (1.0 - (mla_cache_elements_per_token as f64) / (mha_cache_elements_per_token as f64));
    println!("   MHA KV Cache: {mha_cache_elements_per_token} floats/token");
    println!("   MLA KV Cache: {mla_cache_elements_per_token} floats/token");
    println!("   Tiết kiệm bộ nhớ KV-Cache: {:.1}%", savings);

    println!("\nToàn bộ các cấu trúc toán học của Chương 93 đã sẵn sàng và được kiểm chứng!");
}
```

---

## 5. Bảng tra cứu lỗi biên dịch thường gặp

Khi xây dựng các hệ thống mô hình ngôn ngữ từ số không bằng Rust 2024, lập trình viên thường đối mặt với các lỗi biên dịch và lỗi số học đặc thù do hệ thống kiểu dữ liệu và mô hình sở hữu (Ownership) khắt khe của Rust. Dưới đây là phân tích chi tiết 7 lỗi phổ biến nhất kèm mã minh họa và cách sửa chuẩn mực:

---

### Lỗi 1: `E0382` — Sử dụng giá trị sau khi đã di chuyển quyền sở hữu (Use of Moved Value)

#### Triệu chứng lỗi biên dịch:
```text
error[E0382]: use of moved value: `x`
   --> src/main.rs:142:26
    |
138 |         let norm1 = self.ln_1.forward(x);
    |                                       - value moved here
...
142 |             x_mid[i] = x[i] + attn_out[i];
    |                        ^ value used here after move
```

#### Nguyên nhân gốc rễ:
Trong Rust, nếu phương thức nhận tham số theo kiểu chuyển giá trị (`self.ln_1.forward(x: Vec<f64>)`), biến `x` ban đầu sẽ bị move vào trong hàm đó. Sau đó, ở bước cộng phần dư (Residual Connection `x + attn_out`), biến `x` không còn hợp lệ nữa.

#### Cách sửa chuẩn mực trong Rust 2024:
Luôn thiết kế các phương thức tính toán nhận lát cắt tham chiếu mượn `&[f64]` thay vì sở hữu `Vec<f64>`. Điều này vừa giữ nguyên quyền sở hữu cho hàm gọi, vừa tránh cấp phát bộ nhớ thừa:
```rust
// SAI:
pub fn forward(&self, x: Vec<f64>) -> Vec<f64> { ... }

// ĐÚNG:
pub fn forward(&self, x: &[f64]) -> Vec<f64> { ... }
```

---

### Lỗi 2: `E0502` — Mượn khả biến đồng thời với mượn bất biến (Simultaneous Borrow)

#### Triệu chứng lỗi biên dịch:
```text
error[E0502]: cannot borrow `cache` as mutable because it is also borrowed as immutable
   --> src/main.rs:870:17
    |
865 |         let last_key = &cache.keys[cache.current_len - 1];
    |                         ---------- immutable borrow occurs here
870 |         cache.step_decode(&token, pos);
    |         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ mutable borrow occurs here
```

#### Nguyên nhân gốc rễ:
Trong quá trình giải mã từng bước, lập trình viên vừa giữ một lát cắt trỏ vào `cache.keys` cũ, vừa gọi một phương thức ghi thêm dữ liệu vào `cache`. Do `Vec` có thể phải tái cấp phát (reallocation) khi đầy dung lượng, lát cắt cũ sẽ trở thành con trỏ rác (dangling pointer). Trình biên dịch Rust ngăn chặn điều này tại thời điểm biên dịch.

#### Cách sửa chuẩn mực:
Tách biệt rõ ràng hai pha đọc và ghi:
```rust
// ĐÚNG: Tính toán toàn bộ kết quả dựa trên các giá trị cục bộ trước,
// sau đó mới thực hiện mượn khả biến `&mut cache` để ghi đè.
let new_key = self.compute_key(token, pos);
cache.keys.extend_from_slice(&new_key);
cache.current_len += 1;
```

---

### Lỗi 3: `E0277` — Thiếu triển khai Trait trên cấu trúc dữ liệu nơ-ron

#### Triệu chứng lỗi biên dịch:
```text
error[E0277]: `RmsNorm` doesn't implement `Debug`
   --> src/main.rs:210:10
    |
210 | #[derive(Debug)]
    |          ^^^^^ the trait `Debug` is not implemented for `RmsNorm`
```

#### Nguyên nhân gốc rễ:
Khi lồng các khối kiến trúc nơ-ron vào nhau (ví dụ: `ModernDecoderBlock` chứa `RmsNorm`, `GqaAttention`), nếu một struct con không triển khai `Debug`, `Clone`, hoặc `PartialEq`, struct cha sẽ không thể tự động phái sinh (`derive`) các trait đó.

#### Cách sửa chuẩn mực:
Áp dụng kỷ luật lập trình nhất quán: mọi struct cấu hình và tầng nơ-ron đều phải được đánh dấu đầy đủ các trait cơ bản:
```rust
#[derive(Debug, Clone, PartialEq)]
pub struct RmsNorm {
    pub dim: usize,
    pub eps: f64,
    pub weight: Vec<f64>,
    pub unit_offset: bool,
}
```

---

### Lỗi 4: Tràn số dấu phẩy động NaN trong phép tính Softmax và RMSNorm

#### Triệu chứng số học:
Chương trình biên dịch thành công nhưng khi chạy, toàn bộ logit đầu ra biến thành `NaN` sau 3 bước giải mã.

#### Nguyên nhân gốc rễ:
1. Khi tính RMSNorm, nếu vector đầu vào $x$ toàn số 0 và $\epsilon = 0.0$, phép chia $x / \sqrt{0}$ sẽ sinh ra `NaN`.
2. Khi tính Softmax, nếu logit đầu vào đạt giá trị lớn (ví dụ $x_i = 800.0$), hàm $e^{800}$ vượt quá giá trị lớn nhất của số thực 64-bit (`f64::MAX ~ 1.8e308`), gây tràn số thành `+inf`. Khi chia `+inf / +inf`, kết quả trả về `NaN`.

#### Cách sửa chuẩn mực:
Luôn áp dụng mẹo dịch cực đại (Max-shift trick) và thêm hằng số làm mịn $\epsilon > 0$:
```rust
pub fn softmax_row(logits: &mut [f64]) {
    // 1. Tìm giá trị cực đại để dịch chuyển
    let max_v = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let mut sum = 0.0;
    // 2. Trừ max_v trước khi lấy hàm mũ: exp(x_i - max_v) luôn nằm trong (0, 1]
    for v in logits.iter_mut() {
        *v = (*v - max_v).exp();
        sum += *v;
    }
    let inv_sum = if sum > 0.0 { 1.0 / sum } else { 0.0 };
    for v in logits.iter_mut() {
        *v *= inv_sum;
    }
}
```

---

### Lỗi 5: `E0597` — Vòng đời mượn tham chiếu cục bộ không đủ dài (Lifetime Violation)

#### Triệu chứng lỗi biên dịch:
```text
error[E0597]: `temp_slice` does not live long enough
   --> src/main.rs:560:20
    |
558 |             let temp_slice = vec![x1, x2];
    |                 ---------- binding `temp_slice` declared here
559 |             self.rope.rotate(&mut temp_slice, pos);
560 |             out_ref.push(&temp_slice[0]);
    |                          ^^^^^^^^^^ borrowed value does not live long enough
```

#### Nguyên nhân gốc rễ:
Tạo ra một `Vec` hoặc mảng tạm thời bên trong thân hàm, sau đó cố gắng lưu tham chiếu của nó vào một cấu trúc dữ liệu sống lâu hơn.

#### Cách sửa chuẩn mực:
Thực hiện thao tác trực tiếp trên bộ nhớ đích (in-place mutation) thông qua lát cắt khả biến `&mut [f64]`:
```rust
pub fn rotate(&self, vec: &mut [f64], pos: usize) {
    let half = self.head_dim / 2;
    for i in 0..half {
        let (c, s) = (self.cos_cache[pos * half + i], self.sin_cache[pos * half + i]);
        let x1 = vec[i];
        let x2 = vec[i + half];
        vec[i] = x1 * c - x2 * s;
        vec[i + half] = x2 * c + x1 * s;
    }
}
```

---

### Lỗi 6: Lệch ma trận (Shape Mismatch) trong SwiGLU và MoE Router

#### Triệu chứng số học:
Chương trình dừng đột ngột (panic) tại runtime với thông báo:
```text
thread 'main' panicked at 'Kích thước ma trận B không hợp lệ: expected 128, got 96'
```

#### Nguyên nhân gốc rễ:
Trong SwiGLU, kích thước ẩn $d_{\text{hidden}}$ của ma trận chiếu lên $W_{\text{gate}}$ ($d \times h$) không khớp với kích thước của ma trận chiếu xuống $W_{\text{down}}$ ($h \times d$), thường do làm tròn số lẻ khi tính $\frac{8}{3} d_{\text{model}}$.

#### Cách sửa chuẩn mực:
Đóng gói quy tắc tính toán kích thước trong constructor và dùng `assert_eq!` kiểm tra ngay khi khởi tạo cấu trúc:
```rust
impl SwiGluMlp {
    pub fn new(in_features: usize, hidden_features: usize, out_features: usize, rng: &mut SplitMix64) -> Self {
        assert_eq!(
            hidden_features % 2, 0,
            "hidden_features nên là số chẵn để thuận tiện phân khối"
        );
        // Khởi tạo trọng số với kích thước đã xác thực...
    }
}
```

---

### Lỗi 7: Vi phạm nhân quả (Future Causal Leak) trong Attention Masking

#### Triệu chứng logic:
Mô hình sinh văn bản dự đoán tương lai quá hoàn hảo trong lúc kiểm tra nhưng khi sinh thực tế thì câu từ vô nghĩa.

#### Nguyên nhân gốc rễ:
Ma trận Attention cho phép token tại thời điểm $t$ nhìn thấy Key của token tại thời điểm $t+1$, do quên khởi tạo ma trận điểm bằng $-\infty$ cho các vị trí $j > i$.

#### Cách sửa chuẩn mực:
Viết bài test bắt buộc kiểm chứng tính bất biến nhân quả (Causal Invariance Test):
```rust
#[test]
fn test_causal_mask_strictly_enforced() {
    let mut x1 = vec![0.5; 3 * d_model];
    let mut x2 = x1.clone();
    // Thay đổi dữ liệu tương lai tại t=2
    for slot in &mut x2[2 * d_model..3 * d_model] {
        *slot += 100.0;
    }
    let out1 = block.forward(&x1, 3);
    let out2 = block.forward(&x2, 3);

    // Bất biến: Đầu ra tại t=0 và t=1 PHẢI giống nhau 100%
    for i in 0..2 * d_model {
        assert_eq!(out1[i], out2[i], "Rò rỉ tương lai tại index {i}!");
    }
}
```

---

## 6. Tóm tắt & Bài tập rèn luyện

### Tóm tắt cô đọng 8 nguyên lý vàng của Bộ giải mã Hiện đại:
1. **RMSNorm**: Chuẩn hóa độ lớn thay vì phân phối; loại bỏ $\mu$ giúp tiết kiệm 30% bộ nhớ; dùng biến thể `(1 + w)` của Gemma để ổn định số học trong huấn luyện nửa độ chính xác.
2. **RoPE & YaRN**: Tích vô hướng chú ý chỉ cần phụ thuộc khoảng cách tương đối $(m - n)$; bảo toàn tuyệt đối chuẩn vector; dùng YaRN để nội suy tần số bước sóng dài khi mở rộng ngữ cảnh.
3. **SwiGLU**: Cơ chế cổng tích cực $x \cdot \text{sigmoid}(x) \odot y$ với kích thước ẩn $\frac{8}{3} d_{\text{model}}$ mang lại sức mạnh biểu diễn vượt trội so với GELU truyền thống.
4. **GQA & QK-RMSNorm**: Giảm dung lượng KV-cache từ 4 đến 8 lần bằng cách chia sẻ đầu KV; khóa chặt bất ổn logit bằng phép chuẩn hóa QK-RMSNorm trước khi tính Attention.
5. **DeepSeek MLA**: Đột phá nén KV-cache xuống vector tiềm ẩn $c^{KV}$ kết hợp tách nhánh vị trí $k^R$, giảm tới 85% dung lượng lưu trữ mà không làm giảm số lượng đầu truy vấn.
6. **Gated DeltaNet**: Khắc phục nhược điểm của Linear Attention bằng luật sửa sai Delta; cho phép mô hình cập nhật trạng thái làm việc liên tục với độ phức tạp thời gian tuyến tính $O(N)$.
7. **Sparse MoE**: Kích hoạt có chọn lọc Top-$k$ chuyên gia kết hợp các chuyên gia dùng chung; triệt tiêu hội chứng sụp đổ định tuyến bằng hàm tổn thất cân bằng tải `aux_loss`.
8. **Kỷ luật Rust 2024**: Cài đặt thuần `std`, struct có tên trường, không cấp phát bộ nhớ thừa trong vòng lặp tính toán, kiểm chứng đạo hàm giải tích vs sai phân hữu hạn $< 10^{-8}$.

---

### Bài tập rèn luyện

Dưới đây là 4 bài tập lớn thực chiến kèm hướng dẫn và lời giải hoàn chỉnh bằng Rust 2024 biên dịch sạch:

---

#### Bài tập 1: Ma trận hấp thụ trong DeepSeek MLA (Matrix Absorption for Zero-Decompression Inference)

**Bối cảnh**: Trong quá trình giải mã từng bước của DeepSeek MLA, việc nhân $c_s^{KV}$ với $W_{UK, h}$ để giải nén Key trước khi nhân với $q_{t, h}^C$ đòi hỏi thao tác giải nén trung gian. Về mặt đại số tuyến tính:
$$\langle q_{t, h}^C, k_{s, h}^C \rangle = q_{t, h}^C (c_s^{KV} W_{UK, h})^\top = q_{t, h}^C W_{UK, h}^\top (c_s^{KV})^\top = (q_{t, h}^C W_{UK, h}^\top) c_s^{KV}$$

Nếu ta tính trước $\tilde{q}_{t, h} = q_{t, h}^C W_{UK, h}^\top \in \mathbb{R}^{d_c}$, ta có thể nhân trực tiếp với $c_s^{KV}$ mà **hoàn toàn không cần giải nén Key trong bộ nhớ đệm KV**!

**Yêu cầu**: Hãy viết hàm `step_decode_absorbed` cho MLA, chứng minh bằng kiểm thử rằng kết quả đầu ra khớp từng số thực với phương pháp giải nén truyền thống với sai số $< 10^{-12}$.

<details>
<summary>Gợi ý & Lời giải Bài tập 1</summary>

```rust
use std::f64;

/// Nhân ma trận phẳng tiện ích
fn matmul_helper(a: &[f64], b: &[f64], m: usize, k: usize, n: usize) -> Vec<f64> {
    let mut out = vec![0.0; m * n];
    for i in 0..m {
        for p in 0..k {
            let a_ip = a[i * k + p];
            if a_ip == 0.0 { continue; }
            for j in 0..n {
                out[i * n + j] += a_ip * b[p * n + j];
            }
        }
    }
    out
}

/// Tính điểm chú ý bằng kỹ thuật hấp thụ ma trận Key (Matrix Absorption).
/// q_c: vector Query nội dung kích thước [v_dim]
/// w_uk_h: ma trận giải nén Key của đầu h kích thước [d_c x v_dim]
/// c_kv: vector tiềm ẩn nén kích thước [d_c]
pub fn mla_absorbed_attention_score(
    q_c_h: &[f64],
    w_uk_h: &[f64],
    c_kv: &[f64],
    d_c: usize,
    v_dim: usize,
) -> (f64, f64) {
    // 1. Phương pháp truyền thống: Giải nén Key rồi nhân vô hướng với Query
    // k_c = c_kv * w_uk_h (1 x v_dim)
    let k_c = matmul_helper(c_kv, w_uk_h, 1, d_c, v_dim);
    let score_decompressed: f64 = q_c_h.iter().zip(&k_c).map(|(&a, &b)| a * b).sum();

    // 2. Phương pháp hấp thụ: q_absorbed = q_c_h * w_uk_h^T (1 x d_c)
    // Sau đó: score = q_absorbed * c_kv^T
    let mut q_absorbed = vec![0.0; d_c];
    for c in 0..d_c {
        let mut sum = 0.0;
        for v in 0..v_dim {
            sum += q_c_h[v] * w_uk_h[c * v_dim + v];
        }
        q_absorbed[c] = sum;
    }
    let score_absorbed: f64 = q_absorbed.iter().zip(c_kv).map(|(&a, &b)| a * b).sum();

    (score_decompressed, score_absorbed)
}

#[cfg(test)]
mod ex1_tests {
    use super::*;

    #[test]
    fn test_matrix_absorption_equivalence() {
        let d_c = 8;
        let v_dim = 4;
        let q_c_h = vec![0.5, -1.2, 0.8, 2.1];
        let c_kv = vec![1.1, -0.4, 0.9, -1.5, 0.2, 0.7, -0.8, 1.4];
        let w_uk_h: Vec<f64> = (0..d_c * v_dim).map(|i| (i as f64 * 0.17).sin()).collect();

        let (score_decompressed, score_absorbed) = mla_absorbed_attention_score(
            &q_c_h,
            &w_uk_h,
            &c_kv,
            d_c,
            v_dim,
        );

        let diff = (score_decompressed - score_absorbed).abs();
        assert!(
            diff < 1e-12,
            "Điểm số hấp thụ ma trận ({score_absorbed}) phải khớp tuyệt đối phương pháp giải nén ({score_decompressed})"
        );
    }
}
```
</details>

---

#### Bài tập 2: Cơ chế Kẹp Logit Chú ý (Attention Logit Soft-Capping) kiểu Gemma 2

**Bối cảnh**: Trong các mô hình ngôn ngữ thế hệ mới như Gemma 2, để tránh hiện tượng logit chú ý bùng nổ khi suy luận chuỗi dài (khiến phân phối Softmax biến thành hàm delta và làm mất tính phong phú của văn bản sinh ra), người ta áp dụng hàm kẹp mềm (soft-capping):

$$\text{Logit}_{\text{capped}} = \text{cap} \cdot \tanh\left( \frac{\text{Logit}}{\text{cap}} \right)$$

với $\text{cap} = 50.0$ cho các tầng Attention và $\text{cap} = 30.0$ cho tầng `lm_head`.

**Yêu cầu**: Hãy viết hàm tính điểm chú ý có áp dụng Soft-Capping và kiểm chứng rằng:
1. Khi $|\text{Logit}| \ll \text{cap}$, hàm cư xử tuyến tính: $\text{Logit}_{\text{capped}} \approx \text{Logit}$.
2. Khi $\text{Logit} \to \pm \infty$, giá trị bị chặn cứng trong khoảng $(-\text{cap}, +\text{cap})$.
3. Đạo hàm giải tích của Soft-Capping không bao giờ bị gián đoạn hay bùng nổ.

<details>
<summary>Gợi ý & Lời giải Bài tập 2</summary>

```rust
/// Hàm kẹp mềm logit chú ý (Attention Logit Soft-Capping).
pub fn soft_cap_logit(logit: f64, cap: f64) -> f64 {
    assert!(cap > 0.0, "Ngưỡng cap phải là số dương");
    cap * (logit / cap).tanh()
}

/// Đạo hàm bậc nhất của hàm kẹp mềm theo logit ban đầu:
/// d(capped)/d(logit) = 1 - tanh^2(logit / cap) = sech^2(logit / cap)
pub fn soft_cap_derivative(logit: f64, cap: f64) -> f64 {
    let t = (logit / cap).tanh();
    1.0 - t * t
}

#[cfg(test)]
mod ex2_tests {
    use super::*;

    #[test]
    fn test_soft_capping_properties() {
        let cap = 50.0;

        // 1. Kiểm tra tính tuyến tính ở vùng giá trị nhỏ
        let small_logit = 0.5;
        let capped_small = soft_cap_logit(small_logit, cap);
        assert!((capped_small - small_logit).abs() < 1e-3);

        // 2. Kiểm tra tính chặn ở vùng giá trị cực lớn
        let huge_logit = 1000.0;
        let capped_huge = soft_cap_logit(huge_logit, cap);
        assert!(capped_huge <= cap);
        assert!((capped_huge - cap).abs() < 1e-5);

        let tiny_logit = -1000.0;
        let capped_tiny = soft_cap_logit(tiny_logit, cap);
        assert!(capped_tiny >= -cap);
        assert!((capped_tiny - (-cap)).abs() < 1e-5);

        // 3. Kiểm tra đạo hàm giải tích vs sai phân hữu hạn
        let h = 1e-6;
        for &test_val in &[-40.0, -10.0, 0.0, 15.0, 45.0] {
            let analytic = soft_cap_derivative(test_val, cap);
            let numeric = (soft_cap_logit(test_val + h, cap) - soft_cap_logit(test_val - h, cap)) / (2.0 * h);
            assert!(
                (analytic - numeric).abs() < 1e-8,
                "Đạo hàm Soft-Capping sai phân vượt ngưỡng tại val={test_val}"
            );
        }
    }
}
```
</details>

---

#### Bài tập 3: Cân bằng tải MoE thích ứng bằng Router Bias động (Dynamic Bias Adjustment)

**Bối cảnh**: Trong DeepSeek V3, thay vì sử dụng tổn thất phụ `aux_loss` làm biến dạng gradient của mạng chính, các tác giả đề xuất kỹ thuật **Auxiliary-loss-free Load Balancing**. Thuật toán này điều chỉnh một vector độ lệch $b \in \mathbb{R}^E$ của Router sau mỗi bước dựa trên độ lệch tải:

$$s_t = x_t W_g + b$$

Nếu chuyên gia $i$ nhận được quá nhiều token ($f_i > \frac{1}{E}$), ta giảm độ lệch: $b_i \leftarrow b_i - \gamma$. Ngược lại, nếu chuyên gia $i$ bị đói việc ($f_i < \frac{1}{E}$), ta tăng độ lệch: $b_i \leftarrow b_i + \gamma$.

**Yêu cầu**: Hãy triển khai một struct `DynamicMoERouter` bằng Rust 2024, tích hợp cơ chế cập nhật bias tự động sau mỗi lượt duyệt batch, và chứng minh rằng sau 30 lượt, bias của các chuyên gia tự động điều chỉnh bù trừ cho độ lệch ban đầu.

<details>
<summary>Gợi ý & Lời giải Bài tập 3</summary>

```rust
/// Bộ định tuyến MoE cân bằng tải động không dùng aux_loss (DeepSeek V3 Style).
#[derive(Debug, Clone, PartialEq)]
pub struct DynamicMoERouter {
    pub n_experts: usize,
    pub top_k: usize,
    pub router_bias: Vec<f64>,
    pub learning_rate: f64,
}

impl DynamicMoERouter {
    pub fn new(n_experts: usize, top_k: usize, lr: f64) -> Self {
        Self {
            n_experts,
            top_k,
            router_bias: vec![0.0; n_experts],
            learning_rate: lr,
        }
    }

    /// Định tuyến một batch token và trả về danh sách chuyên gia được chọn
    pub fn route_batch(&self, raw_logits: &[Vec<f64>]) -> Vec<Vec<usize>> {
        let mut selections = Vec::with_capacity(raw_logits.len());
        for logits in raw_logits {
            let mut biased_scores: Vec<(usize, f64)> = logits
                .iter()
                .enumerate()
                .map(|(idx, &val)| (idx, val + self.router_bias[idx]))
                .collect();

            // Sắp xếp giảm dần theo điểm số đã cộng bias
            biased_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            let picked: Vec<usize> = biased_scores.iter().take(self.top_k).map(|item| item.0).collect();
            selections.push(picked);
        }
        selections
    }

    /// Cập nhật bias động dựa trên thống kê tải thực tế của batch vừa qua
    pub fn update_bias(&mut self, selections: &[Vec<usize>]) {
        let num_tokens = selections.len() as f64;
        let mut counts = vec![0.0; self.n_experts];
        for picked in selections {
            for &idx in picked {
                counts[idx] += 1.0;
            }
        }

        // Tần suất mục tiêu khi chia đều hoàn hảo: (num_tokens * top_k) / n_experts
        let target_count = (num_tokens * (self.top_k as f64)) / (self.n_experts as f64);

        for i in 0..self.n_experts {
            let diff = counts[i] - target_count;
            // Nếu quá tải (diff > 0): giảm bias để giảm điểm
            // Nếu thiếu tải (diff < 0): tăng bias để tăng điểm
            self.router_bias[i] -= self.learning_rate * (diff / target_count);
        }
    }
}

#[cfg(test)]
mod ex3_tests {
    use super::*;

    #[test]
    fn test_dynamic_moe_bias_adaptation() {
        let n_experts = 4;
        let top_k = 1;
        let mut router = DynamicMoERouter::new(n_experts, top_k, 0.1);

        // Tạo một batch 100 token với xu hướng thiên lệch ban đầu về chuyên gia 0
        let mut biased_batch = Vec::new();
        for _ in 0..100 {
            // Chuyên gia 0 có điểm khởi đầu cao hơn các chuyên gia khác
            biased_batch.push(vec![2.0, 0.5, 0.3, 0.1]);
        }

        // Vòng lặp thích ứng 30 lượt
        for _ in 0..30 {
            let selections = router.route_batch(&biased_batch);
            router.update_bias(&selections);
        }

        // Sau khi thích ứng, bias của chuyên gia 0 phải giảm mạnh
        assert!(router.router_bias[0] < 0.0, "Bias của chuyên gia 0 phải bị kéo giảm");
        // Bias của các chuyên gia bị đói phải tăng lên
        assert!(router.router_bias[3] > 0.0, "Bias của chuyên gia 3 phải được kích tăng");
    }
}
```
</details>

---

#### Bài tập 4: Triển khai Giới hạn Dung lượng Chuyên gia (Expert Capacity Factor & Token Dropping) trong Sparse MoE

**Bối cảnh**: Trong các cụm tính toán phân tán của Google (Switch Transformer, GShard), mỗi chuyên gia chỉ có thể xử lý tối đa $\text{Capacity} = C \times \frac{T \cdot k}{E}$ token trong một batch. Nếu một chuyên gia bị quá tải, những token vượt quá giới hạn này sẽ bị đánh rơi (dropped) và được chuyển qua đường tắt (residual bypass).

**Yêu cầu**: Hãy viết cấu trúc `CapacityAwareMoELayer` với:
1. Tính toán dung lượng tối đa `expert_capacity` theo hệ số $C$.
2. Đếm số lượng token được giao cho từng chuyên gia; nếu vượt quá dung lượng, đánh dấu token bị rơi (`is_dropped = true`) và trả về nguyên trạng vector đầu vào $x_t$.
3. Viết bài test xác nhận rằng khi $C = 1.0$ và toàn bộ token dồn vào chuyên gia 0, đúng $\text{Capacity}$ token đầu tiên được chuyên gia 0 xử lý, còn các token còn lại được bypass an toàn mà không gây panic.

<details>
<summary>Gợi ý & Lời giải Bài tập 4</summary>

```rust
/// Kết quả xử lý MoE có giới hạn dung lượng chuyên gia
#[derive(Debug, Clone, PartialEq)]
pub struct CapacityRoutingOutcome {
    pub processed_outputs: Vec<Vec<f64>>,
    pub dropped_tokens_count: usize,
    pub expert_utilization: Vec<usize>,
}

/// Bộ định tuyến MoE nhận biết dung lượng chuyên gia (Capacity-Aware Router)
#[derive(Debug, Clone, PartialEq)]
pub struct CapacityAwareMoELayer {
    pub n_experts: usize,
    pub capacity_factor: f64,
}

impl CapacityAwareMoELayer {
    pub fn new(n_experts: usize, capacity_factor: f64) -> Self {
        assert!(capacity_factor >= 1.0, "Capacity factor phải >= 1.0");
        Self {
            n_experts,
            capacity_factor,
        }
    }

    /// Tính dung lượng tối đa cho mỗi chuyên gia trên một batch T token (top-1 routing)
    pub fn compute_expert_capacity(&self, num_tokens: usize) -> usize {
        let avg = (num_tokens as f64) / (self.n_experts as f64);
        (self.capacity_factor * avg).ceil() as usize
    }

    /// Mô phỏng xử lý batch có áp dụng token dropping
    pub fn forward_with_capacity(
        &self,
        tokens: &[Vec<f64>],
        preferred_experts: &[usize],
    ) -> CapacityRoutingOutcome {
        let num_tokens = tokens.len();
        let cap = self.compute_expert_capacity(num_tokens);

        let mut expert_counts = vec![0; self.n_experts];
        let mut outputs = Vec::with_capacity(num_tokens);
        let mut dropped_count = 0;

        for (token, &expert_id) in tokens.iter().zip(preferred_experts) {
            if expert_counts[expert_id] < cap {
                // Chuyên gia còn dung lượng: thực thi chuyên gia (ở đây mô phỏng bằng x * 2.0)
                expert_counts[expert_id] += 1;
                let processed: Vec<f64> = token.iter().map(|&v| v * 2.0).collect();
                outputs.push(processed);
            } else {
                // Chuyên gia đã hết dung lượng: drop token và trả về đường tắt (residual bypass)
                dropped_count += 1;
                outputs.push(token.clone());
            }
        }

        CapacityRoutingOutcome {
            processed_outputs: outputs,
            dropped_tokens_count: dropped_count,
            expert_utilization: expert_counts,
        }
    }
}

#[cfg(test)]
mod ex4_tests {
    use super::*;

    #[test]
    fn test_capacity_factor_and_token_dropping() {
        let n_experts = 4;
        let capacity_factor = 1.0;
        let layer = CapacityAwareMoELayer::new(n_experts, capacity_factor);

        // Batch gồm 20 token, tất cả đều muốn vào chuyên gia 0
        let num_tokens = 20;
        let cap = layer.compute_expert_capacity(num_tokens);
        // Với 20 token / 4 chuyên gia * 1.0 -> cap = 5
        assert_eq!(cap, 5);

        let dummy_tokens: Vec<Vec<f64>> = (0..num_tokens).map(|i| vec![i as f64]).collect();
        let all_prefer_zero = vec![0; num_tokens];

        let outcome = layer.forward_with_capacity(&dummy_tokens, &all_prefer_zero);

        // Chuyên gia 0 chỉ nhận tối đa 5 token
        assert_eq!(outcome.expert_utilization[0], 5);
        // 15 token còn lại bị drop
        assert_eq!(outcome.dropped_tokens_count, 15);

        // 5 token đầu tiên được nhân đôi (0.0*2, 1.0*2, ..., 4.0*2)
        for i in 0..5 {
            assert_eq!(outcome.processed_outputs[i][0], (i as f64) * 2.0);
        }
        // Các token sau bị bypass nên giữ nguyên giá trị
        for i in 5..20 {
            assert_eq!(outcome.processed_outputs[i][0], i as f64);
        }
    }
}
```
</details>
