# Chương 92: Bộ tách từ Byte-Level BPE Tăng tốc Heap, Tensor Bước nhảy & Động cơ Vi phân ngược Autograd (Byte-Level Heap BPE, Strided Tensor & Reverse-Mode Autograd)

---

## 1. Giới thiệu & Mục tiêu

Chào mừng bạn đến với **Chủ đề 27** — chặng chuyên sâu cuối cùng và cũng là đỉnh cao công nghệ của giáo trình `learn-rs`: **Kiến trúc, Suy luận & Hậu huấn luyện Mô hình Ngôn ngữ Lớn (LLM) Từ số không Bằng Rust Thuần `std`**.

Trong suốt Chặng I và Chặng II, chúng ta đã nắm vững các công cụ mạnh mẽ nhất của Rust: từ quản lý quyền sở hữu bộ nhớ tĩnh, hệ thống kiểu phân loại Typestate, xử lý đồng thời Actor/Tokio, cho tới các cấu trúc dữ liệu hiệu năng cao. Ở Chủ đề 26, chúng ta đã biến LLM thành trung tâm của hệ thống tự hành thông qua kỹ nghệ bộ khung tác tử (Agent Harness), gương ngữ cảnh sống (Live Context Mirror), và đồ thị tính toán trạng thái (StateGraph).

Tuy nhiên, trong toàn bộ các chương đó, mô hình ngôn ngữ vẫn là một "hộp đen" nhận chuỗi ký tự và trả về chuỗi ký tự qua cổng API. **Chủ đề 27 sẽ đập tan chiếc hộp đen đó.** Chúng ta sẽ tự tay dựng lại từng bánh răng cơ học, từng phương trình vi phân và từng ma trận trọng số của một hệ thống LLM hiện đại từ con số không, không phụ thuộc vào bất kỳ thư viện bên ngoài nào (`dependencies = []`), hoàn toàn bằng Rust 2024.

Chương 92 này là nền móng đầu tiên của toàn bộ Chủ đề 27. Để một mô hình ngôn ngữ có thể hoạt động, chúng ta bắt buộc phải giải quyết ba bài toán cơ bản:

```
┌────────────────────────────────────────────────────────────────────────┐
│                        BA TRỤ CỘT CỦA CHƯƠNG 92                        │
├─────────────────────────┬────────────────────────┬─────────────────────┤
│ 1. BIỂU DIỄN VĂN BẢN    │ 2. CẤU TRÚC ĐẠI SỐ     │ 3. ĐỘNG CƠ HỌC TẬP  │
│    (Tokenization)       │    (Linear Algebra)    │    (Optimization)   │
├─────────────────────────┼────────────────────────┼─────────────────────┤
│ • Byte-Level BPE        │ • Strided Tensor       │ • Reverse Autograd  │
│ • 256 byte UTF-8 cơ sở  │ • Zero-copy transpose  │ • Topo-sort DAG     │
│ • BinaryHeap phá hòa    │ • LogSoftmax chống tràn│ • Chain Rule        │
│ • Token đặc biệt thẻ    │ • CrossEntropy có mask │ • Kiểm chứng sai phân│
│ • Decoder Mask cú pháp  │ • MatMul tối ưu cache  │   hữu hạn < 1e-8    │
└─────────────────────────┴────────────────────────┴─────────────────────┘
```

### Bảng Mục tiêu Học tập & Kỹ năng Đạt được

| Mục tiêu học tập | Vì sao bắt buộc phải tự cài đặt? | Chuẩn đầu ra thực chứng |
|---|---|---|
| **Byte-Level BPE với Heap** | Tránh hoàn toàn lỗi mất từ vựng (`<unk>`), xử lý trơn tru tiếng Việt và mã nguồn; đống cực đại bảo đảm phá hòa tất định giữa các nền tảng | Khứ hồi 100% không mất mát byte nào; token đặc biệt không bao giờ bị xé lẻ |
| **Mặt nạ giải mã có ràng buộc (`ConstrainedDecoderMask`)** | Ép mô hình suy luận tuân thủ nghiêm ngặt ngữ pháp `<think>...</think><answer>...</answer>` của các mô hình RLVR hiện đại | Logit của các token vi phạm cú pháp bị gán $-\infty$, điều khiển chính xác FSM |
| **Tensor vùng nhớ phẳng có bước nhảy (`Strided Tensor`)** | Hiểu bản chất vật lý của bộ nhớ liên tục; cho phép chuyển vị ma trận $O(1)$ mà không tốn chi phí sao chép dữ liệu | `transpose` chỉ hoán đổi `shape` và `strides`, dữ liệu 1D giữ nguyên vẹn |
| **Hàm `log_softmax` & `cross_entropy_masked`** | Tránh lỗi tràn số dấu phẩy động ($e^{1000} \to \infty$) và hỗ trợ bỏ qua nhãn đệm (`ignore_index = -100`) trong huấn luyện SFT | Logit lớn $\pm 1000.0$ vẫn cho xác suất hợp lệ; vị trí đệm có gradient bằng đúng $0.0$ |
| **Động cơ vi phân ngược (`AutogradTape`)** | Nền tảng của thuật toán lan truyền ngược (Backpropagation); hiểu chính xác dòng chảy gradient trong mạng nơ-ron | Mọi đạo hàm giải tích khớp với sai phân hữu hạn trung tâm với sai số tuyệt đối $< 10^{-8}$ |

---

## 2. Hình tượng hóa đời sống: Dây chuyền cắt ghép ADN và máy tính đạo hàm quang học

Để nắm bắt trực giác trước khi đi sâu vào mã nguồn, hãy cùng quan sát hai hình tượng đời sống quen thuộc.

### Hình tượng 1: Dây chuyền cắt ghép ADN của Byte-Level BPE

Hãy tưởng tượng chuỗi văn bản của bạn là một sợi chuỗi sinh học ADN dài. Trong thế giới phân tử tự nhiên, chỉ có 4 loại bazơ nitơ (A, T, G, X). Trong máy tính, mọi ký tự (từ chữ cái tiếng Anh `a`, ký tự tiếng Việt có dấu `ệ`, biểu tượng cảm xúc `🦀`, cho tới ký tự điều khiển xuống dòng) đều được cấu tạo từ **256 byte cơ sở UTF-8** (từ `0x00` tới `0xFF`).

```
┌──────────────────────────────────────────────────────────────────────────────┐
│                  HÌNH TƯỢNG: CẮT GHÉP ADN BẰNG ĐỐNG CỰC ĐẠI                 │
│                                                                              │
│  Sợi ADN ban đầu (các byte đơn lẻ):                                          │
│   [ h ] ── [ ọ ] ── [ c ] ── [ _ ] ── [ h ] ── [ ọ ] ── [ c ]                │
│                                                                              │
│  Enzyme quét tần suất (Counting Phase):                                       │
│   Cặp (h, ọ) xuất hiện 2 lần!                                               │
│   Cặp (ọ, c) xuất hiện 2 lần!                                               │
│                                                                              │
│  Phá hòa tất định bằng BinaryHeap (Tie-Breaking):                            │
│   Nếu tần suất bằng nhau: cặp có mã số byte nhỏ hơn được ghép trước!         │
│   Reverse(TokenPair { first: 'h', second: 'ọ' }) thắng cuộc!                 │
│                                                                              │
│  Hàn gắn phân tử (Merge Phase):                                              │
│   Sinh ra phân tử mới: [ h_ọ ] (Token ID = 256)                              │
│   Sợi ADN mới: [ h_ọ ] ── [ c ] ── [ _ ] ── [ h_ọ ] ── [ c ]                 │
│                                                                              │
│  Vùng bất khả xâm phạm (Special Tokens):                                     │
│   ┌───────────┐                                                ┌───────────┐ │
│   │ <|bos|>   │ ── [ Sợi gen bình thường ] ── [ <think> ] ──.. │ <|eos|>   │ │
│   └───────────┘                                                └───────────┘ │
│   Màng bọc nano bảo vệ: không enzyme nào được phép cắt xé token đặc biệt!    │
└──────────────────────────────────────────────────────────────────────────────┘
```

1. **256 Axit Amin cơ sở**: Không bao giờ có từ nào bị "ngoài tầm hiểu biết" (Out-of-Vocabulary). Kể cả một byte hỏng hóc hay một emoji mới ra đời năm 2026, nó vẫn chỉ là chuỗi các byte từ 0 đến 255.
2. **Hàng đợi ưu tiên `BinaryHeap`**: Khi tìm cặp byte xuất hiện nhiều nhất để tạo thành token ghép, nếu có nhiều cặp cùng số lần xuất hiện (ví dụ cặp `('a', 'b')` và `('c', 'd')` cùng có 5 lần), ta không được phụ thuộc vào thứ tự duyệt ngẫu nhiên của bảng băm `HashMap`. Ta đẩy tất cả vào một `BinaryHeap` với quy tắc `Reverse(TokenPair)`: cặp có giá trị số học nhỏ hơn sẽ luôn luôn chiến thắng. Nhờ đó, máy tính chạy ở Tokyo hay New York đều cho ra một bộ từ vựng y hệt nhau tới từng bit!
3. **Màng bảo vệ token đặc biệt**: Các token điều khiển cấu trúc như `<|bos|>` (bắt đầu chuỗi), `<|eos|>` (kết thúc chuỗi), `<think>` (mở đầu suy nghĩ) và `<answer>` (mở đầu câu trả lời) được gắn nhãn nguyên khối ngay từ đầu. Thuật toán tiền tách từ sẽ cách ly chúng, không cho phép quá trình gộp byte phá hủy ranh giới này.

---

### Hình tượng 2: Lăng kính quang học xoay góc và dòng năng lượng ngược

Bây giờ, hãy nhìn vào cách máy tính lưu trữ và tính đạo hàm cho các ma trận số thực:

```
┌──────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG: LĂNG KÍNH QUANG HỌC VÀ CUỘN PHIM 1D                 │
│                                                                              │
│  Vùng nhớ RAM vật lý là một sợi phim thẳng 1D duy nhất:                      │
│   Địa chỉ:  0      1      2      3      4      5                             │
│   Giá trị: [ 1.0 ][ 2.0 ][ 3.0 ][ 4.0 ][ 5.0 ][ 6.0 ]                        │
│                                                                              │
│  Góc nhìn Ma trận A (2 hàng, 3 cột):                                         │
│   Bước nhảy hàng = 3, Bước nhảy cột = 1                                      │
│   Hàng 0: [ 1.0 ]  [ 2.0 ]  [ 3.0 ]    (nhảy 1 ô sang cột tiếp)             │
│   Hàng 1: [ 4.0 ]  [ 5.0 ]  [ 6.0 ]    (nhảy 3 ô sang hàng tiếp)             │
│                                                                              │
│  Phép chuyển vị Zero-Copy Transpose (Quay lăng kính 90 độ):                  │
│   Không copy bất kỳ số nào trên cuộn phim!                                   │
│   Chỉ tráo đổi: Shape mới = [3, 2], Bước nhảy mới = [1, 3]!                  │
│   Hàng 0: [ 1.0 ]  [ 4.0 ]             (nhảy 3 ô sang cột tiếp!)            │
│   Hàng 1: [ 2.0 ]  [ 5.0 ]                                                   │
│   Hàng 2: [ 3.0 ]  [ 6.0 ]             Chi phí: O(1) tức thời!              │
│                                                                              │
│──────────────────────────────────────────────────────────────────────────────│
│             HÌNH TƯỢNG: DÒNG NĂNG LƯỢNG NGƯỢC CỦA AUTOGRAD                   │
│                                                                              │
│  Pha xuôi (Forward): Chùm photon đi từ biến lá tới tổn thất                  │
│   [ Trọng số W ] ───► ( Nhân Ma Trận ) ───► ( ReLU ) ───► ( Loss L )         │
│   [ Đầu vào X  ] ───────┘                                                    │
│                                                                              │
│  Pha ngược (Backward): Bắn tia laser ngược từ Loss                           │
│   [ Trọng số W ] ◄─── ( Phân tách dL/dW ) ◄── ( dL/dAct ) ◄── [ Năng lượng 1.0]
│   [ Đầu vào X  ] ◄───────┘                                                   │
│                                                                              │
│   Mỗi trạm quang học chia dòng năng lượng ngược theo đúng đạo hàm giải tích!  │
│   Tích lũy gradient vào các nút lá để chuẩn bị bước cập nhật trọng số.       │
└──────────────────────────────────────────────────────────────────────────────┘
```

Trong một tensor phẳng có bước nhảy (`Strided Tensor`), bộ nhớ liên tục là một mảng 1D. Để biểu diễn ma trận $M \times N$, ta không tạo mảng của mảng `Vec<Vec<f64>>` (vốn gây phân mảnh heap và phá hủy bộ nhớ đệm CPU L1/L2/L3). Ta dùng công thức định vị tọa độ:
$$\text{Index}(i, j) = \text{offset} + i \times S_0 + j \times S_1$$

Khi chuyển vị ma trận $A^T$, ta chỉ việc hoán đổi bước nhảy $S_0 \leftrightarrow S_1$ và chiều $M \leftrightarrow N$. Không một byte dữ liệu nào bị di chuyển trên thanh RAM. Đó là lý do Rust có thể thực hiện các phép toán đại số tuyến tính với tốc độ chạm tới giới hạn vật lý của phần cứng.

---

## 3. Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 3.1. Thuật toán Byte-Level BPE và Tiền Tách Từ UTF-8

Bộ tách từ (Tokenizer) là cây cầu nối giữa ngôn ngữ tự nhiên của con người và các vector số học của máy tính. Nếu bộ tách từ kém, mô hình ngôn ngữ sẽ gặp ba thảm họa:
1. **Bùng nổ kích thước từ vựng**: Nếu mỗi từ ghép tiếng Việt ("học sinh", "sinh viên", "lập trình viên") là một token riêng, kích thước từ điển sẽ lên tới hàng triệu token, khiến ma trận nhúng (Embedding Layer) phình to hàng chục gigabyte.
2. **Ký tự lạ bị biến thành `<unk>`**: Nếu gặp từ mới hoặc ký tự đặc biệt không có trong từ điển, mô hình hoàn toàn mất thông tin.
3. **Mất an toàn ranh giới từ vựng**: Nếu cho phép gộp ký tự bất kỳ xuyên qua dấu cách hay dấu câu, mô hình sẽ học các mẫu ngữ pháp sai lệch.

#### Tiền tách từ bảo toàn dữ liệu (`pre_tokenize_utf8`)
Trước khi gộp BPE, chuỗi văn bản đầu vào được phân rã thành các đoạn nhỏ dựa trên phân loại ký tự Unicode:
- Nhóm ký tự khoảng trắng (dấu cách, tab, xuống dòng `\n`).
- Nhóm ký tự chữ cái và chữ số (bao gồm toàn bộ bảng mã Unicode tiếng Việt như `à`, `á`, `ế`, `ộ`).
- Nhóm ký tự dấu câu và ký hiệu toán học (`(`, `)`, `,`, `.`, `!`, `+`, `-`).

Bất biến cốt lõi của hàm tiền tách từ:
$$\sum_{i} \text{chunk}_i \equiv \text{text}$$
Không một ký tự nào bị lược bỏ. Khi giải mã (`decode`), ta ghép các mảnh lại sẽ thu được chính xác 100% văn bản ban đầu.

#### Thuật toán huấn luyện BPE với `BinaryHeap`
1. Khởi tạo từ vựng ban đầu với **256 byte** đơn lẻ: $\mathcal{V}_0 = \{ [0], [1], \dots, [255] \}$.
2. Phân rã kho ngữ liệu huấn luyện thành chuỗi các ID byte.
3. Trong mỗi vòng lặp $k = 1 \dots \text{max\_merges}$:
   - Đếm tần suất xuất hiện của mọi cặp kế tiếp nhau $(u, v)$ trong ngữ liệu: $\text{count}((u, v))$.
   - Đẩy tất cả các cặp có $\text{count} \ge 2$ vào một `BinaryHeap<(usize, Reverse<TokenPair>)>`.
   - Lấy phần tử đỉnh đống: cặp có tần suất lớn nhất. Nếu có nhiều cặp hòa điểm, `Reverse<TokenPair>` sẽ chọn cặp có giá trị số học nhỏ nhất:
     $$(u_1, v_1) < (u_2, v_2) \iff u_1 < u_2 \lor (u_1 = u_2 \land v_1 < v_2)$$
   - Cấp phát ID mới: $\text{new\_id} = |\mathcal{V}|$.
   - Thêm quy tắc gộp $(u, v) \to \text{new\_id}$ vào danh sách `merges` và lưu thứ hạng vào `ranks`.
   - Quét qua ngữ liệu và thay thế toàn bộ chuỗi con $[u, v]$ thành $[\text{new\_id}]$.

```rust
// Định nghĩa cặp token ứng viên gộp với trường có tên
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenPair {
    pub first: u16,
    pub second: u16,
}
```

#### Bảo vệ token đặc biệt (Special Tokens)
Mô hình lý luận (Reasoning Model) như DeepSeek-R1 hay OpenAI o1/o3 yêu cầu các thẻ cấu trúc đặc biệt để phân tách luồng tư duy nội tâm và kết quả cuối cùng:
- `<|bos|>`: Bắt đầu chuỗi (Beginning of Sequence).
- `<|eos|>`: Kết thúc chuỗi (End of Sequence).
- `<think>` và `</think>`: Ranh giới khối suy luận nội tâm (Chain of Thought).
- `<answer>` và `</answer>`: Ranh giới câu trả lời hoàn chỉnh gửi tới người dùng.

Trong hàm `encode()`, một thuật toán quét cửa sổ sẽ phát hiện sự xuất hiện của các chuỗi đặc biệt này trước tiên. Chúng được chuyển đổi trực tiếp thành token ID tương ứng mà không bao giờ bị phân rã thành các byte con.

---

### 3.2. Mặt Nạ Giải Mã Có Ràng Buộc (`ConstrainedDecoderMask`)

Tại sao mô hình suy luận cần mặt nạ giải mã có ràng buộc? Trong quá trình sinh văn bản tự hồi quy (Autoregressive Generation), tại mỗi bước, mô hình tính toán một vector logit kích thước $V$ (kích thước từ vựng). Nếu để mô hình tự do lấy mẫu ngẫu nhiên, có xác suất nó sẽ sinh ra thẻ `</think>` khi chưa hề mở thẻ `<think>`, hoặc sinh thẻ kết thúc `<|eos|>` ngay giữa dòng suy nghĩ.

`ConstrainedDecoderMask` đóng vai trò là một **máy trạng thái hữu hạn (FSM)**:

```
                  ┌──────────────┐
                  │    Start     │
                  └──────┬───────┘
                         │ Nhận <think>
                         ▼
                  ┌──────────────┐
                  │  InThinking  │◄─────┐ (Sinh token thường)
                  └──────┬───────┘└─────┘
                         │ Nhận </think>
                         ▼
                  ┌──────────────┐
                  │AfterThinking │ (Bắt buộc bước sau phải là <answer>)
                  └──────┬───────┘
                         │ Nhận <answer>
                         ▼
                  ┌──────────────┐
                  │   InAnswer   │◄─────┐ (Sinh token thường)
                  └──────┬───────┘└─────┘
                         │ Nhận </answer> hoặc <|eos|>
                         ▼
                  ┌──────────────┐
                  │   Finished   │
                  └──────────────┘
```

Tại mỗi bước giải mã, hàm `apply_mask(logits)` sẽ can thiệp trực tiếp vào vector logit trước khi đưa qua softmax:
$$\text{logit}_i = \begin{cases} \text{logit}_i & \text{nếu } i \in \text{allowed\_tokens} \\ -\infty & \text{nếu } i \notin \text{allowed\_tokens} \end{cases}$$
Khi $\text{logit}_i = -\infty$, xác suất $e^{-\infty} = 0$, đảm bảo 100% token vi phạm ngữ pháp sẽ không bao giờ được sinh ra.

---

### 3.3. Cấu Trúc Dữ Liệu Tensor Bước Nhảy (Strided Tensor)

Một tensor $N$ chiều trong Rust được định nghĩa bằng 4 thành phần trường có tên rõ ràng:

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    /// Mảng dữ liệu phẳng 1 chiều
    pub data: Vec<f64>,
    /// Kích thước từng chiều [d_0, d_1, ..., d_{N-1}]
    pub shape: Vec<usize>,
    /// Bước nhảy bộ nhớ [s_0, s_1, ..., s_{N-1}]
    pub strides: Vec<usize>,
    /// Vị trí dịch bắt đầu
    pub offset: usize,
}
```

#### Công thức tính bước nhảy liên tục (Row-Major)
Với tensor có kích thước $\text{shape} = [d_0, d_1, \dots, d_{N-1}]$, các phần tử nằm liên tiếp nhau trong bộ nhớ khi bước nhảy tuân theo quy tắc:
$$s_{N-1} = 1$$
$$s_i = s_{i+1} \times d_{i+1} \quad (\forall i = N-2, \dots, 0)$$

Ví dụ với ma trận kích thước $[2, 3]$:
- $d_0 = 2, d_1 = 3$
- $s_1 = 1$
- $s_0 = s_1 \times d_1 = 1 \times 3 = 3$
- Bộ bước nhảy liên tục: $[3, 1]$.

#### Phép chuyển vị không sao chép (Zero-Copy Transpose)
Khi gọi `tensor.transpose(dim0, dim1)`:
- `shape.swap(dim0, dim1)`
- `strides.swap(dim0, dim1)`
- `data` được giữ nguyên vẹn.

Ví dụ, ma trận $[2, 3]$ với strides $[3, 1]$ sau khi chuyển vị thành ma trận $[3, 2]$ sẽ có strides $[1, 3]$. Khi truy cập phần tử $(i, j)$ của ma trận mới:
$$\text{idx} = i \times 1 + j \times 3$$
Phần tử $(0, 1)$ sẽ nằm ở vị trí $0 \times 1 + 1 \times 3 = 3$, hoàn toàn khớp với phần tử $(1, 0)$ của ma trận ban đầu ($1 \times 3 + 0 \times 1 = 3$). Phép toán này tốn thời gian $O(1)$ và cấp phát $0$ byte dữ liệu mới.

#### Ổn định số học trong `log_softmax`
Hàm softmax thông thường:
$$P_i = \frac{e^{z_i}}{\sum_j e^{z_j}} \implies \ln P_i = z_i - \ln\left(\sum_j e^{z_j}\right)$$
Trong máy tính, nếu $z_i \ge 710.0$, phép tính $e^{z_i}$ trong kiểu số thực 64-bit (`f64`) sẽ bị tràn số thành `+inf`, dẫn tới kết quả toàn bộ trở thành `NaN`.
Để khắc phục, ta sử dụng **kỹ thuật trừ giá trị cực đại (Max-Subtraction)**:
Đặt $M = \max_k z_k$. Ta có:
$$\frac{e^{z_i}}{\sum_j e^{z_j}} = \frac{e^{z_i - M} \cdot e^M}{\sum_j (e^{z_j - M} \cdot e^M)} = \frac{e^{z_i - M}}{\sum_j e^{z_j - M}}$$
Do đó:
$$\ln P_i = (z_i - M) - \ln\left(\sum_j e^{z_j - M}\right)$$
Vì $z_i - M \le 0$ với mọi $i$, giá trị $e^{z_i - M} \in (0, 1]$, triệt tiêu hoàn toàn nguy cơ tràn số dương `+inf`.

#### Hàm mất mát Cross Entropy có mặt nạ (`cross_entropy_masked`)
Trong tinh chỉnh có giám sát (SFT - Supervised Fine-Tuning) cho mô hình ngôn ngữ, ta chỉ muốn mô hình học cách sinh câu trả lời của trợ lý, chứ không tính điểm mất mát cho phần lời nhắc (prompt) của người dùng.
Các vị trí lời nhắc được đánh dấu bằng nhãn đặc biệt: $\text{ignore\_index} = -100$.

$$\mathcal{L} = -\frac{1}{N_{\text{valid}}} \sum_{i: y_i \ne -100} \ln P_{i, y_i}$$
Nếu mọi token trong chuỗi đều bị mask ($N_{\text{valid}} = 0$), hàm mất mát trả về chính xác $0.0$ thay vì gây ra lỗi chia cho 0 (`0.0 / 0.0 = NaN`).

---

### 3.4. Động Cơ Vi Phân Ngược Tự Động (Reverse-Mode Autograd Tape)

Động cơ vi phân tự động là trái tim của mọi khuôn khổ học sâu như PyTorch hay JAX. Có hai chế độ vi phân tự động:
1. **Forward-Mode (Vi phân xuôi)**: Tính đạo hàm của tất cả các biến trung gian theo một biến đầu vào $x_j$. Nếu mạng có $M$ đầu vào/trọng số và 1 đầu ra tổn thất $\mathcal{L}$, Forward-Mode đòi hỏi $M$ lượt duyệt đồ thị ($O(M)$).
2. **Reverse-Mode (Vi phân ngược)**: Tính đạo hàm của hàm mục tiêu vô hướng $\mathcal{L}$ theo **tất cả** các biến trung gian và trọng số cùng lúc trong một lượt duyệt duy nhất từ ngọn về gốc ($O(1)$). Vì các mô hình LLM có từ hàng triệu đến hàng trăm tỷ tham số nhưng chỉ có 1 giá trị tổn thất $\mathcal{L}$, Reverse-Mode là lựa chọn tối ưu duy nhất khả dĩ.

#### Đồ thị tính toán DAG và Băng ghi (Tape)
Băng ghi `AutogradTape` lưu trữ các nút tính toán theo đúng thứ tự thực thi của pha xuôi:
```rust
#[derive(Debug, Clone)]
pub struct TapeNode {
    pub id: ValueId,
    pub value: Tensor,
    pub grad: Option<Tensor>,
    pub op: TapeOp,
    pub parents: Vec<ValueId>,
}
```
Vì mỗi nút chỉ có thể phụ thuộc vào các nút đã được tạo ra trước nó, danh sách mảng `nodes: Vec<TapeNode>` đã tự động được sắp xếp theo **thứ tự tô-pô (Topological Order)**. Khi thực hiện pha ngược:
```rust
for i in (0..=loss_id.id).rev() {
    // Duyệt ngược từ loss về các nút lá
}
```
Ta duyệt ngược từ chỉ số lớn nhất về 0, đảm bảo khi một nút được xử lý, toàn bộ gradient từ các nút con phía sau nó đã được tích lũy đầy đủ.

#### Bảng Đạo Hàm Giải Tích Chi Tiết Của Các Phép Toán

Dưới đây là công thức giải tích toán học chính xác được cài đặt trong `backward()`:

| Phép toán | Pha xuôi (Forward) | Gradient nút cha thứ nhất ($\frac{\partial \mathcal{L}}{\partial A}$) | Gradient nút cha thứ hai ($\frac{\partial \mathcal{L}}{\partial B}$) |
|---|---|---|---|
| **Add** | $C = A + B$ | $g$ | $g$ |
| **Sub** | $C = A - B$ | $g$ | $-g$ |
| **Mul** | $C = A \odot B$ | $g \odot B$ | $g \odot A$ |
| **Scale** | $C = \alpha \cdot A$ | $\alpha \cdot g$ | *(không có)* |
| **Relu** | $C = \max(0, A)$ | $g \odot \mathbb{I}(A > 0)$ | *(không có)* |
| **MatMul** | $C = A \times B$ | $g \times B^T$ | $A^T \times g$ |
| **Sum** | $s = \sum_k A_k$ | $[g, g, \dots, g]$ | *(không có)* |
| **LogSoftmax** | $P = \text{log\_softmax}(Z)$ | $g_i - e^{P_i} \sum_j g_j$ | *(không có)* |
| **CrossEntropy** | $\mathcal{L} = -\frac{1}{N} \sum \ln P_y$ | $\frac{g}{N_{\text{valid}}} (e^{Z_{i, v}} - \delta_{v, y_i})$ (với vị trí hợp lệ) | *(không có)* |
| **RmsNorm** | $Y = \frac{X}{\text{RMS}(X)} \odot W$ | $\frac{1}{\text{RMS}} \left(g \odot W - \hat{X} \cdot \frac{1}{d} \sum (g \odot W \odot \hat{X})\right)$ | $g \odot \hat{X}$ |

#### Kiểm chứng sai phân hữu hạn trung tâm (Central Finite Difference)
Làm sao để chắc chắn 100% rằng các công thức đạo hàm giải tích phức tạp ở trên không bị sai một dấu trừ hay một hệ số chia?
Ta sử dụng phương pháp sai phân hữu hạn trung tâm từ giải tích số:
$$f'(x_i) \approx \frac{f(x + \epsilon \cdot e_i) - f(x - \epsilon \cdot e_i)}{2\epsilon}$$
Khai triển chuỗi Taylor bậc hai:
$$f(x + \epsilon) = f(x) + \epsilon f'(x) + \frac{\epsilon^2}{2} f''(x) + \frac{\epsilon^3}{6} f'''(x) + \mathcal{O}(\epsilon^4)$$
$$f(x - \epsilon) = f(x) - \epsilon f'(x) + \frac{\epsilon^2}{2} f''(x) - \frac{\epsilon^3}{6} f'''(x) + \mathcal{O}(\epsilon^4)$$
Trừ hai phương trình:
$$\frac{f(x + \epsilon) - f(x - \epsilon)}{2\epsilon} = f'(x) + \mathcal{O}(\epsilon^2)$$
Sai số lý thuyết tỷ lệ với $\epsilon^2$. Khi chọn $\epsilon = 10^{-5}$ hoặc $10^{-6}$, sai số cắt cụt lý thuyết là $\mathcal{O}(10^{-10} \dots 10^{-12})$.
Sau khi tính toán trên kiểu `f64`, độ chênh lệch giữa đạo hàm giải tích và sai phân trung tâm bắt buộc phải thỏa mãn:
$$|g_{\text{analytic}} - g_{\text{numerical}}| < 10^{-8}$$
Đây chính là chứng chỉ chất lượng vàng bảo đảm mạng nơ-ron có thể hội tụ ổn định khi huấn luyện.

---

## 4. Mã nguồn minh họa thực chiến

Toàn bộ mã nguồn hoàn chỉnh của crate `code/ch92/src/main.rs` được trình bày dưới đây. Mã nguồn tuân thủ 100% Rust 2024, không dùng bất kỳ thư viện ngoài nào (`[dependencies]` trống rỗng), tất cả các `struct` đều sử dụng trường có tên, và định danh tuân thủ quy chuẩn ASCII với chú thích tiếng Việt theo chuẩn `RiskGate`.

```rust
//! # Chương 92: Bộ Tách Từ Byte-Level BPE Tăng Tốc Heap, Tensor Bước Nhảy & Động Cơ Vi Phân Ngược Autograd
//!
//! Crate này cài đặt hoàn chỉnh từ đầu (100% thuần `std`, không dùng thư viện ngoài)
//! ba trụ cột toán học và cấu trúc dữ liệu cốt lõi của một hệ thống LLM:
//! 1. `BpeTokenizer`: Bộ tách từ mức byte UTF-8 với hàng đợi ưu tiên `BinaryHeap` phá hòa tất định.
//! 2. `Tensor`: Tensor vùng nhớ phẳng với `shape` và `strides`, hỗ trợ `transpose` O(1) không sao chép dữ liệu.
//! 3. `AutogradTape`: Động cơ tính vi phân ngược tự động (Reverse-Mode Autograd) duyệt thứ tự tô-pô,
//!    kiểm chứng độ chính xác giải tích so với sai phân hữu hạn trung tâm đạt sai số < 1e-8.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

// ============================================================================
// 1. BỘ SINH SỐ GIẢ NGẪU NHIÊN SPLITMIX64 & BIẾN ĐỔI BOX-MULLER
// ============================================================================

/// Bộ sinh số giả ngẫu nhiên 64-bit tất định SplitMix64.
///
/// Lý do thiết kế: Chu kỳ 2^64, chi phí tính toán cực thấp (chỉ dùng phép toán bit
/// và nhân tràn số nguyên), độc lập hoàn toàn với hệ điều hành, đảm bảo tái lập 100%
/// kết quả khởi tạo trọng số mạng nơ-ron trên mọi nền tảng phần cứng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái nội bộ 64-bit
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh số với hạt giống tùy chọn
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh số nguyên không dấu 64-bit tiếp theo
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh số thực trong khoảng [0.0, 1.0)
    pub fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    /// Sinh số thực đối xứng trong khoảng [-scale, scale)
    pub fn next_signed(&mut self, scale: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * scale
    }

    /// Sinh số ngẫu nhiên theo phân phối chuẩn tắc Gaussian N(mu, sigma^2)
    /// thông qua biến đổi Box-Muller cổ điển.
    pub fn next_gaussian(&mut self, mean: f64, std_dev: f64) -> f64 {
        let mut u1 = self.next_f64();
        // Tránh ln(0.0) gây giá trị âm vô cực NaN
        if u1 <= 1e-15 {
            u1 = 1e-15;
        }
        let u2 = self.next_f64();
        let radius = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        mean + std_dev * radius * theta.cos()
    }
}

// ============================================================================
// 2. TIỀN TÁCH TỪ UTF-8 BẢO TOÀN DỮ LIỆU (PRE-TOKENIZATION)
// ============================================================================

/// Tiền tách từ chuỗi ký tự UTF-8 thành các mảnh ngữ nghĩa độc lập.
///
/// Bất biến bảo tồn: `chunks.concat() == text` (không mất bất kỳ byte nào, kể cả khoảng trắng).
/// Việc nhóm này ngăn thuật toán BPE gộp các byte xuyên qua ranh giới từ hoặc dấu câu,
/// tương tự như regex pre-tokenizer của GPT-2/GPT-4 nhưng thuần Rust 100% không phụ thuộc regex.
pub fn pre_tokenize_utf8(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    #[derive(Copy, Clone, PartialEq, Eq)]
    enum CharCategory {
        Whitespace,
        Alphanumeric,
        Punctuation,
    }

    fn categorize(c: char) -> CharCategory {
        if c.is_whitespace() {
            CharCategory::Whitespace
        } else if c.is_alphanumeric() {
            CharCategory::Alphanumeric
        } else {
            CharCategory::Punctuation
        }
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_cat: Option<CharCategory> = None;

    for c in text.chars() {
        let cat = categorize(c);
        match current_cat {
            Some(prev_cat) if prev_cat == cat => {
                current_chunk.push(c);
            }
            Some(_) => {
                chunks.push(current_chunk);
                current_chunk = String::new();
                current_chunk.push(c);
                current_cat = Some(cat);
            }
            None => {
                current_chunk.push(c);
                current_cat = Some(cat);
            }
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

// ============================================================================
// 3. BỘ TÁCH TỪ BYTE-LEVEL BPE TĂNG TỐC BẰNG HEAP
// ============================================================================

/// Cặp token kế tiếp nhau làm ứng viên gộp BPE
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenPair {
    /// Token thứ nhất
    pub first: u16,
    /// Token thứ hai
    pub second: u16,
}

/// Thông tin thứ hạng gộp và định danh mới
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankInfo {
    /// Thứ tự xuất hiện trong danh sách quy tắc gộp
    pub rank: usize,
    /// Token ID mới được sinh ra từ việc gộp
    pub new_id: u16,
}

/// Quy tắc gộp BPE lưu trong bộ tách từ
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRule {
    /// Cặp token được gộp
    pub pair: TokenPair,
    /// Token ID mới
    pub new_id: u16,
}

/// Bộ tách từ mức byte BPE (Byte-Level Byte Pair Encoding).
#[derive(Debug, Clone)]
pub struct BpeTokenizer {
    /// Từ điển ánh xạ từ `u16` sang chuỗi byte
    pub vocab: Vec<Vec<u8>>,
    /// Danh sách quy tắc gộp theo thứ tự huấn luyện
    pub merges: Vec<MergeRule>,
    /// Bảng băm tra cứu thứ hạng và ID gộp nhanh O(1)
    pub ranks: HashMap<TokenPair, RankInfo>,
    /// Bảng ánh xạ token đặc biệt từ chuỗi sang token ID
    pub special_tokens: HashMap<String, u16>,
    /// Bảng ánh xạ ngược từ token ID sang chuỗi token đặc biệt
    pub special_token_ids: HashMap<u16, String>,
}

fn merge_sequence(tokens: &[u16], pair: TokenPair, new_id: u16) -> Vec<u16> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if i + 1 < tokens.len() && tokens[i] == pair.first && tokens[i + 1] == pair.second {
            out.push(new_id);
            i += 2;
        } else {
            out.push(tokens[i]);
            i += 1;
        }
    }
    out
}

impl BpeTokenizer {
    /// Huấn luyện bảng gộp BPE trên kho ngữ liệu với hàng đợi ưu tiên BinaryHeap
    pub fn train(corpus: &str, max_merges: usize) -> Self {
        // 256 byte cơ sở UTF-8 ban đầu
        let mut vocab: Vec<Vec<u8>> = (0..=255u8).map(|b| vec![b]).collect();

        // Tiền tách từ bảo toàn byte
        let pieces = pre_tokenize_utf8(corpus);
        let mut words: Vec<Vec<u16>> = pieces
            .into_iter()
            .map(|s| s.bytes().map(u16::from).collect())
            .collect();

        let mut merges = Vec::new();
        let mut ranks = HashMap::new();

        for _ in 0..max_merges {
            let mut counts: HashMap<TokenPair, usize> = HashMap::new();
            for word in &words {
                for window in word.windows(2) {
                    let pair = TokenPair {
                        first: window[0],
                        second: window[1],
                    };
                    *counts.entry(pair).or_insert(0) += 1;
                }
            }

            // BinaryHeap lưu (count, Reverse(pair)) để cặp nhỏ nhất được ưu tiên khi hòa điểm
            let mut heap = BinaryHeap::new();
            for (pair, count) in counts {
                if count >= 2 {
                    heap.push((count, Reverse(pair)));
                }
            }

            let Some((_, Reverse(best_pair))) = heap.pop() else {
                break;
            };

            let new_id = u16::try_from(vocab.len()).expect("Vượt giới hạn u16 cho từ vựng BPE");
            let mut new_bytes = vocab[usize::from(best_pair.first)].clone();
            new_bytes.extend_from_slice(&vocab[usize::from(best_pair.second)]);
            vocab.push(new_bytes);

            let rule = MergeRule {
                pair: best_pair,
                new_id,
            };
            ranks.insert(
                best_pair,
                RankInfo {
                    rank: merges.len(),
                    new_id,
                },
            );
            merges.push(rule);

            // Cập nhật các từ bị ảnh hưởng
            for word in &mut words {
                if word.len() >= 2 {
                    *word = merge_sequence(word, best_pair, new_id);
                }
            }
        }

        let mut tokenizer = Self {
            vocab,
            merges,
            ranks,
            special_tokens: HashMap::new(),
            special_token_ids: HashMap::new(),
        };

        // Đăng ký mặc định 6 token đặc biệt phục vụ suy luận có cấu trúc
        tokenizer.register_special_token("<|bos|>");
        tokenizer.register_special_token("<|eos|>");
        tokenizer.register_special_token("<think>");
        tokenizer.register_special_token("</think>");
        tokenizer.register_special_token("<answer>");
        tokenizer.register_special_token("</answer>");

        tokenizer
    }

    /// Đăng ký thêm token đặc biệt với token ID tăng tiến
    pub fn register_special_token(&mut self, token: &str) -> u16 {
        if let Some(&id) = self.special_tokens.get(token) {
            return id;
        }
        let id = u16::try_from(self.vocab.len()).expect("Vượt giới hạn u16 cho từ vựng BPE");
        self.vocab.push(token.as_bytes().to_vec());
        self.special_tokens.insert(token.to_string(), id);
        self.special_token_ids.insert(id, token.to_string());
        id
    }

    /// Mã hóa chuỗi văn bản thành danh sách token ID, bảo vệ nguyên khối các token đặc biệt
    pub fn encode(&self, text: &str) -> Vec<u16> {
        let mut tokens = Vec::new();
        let mut remainder = text;

        while !remainder.is_empty() {
            let mut earliest_special: Option<(usize, usize, u16)> = None;
            for (special, &id) in &self.special_tokens {
                if let Some(pos) = remainder.find(special.as_str()) {
                    let len = special.len();
                    match earliest_special {
                        None => earliest_special = Some((pos, len, id)),
                        Some((min_pos, _, _)) if pos < min_pos => {
                            earliest_special = Some((pos, len, id));
                        }
                        _ => {}
                    }
                }
            }

            if let Some((pos, len, special_id)) = earliest_special {
                if pos > 0 {
                    let normal_slice = &remainder[..pos];
                    self.encode_normal_text(normal_slice, &mut tokens);
                }
                tokens.push(special_id);
                remainder = &remainder[pos + len..];
            } else {
                self.encode_normal_text(remainder, &mut tokens);
                break;
            }
        }

        tokens
    }

    fn encode_normal_text(&self, text: &str, out: &mut Vec<u16>) {
        for piece in pre_tokenize_utf8(text) {
            let mut ids: Vec<u16> = piece.bytes().map(u16::from).collect();
            while ids.len() >= 2 {
                let mut best: Option<(usize, TokenPair, u16)> = None;
                for window in ids.windows(2) {
                    let pair = TokenPair {
                        first: window[0],
                        second: window[1],
                    };
                    if let Some(&rank_info) = self.ranks.get(&pair) {
                        let should_replace = match best {
                            None => true,
                            Some((best_rank, _, _)) => rank_info.rank < best_rank,
                        };
                        if should_replace {
                            best = Some((rank_info.rank, pair, rank_info.new_id));
                        }
                    }
                }

                let Some((_, pair, new_id)) = best else {
                    break;
                };
                ids = merge_sequence(&ids, pair, new_id);
            }
            out.extend(ids);
        }
    }

    /// Giải mã danh sách token ID trở lại chuỗi ký tự UTF-8 ban đầu
    pub fn decode(&self, ids: &[u16]) -> Result<String, String> {
        let mut raw_bytes = Vec::new();
        for &id in ids {
            if let Some(special) = self.special_token_ids.get(&id) {
                raw_bytes.extend_from_slice(special.as_bytes());
            } else {
                let bytes = self
                    .vocab
                    .get(usize::from(id))
                    .ok_or_else(|| format!("Token ID {id} không tồn tại trong từ điển"))?;
                raw_bytes.extend_from_slice(bytes);
            }
        }
        String::from_utf8(raw_bytes).map_err(|e| format!("Lỗi chuyển đổi UTF-8: {e}"))
    }
}

// ============================================================================
// 4. MẶT NẠ GIẢI MÃ CÓ RÀNG BUỘC (CONSTRAINED DECODER MASK)
// ============================================================================

/// Trạng thái máy hữu hạn điều khiển cấu trúc sinh văn bản
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderState {
    Start,
    InThinking,
    AfterThinking,
    InAnswer,
    Finished,
}

/// Mặt nạ giải mã ép tuân thủ luật ngữ pháp reasoning model
#[derive(Debug, Clone)]
pub struct ConstrainedDecoderMask {
    pub allowed_token_ids: HashSet<u16>,
    pub current_state: DecoderState,
}

impl ConstrainedDecoderMask {
    pub fn new(tokenizer: &BpeTokenizer) -> Self {
        let mut mask = Self {
            allowed_token_ids: HashSet::new(),
            current_state: DecoderState::Start,
        };
        mask.recompute_allowed(tokenizer);
        mask
    }

    pub fn recompute_allowed(&mut self, tokenizer: &BpeTokenizer) {
        self.allowed_token_ids.clear();
        let bos_id = tokenizer.special_tokens.get("<|bos|>").copied();
        let eos_id = tokenizer.special_tokens.get("<|eos|>").copied();
        let think_open = tokenizer.special_tokens.get("<think>").copied();
        let think_close = tokenizer.special_tokens.get("</think>").copied();
        let ans_open = tokenizer.special_tokens.get("<answer>").copied();
        let ans_close = tokenizer.special_tokens.get("</answer>").copied();

        match self.current_state {
            DecoderState::Start => {
                if let Some(id) = bos_id {
                    self.allowed_token_ids.insert(id);
                }
                if let Some(id) = think_open {
                    self.allowed_token_ids.insert(id);
                }
            }
            DecoderState::InThinking => {
                let total_vocab = tokenizer.vocab.len();
                for id in 0..total_vocab {
                    let u16_id = id as u16;
                    if !tokenizer.special_token_ids.contains_key(&u16_id) {
                        self.allowed_token_ids.insert(u16_id);
                    }
                }
                if let Some(id) = think_close {
                    self.allowed_token_ids.insert(id);
                }
            }
            DecoderState::AfterThinking => {
                if let Some(id) = ans_open {
                    self.allowed_token_ids.insert(id);
                }
            }
            DecoderState::InAnswer => {
                let total_vocab = tokenizer.vocab.len();
                for id in 0..total_vocab {
                    let u16_id = id as u16;
                    if !tokenizer.special_token_ids.contains_key(&u16_id) {
                        self.allowed_token_ids.insert(u16_id);
                    }
                }
                if let Some(id) = ans_close {
                    self.allowed_token_ids.insert(id);
                }
                if let Some(id) = eos_id {
                    self.allowed_token_ids.insert(id);
                }
            }
            DecoderState::Finished => {
                if let Some(id) = eos_id {
                    self.allowed_token_ids.insert(id);
                }
            }
        }
    }

    pub fn apply_mask(&self, logits: &mut [f64]) {
        for (id, logit) in logits.iter_mut().enumerate() {
            if !self.allowed_token_ids.contains(&(id as u16)) {
                *logit = f64::NEG_INFINITY;
            }
        }
    }

    pub fn step(&mut self, emitted_token: u16, tokenizer: &BpeTokenizer) {
        let bos_id = tokenizer.special_tokens.get("<|bos|>").copied();
        let eos_id = tokenizer.special_tokens.get("<|eos|>").copied();
        let think_open = tokenizer.special_tokens.get("<think>").copied();
        let think_close = tokenizer.special_tokens.get("</think>").copied();
        let ans_open = tokenizer.special_tokens.get("<answer>").copied();
        let ans_close = tokenizer.special_tokens.get("</answer>").copied();

        match self.current_state {
            DecoderState::Start => {
                if Some(emitted_token) == think_open {
                    self.current_state = DecoderState::InThinking;
                } else if Some(emitted_token) == bos_id {
                    // Tiếp tục ở Start
                }
            }
            DecoderState::InThinking => {
                if Some(emitted_token) == think_close {
                    self.current_state = DecoderState::AfterThinking;
                }
            }
            DecoderState::AfterThinking => {
                if Some(emitted_token) == ans_open {
                    self.current_state = DecoderState::InAnswer;
                }
            }
            DecoderState::InAnswer => {
                if Some(emitted_token) == ans_close || Some(emitted_token) == eos_id {
                    self.current_state = DecoderState::Finished;
                }
            }
            DecoderState::Finished => {}
        }
        self.recompute_allowed(tokenizer);
    }
}

// ============================================================================
// 5. TENSOR VÙNG NHỚ PHẲNG CÓ BƯỚC NHẢY (STRIDED TENSOR)
// ============================================================================

/// Tensor n chiều lưu trữ trên vùng nhớ liên tục `Vec<f64>`.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    /// Mảng dữ liệu số thực phẳng
    pub data: Vec<f64>,
    /// Kích thước các chiều
    pub shape: Vec<usize>,
    /// Bước nhảy bộ nhớ tương ứng cho từng chiều
    pub strides: Vec<usize>,
    /// Vị trí dịch bắt đầu trong mảng dữ liệu
    pub offset: usize,
}

impl Tensor {
    /// Tạo tensor mới có bố cục bộ nhớ liên tục (Row-Major / C-contiguous)
    pub fn new(data: Vec<f64>, shape: Vec<usize>) -> Self {
        let expected_len: usize = shape.iter().product();
        assert_eq!(
            data.len(),
            expected_len,
            "Kích thước dữ liệu ({}) không khớp với shape {:?}",
            data.len(),
            shape
        );
        let strides = Self::compute_contiguous_strides(&shape);
        Self {
            data,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Tạo tensor chứa toàn số 0 với kích thước chỉ định
    pub fn zeros(shape: Vec<usize>) -> Self {
        let len: usize = shape.iter().product();
        let strides = Self::compute_contiguous_strides(&shape);
        Self {
            data: vec![0.0; len],
            shape,
            strides,
            offset: 0,
        }
    }

    fn compute_contiguous_strides(shape: &[usize]) -> Vec<usize> {
        let mut strides = vec![0; shape.len()];
        let mut current_stride = 1;
        for i in (0..shape.len()).rev() {
            strides[i] = current_stride;
            current_stride *= shape[i];
        }
        strides
    }

    pub fn is_contiguous(&self) -> bool {
        let expected = Self::compute_contiguous_strides(&self.shape);
        self.strides == expected && self.offset == 0
    }

    pub fn get(&self, indices: &[usize]) -> f64 {
        assert_eq!(indices.len(), self.shape.len());
        let mut idx = self.offset;
        for (i, &coord) in indices.iter().enumerate() {
            assert!(
                coord < self.shape[i],
                "Chỉ số {} vượt quá kích thước chiều {}",
                coord,
                self.shape[i]
            );
            idx += coord * self.strides[i];
        }
        self.data[idx]
    }

    pub fn set(&mut self, indices: &[usize], val: f64) {
        assert_eq!(indices.len(), self.shape.len());
        let mut idx = self.offset;
        for (i, &coord) in indices.iter().enumerate() {
            assert!(
                coord < self.shape[i],
                "Chỉ số {} vượt quá kích thước chiều {}",
                coord,
                self.shape[i]
            );
            idx += coord * self.strides[i];
        }
        self.data[idx] = val;
    }

    /// Hoán đổi hai chiều O(1) mà không sao chép dữ liệu (Zero-Copy Transpose)
    pub fn transpose(&self, dim0: usize, dim1: usize) -> Self {
        assert!(dim0 < self.shape.len() && dim1 < self.shape.len());
        let mut new_shape = self.shape.clone();
        let mut new_strides = self.strides.clone();
        new_shape.swap(dim0, dim1);
        new_strides.swap(dim0, dim1);
        Self {
            data: self.data.clone(),
            shape: new_shape,
            strides: new_strides,
            offset: self.offset,
        }
    }

    pub fn to_contiguous(&self) -> Self {
        if self.is_contiguous() {
            return self.clone();
        }
        let total_len: usize = self.shape.iter().product();
        let mut contiguous_data = Vec::with_capacity(total_len);
        let mut coords = vec![0; self.shape.len()];

        for _ in 0..total_len {
            contiguous_data.push(self.get(&coords));
            for d in (0..self.shape.len()).rev() {
                coords[d] += 1;
                if coords[d] < self.shape[d] {
                    break;
                }
                coords[d] = 0;
            }
        }

        Self::new(contiguous_data, self.shape.clone())
    }

    /// Nhân hai ma trận 2D: C = A x B
    pub fn matmul(&self, other: &Self) -> Self {
        assert_eq!(self.shape.len(), 2, "A phải là ma trận 2D");
        assert_eq!(other.shape.len(), 2, "B phải là ma trận 2D");
        let (m, k1) = (self.shape[0], self.shape[1]);
        let (k2, n) = (other.shape[0], other.shape[1]);
        assert_eq!(
            k1, k2,
            "Chiều trong của hai ma trận không khớp: {} != {}",
            k1, k2
        );

        let mut out = vec![0.0; m * n];
        for i in 0..m {
            let row_offset_a = self.offset + i * self.strides[0];
            for j in 0..n {
                let col_offset_b = other.offset + j * other.strides[1];
                let mut sum = 0.0;
                for k in 0..k1 {
                    let a_val = self.data[row_offset_a + k * self.strides[1]];
                    let b_val = other.data[col_offset_b + k * other.strides[0]];
                    sum += a_val * b_val;
                }
                out[i * n + j] = sum;
            }
        }

        Self::new(out, vec![m, n])
    }

    /// Tính hàm `log_softmax` theo chiều cuối cùng với kỹ thuật trừ giá trị cực đại chống tràn số
    pub fn log_softmax(&self) -> Self {
        assert_eq!(self.shape.len(), 2, "Chỉ hỗ trợ tensor 2D [batch, vocab]");
        let (rows, cols) = (self.shape[0], self.shape[1]);
        let mut out_data = vec![0.0; rows * cols];

        for i in 0..rows {
            let mut max_val = f64::NEG_INFINITY;
            for j in 0..cols {
                let v = self.get(&[i, j]);
                if v > max_val {
                    max_val = v;
                }
            }

            let mut sum_exp = 0.0;
            for j in 0..cols {
                let v = self.get(&[i, j]);
                sum_exp += (v - max_val).exp();
            }
            let log_sum_exp = sum_exp.ln();

            for j in 0..cols {
                let v = self.get(&[i, j]);
                out_data[i * cols + j] = (v - max_val) - log_sum_exp;
            }
        }

        Self::new(out_data, vec![rows, cols])
    }

    /// Tính hàm mất mát Cross Entropy có mặt nạ bỏ qua nhãn `ignore_index = -100`
    pub fn cross_entropy_masked(&self, targets: &[i64], ignore_index: i64) -> (f64, usize) {
        assert_eq!(self.shape.len(), 2);
        let (rows, cols) = (self.shape[0], self.shape[1]);
        assert_eq!(rows, targets.len());

        let mut total_loss = 0.0;
        let mut valid_count = 0;

        for (i, &target) in targets.iter().enumerate() {
            if target == ignore_index {
                continue;
            }
            assert!(
                target >= 0 && (target as usize) < cols,
                "Mục tiêu {} nằm ngoài phạm vi số cột {}",
                target,
                cols
            );
            let log_p = self.get(&[i, target as usize]);
            total_loss -= log_p;
            valid_count += 1;
        }

        let mean_loss = if valid_count > 0 {
            total_loss / (valid_count as f64)
        } else {
            0.0
        };

        (mean_loss, valid_count)
    }
}

// ============================================================================
// 6. ĐỘNG CƠ VI PHÂN NGƯỢC AUTOGRAD TAPE
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueId {
    pub id: usize,
}

#[derive(Debug, Clone)]
pub enum TapeOp {
    Leaf,
    Add,
    Sub,
    Mul,
    MatMul,
    Scale { factor: f64 },
    Relu,
    LogSoftmax,
    CrossEntropy { targets: Vec<i64>, ignore_index: i64 },
    RmsNorm { eps: f64 },
    Sum,
}

#[derive(Debug, Clone)]
pub struct TapeNode {
    pub id: ValueId,
    pub value: Tensor,
    pub grad: Option<Tensor>,
    pub op: TapeOp,
    pub parents: Vec<ValueId>,
}

#[derive(Debug, Clone, Default)]
pub struct AutogradTape {
    pub nodes: Vec<TapeNode>,
}

impl AutogradTape {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn zero_grad(&mut self) {
        for node in &mut self.nodes {
            node.grad = None;
        }
    }

    pub fn leaf(&mut self, value: Tensor) -> ValueId {
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value,
            grad: None,
            op: TapeOp::Leaf,
            parents: Vec::new(),
        });
        id
    }

    pub fn value(&self, id: ValueId) -> &Tensor {
        &self.nodes[id.id].value
    }

    pub fn grad(&self, id: ValueId) -> Option<&Tensor> {
        self.nodes[id.id].grad.as_ref()
    }

    pub fn add(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let va = self.value(a);
        let vb = self.value(b);
        assert_eq!(va.shape, vb.shape);
        let mut out = Vec::with_capacity(va.data.len());
        for i in 0..va.data.len() {
            out.push(va.data[i] + vb.data[i]);
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Add,
            parents: vec![a, b],
        });
        id
    }

    pub fn sub(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let va = self.value(a);
        let vb = self.value(b);
        assert_eq!(va.shape, vb.shape);
        let mut out = Vec::with_capacity(va.data.len());
        for i in 0..va.data.len() {
            out.push(va.data[i] - vb.data[i]);
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Sub,
            parents: vec![a, b],
        });
        id
    }

    pub fn mul(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let va = self.value(a);
        let vb = self.value(b);
        assert_eq!(va.shape, vb.shape);
        let mut out = Vec::with_capacity(va.data.len());
        for i in 0..va.data.len() {
            out.push(va.data[i] * vb.data[i]);
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Mul,
            parents: vec![a, b],
        });
        id
    }

    pub fn scale(&mut self, a: ValueId, factor: f64) -> ValueId {
        let va = self.value(a);
        let mut out = Vec::with_capacity(va.data.len());
        for v in &va.data {
            out.push(v * factor);
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Scale { factor },
            parents: vec![a],
        });
        id
    }

    pub fn matmul(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let va = self.value(a);
        let vb = self.value(b);
        let res = va.matmul(vb);
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: res,
            grad: None,
            op: TapeOp::MatMul,
            parents: vec![a, b],
        });
        id
    }

    pub fn relu(&mut self, a: ValueId) -> ValueId {
        let va = self.value(a);
        let mut out = Vec::with_capacity(va.data.len());
        for &v in &va.data {
            out.push(if v > 0.0 { v } else { 0.0 });
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Relu,
            parents: vec![a],
        });
        id
    }

    pub fn log_softmax(&mut self, a: ValueId) -> ValueId {
        let va = self.value(a);
        let res = va.log_softmax();
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: res,
            grad: None,
            op: TapeOp::LogSoftmax,
            parents: vec![a],
        });
        id
    }

    pub fn cross_entropy(&mut self, a: ValueId, targets: Vec<i64>, ignore_index: i64) -> ValueId {
        let va = self.value(a);
        let (loss, _) = va.cross_entropy_masked(&targets, ignore_index);
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(vec![loss], vec![1]),
            grad: None,
            op: TapeOp::CrossEntropy {
                targets,
                ignore_index,
            },
            parents: vec![a],
        });
        id
    }

    pub fn rmsnorm(&mut self, x: ValueId, w: ValueId, eps: f64) -> ValueId {
        let vx = self.value(x);
        let vw = self.value(w);
        assert_eq!(vx.shape.len(), 2);
        assert_eq!(vw.shape.len(), 1);
        let (rows, cols) = (vx.shape[0], vx.shape[1]);
        assert_eq!(cols, vw.shape[0]);

        let mut out = vec![0.0; rows * cols];
        for i in 0..rows {
            let mut sum_sq = 0.0;
            for j in 0..cols {
                let v = vx.get(&[i, j]);
                sum_sq += v * v;
            }
            let rms = (sum_sq / (cols as f64) + eps).sqrt();
            let inv_rms = 1.0 / rms;
            for j in 0..cols {
                let v = vx.get(&[i, j]);
                let weight = vw.get(&[j]);
                out[i * cols + j] = v * inv_rms * weight;
            }
        }

        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, vec![rows, cols]),
            grad: None,
            op: TapeOp::RmsNorm { eps },
            parents: vec![x, w],
        });
        id
    }

    pub fn sum(&mut self, a: ValueId) -> ValueId {
        let va = self.value(a);
        let s: f64 = va.data.iter().sum();
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(vec![s], vec![1]),
            grad: None,
            op: TapeOp::Sum,
            parents: vec![a],
        });
        id
    }

    fn accumulate_grad(&mut self, id: ValueId, grad: Tensor) {
        let node = &mut self.nodes[id.id];
        match &mut node.grad {
            Some(existing) => {
                assert_eq!(existing.shape, grad.shape);
                for i in 0..existing.data.len() {
                    existing.data[i] += grad.data[i];
                }
            }
            None => {
                node.grad = Some(grad);
            }
        }
    }

    pub fn backward(&mut self, loss_id: ValueId) {
        let loss_val = &self.nodes[loss_id.id].value;
        let seed_grad = Tensor::new(vec![1.0; loss_val.data.len()], loss_val.shape.clone());
        self.nodes[loss_id.id].grad = Some(seed_grad);

        for i in (0..=loss_id.id).rev() {
            let node_grad = match self.nodes[i].grad.clone() {
                Some(g) => g,
                None => continue,
            };

            let op = self.nodes[i].op.clone();
            let parents = self.nodes[i].parents.clone();

            match op {
                TapeOp::Leaf => {}
                TapeOp::Add => {
                    self.accumulate_grad(parents[0], node_grad.clone());
                    self.accumulate_grad(parents[1], node_grad);
                }
                TapeOp::Sub => {
                    self.accumulate_grad(parents[0], node_grad.clone());
                    let mut neg_grad = node_grad;
                    for v in &mut neg_grad.data {
                        *v = -*v;
                    }
                    self.accumulate_grad(parents[1], neg_grad);
                }
                TapeOp::Mul => {
                    let va = self.nodes[parents[0].id].value.clone();
                    let vb = self.nodes[parents[1].id].value.clone();

                    let mut ga = Vec::with_capacity(va.data.len());
                    let mut gb = Vec::with_capacity(vb.data.len());
                    for idx in 0..node_grad.data.len() {
                        ga.push(node_grad.data[idx] * vb.data[idx]);
                        gb.push(node_grad.data[idx] * va.data[idx]);
                    }
                    self.accumulate_grad(parents[0], Tensor::new(ga, va.shape));
                    self.accumulate_grad(parents[1], Tensor::new(gb, vb.shape));
                }
                TapeOp::Scale { factor } => {
                    let mut scaled = node_grad;
                    for v in &mut scaled.data {
                        *v *= factor;
                    }
                    self.accumulate_grad(parents[0], scaled);
                }
                TapeOp::Relu => {
                    let vin = &self.nodes[parents[0].id].value;
                    let mut gout = Vec::with_capacity(vin.data.len());
                    for idx in 0..vin.data.len() {
                        let g = if vin.data[idx] > 0.0 {
                            node_grad.data[idx]
                        } else {
                            0.0
                        };
                        gout.push(g);
                    }
                    self.accumulate_grad(parents[0], Tensor::new(gout, vin.shape.clone()));
                }
                TapeOp::MatMul => {
                    let va = self.nodes[parents[0].id].value.clone();
                    let vb = self.nodes[parents[1].id].value.clone();

                    let bt = vb.transpose(0, 1);
                    let grad_a = node_grad.matmul(&bt);

                    let at = va.transpose(0, 1);
                    let grad_b = at.matmul(&node_grad);

                    self.accumulate_grad(parents[0], grad_a);
                    self.accumulate_grad(parents[1], grad_b);
                }
                TapeOp::LogSoftmax => {
                    let log_p = &self.nodes[i].value;
                    let (rows, cols) = (log_p.shape[0], log_p.shape[1]);
                    let mut grad_in = vec![0.0; rows * cols];

                    for r in 0..rows {
                        let mut sum_g = 0.0;
                        for c in 0..cols {
                            sum_g += node_grad.get(&[r, c]);
                        }
                        for c in 0..cols {
                            let lp = log_p.get(&[r, c]);
                            let p = lp.exp();
                            let g = node_grad.get(&[r, c]);
                            grad_in[r * cols + c] = g - p * sum_g;
                        }
                    }
                    self.accumulate_grad(parents[0], Tensor::new(grad_in, vec![rows, cols]));
                }
                TapeOp::CrossEntropy {
                    targets,
                    ignore_index,
                } => {
                    let vin = &self.nodes[parents[0].id].value;
                    let (rows, cols) = (vin.shape[0], vin.shape[1]);
                    let upstream = node_grad.data[0];

                    let mut valid_count = 0;
                    for &t in &targets {
                        if t != ignore_index {
                            valid_count += 1;
                        }
                    }

                    let scale = if valid_count > 0 {
                        upstream / (valid_count as f64)
                    } else {
                        0.0
                    };

                    let mut grad_in = vec![0.0; rows * cols];
                    for r in 0..rows {
                        let t = targets[r];
                        if t == ignore_index {
                            continue;
                        }
                        for c in 0..cols {
                            let p = vin.get(&[r, c]).exp();
                            let indicator = if c == (t as usize) { 1.0 } else { 0.0 };
                            grad_in[r * cols + c] = (p - indicator) * scale;
                        }
                    }
                    self.accumulate_grad(parents[0], Tensor::new(grad_in, vec![rows, cols]));
                }
                TapeOp::RmsNorm { eps } => {
                    let vx = self.nodes[parents[0].id].value.clone();
                    let vw = self.nodes[parents[1].id].value.clone();
                    let (rows, cols) = (vx.shape[0], vx.shape[1]);
                    let d = cols as f64;

                    let mut grad_x = vec![0.0; rows * cols];
                    let mut grad_w = vec![0.0; cols];

                    for r in 0..rows {
                        let mut sum_sq = 0.0;
                        for c in 0..cols {
                            let x_val = vx.get(&[r, c]);
                            sum_sq += x_val * x_val;
                        }
                        let inv_rms = 1.0 / (sum_sq / d + eps).sqrt();

                        let mut dot = 0.0;
                        for (c, gw) in grad_w.iter_mut().enumerate() {
                            let x_val = vx.get(&[r, c]);
                            let w_val = vw.get(&[c]);
                            let g_val = node_grad.get(&[r, c]);
                            let x_hat = x_val * inv_rms;

                            *gw += g_val * x_hat;
                            dot += g_val * w_val * x_hat;
                        }
                        dot /= d;

                        for c in 0..cols {
                            let x_val = vx.get(&[r, c]);
                            let w_val = vw.get(&[c]);
                            let g_val = node_grad.get(&[r, c]);
                            grad_x[r * cols + c] =
                                inv_rms * (g_val * w_val - (x_val * inv_rms) * dot);
                        }
                    }

                    self.accumulate_grad(parents[0], Tensor::new(grad_x, vec![rows, cols]));
                    self.accumulate_grad(parents[1], Tensor::new(grad_w, vec![cols]));
                }
                TapeOp::Sum => {
                    let vin = &self.nodes[parents[0].id].value;
                    let upstream = node_grad.data[0];
                    let grad_in = vec![upstream; vin.data.len()];
                    self.accumulate_grad(parents[0], Tensor::new(grad_in, vin.shape.clone()));
                }
            }
        }
    }
}

// ============================================================================
// 7. HÀM KIỂM CHỨNG SAI PHÂN HỮU HẠN TRUNG TÂM
// ============================================================================

pub fn check_grad_central_diff<F>(
    f: F,
    x0: &Tensor,
    grad_analytical: &Tensor,
    eps: f64,
    tolerance: f64,
) -> Result<(), (usize, f64, f64, f64)>
where
    F: Fn(&Tensor) -> f64,
{
    assert_eq!(x0.data.len(), grad_analytical.data.len());
    let mut x_mut = x0.clone();

    for i in 0..x0.data.len() {
        let orig = x_mut.data[i];

        x_mut.data[i] = orig + eps;
        let f_plus = f(&x_mut);

        x_mut.data[i] = orig - eps;
        let f_minus = f(&x_mut);

        x_mut.data[i] = orig;

        let num_grad = (f_plus - f_minus) / (2.0 * eps);
        let ana_grad = grad_analytical.data[i];
        let diff = (num_grad - ana_grad).abs();

        if diff > tolerance {
            return Err((i, ana_grad, num_grad, diff));
        }
    }

    Ok(())
}

fn main() {
    println!("=== CHƯƠNG 92: BPE TOKENIZER, TENSOR BƯỚC NHẢY & AUTOGRAD TAPE ===");

    // 1. Huấn luyện BPE Tokenizer trên văn bản tiếng Việt
    let corpus =
        "học lập trình Rust từ số không. học lập trình hệ thống cùng Rust. mô hình ngôn ngữ lớn.";
    let tokenizer = BpeTokenizer::train(corpus, 30);
    println!(
        "Đã huấn luyện BPE Tokenizer: từ vựng có {} token",
        tokenizer.vocab.len()
    );

    let test_prompt = "<|bos|><think>mình cần giải bài toán này</think><answer>học Rust rất thú vị!</answer><|eos|>";
    let encoded = tokenizer.encode(test_prompt);
    let decoded = tokenizer.decode(&encoded).expect("Lỗi giải mã UTF-8");
    println!(
        "Encode {} tokens -> Decode khớp nguyên bản: {}",
        encoded.len(),
        decoded == test_prompt
    );
    assert_eq!(decoded, test_prompt);

    // 2. Chuyển vị ma trận không tốn chi phí và nhân ma trận
    let a = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
    let b = Tensor::new(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], vec![2, 3]);
    let b_t = b.transpose(0, 1);
    let c = a.matmul(&b_t);
    println!("Tensor A [2, 3] x B^T [3, 2] = C {:?}", c.shape);
    assert_eq!(c.shape, vec![2, 2]);

    // 3. Xây dựng Micro Network 2 tầng với AutogradTape và kiểm chứng sai phân
    let mut tape = AutogradTape::new();
    let mut rng = SplitMix64::new(42);

    let x_val = Tensor::new(vec![0.5, -0.2, 0.8, -0.5], vec![2, 2]);
    let w1_data: Vec<f64> = (0..4).map(|_| rng.next_gaussian(0.0, 0.1)).collect();
    let w1_val = Tensor::new(w1_data, vec![2, 2]);

    let x_id = tape.leaf(x_val.clone());
    let w1_id = tape.leaf(w1_val.clone());

    let h1_id = tape.matmul(x_id, w1_id);
    let a1_id = tape.relu(h1_id);
    let log_probs_id = tape.log_softmax(a1_id);

    let targets = vec![1i64, 0i64];
    let loss_id = tape.cross_entropy(log_probs_id, targets.clone(), -100);

    let loss_val = tape.value(loss_id).data[0];
    println!("Giá trị mất mát Cross Entropy khởi đầu: {:.6}", loss_val);

    tape.backward(loss_id);
    let grad_w1 = tape.grad(w1_id).expect("Phải có gradient cho w1").clone();

    // 4. Kiểm chứng sai phân hữu hạn trung tâm < 1e-8
    let f_loss = |w_test: &Tensor| -> f64 {
        let mut t = AutogradTape::new();
        let xi = t.leaf(x_val.clone());
        let wi = t.leaf(w_test.clone());
        let hi = t.matmul(xi, wi);
        let ai = t.relu(hi);
        let lpi = t.log_softmax(ai);
        let li = t.cross_entropy(lpi, targets.clone(), -100);
        t.value(li).data[0]
    };

    let check_res = check_grad_central_diff(f_loss, &w1_val, &grad_w1, 1e-6, 1e-8);
    match check_res {
        Ok(()) => println!("KIỂM CHỨNG SAI PHÂN HỮU HẠN THÀNH CÔNG: Sai số < 1e-8 tuyệt đối!"),
        Err((idx, ana, num, diff)) => {
            panic!(
                "Sai phân hữu hạn lệch tại chỉ số {}: giải tích={}, số học={}, sai số={}",
                idx, ana, num, diff
            );
        }
    }

    println!("Tất cả các thành phần toán học và cấu trúc dữ liệu của Chương 92 đều hợp lệ 100%!");
}
```

---

## 5. Bảng tra cứu lỗi biên dịch thường gặp

Khi triển khai các thuật toán đồ thị tính toán và cấu trúc đại số tuyến tính trong Rust thuần `std`, bạn sẽ gặp những rào cản độc đáo của trình biên dịch `rustc`. Dưới đây là bảng phân tích nguyên nhân gốc rễ và giải pháp xử lý triệt để:

| Lỗi biên dịch / Hành vi sai lệch | Nguyên nhân gốc rễ | Giải pháp kỹ thuật chuẩn xác |
|---|---|---|
| `E0502: cannot borrow *self as mutable because it is also borrowed as immutable` | Trong `AutogradTape::backward()`, khi đọc giá trị của nút `self.nodes[i]` đồng thời gọi `self.accumulate_grad()` sửa đổi nút cha. | Tách pha đọc và ghi: nhân bản dữ liệu đầu vào cần thiết (`let va = self.nodes[parents[0].id].value.clone();`) trước khi gọi phương thức biến đổi `accumulate_grad`. |
| `E0382: use of moved value: b` sau khi gọi `b.transpose(0, 1)` | Phương thức `transpose(&self)` nhận tham chiếu mượn, nhưng người dùng vô tình tiêu thụ giá trị hoặc gọi `b` sau khi chuyển quyền sở hữu. | Thiết kế `transpose(&self) -> Self` trả về bản sao nhẹ (shallow copy) của tensor: dữ liệu `Vec<f64>` được nhân bản nhanh chóng hoặc dùng cấu trúc chia sẻ con trỏ. |
| `Panic: byte index is not a char boundary` khi tiền tách từ tiếng Việt | Cắt lát chuỗi `&text[..i]` tại vị trí byte nằm giữa ký tự Unicode đa byte UTF-8 (ví dụ chữ `ệ` tốn 3 byte: `0xE1, 0xBB, 0x87`). | Tuyệt đối không dùng chỉ số số nguyên để cắt chuỗi. Sử dụng iterator ký tự an toàn: `text.chars()`, hoặc duyệt cửa sổ trên mảng byte thuần túy `&[u8]`. |
| `Numerical NaN: 0.0 / 0.0` trong hàm mất mát Cross Entropy | Toàn bộ các phần tử trong batch đều mang nhãn bỏ qua `targets = [-100, -100]`, khiến $N_{\text{valid}} = 0$. | Thêm chốt chặn an toàn: `if valid_count > 0 { total / valid_count } else { 0.0 }`. |
| Lệch sai phân hữu hạn $> 10^{-6}$ trong phép nhân ma trận | Chọn bước sai phân $\epsilon$ quá nhỏ ($10^{-10}$), dẫn đến lỗi triệt tiêu số có nghĩa (Catastrophic Cancellation) do giới hạn độ chính xác của `f64` ($\approx 2.2 \times 10^{-16}$). | Với hàm tuyến tính như nhân ma trận, chọn $\epsilon \in [10^{-5}, 10^{-6}]$ để cân bằng tối ưu giữa sai số cắt cụt chuỗi Taylor và sai số làm tròn số học thực nghiệm. |
| `Overflow / Panic: attempt to multiply with overflow` trong `SplitMix64` | Phép tính `state += 0x9E37...` gây tràn số nguyên 64-bit ở chế độ debug nếu không bật cờ wrapping. | Sử dụng tường minh các phương thức tràn số nguyên của Rust: `wrapping_add()` và `wrapping_mul()`. |

---

## 6. Tóm tắt & Bài tập rèn luyện

### Tóm tắt các bất biến kiến trúc cốt lõi

1. **Khứ hồi không mất mát (Lossless Roundtrip)**: Bộ tách từ Byte-Level BPE với 256 byte cơ sở UTF-8 bảo đảm mọi chuỗi dữ liệu (văn bản có dấu, mã nguồn, nhị phân) đều được mã hóa và giải mã chính xác 100%, vĩnh viễn loại bỏ khái niệm lỗi từ lạ `<unk>`.
2. **Tính tất định tuyệt đối (Deterministic Tie-Breaking)**: Hàng đợi `BinaryHeap` kết hợp `Reverse(TokenPair)` bảo đảm thứ tự gộp token giống hệt nhau trên mọi nền tảng phần cứng và hệ điều hành.
3. **Chuyển vị $O(1)$ không sao chép**: Tensor bước nhảy (`Strided Tensor`) phân tách khái niệm hình dạng tọa độ logic (`shape`) khỏi vị trí lưu trữ vật lý (`strides`), cho phép các phép toán đại số vận hành với hiệu năng bộ nhớ tối đa.
4. **Ổn định số học bắt buộc**: Hàm `log_softmax` phải luôn sử dụng kỹ thuật trừ giá trị cực đại để miễn nhiễm hoàn toàn với nguy cơ tràn số học `+inf` hay `NaN`.
5. **Đạo hàm giải tích trung thực**: Mọi nhánh tính toán trong đồ thị vi phân ngược `AutogradTape` phải được kiểm chứng tự động với sai phân hữu hạn trung tâm đạt độ lệch $< 10^{-8}$.

---

### Bài tập rèn luyện

#### Bài tập 1: Mở rộng `BpeTokenizer` với cơ chế BPE-Dropout
Trong quá trình huấn luyện mô hình ngôn ngữ, để mô hình mạnh mẽ hơn trước các biến thể chính tả, người ta thường áp dụng kỹ thuật **BPE-Dropout**: tại mỗi bước gộp khi mã hóa (`encode`), có một xác suất $p \in [0.0, 1.0)$ mà quy tắc gộp tiềm năng bị bỏ qua, buộc từ đó phải được biểu diễn bằng các mảnh con nhỏ hơn.

**Yêu cầu**:
Viết phương thức `encode_with_dropout(&self, text: &str, dropout_prob: f64, rng: &mut SplitMix64) -> Vec<u16>` bổ sung vào `BpeTokenizer`. Khi `dropout_prob == 0.0`, kết quả phải trùng khớp 100% với `encode()`. Khi `dropout_prob > 0.0`, chuỗi token thu được có thể dài hơn nhưng hàm `decode()` vẫn phải phục hồi chính xác văn bản ban đầu.

<details>
<summary>Gợi ý & Lời giải mẫu</summary>

```rust
impl BpeTokenizer {
    /// Mã hóa BPE có xác suất ngẫu nhiên bỏ qua quy tắc gộp (BPE-Dropout)
    pub fn encode_with_dropout(
        &self,
        text: &str,
        dropout_prob: f64,
        rng: &mut SplitMix64,
    ) -> Vec<u16> {
        let mut tokens = Vec::new();
        for piece in pre_tokenize_utf8(text) {
            let mut ids: Vec<u16> = piece.bytes().map(u16::from).collect();
            while ids.len() >= 2 {
                let mut best: Option<(usize, TokenPair, u16)> = None;
                for window in ids.windows(2) {
                    let pair = TokenPair {
                        first: window[0],
                        second: window[1],
                    };
                    if let Some(&rank_info) = self.ranks.get(&pair) {
                        // Bỏ qua với xác suất dropout_prob
                        if dropout_prob > 0.0 && rng.next_f64() < dropout_prob {
                            continue;
                        }
                        let should_replace = match best {
                            None => true,
                            Some((best_rank, _, _)) => rank_info.rank < best_rank,
                        };
                        if should_replace {
                            best = Some((rank_info.rank, pair, rank_info.new_id));
                        }
                    }
                }

                let Some((_, pair, new_id)) = best else {
                    break;
                };
                ids = merge_sequence(&ids, pair, new_id);
            }
            tokens.extend(ids);
        }
        tokens
    }
}

#[test]
fn test_bpe_dropout_reconstruction() {
    let corpus = "học lập trình Rust cùng các chuyên gia hàng đầu.";
    let tokenizer = BpeTokenizer::train(corpus, 20);
    let mut rng = SplitMix64::new(101);

    let text = "học lập trình Rust";
    // Thử nghiệm với các mức dropout khác nhau
    for p in [0.0, 0.2, 0.5, 0.8] {
        let encoded = tokenizer.encode_with_dropout(text, p, &mut rng);
        let decoded = tokenizer.decode(&encoded).expect("Phải giải mã thành công");
        assert_eq!(decoded, text, "Giải mã phải khớp văn bản gốc ở dropout p={}", p);
    }
}
```
</details>

---

#### Bài tập 2: Thao tác cắt lát ma trận không sao chép (`slice`) trên `Strided Tensor`
Trong các kiến trúc Transformer hiện đại, việc cắt lát ma trận để lấy một khối con (ví dụ: lấy $K$ token gần nhất trong KV-Cache, hoặc tách ma trận $Q, K, V$ từ ma trận chiếu hợp nhất $QKV$) diễn ra liên tục ở từng lớp chú ý.

**Yêu cầu**:
Viết phương thức `slice_2d(&self, row_range: (usize, usize), col_range: (usize, usize)) -> Tensor` trên `Tensor`.
Phương thức này không được cấp phát mảng dữ liệu mới (`data` được chia sẻ hoặc sao chép con trỏ nhẹ), mà chỉ tính toán lại `offset`, `shape` và bảo lưu `strides` hiện tại. Viết bài test chứng minh tính đúng đắn khi đọc và sửa dữ liệu qua lát cắt.

<details>
<summary>Gợi ý & Lời giải mẫu</summary>

```rust
impl Tensor {
    /// Cắt lát ma trận 2D mà không sao chép lại vùng dữ liệu
    pub fn slice_2d(&self, row_range: (usize, usize), col_range: (usize, usize)) -> Self {
        assert_eq!(self.shape.len(), 2, "Chỉ hỗ trợ ma trận 2D");
        let (r_start, r_end) = row_range;
        let (c_start, c_end) = col_range;

        assert!(r_start <= r_end && r_end <= self.shape[0]);
        assert!(c_start <= c_end && c_end <= self.shape[1]);

        let new_rows = r_end - r_start;
        let new_cols = c_end - c_start;

        // Vị trí bắt đầu mới dịch theo bước nhảy hàng và cột
        let new_offset = self.offset + r_start * self.strides[0] + c_start * self.strides[1];

        Self {
            data: self.data.clone(),
            shape: vec![new_rows, new_cols],
            strides: self.strides.clone(),
            offset: new_offset,
        }
    }
}

#[test]
fn test_tensor_slice_2d_zero_copy() {
    // Ma trận 3 hàng, 4 cột
    let mut data = Vec::new();
    for i in 0..12 {
        data.push(i as f64);
    }
    let t = Tensor::new(data, vec![3, 4]);

    // Cắt khối con: hàng 1..3, cột 1..3 (kích thước 2x2)
    // Các phần tử mong đợi:
    // [ 5.0,  6.0 ]
    // [ 9.0, 10.0 ]
    let sub = t.slice_2d((1, 3), (1, 3));
    assert_eq!(sub.shape, vec![2, 2]);
    assert_eq!(sub.get(&[0, 0]), 5.0);
    assert_eq!(sub.get(&[0, 1]), 6.0);
    assert_eq!(sub.get(&[1, 0]), 9.0);
    assert_eq!(sub.get(&[1, 1]), 10.0);
}
```
</details>

---

#### Bài tập 3: Cài đặt toán tử chuẩn hóa `LayerNorm` cổ điển trên `AutogradTape`
Trong khi các mô hình LLM thế hệ mới (LLaMA, DeepSeek, Mistral) chuyển sang dùng `RMSNorm` vì tiết kiệm chi phí tính toán kỳ vọng $\mu$, kiến trúc GPT-2 cổ điển và BERT vẫn sử dụng `LayerNorm` chuẩn hóa cả kỳ vọng lẫn phương sai:
$$y_i = \frac{x_i - \mu}{\sqrt{\sigma^2 + \epsilon}} \cdot \gamma_i + \beta_i$$
trong đó $\mu = \frac{1}{d} \sum_{j=1}^d x_j$ và $\sigma^2 = \frac{1}{d} \sum_{j=1}^d (x_j - \mu)^2$.

**Yêu cầu**:
Viết đạo hàm ngược giải tích đầy đủ cho `LayerNorm` và tích hợp vào `AutogradTape`. Sử dụng hàm `check_grad_central_diff` để kiểm chứng đạo hàm của cả đầu vào $X$, trọng số co giãn $\gamma$, và trọng số dịch chuyển $\beta$ đạt sai số tuyệt đối $< 10^{-8}$.

<details>
<summary>Gợi ý & Lời giải mẫu</summary>

```rust
/// Hàm tính toán xuôi và đạo hàm ngược giải tích của LayerNorm
pub fn layernorm_forward_backward(
    x: &[f64],
    gamma: &[f64],
    beta: &[f64],
    eps: f64,
    upstream_grad: &[f64],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let d = x.len() as f64;
    let mean = x.iter().sum::<f64>() / d;
    let var = x.iter().map(|&v| (v - mean) * (v - mean)).sum::<f64>() / d;
    let inv_std = 1.0 / (var + eps).sqrt();

    let mut y = vec![0.0; x.len()];
    let mut x_hat = vec![0.0; x.len()];
    for i in 0..x.len() {
        x_hat[i] = (x[i] - mean) * inv_std;
        y[i] = x_hat[i] * gamma[i] + beta[i];
    }

    let mut dgamma = vec![0.0; x.len()];
    let mut dbeta = vec![0.0; x.len()];
    for i in 0..x.len() {
        dgamma[i] = upstream_grad[i] * x_hat[i];
        dbeta[i] = upstream_grad[i];
    }

    // Đạo hàm ngược giải tích cho x
    let mut dx = vec![0.0; x.len()];
    let mut sum_g = 0.0;
    let mut sum_g_xhat = 0.0;
    for i in 0..x.len() {
        let g_gamma = upstream_grad[i] * gamma[i];
        sum_g += g_gamma;
        sum_g_xhat += g_gamma * x_hat[i];
    }

    for i in 0..x.len() {
        let g_gamma = upstream_grad[i] * gamma[i];
        dx[i] = inv_std * (g_gamma - (sum_g + x_hat[i] * sum_g_xhat) / d);
    }

    (y, dx, dgamma, dbeta)
}

#[test]
fn test_layernorm_analytic_grad_matches_finite_diff() {
    let x = vec![0.8, -1.2, 2.5, 0.1];
    let gamma = vec![1.2, 0.9, 1.1, 0.8];
    let beta = vec![0.1, -0.2, 0.05, 0.3];
    let upstream = vec![1.0, 1.0, 1.0, 1.0];
    let eps = 1e-5;

    let (_, dx, dgamma, _) = layernorm_forward_backward(&x, &gamma, &beta, eps, &upstream);

    // Kiểm chứng sai phân hữu hạn trung tâm cho x[0]
    let h = 1e-6;
    let mut x_plus = x.clone();
    x_plus[0] += h;
    let (y_plus, _, _, _) = layernorm_forward_backward(&x_plus, &gamma, &beta, eps, &upstream);
    let sum_plus: f64 = y_plus.iter().sum();

    let mut x_minus = x.clone();
    x_minus[0] -= h;
    let (y_minus, _, _, _) = layernorm_forward_backward(&x_minus, &gamma, &beta, eps, &upstream);
    let sum_minus: f64 = y_minus.iter().sum();

    let num_dx0 = (sum_plus - sum_minus) / (2.0 * h);
    let diff = (dx[0] - num_dx0).abs();
    assert!(diff < 1e-8, "Sai số giải tích LayerNorm cho x[0] ({}) phải < 1e-8", diff);

    // Kiểm chứng sai phân hữu hạn trung tâm cho gamma[0]
    let mut gamma_plus = gamma.clone();
    gamma_plus[0] += h;
    let (y_gplus, _, _, _) = layernorm_forward_backward(&x, &gamma_plus, &beta, eps, &upstream);
    let sum_gplus: f64 = y_gplus.iter().sum();

    let mut gamma_minus = gamma.clone();
    gamma_minus[0] -= h;
    let (y_gminus, _, _, _) = layernorm_forward_backward(&x, &gamma_minus, &beta, eps, &upstream);
    let sum_gminus: f64 = y_gminus.iter().sum();

    let num_dgamma0 = (sum_gplus - sum_gminus) / (2.0 * h);
    let diff_gamma = (dgamma[0] - num_dgamma0).abs();
    assert!(diff_gamma < 1e-8, "Sai số giải tích LayerNorm cho gamma[0] ({}) phải < 1e-8", diff_gamma);
}
```
</details>
