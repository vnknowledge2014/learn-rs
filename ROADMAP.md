# Lộ trình học tập (Learning Roadmap)

Tài liệu này trả lời ba câu hỏi mà [`SUMMARY.md`](./SUMMARY.md) không trả lời được: **học theo thứ tự nào**, **bỏ qua được gì**, và **cái gì cố tình không có trong giáo trình**.

- 105 chương · 28 chủ đề · 110 crate · 1.862 bài kiểm thử · toàn bộ mã nguồn biên dịch và kiểm thử được
- `cargo test --workspace` phải xanh trước khi bạn tin bất cứ điều gì trong sách

---

## 1. Ba chặng

| Chặng | Chương | Sau chặng này bạn làm được gì |
|---|---|---|
| **I — Nền tảng** | 01–30 | Viết Rust thành thạo: sở hữu, lập trình hàm, macro, cấu trúc dữ liệu |
| **II — Hệ thống** | 31–69 | Dựng sản phẩm thật: CSDL, bảo mật, phân tán, web, nhúng, HĐH, game |
| **III — Chuyên sâu** | 70–105 | Bảy lĩnh vực Rust có lợi thế không thể thay thế |

Chặng I là **bắt buộc và tuần tự**. Chặng II và III chia thành các nhánh đọc song song được.

---

## 2. Đồ thị phụ thuộc

```
                    ┌─────────────────────────────┐
                    │  CHẶNG I — NỀN TẢNG 01–30   │
                    │  tuần tự, không nhảy cóc     │
                    └──────────────┬──────────────┘
                                   │
      ┌──────────┬─────────────┬───┴────────┬─────────────┬──────────────┐
      ▼          ▼             ▼            ▼             ▼              ▼
  ┌────────┐ ┌────────┐  ┌──────────┐ ┌─────────┐  ┌──────────┐  ┌────────────┐
  │ CSDL   │ │ Bảo    │  │ Phân tán │ │ Web &   │  │ Nhúng &  │  │ Game       │
  │ 31–36  │ │ mật    │  │ 48–54    │ │ Desktop │  │ Phần cứng│  │ 68         │
  │        │ │ 37–42  │  │ 55,59    │ │ 61–63   │  │ 66–67    │  │            │
  └────┬───┘ └───┬────┘  └────┬─────┘ └─────────┘  └────┬─────┘  └────────────┘
       │         │            │                          │
       │         │            │  ┌───────────────────────┘
       │         ▼            │  │
       │   ┌──────────┐       │  │
       │   │ Blockchain       │  │
       │   │ 70–73    │◄──────┘  │
       │   └────┬─────┘          │
       │        │                │
       ▼        ▼                ▼
  ┌─────────────────────────────────────────────┐
  │  HFT 74–78  ──►  Phần cứng 79–81            │
  │      │                  │                    │
  │      └────────┬─────────┘                    │
  │               ▼                              │
  │      Định lượng 82–84                        │
  │               ▼                              │
  │      ★ 85: HỆ SINH THÁI TÍCH HỢP ★          │
  │      (yêu cầu ĐỦ 74–78; nên đọc sau 70–73)  │
  └─────────────────────────────────────────────┘
                                   │
      ┌────────────────────────────┴───────────────────────────┐
      │                                                        │
      ▼                                                        ▼
┌──────────────────────────────────────────┐    ┌──────────────────────────────────────────┐
│ Kỹ nghệ Tác tử 86–90 (yêu cầu 01–30, 56) │    │ Kiến trúc LLM 92–96 (yêu cầu 01–30, 26)  │
│                │                         │    │                │                         │
│                ▼                         │    │                ▼                         │
│ ★ 91: ĐẠI DỰ ÁN TÁC TỬ TỰ HÀNH ★         │    │ ★ 97: ĐẠI DỰ ÁN MICRO-LLM & RLVR ★       │
│ (yêu cầu ĐỦ 86–90)                       │    │ (yêu cầu ĐỦ 92–96; nối công cụ với 86–87)│
└──────────────────────────────────────────┘    └──────────────────────────────────────────┘
                                    │
                                    ▼
 ┌──────────────────────────────────────────────────────────────────────────────────────────┐
 │ Bảo mật Tấn công 98–103 (yêu cầu 01–30, 37–42, 57, 65, 70)                               │
 │   Binary exploit · Mạng · Web & LLM · Kerberos · Leo thang đặc quyền · Xoay chiều & C2   │
 │                                   │                                                      │
 │                                   ▼                                                      │
 │  ★ 104: BỘ CÔNG CỤ RED TEAM (Proof-by-Exploitation) ★                                    │
 │             + ★ 105: TÁC TỬ PENTEST TỰ HÀNH (đa tác tử, RL, Red-vs-Blue) ★               │
 │  (105 nối công cụ với bộ khung tác tử 86–90: StateGraph, HITL, Hiến pháp)                │
 └──────────────────────────────────────────────────────────────────────────────────────────┘
```

**Điều kiện tiên quyết thực sự**, không phải gợi ý:

| Chương | Bắt buộc đọc trước | Vì sao |
|---|---|---|
| 70–73 | 01–30 | Cần `BTreeMap`, trait, xử lý lỗi, kiểm thử |
| 71 | 65 (mạng) | Dùng lại khái niệm đóng gói theo tầng |
| 74–78 | 01–30, và 26 (bảng băm/cây) | Sổ lệnh là cấu trúc dữ liệu trước khi là tài chính |
| 79 | 67 (phần cứng số) | Chương 67 dạy cổng logic; chương 79 dùng nó |
| 80–81 | 74 (bố cục bộ nhớ) | Chương 74 giới thiệu dòng cache, AoS/SoA |
| 82–84 | không có | Đọc độc lập được, chỉ cần chặng I |
| **85** | **74, 75, 76, 77, 78** | Nó **nối** đúng năm chương đó lại |
| 86–90 | 01–30, 56 | Cần cấu trúc dữ liệu, typestate và kiến trúc tác tử cơ bản ở ch56 |
| **91** | **86, 87, 88, 89, 90** | Nó **nối** trọn vẹn bộ khung tác tử tự hành cấp sản xuất |
| 92–96 | 01–30, và 26 | Cần thao tác mảng/lát cắt bộ nhớ, con trỏ thông minh và tensor |
| **97** | **92, 93, 94, 95, 96** | Nó **nối** toàn bộ đường ống Micro-LLM và RLVR từ số không |
| 98 | 01–30, và 37–42 | Cần nền bộ nhớ ảo, buffer overflow, `unsafe` và tư duy OSCP ở ch37–42 |
| 99 | 01–30, 48–50, 65 | Cần socket, luồng hai chiều và mô hình máy trạng thái TCP/DNS ở ch48–50, 65 |
| 100 | 01–30, 57, và 64–65 | Chương 57 dạy SQLi/XSS/SSRF/path traversal; ch64–65 dạng HTTP và bảo mật web |
| 101 | 01–30, và 70 | Cần băm (SHA-256) và xử lý byte ở ch70; MD4/MD5/RC4/AES tự cài lại từ đầu |
| 102 | 01–30, và 37–42 | Cần phân quyền, quyền POSIX và tư duy kiểm thử từ ch37–42 |
| 103 | 01–30, 48–50, 65 | Cần mạng, luồng và mã hoá kênh; chương 103 tự cài ChaCha20/X25519 |
| **104** | **98, 99, 100, 101, 102, 103** | Nó **nối** trọn vẹn sáu chương công cụ khai thác lại |
| **105** | **104**, và 86–91 | Nó nối bộ công cụ với StateGraph/HITL/Hiến pháp ở ch86–91 |

---

## 3. Bảy nhánh theo mục tiêu

Chọn một nhánh, đọc hết, rồi quay lại chọn nhánh khác. Đừng đọc song song bảy nhánh.

### Nhánh A — Kỹ sư hệ thống / backend
`01–30` → `31–36` (CSDL) → `48–54` (phân tán) → `55` (kiểm thử) → `59` (mở rộng) → `61–63` (web/desktop) → `64–65` (HĐH/mạng)

### Nhánh B — Giao dịch định lượng & HFT
`01–30` → `26` → `69` (sổ lệnh nhập môn) → `74–78` → **`85`** → `82–84` (định lượng) → `79–81` (khi cần chạm trần hiệu năng)

> Đây là nhánh dài nhất và cũng là nhánh có mật độ "lỗi thật" cao nhất trong sách. Đọc chương 85 **sau cùng** trong nhóm 74–78: nó tồn tại để cho thấy năm mảnh đúng riêng lẻ vẫn ghép thành một hệ sai.

### Nhánh C — Blockchain & Web3
`01–30` → `65` (mạng) → `70` (blockchain từ số không) → `71` (P2P) → `72` (CosmWasm/Solana) → `73` (Ethereum) → `78` (thị trường DeFi) → `85` (nếu muốn nối với thị trường truyền thống)

### Nhánh D — Hiệu năng & phần cứng
`01–30` → `74` (bố cục bộ nhớ) → `80` (CPU sâu) → `81` (GPU) → `66–67` (nhúng, phần cứng số) → `79` (FPGA cho giao dịch)

### Nhánh E — Kỹ sư Tác tử AI, Bộ khung & Hệ tự hành
`01–30` → `34–35` (CSDL/WAL/MVCC) → `48–50, 54` (hệ phân tán/Actor/lũy đẳng) → `55–57, 59` (kiểm thử/bảo mật web/mở rộng) → `70, 76–77` (mã hóa/đồng hồ ảo/quản trị rủi ro) → `86–90` → **`91` ★**

> Nhánh chuyên sâu về kỹ nghệ harness, quản lý ngân sách KV-cache, chuẩn hóa SKILL.md, đồ thị kép, độ bền WAL/Saga, và điều phối đa tác tử cấp sản xuất. Chương 91 tích hợp toàn bộ thành bộ khung tác tử lập trình tự hành an toàn.

### Nhánh F — Kỹ sư Kiến trúc LLM, Động cơ Suy luận & Hậu huấn luyện RLVR
`01–30` → `55, 64` (kiểm thử/lập lịch HĐH) → `80–81` (CPU sâu/SIMD, GPU/SIMT) → `83–84` (toán số học/xác suất) → `92–96` → **`97` ★** *(đọc nối với `86, 94` để nắm trọn hai phía máy khách–máy chủ của KV-cache)*

> Nhánh xây dựng toàn bộ mô hình ngôn ngữ lớn từ con số 0 bằng thuần Rust std: từ Byte-BPE, autograd ngược, Transformer hiện đại (RMSNorm, RoPE/YaRN, SwiGLU, GQA, MLA, MoE) tới động cơ phục vụ (Muon, Paged/RadixAttention, speculative decoding), căn chỉnh SFT/LoRA/DPO và học tăng cường RLVR (GRPO).

### Nhánh G — Kỹ sư Bảo mật Tấn công & Tác tử Pentest Tự hành
`01–30` → `37–42` (bộ nhớ ảo, buffer overflow, `unsafe`, tư duy OSCP) → `57` (SQLi/XSS/SSRF/path traversal) → `65` (TCP/IP, DNS) → `70` (SHA-256, xử lý byte) → `98` (khai thác nhị phân) → `99` (bộ công cụ mạng) → `100` (khai thác web & bảo mật LLM) → `101` (Kerberos & bẻ khoá băm) → `102` (leo thang đặc quyền) → `103` (xoay chiều & C2) → **`104`** ★ → **`105`** ★ *(đọc nối với `86–90` để có StateGraph, HITL và Hiến pháp)*

> Nhánh dựng bộ công cụ tấn công **thật** bằng Rust thuần `std` — không phụ thuộc ngoài: proxy SOCKS5, chuyển tiếp cổng, nhận dạng dịch vụ, giả mạo ARP/DNS, SQLi mù với máy phân tích SQL thật, bẻ khoá MD4/MD5/NTLM/Kerberos, leo thang đặc quyền có **kiểm chứng sự thật nền**, đường hầm SSH, C2 mã hoá ChaCha20 + X25519. Hai chương cuối khép vòng: Ch 104 bắt buộc mọi phát hiện phải có **PoC khai thác chạy được** (tỷ lệ báo động giả = 0), Ch 105 ghép tất cả vào một tác tử pentest tự hành đa tác tử có phần thưởng RL, đối kháng red-vs-blue và hiến pháp 10 điều.

> **Lưu ý đạo đức**: toàn bộ exploit trong nhánh này chạy trên dữ liệu tổng hợp tất định (`MockFilesystem`, `MockNetwork`, `MockShell`) hoặc trên binary do chính giáo trình tạo ra bằng `unsafe`; không mã nào kết nối tới hệ thống thật ngoài loopback.

---

## 4. Bản đồ phủ OpenAlgo (13 khoá · 407 chương)

Nguồn: <https://www.openalgo.in/learn> (kiểm kê ngày 05/09/2026). Yêu cầu ban đầu là "tất cả các bài học trong phần learn nên được dạy bằng Rust". Dưới đây là trạng thái thật của từng khoá, kể cả những khoá **không** được chuyển và lý do.

| Khoá | Ch. | Mức | Trạng thái trong giáo trình này |
|---|---:|---|---|
| Technical Analysis | 28 | Cơ bản | ✅ **Chương 82** — nến, mẫu hình, SMA/EMA/RSI/MACD/Bollinger/ATR, kèm bất biến chống nhìn trộm tương lai |
| Options Basics | 26 | Cơ bản | ✅ **Chương 83** — quyền mua/bán, moneyness, Greeks, biến động ngụ ý |
| Options Strategies | 27 | Trung cấp | ✅ **Chương 83** — payoff spread/straddle/condor, điểm hoà vốn |
| Statistical Arbitrage | 17 | Chuyên sâu | ✅ **Chương 84** — tính dừng, đồng liên kết, Kalman, trung tính thị trường |
| Quantitative Trading | 78 | Chuyên sâu | ◐ **Chương 84 + 74–78 + 85** — vi cấu trúc, công nghệ HFT và thực thi nằm ở nhóm HFT; thời gian chuỗi, phái sinh, nghiên cứu alpha, kiểm định trung thực ở ch84 |
| Algo Trading with Python | 32 | Trung cấp | ◐ **Chương 69 + 77 + 85** — chỉ báo, tín hiệu, lệnh, quản trị rủi ro được cài lại bằng Rust; phần SDK riêng của OpenAlgo không chuyển |
| Futures Trading | 27 | Cơ bản | ◐ **Một phần** — định cỡ vị thế và hạn mức nằm trong ch77; ký quỹ, đòn bẩy và phần đặc thù thị trường Ấn Độ không chuyển |
| Risk Management | 33 | Cơ bản | ◐ **Chương 77 + 84** — định cỡ, dừng lỗ, hạn mức, VaR/ES, sụt giảm |
| Python for Traders | 40 | Cơ bản | ✗ **Không chuyển** — đây là khoá dạy chính ngôn ngữ Python. Bản tương ứng cho Rust là chặng I (chương 01–30) |
| Stock Market Basics | 18 | Cơ bản | ✗ **Không chuyển** — kiến thức thị trường, không có phần lập trình được |
| AmiBroker AFL | 36 | Cơ bản | ✗ **Không chuyển** — dạy một ngôn ngữ độc quyền khác |
| Taxation for Traders | 19 | Cơ bản | ✗ **Không chuyển** — luật thuế Ấn Độ, thay đổi theo năm và theo quốc gia |
| Trading Psychology | 26 | Cơ bản | ✗ **Không chuyển** — không có phần lập trình được |

**Tổng kết trung thực.** 98 chương thuộc bốn khoá lập trình được cốt lõi được chuyển đầy đủ sang Rust (Technical Analysis, Options Basics, Options Strategies, Statistical Arbitrage). Thêm khoảng 170 chương của bốn khoá còn lại được phủ một phần qua nhóm HFT và quản trị rủi ro. **139 chương thuộc năm khoá cuối bảng không được chuyển**, vì chúng dạy một ngôn ngữ khác, luật pháp một quốc gia, hoặc tâm lý con người — không phải thứ chuyển sang Rust được. Sao chép chúng sang tiếng Việt sẽ là dịch thuật, không phải giảng dạy lập trình.

---

## 5. Bản đồ phủ LeetCPU & LeetGPU

| Nguồn | Quy mô | Trạng thái |
|---|---|---|
| [leetcpu.com](https://www.leetcpu.com/) | 22 bài, 5 nhóm (4 nhóm kỹ thuật + 1 bài chẩn đoán), chạy trên ChampSim | ✅ **Chương 80** — cả 22 bài được ánh xạ sang kỹ thuật tương ứng, xem bảng trong chương |
| [leetgpu.com](https://leetgpu.com/) | 99 thử thách (19 Dễ / 65 Vừa / 15 Khó) | ✅ **Chương 81** — phân loại 11 nhóm chủ đề, quy về 4 kỹ thuật gốc |

Cả hai trang đều là ứng dụng một trang; dữ liệu được thu thập bằng cách kết xuất trong trình duyệt thật rồi đọc DOM. Nội dung chi tiết từng bài của LeetCPU nằm sau đăng nhập và **không** được truy cập — chúng ta dùng danh sách bài công khai làm phân loại, rồi tự cài kỹ thuật bằng Rust.

---

## 6. Bản đồ phủ Harness Patterns & 14 Kho AI / Agent / LLM Quốc Tế (15 Nguồn)

Nguồn: Phân tích kiểm toán đối chiếu toàn bộ mẫu kiến trúc tác tử (`R0`) và 14 kho mã nguồn AI / Agent / LLM quốc tế (`#1`–`#14`) với `learn-rs`.

### Bảng 6.1 — Bản đồ phủ mẫu kiến trúc tác tử (`ashishpatel26/agent-harness-engineering-pattern` — 13 chương · 192 mẫu)

| Nhóm | Số mẫu | Trạng thái trong giáo trình | Mô tả kỹ thuật |
|---|---:|---|---|
| **Đã có trong Ch 01–85** | 50 | ✅ **Chương 56 & các chương khác** | `pack_context`, `lost_in_middle_order`, `run_agent_loop`, `KnowledgeGraph`, `MiniLsmEngine`/WAL, `CircuitBreaker`, Actor `mpsc`, Raft, `safe_path`, `TokenBucket`, `sha256`, cache/SIMD/SIMT |
| **Loại bỏ / Thay bằng rustc** | 68 | ✗ **Không cần thiết trong Rust** | Các mẫu kiểm tra kiểu động Python, runtime monkey-patching, retry vô điều kiện được thay thế hoàn toàn bằng hệ thống kiểu tĩnh, Borrow Checker, Ownership, Enum Result/Option của Rust |
| **Cài đặt mới trong Chủ đề 26** | 74 | ✅ **Chương 86–90** | Vòng lặp SSE, chặn max_tokens, KV-cache prefix freeze, sổ cái PicoDollars, PreToolHook, SecretString, LlmGateway 3 tầng, TurnWal, SessionTree, Saga, Mediator, WorktreeSandbox, LoopDrill |
| **Đại dự án tích hợp Coding-Agent** | 5 | ✅ **Chương 91** | Nối trọn vẹn 5 cơ chế cốt lõi: vòng lặp streaming, công cụ định kiểu, đồ thị kiểm soát, phục hồi sự cố và điều phối tự hành |

> **Kỷ luật phòng sạch cho R0**: Do kho gốc `ashishpatel26` không có giấy phép (`license: null` — All Rights Reserved), toàn bộ 192 mẫu chỉ được tiếp thu dưới dạng phân loại kiến trúc; tuyệt đối không dịch văn xuôi, không chép sơ đồ hay chuyển ngữ mã nguồn Python sang `learn-rs`.

### Bảng 6.2 — Bản đồ phủ 14 Kho GitHub AI / Agent / Context / Loop / Graph / LLM Quốc Tế (`#1`–`#14`)

| # | Kho nguồn | Giấy phép | Quy mô | Trạng thái | Nội dung chắt lọc sang Rust 2024 thuần `std` |
|---|---|---|---:|---|---|
| **#1** | `rohitg00/ai-engineering-from-scratch` | `MIT` | 542 bài, 11k tệp | ◐ **Ch 87–90, 92–97** | Byte-BPE, autograd ngược, MLA, MoE, Paged/RadixAttention, LoRA, GRPO, BM25+RRF, SKILL.md; bỏ thị giác/âm thanh |
| **#2** | `alchaincyf/loop-engineering-orange-book` | `MIT` ("Work") | 9 chương, PDF | ◐ **Ch 88, 90** | Stripe Minions Blueprints, đan xen nút kiểm tra tất định, tách Giám khảo Tầng 1 (Transcript) vs Tầng 2 (Environment) |
| **#3** | `cobusgreyling/loop-engineering` | `MIT` | 723 tệp, 496 tests | ◐ **Ch 89, 90** | Các công cụ `loop-*` (worktree cô lập, context, sync, drill); cài lại bằng Rust 2024 thuần `std` |
| **#4** | `walkinglabs/learn-harness-engineering` | `MIT` | 15 dự án, 2.7k tệp | ◐ **Ch 86, 87, 89** | Pi SessionTree rẽ nhánh không mất dữ liệu, bất biến "Model-visible means logged", kiểm tra kiến trúc; không đụng vào `docs/vi/` |
| **#5** | `jasontang-ai/Context-Engineering` | `MIT` | 298 tệp | ◐ **Ch 86** | Công thức nén ngữ cảnh, bảo toàn tiền tố KV-cache; tự viết mới bằng Rust loại bỏ hoàn toàn lỗi cú pháp Python |
| **#6** | `muratcankoylan/Agent-Skills-for-Context-Engineering` | `MIT` | 511 tệp, 198 tests | ◐ **Ch 87, 90** | Chuẩn SKILL.md 3 tầng, phân cấp AGENTS.md, hiến pháp an toàn SoD RBAC, bánh cóc tự tiến hóa hai tập tách RSI Ratchet |
| **#7** | `dair-ai/Prompt-Engineering-Guide` | `MIT` | 1.5k tệp | ◐ **Ch 88** | Khung suy luận ToT (Tree of Thoughts), Plan-and-Solve, Reflexion, đóng vai trò nền tảng cho đồ thị nhận thức |
| **#8** | `codejunkie99/graph-engineering` | `MIT` | 10 tệp | ◐ **Ch 88** | Đặc tả đồ thị kép InfoGraph (ngữ cảnh 1-hop) + PlanDag (thực thi phi chu trình song song theo sóng Kahn, AoT) |
| **#9** | `datawhalechina/hello-agents` | `CC BY-NC-SA 4.0` | 2.6k tệp | ◐ **Ch 86, 88, 89** | **Tường lửa nghiêm ngặt**: CẤM dịch văn bản/chép hình/chuyển mã; chỉ tiếp thu công thức toán RRF và chuẩn giao thức mở MCP/A2A/ANP |
| **#10** | `langchain-ai/agents-from-scratch` | `MIT` | 85 tệp | ◐ **Ch 88** | Máy trạng thái nút `StateGraph<S>` có điểm ngắt duyệt HITL (Accept/Edit/Respond/Ignore) chuyển hóa thành trí nhớ thủ tục |
| **#11** | `pguso/agents-from-scratch` | `MIT` | 44 tệp | ◐ **Ch 88** | Cơ chế Atom of Thought (AoT) kết hợp biểu diễn đồ thị nhận thức và đánh giá cục bộ |
| **#12** | `decodingai-magazine/building-a-coding-agent-from-scratch-course` | `Apache-2.0` | 700 tệp, 2.6k tests | ◐ **Ch 86, 87, 90, 91** | Vá tệp nguyên tử bằng băm nội dung CAS, trần ngữ cảnh đệ quy, khai thác vết lỗi để đánh giá |
| **#13** | `rasbt/LLMs-from-scratch` | Modified `Apache-2.0` | 365 tệp, 96 tests | ◐ **Ch 92–95** | **Tường lửa sách Manning**: loại trừ sách in và hình ảnh; 100% văn xuôi, sơ đồ và code viết mới bằng Rust 2024 thuần `std` |
| **#14** | `FareedKhan-dev/train-llm-from-scratch` | `MIT` | 288 tệp, 118 tests | ◐ **Ch 93–97** | Cài đặt lại toàn diện kiến trúc Transformer hiện đại, bộ tối ưu Muon/AdamW, căn chỉnh DPO và học tăng cường GRPO |

**Tổng kết trung thực.** Toàn bộ 15 nguồn quốc tế đã được chắt lọc nghiêm ngặt theo nguyên tắc phòng sạch: không sao chép nguyên văn, không phụ thuộc framework cồng kềnh, chuyển hóa toàn bộ các cơ chế tinh hoa thành 12 chương và 12 crate Rust 2024 thuần `std` có kiểm thử độc lập.

---

## 8. Bản đồ phủ 33 Kho Red/Blue Team & Tác tử Pentest Quốc Tế (Chủ đề 28)

Chủ đề 28 (Ch 98–105) không bịa ra nguyên lý: mọi kiến trúc trong đó đều được rút ra từ nghiên cứu sâu 33 kho mã nguồn Red/Blue team hàng đầu. Toàn bộ là **clean-room re-implementation bằng Rust 2024 thuần `std`** — không sao chép một dòng mã nào.

### Bảng 8.1 — Nguyên lý kiến trúc được tiếp thu

| Nguyên lý | Kho nguồn tiêu biểu | Thể hiện trong giáo trình |
|---|---|---|
| **Kiểm chứng sự thật nền** — không tin lời tự báo cáo của tác tử, mà kiểm `whoami` thật | `hackingBuddyGPT` (TU Wien) | Ch 102 `GroundTruthVerifier`, Ch 105 bất biến *Ground-truth Verification* |
| **Chứng minh bằng khai thác** — chỉ báo cáo lỗ hổng kèm PoC chạy được, tỷ lệ báo động giả = 0 | `Strix` | Ch 104 `ProofByExploitation`, Ch 105 `ValidationAgent` |
| **Phân tách đa tác tử** — trinh sát / khai thác / xác thực tách riêng | `PentAGI`, `Strix` | Ch 105 `ReconAgent` · `ExploitAgent` · `ValidationAgent` · `OrchestratorAgent` |
| **Bền vững phiên pentest** — dừng rồi nối lại không mất ngữ cảnh | `PentestGPT` | Ch 105 `SessionManager` |
| **Hàm phần thưởng RL** — thưởng khi chiếm host, phạt khi bị phát hiện hoặc làm sập dịch vụ | `CyberBattleSim` (Microsoft) | Ch 105 `AttackRewardFunction`, `AttackPathOptimizer` (tabular Q-learning) |
| **Vòng lặp tác tử tối giản** — đơn giản nhưng hiệu quả | `hackingBuddyGPT` | Ch 105 `AutonomousPentestAgent::run` |
| **Kiểm thử bảo mật LLM** — prompt injection, jailbreak, rò rỉ dữ liệu huấn luyện | `PyRIT` (Microsoft), `garak`, `promptfoo` | Ch 100 `LlmSecurityProber` |
| **Đối kháng Red-vs-Blue** — đội đỏ tấn công, đội xanh phòng thủ, cả hai cùng tiến bộ | `CyberBattleSim`, `autonomous-red-team` | Ch 105 `BlueTeamDefender`, `RedVsBlueSim` |
| **Phát hiện 0day không cần mô hình** — phân tích tĩnh + heuristic | `vulnhuntr` | Ch 104 `StaticVulnScanner` (14 luật), `VulnDatabase` (64 mục CWE) |

> **Kỷ luật phòng sạch**: Metasploit (BSD), Impacket (Apache-2.0 nhưng Python) và mọi framework khai thác hiện có đều **không** được sao chép. Tài liệu tham khảo chỉ gồm các **đặc tả công khai**: RFC 1321 (MD5), RFC 1320 (MD4), RFC 4120 (Kerberos 5), RFC 3962 (AES-CTS Kerberos), RFC 8439 (ChaCha20), RFC 7748 (X25519), RFC 1928/1929 (SOCKS5), RFC 826 (ARP), RFC 1035 (DNS), RFC 791 (IPv4), MS-NLMP (NTLM), FIPS-197 (AES), FIPS-180-4 (SHA-256).

---

## 9. Cái gì cố tình KHÔNG có trong giáo trình

Ghi rõ để bạn không đi tìm:

- **Mã dùng framework thật trong workspace.** Các chương 61, 62, 63, 66, 68, 72, 79 chỉ giữ **lõi thuần tuý** trong crate, để `cargo test --workspace` chạy được offline, không cần SDK, không cần mạng, không cần bo mạch. Mã dùng Axum, Leptos, Tauri, `cosmwasm-std`, `solana-program`, RHDL nằm trong phần lý thuyết của chương.
- **Lời khuyên đầu tư.** Sách này dạy cách xây hệ thống, không dạy nên mua gì.
- **Số liệu lãi lỗ đáng tin.** Mọi phiên giao dịch trong sách đều là **tổng hợp**. Chương 85 giải thích rõ vì sao con số lãi lỗ của nó là một sản phẩm phụ của mô hình, và vì sao thứ đáng tin là các **bất biến**.
- **Định danh tiếng Việt trong các crate của workspace.** Từ bản 85 chương, mọi tên struct, enum, trường, hàm, biến **và cả tên hàm kiểm thử** trong `code/` đều bằng **tiếng Anh**; (các đoạn mã minh hoạ chỉ nằm trong markdown — không thuộc crate nào — vẫn còn khoảng 420 định danh tiếng Việt, đang chờ chuyển) phần giảng nghĩa nằm ở **comment tiếng Việt** ngay trên định danh. Học viên đọc `OrderBook` sẽ nhận ra ngay khi mở tài liệu một crate thật, còn `SoLenh` thì không. Bảng đối chiếu tên cũ ↔ tên mới nằm ở mục 22 của [`GLOSSARY.md`](./GLOSSARY.md).
- **Trọng số tỷ tham số, FFI CUDA/BLAS hoặc sinh văn bản tự nhiên mở trong crate LLM.** Các crate Chủ đề 27 (`ch92`–`ch97`) cài đặt chính xác 100% toán học BPE, Autograd, RoPE/GQA/MLA/MoE, Muon, PagedAttention/Speculative/Int8, LoRA/DPO/PPO/GRPO trên kích thước vi mô tất định (`d_model = 16..32`) để chạy kiểm thử offline dưới 150ms không thư viện ngoài; phần liên kết thư viện lớn (`candle`, `burn`, `safetensors`) và triển khai cụm GPU phân tán nằm trong phần lý thuyết mở rộng.
- **Mã khai thác chạy trên hệ thống thật.** Toàn bộ Chủ đề 28 (Ch 98–105) là **giáo dục thuần túy**: mọi exploit chạy trên dữ liệu tổng hợp tất định (`MockFilesystem`, `MockNetwork`, `MockShell`) hoặc trên binary do chính giáo trình tạo ra bằng `unsafe`. Không mã nào kết nối tới hệ thống thật ngoài loopback; các socket đều bind `127.0.0.1:0`.
- **Công cụ khai thác dùng sẵn.** Không Metasploit, không Impacket, không `hashcat`, không `sqlmap`. Tất cả được cài lại từ đầu bằng Rust thuần `std` theo đặc tả RFC/FIPS công khai.
- **Nội dung bản quyền của các nguồn tham khảo.** Chúng ta dùng phân loại và tên bài để định hướng, rồi tự cài lại từ đầu bằng Rust.

---

## 10. Cách tự kiểm chứng

```bash
cd code
cargo build --workspace          # phải 0 cảnh báo, 0 lỗi
cargo test  --workspace          # phải toàn xanh
cargo run -p ch85                # hệ sinh thái HFT tích hợp, hai sàn
cargo run -p ch91                # đại dự án tác tử lập trình tự hành an toàn
cargo run -p ch97                # đại dự án đường ống micro-LLM suy luận end-to-end
cargo run -p ch104               # bộ công cụ Red Team, chứng minh bằng khai thác
cargo run -p ch105               # tác tử pentest tự hành đa tác tử, RL, red-vs-blue
```

Nếu một con số trong sách không khớp với thứ máy bạn in ra, hãy tin cái máy bạn in ra và mở một issue.
