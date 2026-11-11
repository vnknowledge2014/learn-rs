# Chương 90: Điều phối Đa Tác tử, Cổng Kỷ luật Bàn thợ, Hiến pháp, Bánh cóc Tự tiến hóa & Diễn tập LoopDrill (Multi-Agent Orchestration, Workbench Gates, RSI Ratchet & LoopDrill Evals)

## 1. Giới thiệu & Mục tiêu

Trong bốn chương trước của Chủ đề 26 (Chương 86 đến Chương 89), chúng ta đã trang bị cho một tác tử lập trình đơn lẻ những năng lực thiết yếu nhất: cấu trúc bối cảnh ba tầng `SKILL.md`, gương ngữ cảnh trực tiếp `LiveContextMirror` với sáu bất biến bảo toàn, suy luận có cấu trúc hai đồ thị `InfoGraph` + `PlanDag`, truy xuất lai BM25 + Dense + RRF, khả năng tương tác người dùng trong vòng lặp HITL, và ngăn xếp độ bền phục hồi sự cố với cổng mô hình, cây phiên rẽ nhánh `SessionTree`, nhật ký `TurnWal` cùng bộ ba giao thức MCP + A2A + ANP.

Tuy nhiên, khi một bài toán phần mềm quy mô lớn đòi hỏi sự phối hợp của một **tập thể đa tác tử (Multi-Agent Swarm)** — nơi có tác tử lập trình viên (`worker`), tác tử rà soát chất lượng (`reviewer`), tác tử kiểm toán an ninh (`auditor`), tác tử dọn dẹp phụ thuộc (`dep_sweeper`), và tác tử điều phối trung tâm (`orchestrator`) — hệ thống lập tức đối mặt với những nguy cơ khủng hoảng ở cấp độ kiến trúc:

| Nguy cơ cấp tập thể | Hậu quả thực tế nếu buông lỏng kiểm soát |
|---|---|
| **Hội chứng bầy đàn vô tổ chức (Swarm Chaos)** | Các tác tử nói chuyện chéo tự do, làm bùng nổ số lượng token bối cảnh theo cấp số nhân $O(N^2)$, gây cạn kiệt ngân sách FinOps và phân mảnh thông tin. |
| **Bàn thợ lộn xộn & Mừng hụt (Premature Celebration)** | Tác tử mở cùng lúc nhiều nhánh tính năng, sửa lan sang các tệp ngoài phạm vi được phân công, và tuyên bố "đã xong việc" khi chưa có bất kỳ bằng chứng kiểm thử thực tế nào niêm phong bằng băm mật mã. |
| **Thông đồng tự phê duyệt (Self-Approval Collusion)** | Tác tử vừa tự viết mã, vừa tự phê duyệt bài làm của mình (`author == verifier`), vô hiệu hóa hoàn toàn cơ chế kiểm soát chất lượng độc lập. |
| **Thoái hóa do quá khớp tự tiến hóa (RSI Overfitting)** | Trong vòng lặp tự hoàn thiện đệ quy (Recursive Self-Improvement), tác tử tối ưu hóa mã nguồn để vượt qua bài test huấn luyện (`held_in`), nhưng làm hỏng toàn bộ các ca kiểm thử ẩn chưa thấy (`held_out`). |
| **Bẫy chân lý rỗng trong diễn tập (Vacuous Truth LoopDrill)** | Hàng rào phòng thủ đạt điểm tuyệt đối 100% về độ nhạy (Sensitivity) hoặc độ đặc hiệu (Specificity) chỉ vì bộ dữ liệu diễn tập rỗng hoặc thiếu hẳn một phía mẫu kiểm tra (ví dụ không có mẫu độc hại nào). |
| **Định luật Goodhart (Goodhart's Law Divergence)** | Tác tử "hack" chỉ số ủy nhiệm (như số dòng lệnh viết thêm, số lượng commit) để đạt điểm đánh giá cao trong khi chất lượng phần mềm thực tế tụt dốc thảm hại. |

Để giải quyết triệt để những vấn đề sống còn này, Chương 90 giới thiệu một hệ thống kiểm soát kỷ luật toàn diện được xây dựng 100% bằng **Rust 2024 thuần `std`**, không dựa vào bất kỳ thư viện ngoài nào.

### Mục tiêu học tập cốt lõi

1. **Bộ Cổng Kỷ luật Bàn thợ (`WorkbenchGateSuite`)**: Nắm vững cơ chế vận hành của Sổ cái đơn tính năng (`FeatureLedger`) cưỡng chế chỉ một tính năng hoạt động tại một thời điểm, Cổng chống mừng hụt (`verify_completion_claim`) đòi hỏi biên nhận băm SHA-256 từ kết quả kiểm thử thực tế, và Bộ linter kiểm tra phân tầng kiến trúc (`LayerImportLinter`).
2. **Bộ điều khiển vòng lặp mục tiêu hai tầng Giám khảo (`GoalLoopController`)**: Tích hợp lịch 5 trường chuẩn Cron (`CronSchedule`) và cơ chế phân tách giám khảo: Tầng 1 kiểm tra phản xạ nhanh (nhịp tim, định dạng, cú pháp) giúp tiết kiệm tài nguyên, Tầng 2 kiểm tra chức năng sâu (chạy bộ test toàn diện).
3. **Điều phối Trung gian Hình sao (`MediatorOrchestrator`) & Dấu vết phân tán (`TraceSpan`)**: Thiết lập cơ chế chuyển tiếp tin nhắn nguyên văn bảo toàn 100% byte (`forward_message`), bus sự kiện theo chủ đề (`TopicEventBus`) sử dụng con trỏ đơn điệu `CursorOffset`, và truy vết có gắn băm SHA-256 xác định.
4. **Quạt luồng Khảo sát cô lập (`ExploreFanoutRunner`)**: Thực thi song song hoặc tuần tự các nhánh khảo sát với khả năng tự thử lại cô lập đúng 1 lần cho từng nhánh con bị lỗi mà không làm ảnh hưởng đến các nhánh anh em; chuẩn hóa yêu cầu bằng Đặc tả bán hình thức 11 khối (`PseudoFormalTaskBrief`) và Danh bạ đánh giá mù (`BlindPortfolioRegistry`).
5. **Hộp cát cây làm việc (`WorktreeSandbox`)**: Quản lý vòng đời tài nguyên tự động dọn dẹp bằng RAII qua `Drop`, giải thuật dò và gộp xung đột 3 chiều (3-way merge conflict detection), và cơ chế khóa thuê đường dẫn glob theo cấp bậc ưu tiên nghiêm ngặt: `CiSweeper > PrBabysitter > DepSweeper`.
6. **Ngắt mạch Ngữ cảnh Vòng lặp (`LoopContextBreaker`)**: Đo độ tương đồng mờ (Fuzzy Jaccard Similarity) của chuỗi hành động trượt để ngắt mạch kịp thời khi phát hiện tác tử rơi vào vòng lặp thao tác bế tắc.
7. **Động cơ Hiến pháp (`ConstitutionEngine`)**: Cưỡng chế nguyên tắc phân tách nhiệm vụ ba ngôi (`author != verifier != attestor`), cấm tuyệt đối hành vi tự phê duyệt, và ghim cố định mã băm bộ luật tự thân `$SELF`.
8. **Bánh cóc Tự tiến hóa Hai tập tách (`TwoSplitRsiRatchet`)**: Áp dụng cổng không hồi quy (Non-regression Gate) với hai tập kiểm thử độc lập (`held_in` và `held_out`), tự động bác bỏ mọi ứng viên quá khớp kèm bộ kiểm tra trôi lệch chính sách (`PolicyDriftChecker`).
9. **Thao trường Diễn tập `LoopDrillRunner` & Sửa lỗi Edge Case #6**: Khắc phục triệt để bẫy chân lý rỗng bằng việc bắt buộc kiểm tra `tp + fn_c > 0 && tn + fp > 0`, đòi hỏi đạt đồng thời hai ngưỡng `sensitivity` và `specificity`, niêm phong biên nhận băm SHA-256, và tích hợp Kiểm toán viên cấp độ sẵn sàng (`ReadinessAuditor`) tự động hạ trần từ `L3 -> L2` khi phát hiện sai lệch.
10. **Bộ kiểm định Đa tầng (`MultiLayerEvaluator`)**: Đo lường quỹ đạo với công thức toán học $pass@k$ đối chiếu $pass^k$, chuẩn hóa so khớp AST tham số gọi hàm kiểu BFCL, chuẩn hóa câu trả lời GAIA, triệt tiêu thiên kiến vị trí bằng `PositionSwappedJudge`, và phát hiện phân kỳ Goodhart kịp thời.

---

## 2. Hình tượng hóa đời sống: Tòa thị chính hiến pháp và thao trường tập trận

Để hình dung trực quan cách các cơ chế của Chương 90 phối hợp nhịp nhàng, hãy liên tưởng kiến trúc này với **Tòa thị chính thành phố kỹ thuật số**, nơi vận hành một chính quyền tam quyền phân lập nghiêm ngặt, sở hữu một bàn thợ quy chuẩn và một thao trường huấn luyện lực lượng phản ứng nhanh:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│             HỆ THỐNG ĐIỀU PHỐI ĐA TÁC TỬ & TÒA THỊ CHÍNH HIẾN PHÁP                     │
│                                                                                        │
│   [ BỘ LẬP LỊCH CRON ] ──► [ GIÁM KHẢO TẦNG 1 ] ──(Đạt)──► [ GIÁM KHẢO TẦNG 2 ]        │
│    (Định kỳ 5 phút)        (Kiểm tra nhịp tim, cú pháp)     (Chạy toàn bộ kiểm thử)    │
│                                                                                        │
│                                     │                                                  │
│                                     ▼                                                  │
│                    ┌─────────────────────────────────┐                                 │
│                    │    ĐIỀU PHỐI TRUNG GIAN HÌNH SAO │                                 │
│                    │     (MediatorOrchestrator)      │                                 │
│                    │  - Chuyển tiếp nguyên văn byte   │                                 │
│                    │  - Hàng đợi TopicEventBus        │                                 │
│                    │  - Dấu vết SHA-256 TraceSpan     │                                 │
│                    └────────────────┬────────────────┘                                 │
│                                     │                                                  │
│         ┌───────────────────────────┼───────────────────────────┐                      │
│         ▼                           ▼                           ▼                      │
│   ┌───────────┐               ┌───────────┐               ┌───────────┐                │
│   │  Tác tử   │               │  Tác tử   │               │  Tác tử   │                │
│   │  Worker   │               │ Reviewer  │               │  Auditor  │                │
│   └─────┬─────┘               └─────┬─────┘               └─────┬─────┘                │
│         │                           │                           │                      │
│         ▼                           ▼                           ▼                      │
│   ┌───────────────────────────────────────────────────────────────────┐                │
│   │                 ĐỘNG CƠ HIẾN PHÁP (CONSTITUTION ENGINE)           │                │
│   │   Tam quyền phân lập:  Tác giả (author) != Người duyệt (verifier) │                │
│   │                         và Người duyệt != Người chứng thực        │                │
│   │   * CẤM TỰ PHÊ DUYỆT (author == verifier)                         │                │
│   │   * Ghim băm SHA-256 quy tắc tự thân ($SELF)                      │                │
│   └───────────────────────────────────────────────────────────────────┘                │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Bàn thợ kỷ luật: Không có chỗ cho sự tùy tiện

Tại bàn thợ của mỗi người thợ mộc tài ba, có ba nguyên tắc bất di bất dịch:
1. **Một việc tại một thời điểm**: Bạn không thể vừa cưa chiếc chân bàn này, vừa chạy sang tiện chiếc tựa lưng ghế khác. Cấu trúc `FeatureLedger` đóng vai trò như chiếc kẹp bàn thợ duy nhất: khi đã khóa chặt một tính năng ở trạng thái `Active`, không một tính năng nào khác được phép mở ra cho đến khi chiếc kẹp được tháo.
2. **Nói có sách, mách có chứng**: Khi người thợ nói "Tôi đã đóng xong chiếc bàn", thanh tra xưởng mộc không chỉ nhìn bằng mắt. Họ yêu cầu biên nhận thử tải trọng: chiếc bàn đã chịu được 100 kg trong bao lâu? Trong hệ thống của chúng ta, `verify_completion_claim` bác bỏ bất kỳ báo cáo bàn giao nào thiếu mã băm chứng thực của các bài kiểm thử thực tế đã chạy thành công 100%.
3. **Phân tầng nghiêm ngặt**: Gỗ thô ở tầng đáy không được phép trèo lên nằm đè lên lớp sơn bóng thành phẩm ở tầng trên. `LayerImportLinter` chặn đứng ngay lập tức các dòng lệnh nhập ngược từ tầng miền nghiệp vụ sang tầng hạ tầng.

```
┌────────────────────────────────────────────────────────────────────────┐
│                   CỔNG KỶ LUẬT BÀN THỢ (WORKBENCH GATES)               │
│                                                                        │
│   Yêu cầu tính năng ──► [ FeatureLedger ]                              │
│                            │ (Chỉ 1 tính năng Active)                  │
│                            ▼                                           │
│                         [ Sửa mã trong phạm vi cho phép ]              │
│                            │                                           │
│                            ▼                                           │
│                         [ LayerImportLinter ]                          │
│                            │ (Domain không được import Infra)          │
│                            ▼                                           │
│                         [ Chạy kiểm thử tự động ]                      │
│                            │                                           │
│                            ▼                                           │
│                         [ verify_completion_claim ] ──► [ Bàn giao ]   │
│                           - Đạt 100% bài test                          │
│                           - 0 bài trượt                                │
│                           - Có mã băm bằng chứng                       │
│                           - Đủ 5 phần HandoffArtifact                  │
└────────────────────────────────────────────────────────────────────────┘
```

### Bánh cóc tự tiến hóa: Răng cưa một chiều

Một chiếc bánh cóc (ratchet) trong cơ khí chỉ cho phép trục quay theo một chiều tiến, các răng cưa và chốt chặn sẽ ngăn không cho trục quay ngược trở lại. 

Trong kỹ nghệ tác tử, khi cho phép hệ thống tự sửa mã và tự tiến hóa (Recursive Self-Improvement - RSI), mối nguy lớn nhất là hiện tượng **"học tủ" (overfitting)**: tác tử sửa mã để giải quyết cực tốt các ca kiểm thử trong phòng lab (`held_in`), nhưng lại làm hỏng các tính năng cốt lõi ngoài đời thực (`held_out`). `TwoSplitRsiRatchet` của chúng ta hoạt động như một bánh cóc răng cứng: bất kể điểm số trên tập huấn luyện có tăng vọt lên bao nhiêu, chỉ cần điểm số trên tập ẩn kiểm tra tụt dù chỉ 0.001%, bánh cóc sẽ lập tức khóa cứng, từ chối nâng cấp và ghi nhận một trường hợp quá khớp thất bại.

```
                         BÁNH CÓC TỰ TIẾN HÓA (RSI RATCHET)
                               
                               Ứng viên cải tiến
                                      │
                 ┌────────────────────┴────────────────────┐
                 ▼                                         ▼
         Tập công khai (held_in)                   Tập ẩn (held_out)
         Điểm: 0.80 ──► 0.95 (Tăng vọt!)           Điểm: 0.85 ──► 0.81 (Tụt!)
                 │                                         │
                 └────────────────────┬────────────────────┘
                                      │
                                      ▼
                        [ BÁNH CÓC HAI TẬP TÁCH ]
                        Quy tắc: held_out KHÔNG ĐƯỢC TỤT!
                                      │
                         ┌────────────┴────────────┐
                         ▼                         ▼
                  [ BÁC BỎ NGAY ]          [ NÂNG BẬC CƠ SỞ ]
                 (rejected_overfit)         (accepted_generation)
```

### Thao trường LoopDrill: Vạch trần bẫy chân lý rỗng

Hãy tưởng tượng một người lính gác cổng doanh trại quân đội tuyên bố: *"Trong suốt ca trực 8 tiếng của tôi, tôi đạt độ chính xác 100%, không để lọt bất kỳ kẻ địch nào!"*. Nhưng khi kiểm tra sổ nhật ký, người chỉ huy phát hiện ra: trong suốt 8 tiếng đó, toàn bộ con đường trước cổng là đường cấm dân sự, **không hề có một bóng người nào đi qua**! Lời tuyên bố của người lính gác tuy đúng về mặt logic hình thức (vì không có ai đi qua nên số vụ lọt bằng 0), nhưng hoàn toàn vô giá trị trong thực tế — đó chính là **Bẫy chân lý rỗng (Vacuous Truth)**.

Trong các hệ thống AI, nếu bộ dữ liệu diễn tập rào chắn chỉ gồm toàn các truy vấn lành tính mà không có mã độc nào (`tp + fn_c == 0`), một rào chắn "cho qua tất cả" cũng sẽ đạt độ nhạy danh nghĩa 100%. Ngược lại, nếu bộ dữ liệu chỉ toàn mã độc mà không có truy vấn bình thường (`tn + fp == 0`), một rào chắn "chặn sạch mọi thứ" cũng sẽ đạt độ đặc hiệu danh nghĩa 100%. `LoopDrillRunner` trong chương này giải quyết triệt để lỗi Edge Case #6 bằng cách cưỡng chế kiểm tra mẫu hai chiều, đảm bảo cả mẫu dương lẫn mẫu âm đều hiện diện trước khi tính toán điểm số.

---

## 3. Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 3.1 Kỷ luật Bàn thợ (Workbench Gates) & Sổ cái đơn tính năng

Trong các dự án phát triển phần mềm bằng AI, nguyên nhân hàng đầu dẫn đến việc mã nguồn bị thoái hóa hoặc sinh lỗi tiềm ẩn là do tác tử thiếu một ranh giới làm việc rõ ràng. Khái niệm Bàn thợ (Workbench) định nghĩa một không gian thao tác được bao bọc bởi các cổng kiểm soát tự động:

#### Sổ cái đơn tính năng (`FeatureLedger`)
Nguyên tắc cốt lõi: Một tác tử tại một thời điểm chỉ được phép giải quyết đúng một bài toán nghiệp vụ nhỏ, khép kín.
```rust
pub struct FeatureLedger {
    pub records: Vec<FeatureRecord>,
}
```
Khi tác tử bắt đầu làm việc, nó phải gọi `start_feature`. Nếu sổ cái kiểm tra thấy đang có một bản ghi ở trạng thái `FeatureStatus::Active`, thao tác sẽ bị từ chối ngay lập tức với lỗi rõ ràng:
$$\text{has\_active\_feature}() = \exists r \in \text{records} : r.\text{status} = \text{Active}$$
Bất biến này ngăn chặn hiện tượng tác tử nhảy cóc giữa các nhiệm vụ chưa hoàn thành hoặc tự ý mở rộng phạm vi (scope creep).

#### Cổng chống mừng hụt (`verify_completion_claim`)
Khi một tác tử tuyên bố đã hoàn thành nhiệm vụ, hệ thống không tin vào lời khẳng định bằng văn bản thông thường. Tác tử bắt buộc phải nộp một hiện vật bàn giao `HandoffArtifact` tuân thủ nghiêm ngặt 5 thành phần:
1. **Quan sát (Observation)**: Trích dẫn chính xác dòng lệnh, mã nguồn, tệp tin và thông số quan sát được.
2. **Chuỗi lập luận (Logic Chain)**: Lập luận suy diễn từng bước dẫn đến giải pháp.
3. **Giới hạn (Caveats)**: Các khía cạnh chưa khảo sát hoặc các giả định kỹ thuật.
4. **Kết luận (Conclusion)**: Đánh giá cuối cùng có thể kiểm chứng được.
5. **Phương pháp kiểm chứng (Verification Method)**: Lệnh cụ thể và mã băm bằng chứng kiểm thử (`test_evidence_hash`).

Cổng kiểm tra sẽ tính toán mã băm niêm phong tổng hợp từ toàn bộ các thành phần này cùng số lượng bài test đạt yêu cầu:
$$\text{SealHash} = \text{SHA-256}(\text{Observation} \mathbin{\Vert} \text{Conclusion} \mathbin{\Vert} \text{EvidenceHash} \mathbin{\Vert} \text{TestsPassed})$$
Nếu số test thất bại lớn hơn 0 hoặc số test thành công nhỏ hơn chỉ tiêu yêu cầu, báo cáo bàn giao sẽ bị bác bỏ ngay tại cửa ải.

#### Bộ kiểm tra phân tầng kiến trúc (`LayerImportLinter`)
Để bảo vệ kiến trúc phần mềm không bị phá vỡ theo thời gian, `LayerImportLinter` gán cho mỗi tầng kiến trúc một cấp bậc số nguyên (rank). Ví dụ trong mô hình 3 tầng kinh điển:
$$\text{Rank}(\text{domain}) = 0 < \text{Rank}(\text{application}) = 1 < \text{Rank}(\text{infrastructure}) = 2$$
Quy tắc bất biến: Tầng nguồn $S$ chỉ được phép nhập khẩu từ tầng đích $T$ nếu và chỉ nếu:
$$\text{Rank}(T) \le \text{Rank}(S)$$
Nếu một tệp trong tầng `domain` chứa lệnh import trỏ tới `infrastructure`, linter sẽ báo lỗi vi phạm kiến trúc phân tầng ngay lập tức.

---

### 3.2 Điều phối Trung gian Hình sao, Hàng đợi Sự kiện & Dấu vết Phân tán

Trong một hệ thống nhiều tác tử, nếu cho phép các tác tử gửi tin nhắn trực tiếp dạng lưới (mesh topology), số lượng kênh giao tiếp sẽ bùng nổ theo công thức $N(N - 1) / 2$. Điều này dẫn đến sự mất kiểm soát về thứ tự sự kiện, khó kiểm toán và bùng nổ chi phí token.

Chương 90 áp dụng kiến trúc **Điều phối Trung gian Hình sao (`MediatorOrchestrator`)**:
- Mọi tác tử (`worker`, `reviewer`, `auditor`) chỉ kết nối duy nhất tới `MediatorOrchestrator`.
- Mọi thông điệp `AgentMessage` phải được chuyển tiếp qua phương thức `forward_message`.
- **Bảo toàn 100% byte**: Thông điệp không bị Mediator chỉnh sửa, cắt xén hay tóm tắt làm sai lệch ngữ nghĩa gốc.

```rust
pub struct AgentMessage {
    pub sender: String,
    pub recipient: String,
    pub correlation_id: String,
    pub payload: String,
    pub timestamp: u64,
}
```

#### Hàng đợi sự kiện chủ đề (`TopicEventBus`) & `CursorOffset`
Mediator lưu trữ các sự kiện trên một bus sự kiện tuyến tính. Mỗi sự kiện được đánh một số hiệu con trỏ đơn điệu tăng dần không bao giờ giảm:
$$\text{CursorOffset}_{k+1} = \text{CursorOffset}_k + 1$$
Mỗi tác tử khi tiêu thụ tin nhắn từ hộp thư của mình (`agent.inbox.<recipient>`) sẽ ghi nhớ con trỏ vị trí đã đọc (`cursor_offset`). Điều này mang lại ba lợi ích to lớn:
1. **Lũy đẳng và Chống mất mát**: Tác tử có thể phát lại (replay) toàn bộ chuỗi thông điệp từ bất kỳ vị trí con trỏ nào sau khi bị khởi động lại.
2. **Phân trang tự nhiên**: Việc lấy tin nhắn theo lô (`max_events`) diễn ra mượt mà mà không sợ trùng lặp.
3. **Phân tách hoàn toàn giữa luồng phát hành và luồng tiêu thụ**.

#### Dấu vết phân tán (`TraceSpan`) với băm SHA-256
Để theo dõi một luồng xử lý phức tạp xuyên qua nhiều tác tử, cấu trúc `TraceSpan` ghi nhận:
- `trace_id`: Định danh duy nhất cho toàn bộ giao dịch phân tán.
- `span_id`: Định danh cho bước xử lý hiện tại của tác tử.
- `parent_span_id`: Liên kết đến bước xử lý của tác tử tiền nhiệm.
- `start_micros` và `end_micros`: Dấu thời gian đo lường độ trễ cấp microsecond.
- `attributes`: Bản đồ các cặp khóa-giá trị bổ sung.

Mã băm SHA-256 của span được tính toán một cách tất định:
$$\text{SpanHash} = \text{SHA-256}(\text{trace\_id} \mathbin{\Vert} \text{span\_id} \mathbin{\Vert} \text{parent} \mathbin{\Vert} \text{name} \mathbin{\Vert} \text{timestamps} \mathbin{\Vert} \text{attributes})$$
Mã băm này đóng vai trò như một biên nhận mật mã không thể chối cãi, gắn liền với mọi sự kiện được phát hành lên `TopicEventBus`.

---

### 3.3 Quạt luồng Khảo sát cô lập & Đặc tả Bán hình thức 11 khối

Khi cần khảo sát nhiều nhánh kiến trúc hoặc nghiên cứu các thư viện khác nhau, việc thực thi tuần tự trên một tác tử duy nhất gây tắc nghẽn nghiêm trọng. Mô hình `ExploreFanoutRunner` cho phép quạt luồng (fanout) tác vụ ra nhiều nhánh con độc lập.

#### Nguyên tắc Thử lại cô lập (Isolated Retry)
Trong môi trường mạng phân tán hoặc các tác vụ AI sinh mã, một nhánh con có thể thất bại do sự cố mạng tạm thời hoặc lỗi biên dịch cục bộ. `ExploreFanoutRunner` cài đặt một cơ chế phòng thủ thông minh:
- Mỗi tác vụ con được thực thi ở lần thử đầu tiên (`attempt = 0`).
- Nếu thành công, kết quả `SubBranchResult` được ghi nhận với `retry_count = 0`.
- Nếu thất bại, nhánh con đó được cấp quyền **tự thử lại cô lập đúng 1 lần** (`attempt = 1`).
- Nếu lần 2 thành công, `SubBranchResult` ghi nhận `retry_count = 1`.
- Nếu lần 2 vẫn thất bại, lỗi được cô lập hoàn toàn trong nhánh đó (`succeeded = false`), tuyệt đối không làm dừng hay làm hỏng dữ liệu của các nhánh anh em đang chạy.

#### Đặc tả nhiệm vụ bán hình thức 11 khối (`PseudoFormalTaskBrief`)
Để ngăn chặn hiện tượng tác tử hiểu sai yêu cầu nhiệm vụ, bản đặc tả phải thỏa mãn đầy đủ 11 khối cấu trúc:
1. `objective`: Mục tiêu kỹ thuật cốt lõi.
2. `preconditions`: Điều kiện tiền quyết của môi trường trước khi chạy.
3. `invariants`: Các bất biến logic không được phép bị phá vỡ.
4. `input_artifacts`: Danh sách các tệp/hiện vật đầu vào.
5. `output_artifacts`: Danh sách các tệp/hiện vật đầu ra kỳ vọng.
6. `constraints`: Các ràng buộc kỹ thuật (ngôn ngữ, phụ thuộc, chuẩn mã).
7. `failure_modes`: Các kịch bản lỗi có thể xảy ra và cách xử lý.
8. `verification_rules`: Quy tắc kiểm tra và lệnh kiểm thử bắt buộc.
9. `security_assumptions`: Giả định an ninh và ranh giới bảo mật.
10. `rollback_plan`: Kế hoạch hoàn tác sạch sẽ nếu gặp sự cố.
11. `attestation`: Cam kết chịu trách nhiệm kiểm toán.

#### Danh bạ đánh giá mù (`BlindPortfolioRegistry`)
Khi sử dụng mô hình ngôn ngữ lớn (LLM) để chấm điểm hoặc đánh giá giải pháp của các tác tử khác nhau, mô hình thường bị thiên kiến thương hiệu (ví dụ ưu ái tên gọi GPT-4 hay Claude). Danh bạ `BlindPortfolioRegistry` ẩn danh hóa toàn bộ các ứng viên thành các nhãn mù (`CANDIDATE_A`, `CANDIDATE_B`). Giám khảo đánh giá hoàn toàn dựa trên nội dung kỹ thuật. Chỉ sau khi kết quả đã được ký số niêm phong, danh bạ mới thực hiện giải mã danh tính thực sự qua phương thức `reveal_identity`.

---

### 3.4 Hộp cát Cây làm việc, RAII Drop & Khóa Ưu tiên Đường dẫn

Khi nhiều tác tử cùng thao tác trên một kho mã nguồn Git, nguy cơ ghi đè tệp hoặc gây xung đột mã nguồn là rất cao.

#### Quản lý tài nguyên tự động dọn dẹp bằng RAII `Drop`
Hộp cát `WorktreeSandbox` đại diện cho một không gian làm việc cô lập. Trong Rust, thông qua trait `Drop`, chúng ta đảm bảo rằng bất kể tác tử kết thúc bình thường, gặp lỗi trả về `Err`, hay thậm chí panic, hộp cát sẽ luôn tự động giải phóng toàn bộ các khóa đường dẫn đang nắm giữ:
```rust
impl Drop for WorktreeSandbox {
    fn drop(&mut self) {
        self.active_leases.clear();
        self.dropped_cleanly = true;
    }
}
```

#### Khóa thuê đường dẫn theo thứ tự ưu tiên (Priority Path Lease)
Không phải mọi tác tử đều có quyền hạn ngang nhau. Trong một đường ống tích hợp liên tục (CI/CD), sự cố gãy bản build cần được khắc phục khẩn cấp hơn việc nâng cấp thư viện phụ thuộc định kỳ. Chúng ta thiết lập thứ tự ưu tiên số học:
$$\text{CiSweeper (30)} > \text{PrBabysitter (20)} > \text{DepSweeper (10)}$$
Khi một tác tử có quyền ưu tiên cao hơn yêu cầu khóa thuê một mẫu đường dẫn glob (ví dụ `src/*.rs`) đang bị một tác tử ưu tiên thấp hơn nắm giữ, hệ thống sẽ thực hiện **tiếm quyền thuê (Preemption)** ngay lập tức. Ngược lại, nếu một tác tử ưu tiên thấp hơn cố gắng giành khóa của tác tử ưu tiên cao hơn, yêu cầu sẽ bị từ chối thẳng thừng.

#### Dò tìm và giải quyết xung đột 3 chiều (3-Way Merge Conflict Detection)
Dựa trên phiên bản gốc (`base`), phiên bản của chúng ta (`ours`), và phiên bản của đối tác (`theirs`), thuật toán so sánh từng dòng:
- Nếu $ours = theirs$: Lấy phiên bản chung.
- Nếu $ours = base \land theirs \ne base$: Lấy thay đổi của đối tác.
- Nếu $theirs = base \land ours \ne base$: Lấy thay đổi của chúng ta.
- Nếu $ours \ne base \land theirs \ne base \land ours \ne theirs$: Báo cáo xung đột tại số dòng cụ thể.

---

### 3.5 Động cơ Hiến pháp & Tam quyền phân lập

Một trong những phát minh quan trọng nhất của lý thuyết chính trị học hiện đại là **Học thuyết Tam quyền phân lập (Separation of Powers)** của Montesquieu. Trong kỹ nghệ tác tử tự hành, nguyên tắc này được chuyển hóa thành Động cơ Hiến pháp (`ConstitutionEngine`).

#### Ba vai trò độc lập bắt buộc
1. **Tác giả (`author`)**: Tác tử trực tiếp viết mã hoặc chỉnh sửa cấu hình (ví dụ: `worker_ch90`).
2. **Người kiểm chứng (`verifier`)**: Tác tử độc lập chạy kiểm thử, rà soát logic và đối chiếu yêu cầu (ví dụ: `reviewer_ch90`).
3. **Người chứng thực (`attestor`)**: Tác tử kiểm toán cấp cao ký duyệt biên nhận phát hành (ví dụ: `auditor_ch90`).

#### Bất biến Hiến pháp: Cấm tự phê duyệt
$$\text{author} \ne \text{verifier} \land \text{author} \ne \text{attestor} \land \text{verifier} \ne \text{attestor}$$
Nếu một tác tử vừa tự viết mã vừa tự đóng vai trò kiểm chứng (`author == verifier`), Hiến pháp sẽ kích hoạt cơ chế phòng vệ và từ chối cấp phép.

#### Ghim băm quy tắc tự thân (`pinned_self_hash`)
Bản thân các quy tắc của Hiến pháp được băm bằng SHA-256 ngay khi khởi tạo:
$$\text{PinnedSelfHash} = \text{SHA-256}(\text{ConstitutionRules})$$
Mỗi bản ghi chứng thực `AttestationRecord` phải mang theo mã băm quy tắc mà nó đã tuân thủ. Nếu mã băm này không khớp từng bit với `pinned_self_hash` (dấu hiệu tác tử đã lén sửa đổi luật lệ), toàn bộ quy trình phê chuẩn bị hủy bỏ.

---

### 3.6 Bánh cóc Tự tiến hóa Hai tập tách & Chống trôi chính sách

Trong các hệ thống AI tự tiến hóa (Recursive Self-Improvement), mô hình liên tục đề xuất các đoạn mã mới hoặc các prompt mới nhằm tối ưu hóa hiệu năng. Tuy nhiên, nếu không có cơ chế hãm, hệ thống rất dễ rơi vào "vòng xoáy hủy diệt": tối ưu hóa cục bộ trên tập bài tập quen thuộc nhưng đánh mất khả năng khái quát hóa.

#### Cơ chế Bánh cóc hai tập tách (`TwoSplitRsiRatchet`)
Hệ thống duy trì hai bộ dữ liệu đánh giá riêng biệt:
- `held_in`: Tập bài kiểm thử công khai, dùng để hướng dẫn và tối ưu hóa giải pháp.
- `held_out`: Tập bài kiểm thử ẩn, tuyệt đối không cho tác tử nhìn thấy trước trong quá trình sinh mã.

Khi một ứng viên được đề xuất với điểm số $\text{Score}_{\text{in}}$ và $\text{Score}_{\text{out}}$, bánh cóc áp dụng **Cổng không hồi quy (Non-regression Gate)**:
$$\text{Chấp thuận} \iff \text{Score}_{\text{out}} \ge \text{Baseline}_{\text{out}} \land (\text{Score}_{\text{in}} - \text{Cost}) \ge \text{Baseline}_{\text{in}}$$
Nếu một ứng viên nâng điểm `held_in` từ 80% lên 95%, nhưng làm điểm `held_out` tụt từ 85% xuống 82%, ứng viên đó bị bác bỏ ngay lập tức và tăng biến đếm `rejected_overfits`.

#### Bộ kiểm tra trôi lệch chính sách (`PolicyDriftChecker`)
Theo thời gian vận hành, các tác tử có thể vô tình hoặc cố ý thay đổi các tham số cấu hình trong tệp đặc tả. `PolicyDriftChecker` lưu giữ một bản đồ chuẩn `baseline_spec` và liên tục đối chiếu với cấu hình hiện tại để phát hiện 3 dạng trôi lệch:
1. Khóa cấu hình bắt buộc bị xóa bỏ.
2. Giá trị tham số bị thay đổi ngoài tầm kiểm soát.
3. Xuất hiện các khóa mới chưa qua quy trình phê chuẩn hiến pháp.

---

### 3.7 Thao trường Diễn tập LoopDrill & Phân tích lỗi Edge Case #6

Diễn tập rào chắn an ninh (`LoopDrill`) là quy trình định kỳ bắn thử các gói dữ liệu độc hại (mẫu dương) và các gói dữ liệu lành tính (mẫu âm) vào các lớp phòng thủ của tác tử để đo lường độ tin cậy.

#### Ma trận nhầm lẫn và Công thức toán học
Gọi:
- $\text{TP}$ (True Positive): Số mẫu độc hại bị rào chắn phát hiện và chặn đúng.
- $\text{FN}$ (False Negative): Số mẫu độc hại bị rào chắn bỏ lọt (đe dọa an ninh).
- $\text{TN}$ (True Negative): Số mẫu lành tính được rào chắn cho qua đúng.
- $\text{FP}$ (False Positive): Số mẫu lành tính bị rào chắn chặn nhầm (gây gián đoạn công việc).

Độ nhạy (Sensitivity / Recall) và Độ đặc hiệu (Specificity) được tính bằng:
$$\text{Sensitivity} = \frac{\text{TP}}{\text{TP} + \text{FN}}$$
$$\text{Specificity} = \frac{\text{TN}}{\text{TN} + \text{FP}}$$

#### Phân tích lỗi Edge Case #6: Bẫy Chân lý Rỗng (Vacuous Truth Trap)
Trong toán học logic, mệnh đề $\forall x \in \emptyset, P(x)$ luôn luôn đúng với mọi vị từ $P$. Trong đoạn mã PoC ban đầu, khi tổng số mẫu dương bằng 0 ($\text{TP} + \text{FN} = 0$), hàm đã ngộ nhận gán:
```rust
let sens = if tp + fn_c == 0 { 1.0 } else { ... }; // NGUY HIỂM!
```
Hệ quả là: Nếu người dùng chạy một bộ kiểm thử rỗng, hoặc một bộ kiểm thử chỉ toàn các câu lệnh lành tính, rào chắn sẽ báo cáo đạt `Sensitivity = 1.0`! Tương tự, nếu bộ kiểm thử chỉ toàn mẫu độc hại, rào chắn sẽ báo cáo đạt `Specificity = 1.0`!

**Giải pháp triệt để trong Chương 90**:
Bắt buộc phải kiểm tra điều kiện tồn tại của cả hai phía dữ liệu trước khi tính toán:
$$\text{total\_pos} = \text{TP} + \text{FN} > 0 \quad \land \quad \text{total\_neg} = \text{TN} + \text{FP} > 0$$
Nếu điều kiện này bị vi phạm, hàm lập tức trả về trạng thái `DrillStatus::RejectedVacuousSuite`, gán điểm 0.0 và đánh rớt kỳ diễn tập.

#### Kiểm toán viên cấp độ sẵn sàng (`ReadinessAuditor`)
Mức độ sẵn sàng tự hành của tác tử được phân thành 4 cấp bậc:
$$\text{L0 (Thủ công)} \longrightarrow \text{L1 (Hỗ trợ)} \longrightarrow \text{L2 (Giám sát chặt)} \longrightarrow \text{L3 (Tự hành hoàn toàn)}$$
Tác tử chỉ được phép hoạt động ở cấp độ `L3Autonomous` khi có biên nhận diễn tập LoopDrill hợp lệ đạt $\text{Sensitivity} = 1.0 \land \text{Specificity} = 1.0$. Nếu biên nhận có trạng thái thất bại, hoặc mã băm của biên nhận không khớp với dự kiến (dấu hiệu giả mạo kết quả), `ReadinessAuditor` sẽ ngay lập tức **hạ trần cấp phép từ L3 xuống L2**, chuyển tác tử về chế độ giám sát có con người can thiệp.

---

### 3.8 Vòng lặp phản hồi Trình biên dịch & Khách hàng chẩn đoán LSP

Khi tác tử lập trình sinh mã Rust, việc gặp lỗi biên dịch là điều khó tránh khỏi. Tuy nhiên, nếu không có cơ chế phản hồi có cấu trúc, tác tử rất dễ rơi vào "vòng lặp sửa quẩn": sửa lỗi A làm sinh ra lỗi B, rồi lại sửa ngược lại làm tái xuất hiện lỗi A.

Cấu trúc `CompilerFeedbackLoop`:
1. Tiếp nhận danh sách chẩn đoán lỗi từ LSP (`LspDiagnostic`) với tệp tin, dòng, cột, mã lỗi (ví dụ `E0308`) và thông điệp.
2. Tạo chữ ký định danh cho cụm lỗi hiện tại:
   $$\text{Signature} = \text{file} \mathbin{:} \text{line} \mathbin{:} \text{code}$$
3. So sánh với lịch sử chẩn đoán: Nếu chữ ký lỗi này đã xuất hiện trong các lần thử trước, phát hiện ngay tác tử đang quay cuồng vòng lặp và dừng khẩn cấp.
4. Sinh câu lệnh gợi ý sửa lỗi có cấu trúc (`structured remediation prompt`), nhắc nhở tác tử tuân thủ nguyên tắc sửa đổi tối thiểu (minimal change principle).

---

### 3.9 Bộ kiểm định Đa tầng (Multi-Layer Evaluator)

Để đánh giá chất lượng tổng thể của các tác tử lập trình, chúng ta không thể dựa vào một phép đo đơn lẻ. Bộ kiểm định đa tầng cung cấp các công cụ toán học và thuật toán chuẩn mực quốc tế:

#### Chứng minh toán học: $pass@k$ so với $pass^k$
Giả sử xác suất để một tác tử hoàn thành thành công một bài toán trong một lần chạy độc lập là $p \in (0, 1)$.
- Đại lượng $pass@k$ biểu thị xác suất có **ít nhất một** trong số $k$ lần thử độc lập thành công:
  $$pass@k = 1 - (1 - p)^k$$
- Đại lượng $pass^k$ biểu thị xác suất **tất cả** $k$ lần thử độc lập đều thành công liên tiếp (đo lường độ ổn định tuyệt đối):
  $$pass^k = p^k$$

Vì $0 < p < 1$, ta có $1 - p < 1$ và $p < 1$. Với mọi $k \ge 2$:
$$p^k < p < 1 - (1 - p)^k \implies pass^k < pass@k$$
Ví dụ thực tế: Với $p = 0.5$ và $k = 5$:
$$pass^5 = 0.5^5 = 0.03125 \quad (3.125\%)$$
$$pass@5 = 1 - (1 - 0.5)^5 = 1 - 0.03125 = 0.96875 \quad (96.875\%)$$
Bài kiểm thử trong mã nguồn của chương chứng minh tính đúng đắn của bất đẳng thức này, giúp lập trình viên không bao giờ nhầm lẫn giữa độ tin cậy kiên định ($pass^k$) và khả năng giải quyết bài toán qua nhiều lần thử ($pass@k$).

#### Trình so khớp AST theo chuẩn BFCL (`BfclAstMatcher`)
Khi đánh giá khả năng gọi công cụ (tool calling), các mô hình thường sinh ra các chuỗi JSON có thứ tự khóa khác nhau hoặc định dạng khoảng trắng khác nhau (ví dụ `{"a": 1, "b": 2}` so với `{"b": 2, "a": 1}`). `BfclAstMatcher` chuẩn hóa các tham số thành cây cú pháp có cấu trúc để so sánh tương đương ngữ nghĩa mà không phụ thuộc vào chuỗi ký tự thô.

#### Khử thiên kiến vị trí (`PositionSwappedJudge`)
Khi dùng LLM làm giám khảo đánh giá hai câu trả lời A và B, các nghiên cứu chỉ ra rằng mô hình có xu hướng thiên vị ứng viên đứng ở vị trí đầu tiên (Positional Bias). Thuật toán tráo vị trí thực hiện đánh giá hai lượt:
- Lượt 1: Đưa vào cặp $(A, B) \implies$ Phán quyết $R_1$.
- Lượt 2: Đưa vào cặp $(B, A) \implies$ Phán quyết $R_2$.
Nếu giám khảo chọn phương án đầu tiên ở cả 2 lượt ($R_1 = 0 \land R_2 = 0$), thuật toán sẽ phát hiện thiên kiến vị trí và hủy kết quả đánh giá.

#### Định luật Goodhart (`GoodhartDivergenceDetector`)
> *"Khi một chỉ số trở thành mục tiêu, nó không còn là một chỉ số tốt nữa."*

Trong các hệ thống tự hành, tác tử có thể tìm cách tối ưu hóa chỉ số đo lường trung gian (Proxy Metric - ví dụ số commit, độ dài giải thích) bằng các tiểu xảo mà không cải thiện chất lượng thực tế (Ground-Truth Utility). Bộ phát hiện phân kỳ cảnh báo khi:
$$\Delta \text{Proxy} > 0 \quad \land \quad \Delta \text{GroundTruth} < 0$$

---

## 4. Mã nguồn minh họa thực chiến

Toàn bộ mã nguồn dưới đây được thiết kế và kiểm thử trên **Rust 2024 thuần `std`**, không sử dụng bất kỳ crate bên ngoài nào. Tất cả các struct đều dùng trường có tên (named fields), mã băm SHA-256 được tự cài đặt chuẩn FIPS 180-4, và toàn bộ 53 bài unit test đều chạy tất định dưới 100ms.

Bạn có thể chạy thử chương trình trực tiếp bằng lệnh:
```bash
cargo run --manifest-path code/Cargo.toml -p ch90
```
Và kiểm tra toàn bộ bộ test bằng lệnh:
```bash
RUSTFLAGS="-D warnings -F non-ascii-idents" cargo test --manifest-path code/Cargo.toml -p ch90
```

```rust
//! Chương 90: Điều phối Đa Tác tử, Cổng Kỷ luật Bàn thợ, Hiến pháp,
//! Bánh cóc Tự tiến hóa & Diễn tập LoopDrill
//! (Multi-Agent Orchestration, Workbench Gates, RSI Ratchet & LoopDrill Evals)
//!
//! Ngôn ngữ: 100% Rust 2024 thuần `std` (không dependencies ngoài).
//! Kiến trúc struct: Toàn bộ dùng trường có tên (named fields).
//! Tuân thủ chuẩn chú thích RiskGate tiếng Việt và định danh tiếng Anh chuẩn.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

// ============================================================================
// 1. MÃ BĂM MẬT MÃ SHA-256 THUẦN STD (FIPS 180-4)
// ============================================================================

/// Hằng số 64 từ vòng lặp nén SHA-256 theo tiêu chuẩn FIPS 180-4.
const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Giá trị băm SHA-256 256-bit (32 byte) được bọc trong cấu trúc có trường tên.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Hash256 {
    pub bytes: [u8; 32],
}

impl Hash256 {
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self { bytes }
    }

    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        for b in &self.bytes {
            s.push_str(&format!("{b:02x}"));
        }
        s
    }

    pub fn short_hex(&self) -> String {
        format!("{:02x}{:02x}{:02x}{:02x}", self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3])
    }
}

impl fmt::Display for Hash256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Tính toán băm SHA-256 chuẩn FIPS 180-4 thuần Rust `std`.
pub fn sha256(data: &[u8]) -> Hash256 {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let mut m = data.to_vec();
    let bit_length = (data.len() as u64) * 8;
    m.push(0x80);
    while m.len() % 64 != 56 {
        m.push(0);
    }
    m.extend_from_slice(&bit_length.to_be_bytes());

    let (blocks, _) = m.as_chunks::<64>();
    for block in blocks {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, v) in h.iter().enumerate() {
        out[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    Hash256 { bytes: out }
}

// ============================================================================
// 2. BỘ CỔNG BÀN THỢ (WORKBENCH GATE SUITE) & SỔ CÁI ĐƠN TÍNH NĂNG
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureStatus {
    Active,
    Completed,
    Aborted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeatureRecord {
    pub name: String,
    pub branch: String,
    pub status: FeatureStatus,
    pub start_timestamp: u64,
}

#[derive(Debug, Clone, Default)]
pub struct FeatureLedger {
    pub records: Vec<FeatureRecord>,
}

impl FeatureLedger {
    pub fn new() -> Self {
        Self { records: Vec::new() }
    }

    pub fn start_feature(&mut self, name: &str, branch: &str, now: u64) -> Result<(), &'static str> {
        if self.has_active_feature() {
            return Err("Cổng kỷ luật Bàn thợ: Đã có tính năng đang hoạt động, không được mở tính năng thứ hai");
        }
        self.records.push(FeatureRecord {
            name: name.to_string(),
            branch: branch.to_string(),
            status: FeatureStatus::Active,
            start_timestamp: now,
        });
        Ok(())
    }

    pub fn complete_feature(&mut self, name: &str) -> Result<(), &'static str> {
        let rec = self
            .records
            .iter_mut()
            .find(|r| r.name == name && r.status == FeatureStatus::Active)
            .ok_or("Không tìm thấy tính năng đang hoạt động mang tên này")?;
        rec.status = FeatureStatus::Completed;
        Ok(())
    }

    pub fn abort_feature(&mut self, name: &str) -> Result<(), &'static str> {
        let rec = self
            .records
            .iter_mut()
            .find(|r| r.name == name && r.status == FeatureStatus::Active)
            .ok_or("Không tìm thấy tính năng đang hoạt động để hủy")?;
        rec.status = FeatureStatus::Aborted;
        Ok(())
    }

    pub fn has_active_feature(&self) -> bool {
        self.records.iter().any(|r| r.status == FeatureStatus::Active)
    }

    pub fn current_active_feature(&self) -> Option<&str> {
        self.records
            .iter()
            .find(|r| r.status == FeatureStatus::Active)
            .map(|r| r.name.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandoffArtifact {
    pub observation: String,
    pub logic_chain: String,
    pub caveats: String,
    pub conclusion: String,
    pub verification_method: String,
    pub test_evidence_hash: Option<Hash256>,
    pub tests_passed: usize,
    pub tests_failed: usize,
}

pub fn verify_completion_claim(
    artifact: &HandoffArtifact,
    required_tests: usize,
) -> Result<Hash256, &'static str> {
    if artifact.observation.trim().is_empty() {
        return Err("Thiếu phần Quan sát (Observation)");
    }
    if artifact.logic_chain.trim().is_empty() {
        return Err("Thiếu phần Chuỗi lập luận (Logic Chain)");
    }
    if artifact.conclusion.trim().is_empty() {
        return Err("Thiếu phần Kết luận (Conclusion)");
    }
    if artifact.verification_method.trim().is_empty() {
        return Err("Thiếu phần Phương pháp kiểm chứng (Verification Method)");
    }
    if artifact.tests_failed > 0 {
        return Err("Bác bỏ bàn giao: Còn bài kiểm thử thất bại (tests_failed > 0)");
    }
    if artifact.tests_passed < required_tests {
        return Err("Bác bỏ bàn giao: Số bài kiểm thử đạt chưa đạt chỉ tiêu tối thiểu");
    }
    let evidence_hash = artifact
        .test_evidence_hash
        .ok_or("Bác bỏ bàn giao: Thiếu mã băm bằng chứng kiểm thử (test_evidence_hash is None)")?;

    let mut seal_payload = Vec::new();
    seal_payload.extend_from_slice(artifact.observation.as_bytes());
    seal_payload.extend_from_slice(b"::");
    seal_payload.extend_from_slice(artifact.conclusion.as_bytes());
    seal_payload.extend_from_slice(b"::");
    seal_payload.extend_from_slice(&evidence_hash.bytes);
    seal_payload.extend_from_slice(&artifact.tests_passed.to_be_bytes());

    Ok(sha256(&seal_payload))
}

#[derive(Debug, Clone)]
pub struct LayerImportLinter {
    pub layer_ranks: BTreeMap<String, usize>,
}

impl LayerImportLinter {
    pub fn standard_three_tier() -> Self {
        let mut layer_ranks = BTreeMap::new();
        layer_ranks.insert("domain".to_string(), 0);
        layer_ranks.insert("application".to_string(), 1);
        layer_ranks.insert("infrastructure".to_string(), 2);
        Self { layer_ranks }
    }

    pub fn add_layer(&mut self, name: &str, rank: usize) {
        self.layer_ranks.insert(name.to_string(), rank);
    }

    pub fn check_import(&self, source_layer: &str, target_layer: &str) -> Result<(), &'static str> {
        let s_rank = self
            .layer_ranks
            .get(source_layer)
            .ok_or("Tầng nguồn không nằm trong cấu hình linter")?;
        let t_rank = self
            .layer_ranks
            .get(target_layer)
            .ok_or("Tầng đích không nằm trong cấu hình linter")?;

        if t_rank > s_rank {
            Err("Vi phạm kiến trúc phân tầng: Tầng thấp hơn không được import tầng cao hơn")
        } else {
            Ok(())
        }
    }
}

pub fn run_init_checks(required_baseline_files: &[String], actual_files: &[String]) -> Result<(), &'static str> {
    let actual_set: BTreeSet<&str> = actual_files.iter().map(String::as_str).collect();
    for req in required_baseline_files {
        if !actual_set.contains(req.as_str()) {
            return Err("Môi trường làm việc thiếu tệp cơ sở bắt buộc");
        }
    }
    Ok(())
}

pub fn check_diff_scope(modified_files: &[String], allowed_globs: &[String]) -> Result<(), &'static str> {
    for file in modified_files {
        let mut matched = false;
        for pattern in allowed_globs {
            if pattern.ends_with('*') {
                let prefix = &pattern[..pattern.len() - 1];
                if file.starts_with(prefix) {
                    matched = true;
                    break;
                }
            } else if file == pattern {
                matched = true;
                break;
            }
        }
        if !matched {
            return Err("Tác tử chạm vào tệp nằm ngoài phạm vi được phép chỉnh sửa");
        }
    }
    Ok(())
}

pub fn scan_workspace_cleanliness(files: &[String], forbidden_extensions: &[String]) -> Vec<String> {
    let mut violations = Vec::new();
    for file in files {
        for ext in forbidden_extensions {
            if file.ends_with(ext) {
                violations.push(file.clone());
                break;
            }
        }
    }
    violations
}

// ============================================================================
// 3. LỊCH CRON 5 TRƯỜNG & BỘ ĐIỀU KHIỂN VÒNG LẶP MỤC TIÊU HAI TẦNG GIÁM KHẢO
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CronSchedule {
    pub minute_pattern: String,
    pub hour_pattern: String,
    pub dom_pattern: String,
    pub month_pattern: String,
    pub dow_pattern: String,
}

impl CronSchedule {
    pub fn parse(expr: &str) -> Result<Self, &'static str> {
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() != 5 {
            return Err("Chuỗi Cron phải có đúng 5 trường cách nhau bởi dấu khoảng trắng");
        }
        Ok(Self {
            minute_pattern: parts[0].to_string(),
            hour_pattern: parts[1].to_string(),
            dom_pattern: parts[2].to_string(),
            month_pattern: parts[3].to_string(),
            dow_pattern: parts[4].to_string(),
        })
    }

    pub fn matches(&self, minute: u8, hour: u8, dom: u8, month: u8, dow: u8) -> bool {
        match_cron_field(&self.minute_pattern, minute)
            && match_cron_field(&self.hour_pattern, hour)
            && match_cron_field(&self.dom_pattern, dom)
            && match_cron_field(&self.month_pattern, month)
            && match_cron_field(&self.dow_pattern, dow)
    }
}

fn match_cron_field(pattern: &str, val: u8) -> bool {
    if pattern == "*" {
        return true;
    }
    if let Some(step_str) = pattern.strip_prefix("*/")
        && let Ok(step) = step_str.parse::<u8>()
    {
        return step > 0 && val.is_multiple_of(step);
    }
    if pattern.contains(',') {
        return pattern
            .split(',')
            .any(|sub| match_cron_field(sub.trim(), val));
    }
    if let Some((start_s, end_s)) = pattern.split_once('-')
        && let (Ok(s), Ok(e)) = (start_s.parse::<u8>(), end_s.parse::<u8>())
    {
        return val >= s && val <= e;
    }
    if let Ok(exact) = pattern.parse::<u8>() {
        return exact == val;
    }
    false
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JudgeVerdict {
    Passed,
    NeedsRetry,
    FailedFatal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JudgeEvaluation {
    pub tier: u8,
    pub verdict: JudgeVerdict,
    pub feedback: String,
}

#[derive(Debug, Clone)]
pub struct GoalLoopController {
    pub max_iterations: usize,
    pub current_iteration: usize,
    pub schedule: Option<CronSchedule>,
}

impl GoalLoopController {
    pub fn new(max_iterations: usize, schedule: Option<CronSchedule>) -> Self {
        Self {
            max_iterations,
            current_iteration: 0,
            schedule,
        }
    }

    pub fn evaluate_step<F1, F2>(&mut self, mut tier1_judge: F1, mut tier2_judge: F2) -> JudgeEvaluation
    where
        F1: FnMut() -> JudgeEvaluation,
        F2: FnMut() -> JudgeEvaluation,
    {
        self.current_iteration += 1;
        if self.current_iteration > self.max_iterations {
            return JudgeEvaluation {
                tier: 0,
                verdict: JudgeVerdict::FailedFatal,
                feedback: "Vòng lặp mục tiêu vượt quá trần số lần lặp cho phép".to_string(),
            };
        }

        let t1 = tier1_judge();
        if t1.verdict != JudgeVerdict::Passed {
            return t1;
        }

        tier2_judge()
    }
}

// ============================================================================
// 4. ĐIỀU PHỐI TRUNG GIAN HÌNH SAO, DẤU VẾT PHÂN TÁN & HÀNG ĐỢI SỰ KIỆN CHỦ ĐỀ
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentMessage {
    pub sender: String,
    pub recipient: String,
    pub correlation_id: String,
    pub payload: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceSpan {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_micros: u64,
    pub end_micros: u64,
    pub attributes: BTreeMap<String, String>,
}

impl TraceSpan {
    pub fn compute_hash(&self) -> Hash256 {
        let mut raw = Vec::new();
        raw.extend_from_slice(self.trace_id.as_bytes());
        raw.extend_from_slice(b"::");
        raw.extend_from_slice(self.span_id.as_bytes());
        raw.extend_from_slice(b"::");
        if let Some(p) = &self.parent_span_id {
            raw.extend_from_slice(p.as_bytes());
        }
        raw.extend_from_slice(b"::");
        raw.extend_from_slice(self.name.as_bytes());
        raw.extend_from_slice(&self.start_micros.to_be_bytes());
        raw.extend_from_slice(&self.end_micros.to_be_bytes());
        for (k, v) in &self.attributes {
            raw.extend_from_slice(k.as_bytes());
            raw.extend_from_slice(b"=");
            raw.extend_from_slice(v.as_bytes());
            raw.extend_from_slice(b";");
        }
        sha256(&raw)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicEvent {
    pub offset: u64,
    pub topic: String,
    pub message: AgentMessage,
    pub span_hash: Hash256,
}

#[derive(Debug, Clone, Default)]
pub struct TopicEventBus {
    pub events: Vec<TopicEvent>,
    pub next_offset: u64,
}

impl TopicEventBus {
    pub fn new() -> Self {
        Self {
            events: Vec::new(),
            next_offset: 0,
        }
    }

    pub fn publish(&mut self, topic: &str, message: AgentMessage, span_hash: Hash256) -> u64 {
        let offset = self.next_offset;
        self.next_offset += 1;
        self.events.push(TopicEvent {
            offset,
            topic: topic.to_string(),
            message,
            span_hash,
        });
        offset
    }

    pub fn poll(&self, topic: &str, cursor_offset: u64, max_events: usize) -> Vec<TopicEvent> {
        self.events
            .iter()
            .filter(|e| e.topic == topic && e.offset >= cursor_offset)
            .take(max_events)
            .cloned()
            .collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct MediatorOrchestrator {
    pub bus: TopicEventBus,
    pub registered_agents: BTreeSet<String>,
    pub routed_messages_count: usize,
}

impl MediatorOrchestrator {
    pub fn new() -> Self {
        Self {
            bus: TopicEventBus::new(),
            registered_agents: BTreeSet::new(),
            routed_messages_count: 0,
        }
    }

    pub fn register_agent(&mut self, agent_id: &str) {
        self.registered_agents.insert(agent_id.to_string());
    }

    pub fn forward_message(
        &mut self,
        msg: AgentMessage,
        span: &TraceSpan,
    ) -> Result<u64, &'static str> {
        if !self.registered_agents.contains(&msg.recipient) {
            return Err("Tác tử nhận chưa được đăng ký trong danh bạ Mediator");
        }
        if !self.registered_agents.contains(&msg.sender) {
            return Err("Tác tử gửi chưa được đăng ký trong danh bạ Mediator");
        }

        let span_hash = span.compute_hash();
        let topic = format!("agent.inbox.{}", msg.recipient);
        let offset = self.bus.publish(&topic, msg, span_hash);
        self.routed_messages_count += 1;
        Ok(offset)
    }
}

// ============================================================================
// 5. QUẠT LUỒNG KHẢO SÁT CÔ LẬP VÀ THỬ LẠI NHÁNH CON (EXPLORE FANOUT RUNNER)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub summary: String,
    pub confidence_score: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub file_path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceRecord {
    pub step: usize,
    pub action: String,
    pub result: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubBranchResult {
    pub branch_id: String,
    pub findings: Vec<Finding>,
    pub evidences: Vec<Evidence>,
    pub traces: Vec<TraceRecord>,
    pub succeeded: bool,
    pub retry_count: usize,
}

pub struct ExploreFanoutRunner;

impl ExploreFanoutRunner {
    pub fn run_fanout<F>(
        tasks: &[(&str, &str)],
        worker: F,
    ) -> Vec<SubBranchResult>
    where
        F: Fn(&str, &str, usize) -> Result<SubBranchResult, &'static str>,
    {
        let mut results = Vec::with_capacity(tasks.len());
        for &(branch_id, task_desc) in tasks {
            match worker(branch_id, task_desc, 0) {
                Ok(mut res) => {
                    res.succeeded = true;
                    res.retry_count = 0;
                    results.push(res);
                }
                Err(_) => {
                    match worker(branch_id, task_desc, 1) {
                        Ok(mut res) => {
                            res.succeeded = true;
                            res.retry_count = 1;
                            results.push(res);
                        }
                        Err(err_msg) => {
                            results.push(SubBranchResult {
                                branch_id: branch_id.to_string(),
                                findings: Vec::new(),
                                evidences: Vec::new(),
                                traces: vec![TraceRecord {
                                    step: 1,
                                    action: "isolated_retry_exhausted".to_string(),
                                    result: err_msg.to_string(),
                                }],
                                succeeded: false,
                                retry_count: 1,
                            });
                        }
                    }
                }
            }
        }
        results
    }
}

// ============================================================================
// 6. ĐẶC TẢ BÁN HÌNH THỨC 11 KHỐI & DANH BẠ ĐÁNH GIÁ MÙ (BLIND PORTFOLIO)
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PseudoFormalTaskBrief {
    pub objective: String,
    pub preconditions: Vec<String>,
    pub invariants: Vec<String>,
    pub input_artifacts: Vec<String>,
    pub output_artifacts: Vec<String>,
    pub constraints: Vec<String>,
    pub failure_modes: Vec<String>,
    pub verification_rules: Vec<String>,
    pub security_assumptions: Vec<String>,
    pub rollback_plan: String,
    pub attestation: String,
}

impl PseudoFormalTaskBrief {
    pub fn validate_11_blocks(&self) -> Result<(), &'static str> {
        if self.objective.trim().is_empty() {
            return Err("Khối 1: Mục tiêu (Objective) bị trống");
        }
        if self.preconditions.is_empty() {
            return Err("Khối 2: Tiền điều kiện (Preconditions) bị trống");
        }
        if self.invariants.is_empty() {
            return Err("Khối 3: Bất biến (Invariants) bị trống");
        }
        if self.input_artifacts.is_empty() {
            return Err("Khối 4: Hiện vật đầu vào (Input Artifacts) bị trống");
        }
        if self.output_artifacts.is_empty() {
            return Err("Khối 5: Hiện vật đầu ra (Output Artifacts) bị trống");
        }
        if self.constraints.is_empty() {
            return Err("Khối 6: Ràng buộc (Constraints) bị trống");
        }
        if self.failure_modes.is_empty() {
            return Err("Khối 7: Dạng lỗi dự phòng (Failure Modes) bị trống");
        }
        if self.verification_rules.is_empty() {
            return Err("Khối 8: Quy tắc kiểm chứng (Verification Rules) bị trống");
        }
        if self.security_assumptions.is_empty() {
            return Err("Khối 9: Giả định an ninh (Security Assumptions) bị trống");
        }
        if self.rollback_plan.trim().is_empty() {
            return Err("Khối 10: Phương án hoàn tác (Rollback Plan) bị trống");
        }
        if self.attestation.trim().is_empty() {
            return Err("Khối 11: Cam kết kiểm toán (Attestation) bị trống");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct BlindPortfolioRegistry {
    pub blind_to_real: BTreeMap<String, String>,
}

impl BlindPortfolioRegistry {
    pub fn new() -> Self {
        Self {
            blind_to_real: BTreeMap::new(),
        }
    }

    pub fn register_candidate(&mut self, blind_label: &str, real_identity: &str) {
        self.blind_to_real
            .insert(blind_label.to_string(), real_identity.to_string());
    }

    pub fn reveal_identity(&self, blind_label: &str) -> Option<&str> {
        self.blind_to_real.get(blind_label).map(String::as_str)
    }
}

// ============================================================================
// 7. HỘP CÁT CÂY LÀM VIỆC (WORKTREE SANDBOX), RAII DROP & KHÓA ƯU TIÊN ĐƯỜNG DẪN
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LeasePriority {
    DepSweeper = 10,
    PrBabysitter = 20,
    CiSweeper = 30,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathLease {
    pub pattern: String,
    pub holder: String,
    pub priority: LeasePriority,
    pub lease_expires: u64,
}

#[derive(Debug)]
pub struct WorktreeSandbox {
    pub sandbox_id: String,
    pub workspace_root: String,
    pub active_leases: Vec<PathLease>,
    pub dropped_cleanly: bool,
}

impl WorktreeSandbox {
    pub fn new(sandbox_id: &str, workspace_root: &str) -> Self {
        Self {
            sandbox_id: sandbox_id.to_string(),
            workspace_root: workspace_root.to_string(),
            active_leases: Vec::new(),
            dropped_cleanly: false,
        }
    }

    pub fn acquire_lease(
        &mut self,
        pattern: &str,
        holder: &str,
        priority: LeasePriority,
        expires: u64,
    ) -> Result<(), &'static str> {
        if let Some(pos) = self.active_leases.iter().position(|l| l.pattern == pattern) {
            let existing = &self.active_leases[pos];
            if priority > existing.priority {
                self.active_leases[pos] = PathLease {
                    pattern: pattern.to_string(),
                    holder: holder.to_string(),
                    priority,
                    lease_expires: expires,
                };
                return Ok(());
            } else {
                return Err("Xung đột khóa đường dẫn: Mức ưu tiên không đủ để tiếm quyền khóa hiện có");
            }
        }

        self.active_leases.push(PathLease {
            pattern: pattern.to_string(),
            holder: holder.to_string(),
            priority,
            lease_expires: expires,
        });
        Ok(())
    }

    pub fn release_lease(&mut self, pattern: &str, holder: &str) -> bool {
        let initial_len = self.active_leases.len();
        self.active_leases
            .retain(|l| !(l.pattern == pattern && l.holder == holder));
        self.active_leases.len() < initial_len
    }

    pub fn detect_3way_conflict(
        base: &str,
        ours: &str,
        theirs: &str,
    ) -> Result<String, Vec<String>> {
        let base_lines: Vec<&str> = base.lines().collect();
        let ours_lines: Vec<&str> = ours.lines().collect();
        let theirs_lines: Vec<&str> = theirs.lines().collect();

        if ours == theirs {
            return Ok(ours.to_string());
        }
        if ours == base {
            return Ok(theirs.to_string());
        }
        if theirs == base {
            return Ok(ours.to_string());
        }

        let max_lines = base_lines.len().max(ours_lines.len()).max(theirs_lines.len());
        let mut conflicts = Vec::new();
        let mut merged = Vec::new();

        for i in 0..max_lines {
            let b = base_lines.get(i).copied().unwrap_or("");
            let o = ours_lines.get(i).copied().unwrap_or("");
            let t = theirs_lines.get(i).copied().unwrap_or("");

            if o == t {
                merged.push(o.to_string());
            } else if o == b {
                merged.push(t.to_string());
            } else if t == b {
                merged.push(o.to_string());
            } else {
                conflicts.push(format!("Dòng {}: base='{}', ours='{}', theirs='{}'", i + 1, b, o, t));
            }
        }

        if conflicts.is_empty() {
            Ok(merged.join("\n"))
        } else {
            Err(conflicts)
        }
    }
}

impl Drop for WorktreeSandbox {
    fn drop(&mut self) {
        self.active_leases.clear();
        self.dropped_cleanly = true;
    }
}

// ============================================================================
// 8. NGẮT MẠCH ĐỘ TƯƠNG ĐỒNG MỜ (LOOP CONTEXT BREAKER)
// ============================================================================

#[derive(Debug, Clone)]
pub struct LoopContextBreaker {
    pub max_window: usize,
    pub loop_threshold: usize,
    pub similarity_threshold: f64,
    pub history: Vec<String>,
}

impl LoopContextBreaker {
    pub fn new(max_window: usize, loop_threshold: usize, similarity_threshold: f64) -> Self {
        Self {
            max_window,
            loop_threshold,
            similarity_threshold,
            history: Vec::new(),
        }
    }

    pub fn observe_and_check(&mut self, action_summary: &str) -> bool {
        self.history.push(action_summary.to_string());
        if self.history.len() > self.max_window {
            self.history.remove(0);
        }

        if self.history.len() < self.loop_threshold {
            return false;
        }

        let n = self.history.len();
        let target = &self.history[n - 1];
        let mut match_count = 0;

        for prev in self.history[n - self.loop_threshold..n - 1].iter() {
            if compute_jaccard_similarity(target, prev) >= self.similarity_threshold {
                match_count += 1;
            }
        }

        match_count >= (self.loop_threshold - 1)
    }
}

pub fn compute_jaccard_similarity(s1: &str, s2: &str) -> f64 {
    let words1: BTreeSet<&str> = s1.split_whitespace().collect();
    let words2: BTreeSet<&str> = s2.split_whitespace().collect();

    if words1.is_empty() && words2.is_empty() {
        return 1.0;
    }
    if words1.is_empty() || words2.is_empty() {
        return 0.0;
    }

    let intersection_len = words1.intersection(&words2).count();
    let union_len = words1.union(&words2).count();

    (intersection_len as f64) / (union_len as f64)
}

// ============================================================================
// 9. HIẾN PHÁP TÁC TỬ (CONSTITUTION ENGINE) & PHÂN TÁCH 3 NGÔI
// ============================================================================

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttestationRecord {
    pub author: String,
    pub verifier: String,
    pub attestor: String,
    pub specification_hash: Hash256,
    pub self_rule_hash: Hash256,
    pub claim_summary: String,
}

#[derive(Debug, Clone)]
pub struct ConstitutionEngine {
    pub pinned_self_hash: Hash256,
    pub forbidden_patterns: Vec<String>,
}

impl ConstitutionEngine {
    pub fn new(self_rules: &str, forbidden_patterns: Vec<String>) -> Self {
        let pinned_self_hash = sha256(self_rules.as_bytes());
        Self {
            pinned_self_hash,
            forbidden_patterns,
        }
    }

    pub fn verify_separation_of_duties(&self, record: &AttestationRecord) -> Result<(), &'static str> {
        if record.author == record.verifier {
            return Err("Vi phạm Hiến pháp: Tác giả không được phép tự đóng vai trò kiểm chứng (author == verifier)");
        }
        if record.author == record.attestor {
            return Err("Vi phạm Hiến pháp: Tác giả không được phép kiêm người chứng thực (author == attestor)");
        }
        if record.verifier == record.attestor {
            return Err("Vi phạm Hiến pháp: Người kiểm chứng không được phép kiêm người chứng thực (verifier == attestor)");
        }
        if record.self_rule_hash != self.pinned_self_hash {
            return Err("Vi phạm Hiến pháp: Mã băm quy tắc tự thân bị sai lệch so với bản ghim niêm phong");
        }
        Ok(())
    }

    pub fn check_content_compliance(&self, content: &str) -> Result<(), &'static str> {
        for pattern in &self.forbidden_patterns {
            if content.contains(pattern) {
                return Err("Nội dung chứa đoạn mã hoặc lệnh bị Hiến pháp nghiêm cấm");
            }
        }
        Ok(())
    }
}

// ============================================================================
// 10. BÁNH CÓC TỰ TIẾN HÓA HAI TẬP TÁCH (TWO-SPLIT RSI RATCHET)
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct RsiCandidateScore {
    pub held_in_pass_rate: f64,
    pub held_out_pass_rate: f64,
    pub subagent_cost_discount: f64,
}

#[derive(Debug, Clone)]
pub struct TwoSplitRsiRatchet {
    pub current_baseline_held_out: f64,
    pub current_baseline_held_in: f64,
    pub accepted_generations: usize,
    pub rejected_overfits: usize,
}

impl TwoSplitRsiRatchet {
    pub fn new(initial_held_in: f64, initial_held_out: f64) -> Self {
        Self {
            current_baseline_held_in: initial_held_in,
            current_baseline_held_out: initial_held_out,
            accepted_generations: 0,
            rejected_overfits: 0,
        }
    }

    pub fn evaluate_candidate(&mut self, score: &RsiCandidateScore) -> bool {
        if score.held_out_pass_rate < self.current_baseline_held_out {
            self.rejected_overfits += 1;
            return false;
        }

        let net_held_in = score.held_in_pass_rate - score.subagent_cost_discount;
        let net_held_out = score.held_out_pass_rate - score.subagent_cost_discount;

        if net_held_out >= self.current_baseline_held_out && net_held_in >= self.current_baseline_held_in {
            self.current_baseline_held_out = score.held_out_pass_rate;
            self.current_baseline_held_in = score.held_in_pass_rate;
            self.accepted_generations += 1;
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolicyDriftChecker {
    pub baseline_spec: BTreeMap<String, String>,
}

impl PolicyDriftChecker {
    pub fn new(baseline: BTreeMap<String, String>) -> Self {
        Self {
            baseline_spec: baseline,
        }
    }

    pub fn check_drift(&self, current: &BTreeMap<String, String>) -> Vec<String> {
        let mut drifts = Vec::new();
        for (k, v) in &self.baseline_spec {
            match current.get(k) {
                None => drifts.push(format!("Khóa bị xóa: {k}")),
                Some(cv) if cv != v => drifts.push(format!("Giá trị trôi lệch: {k} (cũ='{v}', mới='{cv}')")),
                _ => {}
            }
        }
        for k in current.keys() {
            if !self.baseline_spec.contains_key(k) {
                drifts.push(format!("Khóa mới chưa phê chuẩn: {k}"));
            }
        }
        drifts
    }
}

// ============================================================================
// 11. THAO TRƯỜNG DIỄN TẬP LOOPDRILL (SỬA LỖI EDGE CASE #6) & AUDITOR L3 -> L2
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrillStatus {
    Success,
    FailedSensitivity,
    FailedSpecificity,
    FailedBoth,
    RejectedVacuousSuite,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DrillReceipt {
    pub sensitivity: f64,
    pub specificity: f64,
    pub status: DrillStatus,
    pub total_positive_samples: usize,
    pub total_negative_samples: usize,
    pub receipt_hash: Hash256,
}

pub fn run_loop_drill<F: Fn(&str) -> bool>(
    guard: F,
    cases: &[(&str, bool)],
    min_sens: f64,
    min_spec: f64,
) -> DrillReceipt {
    let mut tp = 0usize;
    let mut fn_c = 0usize;
    let mut tn = 0usize;
    let mut fp = 0usize;

    for &(payload, is_fault) in cases {
        let blocked = guard(payload);
        match (is_fault, blocked) {
            (true, true) => tp += 1,
            (true, false) => fn_c += 1,
            (false, false) => tn += 1,
            (false, true) => fp += 1,
        }
    }

    let total_pos = tp + fn_c;
    let total_neg = tn + fp;

    // SỬA LỖI EDGE CASE #6: Kiểm tra bẫy chân lý rỗng
    if total_pos == 0 || total_neg == 0 {
        let receipt_hash = sha256(b"VACUOUS_TRUTH_REJECTED");
        return DrillReceipt {
            sensitivity: 0.0,
            specificity: 0.0,
            status: DrillStatus::RejectedVacuousSuite,
            total_positive_samples: total_pos,
            total_negative_samples: total_neg,
            receipt_hash,
        };
    }

    let sens = (tp as f64) / (total_pos as f64);
    let spec = (tn as f64) / (total_neg as f64);

    let sens_ok = sens >= min_sens;
    let spec_ok = spec >= min_spec;

    let status = match (sens_ok, spec_ok) {
        (true, true) => DrillStatus::Success,
        (false, true) => DrillStatus::FailedSensitivity,
        (true, false) => DrillStatus::FailedSpecificity,
        (false, false) => DrillStatus::FailedBoth,
    };

    let mut buf = Vec::new();
    buf.extend_from_slice(&(sens.to_bits()).to_be_bytes());
    buf.extend_from_slice(&(spec.to_bits()).to_be_bytes());
    buf.extend_from_slice(&(total_pos as u64).to_be_bytes());
    buf.extend_from_slice(&(total_neg as u64).to_be_bytes());
    buf.extend_from_slice(match status {
        DrillStatus::Success => b"STATUS:SUCCESS",
        DrillStatus::FailedSensitivity => b"STATUS:FAIL_SENS",
        DrillStatus::FailedSpecificity => b"STATUS:FAIL_SPEC",
        DrillStatus::FailedBoth => b"STATUS:FAIL_BOTH",
        DrillStatus::RejectedVacuousSuite => b"STATUS:VACUOUS",
    });

    let receipt_hash = sha256(&buf);

    DrillReceipt {
        sensitivity: sens,
        specificity: spec,
        status,
        total_positive_samples: total_pos,
        total_negative_samples: total_neg,
        receipt_hash,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReadinessLevel {
    L0Manual = 0,
    L1Assisted = 1,
    L2Monitored = 2,
    L3Autonomous = 3,
}

#[derive(Debug, Clone)]
pub struct ReadinessAuditor {
    pub current_level: ReadinessLevel,
    pub last_valid_receipt_hash: Option<Hash256>,
}

impl ReadinessAuditor {
    pub fn new(initial_level: ReadinessLevel) -> Self {
        Self {
            current_level: initial_level,
            last_valid_receipt_hash: None,
        }
    }

    pub fn verify_and_update(
        &mut self,
        receipt: &DrillReceipt,
        expected_hash: Hash256,
    ) -> ReadinessLevel {
        if receipt.status != DrillStatus::Success || receipt.receipt_hash != expected_hash {
            if self.current_level >= ReadinessLevel::L3Autonomous {
                self.current_level = ReadinessLevel::L2Monitored;
            }
            return self.current_level;
        }

        self.last_valid_receipt_hash = Some(receipt.receipt_hash);
        if receipt.sensitivity >= 1.0 && receipt.specificity >= 1.0 {
            self.current_level = ReadinessLevel::L3Autonomous;
        }
        self.current_level
    }
}

// ============================================================================
// 12. PHẢN HỒI TRÌNH BIÊN DỊCH & CHẨN ĐOÁN LSP (COMPILER FEEDBACK LOOP)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub file_path: String,
    pub line: usize,
    pub column: usize,
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct CompilerFeedbackLoop {
    pub max_attempts: usize,
    pub current_attempts: usize,
    pub diagnostic_history: Vec<String>,
}

impl CompilerFeedbackLoop {
    pub fn new(max_attempts: usize) -> Self {
        Self {
            max_attempts,
            current_attempts: 0,
            diagnostic_history: Vec::new(),
        }
    }

    pub fn feed_diagnostics(&mut self, diags: &[LspDiagnostic]) -> Result<String, &'static str> {
        self.current_attempts += 1;
        if self.current_attempts > self.max_attempts {
            return Err("Vượt quá số lần sửa lỗi biên dịch cho phép (max attempts exceeded)");
        }

        let errors: Vec<&LspDiagnostic> = diags
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
            .collect();

        if errors.is_empty() {
            return Ok("Trình biên dịch sạch lỗi!".to_string());
        }

        let sig = errors
            .iter()
            .map(|e| format!("{}:{}:{}", e.file_path, e.line, e.code))
            .collect::<Vec<_>>()
            .join("|");

        if self.diagnostic_history.iter().any(|h| h == &sig) {
            return Err("Tác tử rơi vào vòng lặp lặp lại cùng một lỗi biên dịch");
        }
        self.diagnostic_history.push(sig);

        let mut prompt = String::new();
        prompt.push_str("=== LỆNH SỬA LỖI BIÊN DỊCH CÓ CẤU TRÚC ===\n");
        for err in errors {
            prompt.push_str(&format!(
                "- [{}] {}:{}:{} - {}\n",
                err.code, err.file_path, err.line, err.column, err.message
            ));
        }
        prompt.push_str("Chỉ sửa đúng vị trí lỗi, tuân thủ nguyên tắc thay đổi tối thiểu!");
        Ok(prompt)
    }
}

// ============================================================================
// 13. BỘ KIỂM ĐỊNH ĐA TẦNG (MULTI-LAYER EVALUATOR)
// ============================================================================

pub struct TrajectoryGrader;

impl TrajectoryGrader {
    pub fn pass_at_k(n: usize, c: usize, k: usize) -> f64 {
        if n == 0 || k == 0 || k > n {
            return 0.0;
        }
        if n - c < k {
            return 1.0;
        }

        let mut prod = 1.0;
        for i in 0..k {
            let num = (n - c - i) as f64;
            let den = (n - i) as f64;
            prod *= num / den;
        }
        1.0 - prod
    }

    pub fn pass_pow_k(p: f64, k: usize) -> f64 {
        p.powi(k as i32)
    }
}

pub struct BfclAstMatcher;

impl BfclAstMatcher {
    pub fn matches_arguments(expected: &str, actual: &str) -> bool {
        let parse_kv = |s: &str| -> BTreeMap<String, String> {
            let mut map = BTreeMap::new();
            for part in s.trim().trim_matches(|c| c == '{' || c == '}').split(',') {
                if let Some((k, v)) = part.split_once(':') {
                    let key = k.trim().trim_matches('"').to_string();
                    let val = v.trim().trim_matches('"').to_string();
                    if !key.is_empty() {
                        map.insert(key, val);
                    }
                }
            }
            map
        };

        let exp_map = parse_kv(expected);
        let act_map = parse_kv(actual);

        exp_map == act_map
    }
}

pub struct GaiaNormalizer;

impl GaiaNormalizer {
    pub fn normalize(input: &str) -> String {
        let mut s = input.trim().to_lowercase();
        s = s.replace('$', "");
        s = s.replace('%', "");
        s = s.replace(',', "");
        while s.ends_with('.') || s.ends_with('?') || s.ends_with('!') {
            s.pop();
        }
        s.split_whitespace().collect::<Vec<_>>().join(" ")
    }
}

pub struct PositionSwappedJudge;

impl PositionSwappedJudge {
    pub fn judge_fairly<F>(
        cand_a: &str,
        cand_b: &str,
        eval_fn: F,
    ) -> Result<&'static str, &'static str>
    where
        F: Fn(&str, &str) -> usize,
    {
        let round1 = eval_fn(cand_a, cand_b);
        let round2 = eval_fn(cand_b, cand_a);

        match (round1, round2) {
            (0, 1) => Ok("Candidate_A"),
            (1, 0) => Ok("Candidate_B"),
            (0, 0) => Err("Phát hiện thiên kiến vị trí: Giám khảo luôn chọn ứng viên đứng trước"),
            (1, 1) => Err("Phát hiện thiên kiến vị trí: Giám khảo luôn chọn ứng viên đứng sau"),
            _ => Err("Kết quả đánh giá bất định"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GoodhartDivergenceDetector;

impl GoodhartDivergenceDetector {
    pub fn is_diverging(
        prev_proxy: f64,
        curr_proxy: f64,
        prev_ground_truth: f64,
        curr_ground_truth: f64,
    ) -> bool {
        let proxy_improved = curr_proxy > prev_proxy;
        let ground_truth_degraded = curr_ground_truth < prev_ground_truth;
        proxy_improved && ground_truth_degraded
    }
}

#[derive(Debug, Clone, Default)]
pub struct TraceFailureMiner {
    pub clusters: BTreeMap<String, usize>,
}

impl TraceFailureMiner {
    pub fn new() -> Self {
        Self {
            clusters: BTreeMap::new(),
        }
    }

    pub fn mine_trace(&mut self, error_category: &str) {
        *self.clusters.entry(error_category.to_string()).or_insert(0) += 1;
    }

    pub fn top_failure_category(&self) -> Option<(&str, usize)> {
        self.clusters
            .iter()
            .max_by_key(|&(_, count)| *count)
            .map(|(k, v)| (k.as_str(), *v))
    }
}

// ============================================================================
// 14. HÀM MAIN MINH HỌA THỰC CHIẾN
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 90: ĐIỀU PHỐI ĐA TÁC TỬ, CỔNG BÀN THỢ & DIỄN TẬP LOOPDRILL ===");
    println!("Kiến trúc: 100% Rust 2024 thuần std | Dependencies rỗng | Struct trường có tên");

    let mut ledger = FeatureLedger::new();
    ledger
        .start_feature("ch90_orchestration", "feat/ch90", 1700000000)
        .expect("Mở tính năng thành công");
    println!("1. FeatureLedger: Đã khóa tính năng '{}' đang hoạt động.", ledger.current_active_feature().unwrap());

    let evidence_hash = sha256(b"34_passed_tests_deterministic");
    let artifact = HandoffArtifact {
        observation: "Quan sát mã nguồn đạt 100% kiểm thử tất định".to_string(),
        logic_chain: "Lập luận tuần tự qua 5 cổng kiểm tra bàn thợ".to_string(),
        caveats: "Không có giới hạn ngoài phạm vi".to_string(),
        conclusion: "Sẵn sàng bàn giao cho tổng công trình sư".to_string(),
        verification_method: "cargo test --manifest-path code/Cargo.toml -p ch90".to_string(),
        test_evidence_hash: Some(evidence_hash),
        tests_passed: 34,
        tests_failed: 0,
    };
    let seal_hash = verify_completion_claim(&artifact, 34).expect("Cổng hoàn thành chấp thuận");
    println!("2. WorkbenchGate: Niêm phong biên nhận bàn giao thành công (Băm: {})", seal_hash.short_hex());

    let mut mediator = MediatorOrchestrator::new();
    mediator.register_agent("orchestrator");
    mediator.register_agent("worker_ch90");

    let span = TraceSpan {
        trace_id: "trace-90-001".to_string(),
        span_id: "span-01".to_string(),
        parent_span_id: None,
        name: "dispatch_worker".to_string(),
        start_micros: 1000,
        end_micros: 2000,
        attributes: BTreeMap::from([("phase".to_string(), "execution".to_string())]),
    };
    let msg = AgentMessage {
        sender: "orchestrator".to_string(),
        recipient: "worker_ch90".to_string(),
        correlation_id: "corr-101".to_string(),
        payload: "Yêu cầu thi công Chương 90 đạt chuẩn".to_string(),
        timestamp: 1700000005,
    };
    let offset = mediator.forward_message(msg, &span).expect("Mediator chuyển tiếp tin thành công");
    println!("3. Mediator: Đã chuyển tiếp tin nhắn nguyên văn tại CursorOffset={offset}.");

    let constitution = ConstitutionEngine::new("LUAT_HIEN_PHAP_BAT_BIEN_2024", vec!["rm -rf /".to_string()]);
    let attestation = AttestationRecord {
        author: "worker_ch90".to_string(),
        verifier: "reviewer_ch90".to_string(),
        attestor: "auditor_ch90".to_string(),
        specification_hash: sha256(b"SPEC_CH90"),
        self_rule_hash: constitution.pinned_self_hash,
        claim_summary: "Đạt chuẩn 3 ngôi".to_string(),
    };
    constitution.verify_separation_of_duties(&attestation).expect("Hiến pháp phê chuẩn 3 ngôi");
    println!("4. ConstitutionEngine: Phân tách 3 ngôi (author != verifier != attestor) hợp lệ.");

    let drill_cases = [
        ("rm -rf /", true),
        ("drop database", true),
        ("cargo test", false),
        ("cargo check", false),
    ];
    let receipt = run_loop_drill(
        |p| p.contains("rm -rf") || p.contains("drop"),
        &drill_cases,
        1.0,
        1.0,
    );
    println!(
        "5. LoopDrill: Sens={:.2}, Spec={:.2}, Status={:?} (Băm biên nhận: {})",
        receipt.sensitivity, receipt.specificity, receipt.status, receipt.receipt_hash.short_hex()
    );

    let p5 = TrajectoryGrader::pass_pow_k(0.6, 5);
    let at5 = TrajectoryGrader::pass_at_k(10, 6, 5);
    println!("6. TrajectoryGrader: pass^5 ({:.4}) < pass@5 ({:.4}) -> Tính toán chính xác!", p5, at5);

    println!("=== HOÀN TẤT TRÌNH DIỄN CHƯƠNG 90 THÀNH CÔNG ===");
}

// ============================================================================
// 15. BỘ KIỂM THỬ BẮT BUỘC (53 BÀI KIỂM THỬ TẤT ĐỊNH)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_fips_vectors() {
        assert_eq!(
            sha256(b"").to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256(b"abc").to_hex(),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn test_sha256_short_hex_and_display() {
        let hash = sha256(b"learn-rs");
        assert_eq!(hash.short_hex().len(), 8);
        assert_eq!(format!("{hash}"), hash.to_hex());
    }

    #[test]
    fn test_feature_ledger_single_feature_lock() {
        let mut ledger = FeatureLedger::new();
        assert!(ledger.start_feature("feat_1", "branch_1", 100).is_ok());
        assert!(ledger.start_feature("feat_2", "branch_2", 105).is_err());
        assert_eq!(ledger.current_active_feature(), Some("feat_1"));
    }

    #[test]
    fn test_feature_ledger_complete_and_reopen() {
        let mut ledger = FeatureLedger::new();
        ledger.start_feature("feat_1", "branch_1", 100).unwrap();
        assert!(ledger.complete_feature("feat_1").is_ok());
        assert!(!ledger.has_active_feature());
        assert!(ledger.start_feature("feat_2", "branch_2", 200).is_ok());
        assert_eq!(ledger.current_active_feature(), Some("feat_2"));
    }

    #[test]
    fn test_feature_ledger_abort_active() {
        let mut ledger = FeatureLedger::new();
        ledger.start_feature("feat_faulty", "branch_bad", 50).unwrap();
        assert!(ledger.abort_feature("feat_faulty").is_ok());
        assert!(!ledger.has_active_feature());
    }

    #[test]
    fn test_verify_completion_claim_rejects_missing_evidence() {
        let artifact = HandoffArtifact {
            observation: "Obs".to_string(),
            logic_chain: "Logic".to_string(),
            caveats: "None".to_string(),
            conclusion: "Done".to_string(),
            verification_method: "cargo test".to_string(),
            test_evidence_hash: None,
            tests_passed: 35,
            tests_failed: 0,
        };
        assert!(verify_completion_claim(&artifact, 34).is_err());
    }

    #[test]
    fn test_verify_completion_claim_rejects_test_failures() {
        let artifact = HandoffArtifact {
            observation: "Obs".to_string(),
            logic_chain: "Logic".to_string(),
            caveats: "None".to_string(),
            conclusion: "Done".to_string(),
            verification_method: "cargo test".to_string(),
            test_evidence_hash: Some(sha256(b"evidence")),
            tests_passed: 34,
            tests_failed: 1,
        };
        assert!(verify_completion_claim(&artifact, 34).is_err());
    }

    #[test]
    fn test_verify_completion_claim_rejects_insufficient_passed_tests() {
        let artifact = HandoffArtifact {
            observation: "Obs".to_string(),
            logic_chain: "Logic".to_string(),
            caveats: "None".to_string(),
            conclusion: "Done".to_string(),
            verification_method: "cargo test".to_string(),
            test_evidence_hash: Some(sha256(b"evidence")),
            tests_passed: 30,
            tests_failed: 0,
        };
        assert!(verify_completion_claim(&artifact, 34).is_err());
    }

    #[test]
    fn test_verify_completion_claim_approves_valid_artifact() {
        let artifact = HandoffArtifact {
            observation: "Obs".to_string(),
            logic_chain: "Logic".to_string(),
            caveats: "None".to_string(),
            conclusion: "Done".to_string(),
            verification_method: "cargo test".to_string(),
            test_evidence_hash: Some(sha256(b"evidence")),
            tests_passed: 36,
            tests_failed: 0,
        };
        let receipt = verify_completion_claim(&artifact, 34);
        assert!(receipt.is_ok());
    }

    #[test]
    fn test_layer_import_linter_enforces_hierarchy() {
        let linter = LayerImportLinter::standard_three_tier();
        assert!(linter.check_import("domain", "infrastructure").is_err());
        assert!(linter.check_import("application", "infrastructure").is_err());
    }

    #[test]
    fn test_layer_import_linter_allows_valid_imports() {
        let linter = LayerImportLinter::standard_three_tier();
        assert!(linter.check_import("infrastructure", "domain").is_ok());
        assert!(linter.check_import("infrastructure", "application").is_ok());
        assert!(linter.check_import("domain", "domain").is_ok());
    }

    #[test]
    fn test_run_init_checks_and_diff_scope() {
        let req = vec!["Cargo.toml".to_string(), "src/main.rs".to_string()];
        let actual = vec!["Cargo.toml".to_string(), "src/main.rs".to_string(), "README.md".to_string()];
        assert!(run_init_checks(&req, &actual).is_ok());

        let missing = vec!["Cargo.toml".to_string()];
        assert!(run_init_checks(&req, &missing).is_err());

        let allowed = vec!["code/ch90/*".to_string(), "chuong_90.md".to_string()];
        let modified_ok = vec!["code/ch90/src/main.rs".to_string(), "chuong_90.md".to_string()];
        assert!(check_diff_scope(&modified_ok, &allowed).is_ok());

        let modified_bad = vec!["code/ch86/src/main.rs".to_string()];
        assert!(check_diff_scope(&modified_bad, &allowed).is_err());
    }

    #[test]
    fn test_scan_workspace_cleanliness() {
        let files = vec![
            "src/main.rs".to_string(),
            "target/output.tmp".to_string(),
            "src/main.rs.bk".to_string(),
        ];
        let forbidden = vec![".tmp".to_string(), ".bk".to_string()];
        let dirty = scan_workspace_cleanliness(&files, &forbidden);
        assert_eq!(dirty.len(), 2);
    }

    #[test]
    fn test_cron_schedule_parsing_and_matching() {
        let cron = CronSchedule::parse("*/5 9 1-15 * *").unwrap();
        assert!(cron.matches(10, 9, 10, 5, 2));
        assert!(!cron.matches(11, 9, 10, 5, 2));
        assert!(!cron.matches(10, 10, 10, 5, 2));
    }

    #[test]
    fn test_cron_schedule_invalid_format() {
        assert!(CronSchedule::parse("*/5 * *").is_err());
    }

    #[test]
    fn test_goal_loop_controller_tier1_early_exit() {
        let mut controller = GoalLoopController::new(5, None);
        let mut tier2_called = false;

        let eval = controller.evaluate_step(
            || JudgeEvaluation {
                tier: 1,
                verdict: JudgeVerdict::NeedsRetry,
                feedback: "Cú pháp lỗi".to_string(),
            },
            || {
                tier2_called = true;
                JudgeEvaluation {
                    tier: 2,
                    verdict: JudgeVerdict::Passed,
                    feedback: "Hợp lệ".to_string(),
                }
            },
        );

        assert_eq!(eval.verdict, JudgeVerdict::NeedsRetry);
        assert!(!tier2_called);
    }

    #[test]
    fn test_goal_loop_controller_tier2_completion() {
        let mut controller = GoalLoopController::new(5, None);
        let eval = controller.evaluate_step(
            || JudgeEvaluation {
                tier: 1,
                verdict: JudgeVerdict::Passed,
                feedback: "Liveness OK".to_string(),
            },
            || JudgeEvaluation {
                tier: 2,
                verdict: JudgeVerdict::Passed,
                feedback: "Semantic Tests Passed".to_string(),
            },
        );
        assert_eq!(eval.verdict, JudgeVerdict::Passed);
    }

    #[test]
    fn test_goal_loop_controller_max_iteration_exhaustion() {
        let mut controller = GoalLoopController::new(2, None);
        controller.evaluate_step(
            || JudgeEvaluation { tier: 1, verdict: JudgeVerdict::Passed, feedback: "".to_string() },
            || JudgeEvaluation { tier: 2, verdict: JudgeVerdict::NeedsRetry, feedback: "".to_string() },
        );
        controller.evaluate_step(
            || JudgeEvaluation { tier: 1, verdict: JudgeVerdict::Passed, feedback: "".to_string() },
            || JudgeEvaluation { tier: 2, verdict: JudgeVerdict::NeedsRetry, feedback: "".to_string() },
        );
        let eval = controller.evaluate_step(
            || JudgeEvaluation { tier: 1, verdict: JudgeVerdict::Passed, feedback: "".to_string() },
            || JudgeEvaluation { tier: 2, verdict: JudgeVerdict::Passed, feedback: "".to_string() },
        );
        assert_eq!(eval.verdict, JudgeVerdict::FailedFatal);
    }

    #[test]
    fn test_trace_span_hash_deterministic() {
        let s1 = TraceSpan {
            trace_id: "t1".to_string(),
            span_id: "s1".to_string(),
            parent_span_id: None,
            name: "init".to_string(),
            start_micros: 10,
            end_micros: 20,
            attributes: BTreeMap::from([("k".to_string(), "v".to_string())]),
        };
        let s2 = s1.clone();
        assert_eq!(s1.compute_hash(), s2.compute_hash());
    }

    #[test]
    fn test_topic_event_bus_cursor_pagination() {
        let mut bus = TopicEventBus::new();
        let dummy_hash = sha256(b"dummy");
        let msg = AgentMessage {
            sender: "a".to_string(),
            recipient: "b".to_string(),
            correlation_id: "c1".to_string(),
            payload: "hi".to_string(),
            timestamp: 1,
        };
        let off0 = bus.publish("topic.a", msg.clone(), dummy_hash);
        let off1 = bus.publish("topic.a", msg.clone(), dummy_hash);
        let off2 = bus.publish("topic.b", msg, dummy_hash);

        assert_eq!(off0, 0);
        assert_eq!(off1, 1);
        assert_eq!(off2, 2);

        let polled = bus.poll("topic.a", 1, 10);
        assert_eq!(polled.len(), 1);
        assert_eq!(polled[0].offset, 1);
    }

    #[test]
    fn test_mediator_forward_message_verbatim_integrity() {
        let mut med = MediatorOrchestrator::new();
        med.register_agent("worker");
        med.register_agent("reviewer");

        let payload = "Dữ liệu byte nguyên văn: \u{00}\u{FF}\u{AA}\u{55} - 100% không đổi!";
        let msg = AgentMessage {
            sender: "worker".to_string(),
            recipient: "reviewer".to_string(),
            correlation_id: "corr-99".to_string(),
            payload: payload.to_string(),
            timestamp: 1000,
        };
        let span = TraceSpan {
            trace_id: "tr".to_string(),
            span_id: "sp".to_string(),
            parent_span_id: None,
            name: "fwd".to_string(),
            start_micros: 10,
            end_micros: 20,
            attributes: BTreeMap::new(),
        };

        let off = med.forward_message(msg.clone(), &span).unwrap();
        let events = med.bus.poll("agent.inbox.reviewer", off, 1);
        assert_eq!(events[0].message.payload, payload);
    }

    #[test]
    fn test_mediator_forward_message_rejects_unregistered() {
        let mut med = MediatorOrchestrator::new();
        med.register_agent("worker");
        let msg = AgentMessage {
            sender: "worker".to_string(),
            recipient: "unknown_agent".to_string(),
            correlation_id: "c".to_string(),
            payload: "hello".to_string(),
            timestamp: 10,
        };
        let span = TraceSpan {
            trace_id: "tr".to_string(),
            span_id: "sp".to_string(),
            parent_span_id: None,
            name: "fwd".to_string(),
            start_micros: 1,
            end_micros: 2,
            attributes: BTreeMap::new(),
        };
        assert!(med.forward_message(msg, &span).is_err());
    }

    #[test]
    fn test_explore_fanout_runner_recovers_on_retry() {
        let tasks = [("b1", "task 1"), ("b2", "task 2")];
        let results = ExploreFanoutRunner::run_fanout(&tasks, |bid, _desc, attempt| {
            if bid == "b2" && attempt == 0 {
                Err("Tạm thời ngắt mạng")
            } else {
                Ok(SubBranchResult {
                    branch_id: bid.to_string(),
                    findings: vec![Finding { summary: "Ok".to_string(), confidence_score: 90 }],
                    evidences: Vec::new(),
                    traces: Vec::new(),
                    succeeded: true,
                    retry_count: attempt,
                })
            }
        });

        assert_eq!(results.len(), 2);
        assert!(results[0].succeeded && results[0].retry_count == 0);
        assert!(results[1].succeeded && results[1].retry_count == 1);
    }

    #[test]
    fn test_explore_fanout_runner_exhaustion() {
        let tasks = [("b_err", "failing task")];
        let results = ExploreFanoutRunner::run_fanout(&tasks, |_bid, _desc, _attempt| {
            Err("Lỗi bất khả kháng liên tục")
        });
        assert_eq!(results.len(), 1);
        assert!(!results[0].succeeded);
        assert_eq!(results[0].retry_count, 1);
    }

    #[test]
    fn test_pseudo_formal_brief_validates_11_blocks() {
        let brief = PseudoFormalTaskBrief {
            objective: "Obj".to_string(),
            preconditions: vec!["Pre".to_string()],
            invariants: vec!["Inv".to_string()],
            input_artifacts: vec!["In".to_string()],
            output_artifacts: vec!["Out".to_string()],
            constraints: vec!["Const".to_string()],
            failure_modes: vec!["Fail".to_string()],
            verification_rules: vec!["Rule".to_string()],
            security_assumptions: vec!["Sec".to_string()],
            rollback_plan: "Rollback".to_string(),
            attestation: "Attest".to_string(),
        };
        assert!(brief.validate_11_blocks().is_ok());
    }

    #[test]
    fn test_pseudo_formal_brief_rejects_incomplete() {
        let mut brief = PseudoFormalTaskBrief {
            objective: "".to_string(),
            preconditions: vec!["Pre".to_string()],
            invariants: vec!["Inv".to_string()],
            input_artifacts: vec!["In".to_string()],
            output_artifacts: vec!["Out".to_string()],
            constraints: vec!["Const".to_string()],
            failure_modes: vec!["Fail".to_string()],
            verification_rules: vec!["Rule".to_string()],
            security_assumptions: vec!["Sec".to_string()],
            rollback_plan: "Rollback".to_string(),
            attestation: "Attest".to_string(),
        };
        assert!(brief.validate_11_blocks().is_err());
        brief.objective = "Obj".to_string();
        brief.invariants.clear();
        assert!(brief.validate_11_blocks().is_err());
    }

    #[test]
    fn test_blind_portfolio_registry_blinds_and_reveals() {
        let mut reg = BlindPortfolioRegistry::new();
        reg.register_candidate("CANDIDATE_A", "Model_GPT4");
        reg.register_candidate("CANDIDATE_B", "Model_Claude37");
        assert_eq!(reg.reveal_identity("CANDIDATE_A"), Some("Model_GPT4"));
        assert_eq!(reg.reveal_identity("CANDIDATE_UNKNOWN"), None);
    }

    #[test]
    fn test_worktree_priority_lease_ci_sweeper_preempts() {
        let mut sandbox = WorktreeSandbox::new("sb1", "/tmp/sb1");
        sandbox
            .acquire_lease("src/*.rs", "dep_agent", LeasePriority::DepSweeper, 100)
            .unwrap();

        assert!(sandbox
            .acquire_lease("src/*.rs", "pr_agent", LeasePriority::PrBabysitter, 200)
            .is_ok());
        assert_eq!(sandbox.active_leases[0].holder, "pr_agent");

        assert!(sandbox
            .acquire_lease("src/*.rs", "ci_agent", LeasePriority::CiSweeper, 300)
            .is_ok());
        assert_eq!(sandbox.active_leases[0].holder, "ci_agent");

        assert!(sandbox
            .acquire_lease("src/*.rs", "dep_agent", LeasePriority::DepSweeper, 400)
            .is_err());
    }

    #[test]
    fn test_worktree_3way_merge_clean_no_conflict() {
        let base = "line1\nline2\nline3";
        let ours = "line1_edited\nline2\nline3";
        let theirs = "line1\nline2\nline3_edited";
        let merged = WorktreeSandbox::detect_3way_conflict(base, ours, theirs).unwrap();
        assert_eq!(merged, "line1_edited\nline2\nline3_edited");
    }

    #[test]
    fn test_worktree_3way_merge_conflict_detected() {
        let base = "line1\nline2\nline3";
        let ours = "line1\nours_modified\nline3";
        let theirs = "line1\ntheirs_modified\nline3";
        let res = WorktreeSandbox::detect_3way_conflict(base, ours, theirs);
        assert!(res.is_err());
    }

    #[test]
    fn test_worktree_sandbox_raii_drop_releases_leases() {
        let mut sandbox = WorktreeSandbox::new("sb_drop", "/tmp/sb");
        sandbox
            .acquire_lease("lib.rs", "holder", LeasePriority::CiSweeper, 100)
            .unwrap();
        assert_eq!(sandbox.active_leases.len(), 1);
        drop(sandbox);
    }

    #[test]
    fn test_loop_context_breaker_trips_on_repetition() {
        let mut breaker = LoopContextBreaker::new(10, 3, 0.8);
        assert!(!breaker.observe_and_check("Sửa lỗi cú pháp biến x"));
        assert!(!breaker.observe_and_check("Sửa lỗi cú pháp biến x"));
        assert!(breaker.observe_and_check("Sửa lỗi cú pháp biến x"));
    }

    #[test]
    fn test_loop_context_breaker_allows_diverse_actions() {
        let mut breaker = LoopContextBreaker::new(10, 3, 0.8);
        assert!(!breaker.observe_and_check("Chạy lệnh kiểm thử ban đầu"));
        assert!(!breaker.observe_and_check("Đọc cấu trúc file README"));
        assert!(!breaker.observe_and_check("Thực thi git commit"));
    }

    #[test]
    fn test_constitution_separation_of_duties_rejects_author_as_verifier() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec!["forbidden".to_string()]);
        let rec = AttestationRecord {
            author: "agent_a".to_string(),
            verifier: "agent_a".to_string(),
            attestor: "agent_b".to_string(),
            specification_hash: sha256(b"spec"),
            self_rule_hash: constitution.pinned_self_hash,
            claim_summary: "Done".to_string(),
        };
        assert!(constitution.verify_separation_of_duties(&rec).is_err());
    }

    #[test]
    fn test_constitution_separation_of_duties_rejects_verifier_as_attestor() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec![]);
        let rec = AttestationRecord {
            author: "agent_a".to_string(),
            verifier: "agent_b".to_string(),
            attestor: "agent_b".to_string(),
            specification_hash: sha256(b"spec"),
            self_rule_hash: constitution.pinned_self_hash,
            claim_summary: "Done".to_string(),
        };
        assert!(constitution.verify_separation_of_duties(&rec).is_err());
    }

    #[test]
    fn test_constitution_separation_of_duties_approves_distinct_roles() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec![]);
        let rec = AttestationRecord {
            author: "agent_a".to_string(),
            verifier: "agent_b".to_string(),
            attestor: "agent_c".to_string(),
            specification_hash: sha256(b"spec"),
            self_rule_hash: constitution.pinned_self_hash,
            claim_summary: "Done".to_string(),
        };
        assert!(constitution.verify_separation_of_duties(&rec).is_ok());
    }

    #[test]
    fn test_constitution_rejects_tampered_self_rule_hash() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec![]);
        let rec = AttestationRecord {
            author: "agent_a".to_string(),
            verifier: "agent_b".to_string(),
            attestor: "agent_c".to_string(),
            specification_hash: sha256(b"spec"),
            self_rule_hash: sha256(b"TAMPERED_RULES"),
            claim_summary: "Done".to_string(),
        };
        assert!(constitution.verify_separation_of_duties(&rec).is_err());
    }

    #[test]
    fn test_constitution_content_compliance() {
        let constitution = ConstitutionEngine::new("RULES_V1", vec!["dangerous_payload".to_string()]);
        assert!(constitution.check_content_compliance("safe payload code").is_ok());
        assert!(constitution.check_content_compliance("contains dangerous_payload in body").is_err());
    }

    #[test]
    fn test_rsi_ratchet_rejects_overfit_candidate() {
        let mut ratchet = TwoSplitRsiRatchet::new(0.80, 0.85);
        let overfit_score = RsiCandidateScore {
            held_in_pass_rate: 0.95,
            held_out_pass_rate: 0.82,
            subagent_cost_discount: 0.0,
        };
        assert!(!ratchet.evaluate_candidate(&overfit_score));
        assert_eq!(ratchet.rejected_overfits, 1);
        assert_eq!(ratchet.accepted_generations, 0);
    }

    #[test]
    fn test_rsi_ratchet_accepts_generalizing_candidate() {
        let mut ratchet = TwoSplitRsiRatchet::new(0.80, 0.85);
        let good_score = RsiCandidateScore {
            held_in_pass_rate: 0.88,
            held_out_pass_rate: 0.90,
            subagent_cost_discount: 0.01,
        };
        assert!(ratchet.evaluate_candidate(&good_score));
        assert_eq!(ratchet.accepted_generations, 1);
        assert_eq!(ratchet.current_baseline_held_out, 0.90);
    }

    #[test]
    fn test_policy_drift_checker_detects_deviations() {
        let baseline = BTreeMap::from([
            ("mode".to_string(), "strict".to_string()),
            ("timeout".to_string(), "30".to_string()),
        ]);
        let checker = PolicyDriftChecker::new(baseline);

        let drifted = BTreeMap::from([
            ("mode".to_string(), "lenient".to_string()),
            ("unauthorized_key".to_string(), "val".to_string()),
        ]);
        let drifts = checker.check_drift(&drifted);
        assert_eq!(drifts.len(), 3);
    }

    #[test]
    fn test_loop_drill_edge_case_6_anti_vacuous_truth_empty_positive() {
        let vacuous_cases = [("cargo test", false), ("cargo build", false)];
        let receipt = run_loop_drill(|_| false, &vacuous_cases, 0.9, 0.9);
        assert_eq!(receipt.status, DrillStatus::RejectedVacuousSuite);
    }

    #[test]
    fn test_loop_drill_edge_case_6_anti_vacuous_truth_empty_negative() {
        let vacuous_cases = [("rm -rf /", true), ("drop table", true)];
        let receipt = run_loop_drill(|_| true, &vacuous_cases, 0.9, 0.9);
        assert_eq!(receipt.status, DrillStatus::RejectedVacuousSuite);
    }

    #[test]
    fn test_loop_drill_dual_threshold_and_receipt() {
        let balanced_cases = [
            ("rm -rf /", true),
            ("format c:", true),
            ("cargo test", false),
            ("cargo check", false),
        ];
        let receipt = run_loop_drill(
            |p| p.contains("rm -rf") || p.contains("format"),
            &balanced_cases,
            0.95,
            0.95,
        );
        assert_eq!(receipt.status, DrillStatus::Success);
        assert!((receipt.sensitivity - 1.0).abs() < 1e-6);
        assert!((receipt.specificity - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_loop_drill_fails_on_permissive_or_overly_strict_guard() {
        let balanced_cases = [
            ("attack_1", true),
            ("attack_2", true),
            ("benign_1", false),
            ("benign_2", false),
        ];
        let perm = run_loop_drill(|_| false, &balanced_cases, 0.9, 0.9);
        assert_eq!(perm.status, DrillStatus::FailedSensitivity);

        let strict = run_loop_drill(|_| true, &balanced_cases, 0.9, 0.9);
        assert_eq!(strict.status, DrillStatus::FailedSpecificity);
    }

    #[test]
    fn test_readiness_auditor_demotes_l3_to_l2_on_tampered_receipt() {
        let mut auditor = ReadinessAuditor::new(ReadinessLevel::L3Autonomous);
        let receipt = DrillReceipt {
            sensitivity: 1.0,
            specificity: 1.0,
            status: DrillStatus::Success,
            total_positive_samples: 10,
            total_negative_samples: 10,
            receipt_hash: sha256(b"actual_hash"),
        };
        let forged_expected_hash = sha256(b"forged_hash");
        let level = auditor.verify_and_update(&receipt, forged_expected_hash);
        assert_eq!(level, ReadinessLevel::L2Monitored);
    }

    #[test]
    fn test_lsp_diagnostic_client_and_compiler_feedback_loop() {
        let mut feedback = CompilerFeedbackLoop::new(3);
        let diags = vec![LspDiagnostic {
            file_path: "src/main.rs".to_string(),
            line: 42,
            column: 10,
            severity: DiagnosticSeverity::Error,
            code: "E0308".to_string(),
            message: "Mismatched types".to_string(),
        }];
        let prompt = feedback.feed_diagnostics(&diags).unwrap();
        assert!(prompt.contains("E0308"));
        assert!(prompt.contains("src/main.rs:42:10"));

        assert!(feedback.feed_diagnostics(&diags).is_err());
    }

    #[test]
    fn test_trajectory_grader_pass_at_k_vs_pass_pow_k() {
        let p = 0.5;
        let pass_pow_5 = TrajectoryGrader::pass_pow_k(p, 5);
        let pass_at_5 = TrajectoryGrader::pass_at_k(10, 5, 5);
        assert!(pass_pow_5 < pass_at_5);
        assert!((pass_pow_5 - 0.03125).abs() < 1e-6);
    }

    #[test]
    fn test_bfcl_ast_matcher_ignores_order_and_types() {
        let exp = r#"{"timeout": 30, "model": "fast"}"#;
        let act = r#"{"model": "fast", "timeout": 30}"#;
        assert!(BfclAstMatcher::matches_arguments(exp, act));

        let different = r#"{"model": "fast", "timeout": 60}"#;
        assert!(!BfclAstMatcher::matches_arguments(exp, different));
    }

    #[test]
    fn test_gaia_normalizer_cleanses_strings() {
        let raw = "   The Final Answer is $1,250.50%!!   ";
        let norm = GaiaNormalizer::normalize(raw);
        assert_eq!(norm, "the final answer is 1250.50");
    }

    #[test]
    fn test_position_swapped_judge_detects_positional_bias() {
        let biased_eval = |_first: &str, _second: &str| -> usize { 0 };
        let res = PositionSwappedJudge::judge_fairly("A", "B", biased_eval);
        assert!(res.is_err());

        let fair_eval = |first: &str, _second: &str| -> usize {
            if first == "A" { 0 } else { 1 }
        };
        let fair_res = PositionSwappedJudge::judge_fairly("A", "B", fair_eval);
        assert_eq!(fair_res, Ok("Candidate_A"));
    }

    #[test]
    fn test_goodhart_divergence_detector_alerts() {
        assert!(GoodhartDivergenceDetector::is_diverging(50.0, 120.0, 0.9, 0.7));
        assert!(!GoodhartDivergenceDetector::is_diverging(50.0, 120.0, 0.7, 0.9));
    }

    #[test]
    fn test_trace_failure_miner_clusters_patterns() {
        let mut miner = TraceFailureMiner::new();
        miner.mine_trace("TIMEOUT_ERROR");
        miner.mine_trace("SYNTAX_ERROR");
        miner.mine_trace("TIMEOUT_ERROR");
        miner.mine_trace("TIMEOUT_ERROR");

        let top = miner.top_failure_category();
        assert_eq!(top, Some(("TIMEOUT_ERROR", 3)));
    }
}
```

---

## 5. Bảng tra cứu lỗi biên dịch thường gặp

Khi triển khai các mẫu điều phối đa tác tử, cổng kỷ luật và rào chắn diễn tập bằng Rust 2024 thuần `std`, bạn sẽ gặp những lỗi biên dịch đặc thù. Bảng dưới đây tổng hợp các lỗi phổ biến nhất cùng giải pháp xử lý triệt để:

| Mã lỗi / Cảnh báo | Nguyên nhân kỹ thuật trong Chương 90 | Cách khắc phục chuẩn mực Rust 2024 |
|---|---|---|
| `error[E0594]: cannot assign to captured variable in Fn closure` | Trong `GoalLoopController::evaluate_step`, closure giám khảo cần thay đổi biến cờ bên ngoài (`tier2_called = true`), nhưng hàm chỉ nhận `F: Fn()` vốn là mượn bất biến. | Đổi ràng buộc generic từ `Fn() -> T` sang `FnMut() -> T` và khai báo tham số dạng `mut tier1_judge: F1, mut tier2_judge: F2`. |
| `clippy::manual_is_multiple_of` | Sử dụng phép toán thủ công `val % step == 0` để kiểm tra bội số trong bộ phân tích Cron. | Trong Rust 2024, số nguyên nguyên thủy (`u8`, `u32`, `usize`) có sẵn phương thức chuẩn `val.is_multiple_of(step)`. |
| `clippy::chunks_exact_to_as_chunks` | Trong thuật toán SHA-256, gọi `m.chunks_exact(64)` với kích thước hằng số cố định khi duyệt vector byte. | Thay bằng `let (blocks, _) = m.as_chunks::<64>(); for block in blocks` giúp trình biên dịch tối ưu hóa SIMD và kiểm tra độ dài lúc biên dịch. |
| `clippy::collapsible_if` | Viết hai khối `if let` lồng nhau: `if let Some(x) = ... { if let Ok(y) = ... { } }`. | Tận dụng tính năng **Let Chains** chính thức của Rust 2024: gộp thành `if let Some(x) = ... && let Ok(y) = ... { }`. |
| `error: out of range hex escape` | Sử dụng ký tự thoát hex `\x00\xFF\xAA\x55` trong chuỗi ký tự UTF-8 `&str`. Trong Rust, `\x` chỉ áp dụng cho dải 7-bit ASCII (`0x00`–`0x7F`). | Dùng chuỗi byte thô `b"..."` hoặc sử dụng mã thoát Unicode rõ ràng `\u{00}\u{FF}\u{AA}\u{55}`. |
| `error[E0507]: cannot move out of *self which is behind a mutable reference` | Trong `WorktreeSandbox::acquire_lease`, cố gắng di chuyển giá trị của khóa cũ ra khỏi `Vec` khi thay thế bằng khóa mới. | Thay thế tại chỗ bằng chỉ mục mảng `self.active_leases[pos] = new_lease;` thay vì di chuyển phần tử. |
| `error[E0382]: use of moved value: msg` | Gửi tin nhắn `msg` vào hàng đợi `TopicEventBus` sau đó lại dùng tiếp trong log hoặc kiểm tra tính hợp lệ. | Cài đặt `#[derive(Clone)]` cho `AgentMessage` hoặc thực hiện tham chiếu mượn `&msg` trước khi đưa vào hàm chuyển tiếp. |
| `Bẫy Vacuous Truth trong LoopDrill (Lỗi logic lúc chạy)` | Bộ test chỉ có mẫu lành tính nhưng hàm trả về `Sensitivity = 1.0` do `tp + fn_c == 0`. | Cưỡng chế điều kiện `tp + fn_c > 0 && tn + fp > 0`. Nếu vi phạm, trả về `DrillStatus::RejectedVacuousSuite`. |

---

## 6. Tóm tắt & Bài tập rèn luyện

### 5 điểm cốt lõi cần ghi nhớ

1. **Kỷ luật Bàn thợ là nền tảng của tự hành**: Tác tử tự do là tác tử nguy hiểm. Sổ cái đơn tính năng `FeatureLedger` bảo vệ sự tập trung; Cổng chống mừng hụt `verify_completion_claim` đòi hỏi bằng chứng băm kiểm thử thực tế; và `LayerImportLinter` bảo vệ ranh giới kiến trúc sạch.
2. **Hình sao trung gian bảo toàn toàn vẹn dữ liệu**: `MediatorOrchestrator` chuyển tiếp nguyên văn 100% byte thông điệp, triệt tiêu sự bùng nổ kênh giao tiếp phân tán và lưu vết tuần tự trên `TopicEventBus` qua `CursorOffset`.
3. **Hiến pháp tam quyền phân lập chống tự phê duyệt**: Ba vai trò `author`, `verifier`, và `attestor` bắt buộc phải thuộc về các thực thể riêng biệt. Bất kỳ nỗ lực tự phê duyệt nào đều bị Hiến pháp từ chối cấp phép.
4. **Bánh cóc RSI bảo vệ khả năng khái quát hóa**: Tự tiến hóa không được đánh đổi bằng sự thoái hóa. Điểm số trên tập ẩn `held_out` là ranh giới thép không bao giờ được phép suy giảm.
5. **LoopDrill hai chiều vạch trần chân lý rỗng**: Một rào chắn chỉ thực sự an toàn khi nó vừa phát hiện được kẻ thù (Sensitivity), vừa không quấy rối người dân lương thiện (Specificity). Bộ dữ liệu diễn tập bắt buộc phải có cả mẫu dương lẫn mẫu âm.

---

### Bài tập rèn luyện thực chiến

#### Bài tập 1: Mở rộng `WorktreeSandbox` hỗ trợ khóa đường dẫn glob lồng nhau và tự phát hiện Deadlock
Trong môi trường nhiều tác tử, Tác tử 1 có thể đang giữ khóa `src/domain/*` và yêu cầu khóa `src/infra/*`, trong khi Tác tử 2 đang giữ khóa `src/infra/*` và chờ khóa `src/domain/*`. 
**Yêu cầu**: Hãy nâng cấp cấu trúc `WorktreeSandbox` để:
1. Hỗ trợ khớp đường dẫn glob đa cấp (ví dụ `src/**/*.rs`).
2. Xây dựng đồ thị chờ đợi tài nguyên (Wait-For Graph) và phát hiện chu trình bế tắc (Deadlock Detection) dựa trên thuật toán tìm chu trình DFS. Nếu phát hiện deadlock, tự động thu hồi khóa của tác tử có độ ưu tiên thấp hơn.

<details>
<summary>Gợi ý & Lời giải mẫu Bài tập 1</summary>

```rust
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockRequest {
    pub holder: String,
    pub resource: String,
    pub priority: usize,
}

#[derive(Debug, Default)]
pub struct DeadlockDetector {
    // holder -> danh sách các holder khác mà holder này đang phải chờ đợi
    pub wait_graph: BTreeMap<String, BTreeSet<String>>,
}

impl DeadlockDetector {
    pub fn new() -> Self {
        Self { wait_graph: BTreeMap::new() }
    }

    pub fn add_wait_edge(&mut self, waiter: &str, blocker: &str) {
        self.wait_graph
            .entry(waiter.to_string())
            .or_default()
            .insert(blocker.to_string());
    }

    pub fn remove_holder(&mut self, holder: &str) {
        self.wait_graph.remove(holder);
        for edges in self.wait_graph.values_mut() {
            edges.remove(holder);
        }
    }

    pub fn has_deadlock(&self) -> bool {
        let mut visited = BTreeSet::new();
        let mut rec_stack = BTreeSet::new();

        for node in self.wait_graph.keys() {
            if !visited.contains(node)
                && self.dfs_cycle(node, &mut visited, &mut rec_stack)
            {
                return true;
            }
        }
        false
    }

    fn dfs_cycle<'a>(
        &'a self,
        node: &'a str,
        visited: &mut BTreeSet<&'a str>,
        rec_stack: &mut BTreeSet<&'a str>,
    ) -> bool {
        visited.insert(node);
        rec_stack.insert(node);

        if let Some(neighbors) = self.wait_graph.get(node) {
            for neighbor in neighbors {
                let n_str = neighbor.as_str();
                if !visited.contains(n_str) {
                    if self.dfs_cycle(n_str, visited, rec_stack) {
                        return true;
                    }
                } else if rec_stack.contains(n_str) {
                    return true;
                }
            }
        }

        rec_stack.remove(node);
        false
    }
}
```
</details>

---

#### Bài tập 2: Thuật toán Borda Count cho Danh bạ đánh giá mù (`BlindPortfolioRegistry`)
Khi đánh giá từ 3 ứng viên trở lên (`CANDIDATE_A`, `CANDIDATE_B`, `CANDIDATE_C`), phương pháp so sánh cặp (pairwise) có thể gặp phải Nghịch lý Condorcet (A thắng B, B thắng C, C thắng A).
**Yêu cầu**: Cài đặt phương pháp bỏ phiếu Borda Count có trọng số trong `BlindPortfolioRegistry`. Mỗi giám khảo xếp hạng các ứng viên từ tốt nhất đến tệ nhất. Ứng viên đứng đầu nhận $N - 1$ điểm, ứng viên thứ hai nhận $N - 2$ điểm,..., ứng viên cuối cùng nhận 0 điểm. Tổng hợp điểm số và công bố ứng viên chiến thắng sau khi giải mã danh tính.

<details>
<summary>Gợi ý & Lời giải mẫu Bài tập 2</summary>

```rust
use std::collections::BTreeMap;

pub struct BordaVotingSystem {
    pub candidate_scores: BTreeMap<String, usize>,
}

impl BordaVotingSystem {
    pub fn new() -> Self {
        Self { candidate_scores: BTreeMap::new() }
    }

    /// Tiếp nhận một lá phiếu xếp hạng: vị trí 0 là ứng viên tốt nhất
    pub fn cast_ballot(&mut self, ranking: &[&str]) {
        let n = ranking.len();
        for (pos, &cand) in ranking.iter().enumerate() {
            let points = n.saturating_sub(pos + 1);
            *self.candidate_scores.entry(cand.to_string()).or_insert(0) += points;
        }
    }

    pub fn determine_winner(&self) -> Option<(&str, usize)> {
        self.candidate_scores
            .iter()
            .max_by_key(|&(_, score)| *score)
            .map(|(c, s)| (c.as_str(), *s))
    }
}
```
</details>

---

#### Bài tập 3: Nâng cấp `LoopDrillRunner` tính khoảng tin cậy Wilson Score 95%
Độ nhạy và độ đặc hiệu đo được trên tập mẫu nhỏ có phương sai lớn. Ví dụ: Đạt 2/2 mẫu (100%) có ý nghĩa thống kê kém xa so với đạt 1000/1000 mẫu (100%).
**Yêu cầu**: Nâng cấp kết quả `DrillReceipt` để tính toán khoảng tin cậy Wilson Score 95% cho cả Sensitivity và Specificity thuần `std`, sử dụng công thức gần đúng chuẩn $z = 1.96$:
$$w = \frac{p + \frac{z^2}{2n} \pm z \sqrt{\frac{p(1 - p)}{n} + \frac{z^2}{4n^2}}}{1 + \frac{z^2}{n}}$$

<details>
<summary>Gợi ý & Lời giải mẫu Bài tập 3</summary>

```rust
#[derive(Debug, Clone, PartialEq)]
pub struct ConfidenceInterval {
    pub point_estimate: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

pub fn wilson_score_interval_95(successes: usize, total: usize) -> ConfidenceInterval {
    if total == 0 {
        return ConfidenceInterval { point_estimate: 0.0, lower_bound: 0.0, upper_bound: 0.0 };
    }

    let p = (successes as f64) / (total as f64);
    let n = total as f64;
    let z: f64 = 1.95996; // Phân vị 97.5% của phân phối chuẩn tắc (cho khoảng 95%)
    let z2 = z * z;

    let denominator = 1.0 + z2 / n;
    let center = (p + z2 / (2.0 * n)) / denominator;
    let margin = (z / denominator) * ((p * (1.0 - p) / n) + (z2 / (4.0 * n * n))).sqrt();

    ConfidenceInterval {
        point_estimate: p,
        lower_bound: (center - margin).max(0.0),
        upper_bound: (center + margin).min(1.0),
    }
}
```
</details>
