# Chương 109: Agentic OSINT & Evidence Graph — ReAct Harness, Sáu Nguồn Passive Giả, Provenance & Báo Cáo Tái Lập

> **Phạm vi hợp pháp và giới hạn bắt buộc:** chương này phục vụ học tập phòng thủ và điều tra bề mặt tấn công trong phạm vi được ủy quyền bằng kỹ nghệ tác tử. `MockOSINTAgent` chỉ chạy trên dữ liệu tổng hợp trong bộ nhớ: mọi tên miền đều dùng TLD dành riêng `.example` (RFC 2606), mọi địa chỉ IP đều thuộc dải riêng tư RFC 1918 (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`) và mọi địa chỉ liên hệ đều là role mailbox tổ chức (`security@corp.example`). Crate **không** mở socket, **không** gọi API WHOIS/DNS/CT/GitHub/Wayback thật, **không** thu thập hoặc suy diễn dữ liệu cá nhân (PII), **không** thực hiện active probing và **không** lưu bí mật thật.

## 1. Từ ban biên tập điều tra tới đồ thị bằng chứng có nguồn

Hãy tưởng tượng một ban biên tập kiểm chứng tin tức điều tra một công ty giả định. Họ có sáu kho lưu trữ công khai độc lập: sổ đăng ký kinh doanh, bản đồ tên miền, sổ nhật ký chứng chỉ, kho mã nguồn công khai, hồ sơ truyền thông chính thức của tổ chức và kho lưu trữ web cũ từ năm trước.

Nếu giao việc này cho một thực tập sinh vội vàng không có kỷ luật:

1. Thực tập sinh có thể gọi điện thẳng vào phòng máy chủ nội bộ thay vì chỉ đọc hồ sơ công khai (vượt ranh giới passive sang active).
2. Thấy hai trang ghi khác nhau — sổ hiện tại ghi văn phòng đã đóng, kho lưu trữ năm ngoái ghi vẫn đang mở — thực tập sinh tự ý xóa dòng cũ đi cho "đẹp báo cáo".
3. Mất điện giữa chừng khi đọc tới kho thứ tư, khởi động lại rồi đọc lại từ đầu và tính gộp thành hai công ty trùng nhau.
4. Viết một câu kết luận ấn tượng trong báo cáo mà không đính kèm trích dẫn hồ sơ nào.

Một **tác tử OSINT an toàn (Agentic OSINT)** phải vận hành như một ban biên tập có sổ kiểm toán: mọi câu hỏi đều có phạm vi ủy quyền, mọi quan sát đều truy ngược tới nguồn gốc, mọi mâu thuẫn được giữ nguyên thay vì tẩy xóa, dừng giữa chừng có thể tiếp tục từ điểm kiểm soát, và bất kỳ phát biểu nào thiếu bằng chứng đều bị từ chối trước khi xuất bản.

```text
Seed Scope (.example + passive-only)
   │
   ▼
ReAct Loop: Observe ─► Decide ─► ToolRegistry (6 nguồn mock) ─► Validate
   │                                                               │
   ├──► Hết budget (max_steps / max_tool_calls) ──► Halt an toàn   │
   ├──► Lỗi tạm thời (TimeoutOnce) ───────────────► Checkpoint     │
   │                                                               ▼
   └──────────────────────────────────────────────► EvidenceGraph (Supports / Contradicts / SameAs)
                                                          │
                                                          ▼
                                                    OsintReport (Fact / Inference / Conflict / Unknown)
```

### Mục tiêu học tập

Sau chương này, bạn có thể:

- Phân biệt ranh giới pháp lý và kỹ thuật giữa thu thập thụ động (passive), active probing và việc giảm thiểu dữ liệu riêng tư (privacy minimization).
- Thiết kế vòng lặp ReAct có kiểm soát ngân sách (`max_steps`, `max_tool_calls`), phát hiện chu trình theo định danh chuẩn hóa và điểm dừng tường minh.
- Xây dựng `ToolRegistry` định kiểu với allowlist khả năng, kiểm tra schema đầu vào/đầu ra và mô phỏng lỗi tạm thời để kiểm thử khả năng phục hồi.
- Mô hình hóa sáu nguồn thụ động giả (`Whois`, `Dns`, `CtLogs`, `GitHub`, `SocialMetadata`, `Wayback`) thành chuỗi `Observation → Evidence → Entity`.
- Xử lý hợp nhất thực thể qua cạnh `SameAs` mà không nhân đôi bằng chứng, đồng thời bảo toàn mâu thuẫn qua cạnh `Contradicts` và trạng thái `Conflicted`.
- Lưu và phục hồi trạng thái theo từng `Stage` bằng `Checkpoint` gắn `input_hash` SHA-256, đảm bảo đầu ra JSON/Markdown trùng khớp từng byte sau khi khôi phục.
- Phân tách rõ `Fact`, `Inference`, `Conflict` và `Unknown` trong `OsintReport`, tự động chặn `UnsupportedClaim` và `ConflictedFactClaim`.

Tiên quyết nên đọc trước: Chương 25–30 cho cấu trúc dữ liệu/đồ thị, Chương 65 cho DNS/mạng nền tảng và Chương 86–91 cho kỹ nghệ harness tác tử (đặc biệt StateGraph, HITL, Checkpoint/WAL và đánh giá trích dẫn). Crate `ch109` dùng Rust 2024 edition, thuần `std`, `#![forbid(unsafe_code)]`, không có dependency ngoài.

---

## 2. Phạm vi, ranh giới thụ động và giảm thiểu dữ liệu riêng tư

### 2.1 Bốn hàng rào trước khi gọi bất kỳ công cụ nào

Trong kỹ nghệ OSINT có ủy quyền, công cụ mạnh không có nghĩa là được phép truy vấn tùy tiện. `OsintScope` đóng vai trò cổng kiểm tra bắt buộc cho từng `ToolRequest`:

| Rào kiểm tra | Quy tắc trong `ch109` | Lỗi trả về nếu vi phạm |
|---|---|---|
| Định danh đợt kiểm tra | `request.engagement_id == scope.engagement_id` | `OsintError::OutOfScope` |
| Ranh giới thụ động | `request.mode == CollectionMode::Passive` (cấm tuyệt đối `ActiveProbe`) | `OsintError::ActiveProbeProhibited` |
| Phạm vi tên miền & org | Tên miền chuẩn hóa kết thúc `.example` và thuộc `root_domains`; org thuộc `allowed_org_handles` | `OsintError::OutOfScope` / `InvalidDomain` |
| Nguồn được phép | `request.source` nằm trong cả `scope.allowed_sources` và `ToolRegistry` | `OsintError::ToolNotAllowed` |

Ngay cả khi một caller cố sửa cờ `scope.allow_active_probing = true`, hàm `scope.validate()` vẫn từ chối với `ActiveProbeProhibited`. Đây là phòng vệ theo thiết kế (defense-in-depth): mô hình không chứa đường thực thi cho active probing.

### 2.2 Chuẩn hóa định danh, RFC 1918 và cấm PII/Secret thô

Nhiều lỗ hổng trong tác tử thu thập thông tin đến từ việc so sánh chuỗi thô chưa chuẩn hóa hoặc vô tình lưu dữ liệu nhạy cảm:

1. **Tên miền (`normalize_example_domain`)**: chuyển ASCII về chữ thường, cắt tối đa một dấu chấm `.` ở cuối FQDN (`API.CORP.EXAMPLE.` thành `api.corp.example`), từ chối khoảng trắng, chuỗi rỗng, nhãn bắt đầu/kết thúc bằng `-` và từ chối mọi TLD ngoài `.example`.
2. **Địa chỉ IP (`is_rfc1918_ipv4`)**: kiểm tra chính xác bốn octet thập phân (không nhận số 0 đứng đầu kiểu octal như `010.0.0.1`), chỉ chấp nhận `10.0.0.0/8`, `172.16.0.0/12` (`172.16..=172.31`) và `192.168.0.0/16`. Các địa chỉ ranh giới như `172.15.255.255`, `172.32.0.1`, `127.0.0.1` hay `8.8.8.8` đều bị chặn bằng `InvalidRfc1918Ip`.
3. **Giảm thiểu dữ liệu cá nhân (`PersonalDataProhibited`)**: từ chối toàn bộ thuộc tính liên quan tới con người (`person_name`, `full_name`, `phone`, `ssn`, `home_address`, `personal_email`, `employee_name`) và mọi email không nằm trong danh sách role mailbox tổ chức (`security@`, `noc@`, `hostmaster@`, `cert-admin@`, `ops@`, `abuse@`, `postmaster@`) thuộc miền trong scope. Ví dụ `alice.nguyen@corp.example` bị chặn ngay tại lớp kiểm tra bản ghi.
4. **Kiểm soát secret (`RawSecretProhibited`)**: mọi thuộc tính chứa tên nhạy cảm (`secret`, `token`, `api_key`, `password`, `private_key`, `credential`) bắt buộc mang giá trị `SYNTHETIC-REDACTED-NOT-A-SECRET`. Hệ thống cũng quét tiền tố token phổ biến (như khóa API hay header xác thực) trên toàn bộ bản ghi và báo cáo.

---

## 3. Vòng lặp ReAct có phanh và đối chiếu kiến trúc phòng sạch

### 3.1 Sáu bước trong mỗi vòng lặp và cơ chế phanh kép

Vòng lặp ReAct (Reasoning and Acting) trong tác tử tự hành dễ gặp hai lỗi kinh điển: lặp vô hạn khi hai nguồn trỏ qua lại lẫn nhau, hoặc đốt sạch hạn ngạch lời gọi khi mở rộng đồ thị quá sâu. `MockOSINTAgent` đặt ba chốt chặn cứng:

- **Chuẩn hóa khóa vòng lặp (`visited_calls`)**: lưu cặp `(SourceKind, canonical_target)` đã chuẩn hóa. Nếu một nguồn cố đưa lại `API.CORP.EXAMPLE.` sau khi `api.corp.example` đã xử lý, tác tử nhận diện trùng khóa và ghi `AgentAction::SkipVisited` thay vì gọi lại tool.
- **Ngân sách kép (`max_steps`, `max_tool_calls`)**: tách riêng số bước suy luận/kiểm soát (`steps_used`) và số lần thực thi công cụ (`tool_calls_used`). Khi chạm một trong hai ngưỡng trước khi gọi tool tiếp theo, tác tử chuyển ngay sang `StopReason::BudgetExhausted`, ghi nhận các mục tiêu còn dở trong `unknowns` và xuất báo cáo hợp lệ với dữ liệu đã thu được.
- **Chặn target khám phá ngoài scope (`enqueue_discovered`)**: các target do một công cụ phát hiện (ví dụ subdomain trong CT logs hay org handle trong WHOIS) vẫn phải đi qua `canonicalize_target` theo `OsintScope`; mọi tên miền ngoài phạm vi bị bỏ qua ngay ở hàng đợi mở rộng.

### 3.2 Tham khảo kiến trúc clean-room

Thiết kế của `ch109` tiếp thu nguyên lý kiến trúc từ bốn dự án OSINT nguồn mở theo hướng phòng sạch (viết mới hoàn toàn bằng Rust 2024 thuần `std`, không sao chép mã nguồn):

| Dự án tham khảo | Ý tưởng kiến trúc cốt lõi | Áp dụng trong `code/ch109` |
|---|---|---|
| **SYNINT** | Tác tử local-first và đường ống nhiều chặng lưu checkpoint khôi phục được | Mô hình 6 `Stage` tuần tự + `Checkpoint` gắn `input_hash` SHA-256 |
| **SpiderFoot (fork)** | Sổ đăng bạ module định kiểu theo loại dữ liệu vào/ra | Trait `OsintTool` + `ToolRegistry` kiểm tra `ToolTargetKind` và allowlist |
| **argus-os** | Điều phối chuỗi công cụ mở rộng có kiểm soát từ seed ban đầu | `discovered_targets` quay về hàng đợi `pending_targets` sau khi qua `OsintScope` |
| **agentic-osint-agent** | Kết hợp vòng lặp tác tử với đồ thị bằng chứng và báo cáo có trích dẫn | `EvidenceGraph` tách `Fact` / `Inference` / `Conflict` / `Unknown` và `OsintReport::validate` |

---

## 4. ToolRegistry định kiểu và sáu nguồn thụ động giả

### 4.1 Hợp đồng `OsintTool` và kiểm tra schema

Thay vì truyền chuỗi lệnh tự do vào shell hay phân tích HTML không định kiểu, mỗi nguồn là một kiểu triển khai trait `OsintTool`:

- `source_kind() -> SourceKind`: định danh nguồn duy nhất.
- `expected_target_kind() -> ToolTargetKind`: phân tách rạch ròi `Domain` với `OrgHandle` để tránh truyền nhầm tên miền vào tool chỉ nhận handle tổ chức (`SchemaMismatch`).
- `query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError>`: trả về tập bản ghi đã chuẩn bị sẵn của fixture.

Ngay sau `tool.query(...)`, `ToolRegistry::execute` kiểm chứng từng `RawFindingRecord`: không rỗng, timestamp đúng chuẩn ISO 8601 UTC (`YYYY-MM-DDTHH:MM:SSZ`), `weight_milli <= 1000`, hợp lệ theo RFC 1918/RFC 2606/PII/secret, rồi mới đóng gói thành `ToolResult` kèm `raw_digest` SHA-256. Registry đồng thời theo dõi `invocation_count(source, target)` và cho phép tiêm ba loại lỗi giả qua `SimulatedFault`:

- `TimeoutOnce`: lỗi tạm thời tự biến mất sau một lần kích hoạt, dùng để kiểm chứng `Checkpoint`/`resume`.
- `SourceUnavailable`: lỗi bền bỉ cho tới khi `clear_fault` được gọi.
- `MalformedPayload`: giả lập nguồn trả dữ liệu hỏng để kiểm tra xử lý lỗi.

### 4.2 Đặc tả sáu nguồn thụ động trong `ToolRegistry::standard()`

| Nguồn (`SourceKind`) | `ToolTargetKind` | Vai trò trong fixture `corp.example` | Dữ liệu kiểm chứng trọng điểm |
|---|---|---|---|
| `Whois` | `Domain` | Đăng ký tên miền gốc & tổ chức | Tạo `org:corp-example`, `role:security@corp.example`, phát hiện handle `corp-example-labs` |
| `Dns` | `Domain` | Bản ghi A/MX/CNAME (IP RFC 1918) tại `2026-01-01T00:00:00Z` | `api.corp.example` và `portal.corp.example` cùng về `10.20.30.15`; `staging-old.corp.example` ghi `10.20.30.20` (`decommissioned`) |
| `CtLogs` | `Domain` | Sổ nhật ký chứng chỉ tổng hợp (SAN & trạng thái) | Phát hiện `api`, `portal`, `staging-old`; chứng chỉ `staging-old.corp.example-2025` đang `expired-unrevoked` |
| `GitHub` | `OrgHandle` | Tổ chức & kho mã công khai `corp-example-labs` | Đánh dấu `alias_of("org:corp-example")` cho `SameAs`; `config_secret` mang nhãn `SYNTHETIC-REDACTED-NOT-A-SECRET` |
| `SocialMetadata` | `OrgHandle` | Hồ sơ tổ chức công khai (không có dữ liệu cá nhân) | Xác nhận chéo `official_domain = corp.example` giúp tăng `Confidence` đa nguồn |
| `Wayback` | `Domain` | Ảnh chụp lịch sử tại `2025-06-15T00:00:00Z` | `staging-old.corp.example` ghi `a_record = 10.20.30.99`, `deployment_state = active` — tạo mâu thuẫn có chủ đích với `Dns` |

---

## 5. Phân giải thực thể: chuẩn hóa, hợp nhất `SameAs` và bảo toàn mâu thuẫn

### 5.1 Quy tắc hợp nhất không mất bằng chứng

Trong thực tế, cùng một tổ chức thường xuất hiện dưới nhiều tên gọi khác nhau: hồ sơ đăng ký ghi `org:corp-example`, còn kho mã nguồn ghi `org:corp-example-labs`. `MockGitHubTool` gắn `alias_of = Some("org:corp-example")`, làm `EvidenceGraph::ingest_result` sinh cạnh `EdgeKind::SameAs`.

Khi chạy `merge_same_as_entities()`:

1. Kiểm tra hai thực thể phải có **cùng `EntityKind`**. Cố gộp một `Domain` vào `Organization` bị chặn ngay bằng `OsintError::AliasMismatch`.
2. Di chuyển thuộc tính và `evidence_ids` từ thực thể phụ sang thực thể chính thông qua `BTreeSet` (không nhân đôi ID bằng chứng), lưu bí danh cũ vào `entity.aliases` và `merged_aliases`.
3. Giữ nguyên cạnh `SameAs` trong đồ thị làm dấu vết kiểm toán; mọi truy vấn `resolve_entity_id` sau này đều tự động quy về thực thể chính.
4. Nếu hai thực thể cùng tên thuộc tính nhưng mang giá trị khác nhau, hàm hợp nhất chuyển thuộc tính đó sang `Conflicted` và sinh cạnh `Contradicts` thay vì ghi đè.

### 5.2 Giữ nguyên mâu thuẫn giữa `Dns` (2026) và `Wayback` (2025)

```text
domain:staging-old.corp.example
  ├─ Dns     (2026-01-01T00:00:00Z): a_record=10.20.30.20, deployment_state=decommissioned
  └─ Wayback (2025-06-15T00:00:00Z): a_record=10.20.30.99, deployment_state=active
        │
        ▼
  ResolutionStatus::Conflicted
   • resolved_value = None
   • giữ nguyên cả 2 AttributeObservation + evidence_ids
   • sinh EdgeKind::Contradicts
   • Confidence giảm xuống mức thấp (<= 450 milli)
```

Một hệ thống ngây thơ thường giữ giá trị mới nhất hoặc giữ giá trị đầu tiên rồi vứt phần còn lại. Cách làm đó che mất dấu vết hạ tầng cũ (thay đổi IP từ `10.20.30.99` sang `10.20.30.20`) và có thể khiến nhóm phòng thủ bỏ sót dịch vụ chưa thu hồi hoàn toàn. Khi gặp ≥ 2 giá trị khác nhau cho cùng một thuộc tính, `AttributeResolution` chuyển `status` sang `Conflicted`, đặt `resolved_value = None` và ghi lý do chi tiết trong `Confidence.rationale`.

---

## 6. `EvidenceGraph` & `Confidence`: truy vết `Fact → Evidence → Observation → Source`

### 6.1 Biểu diễn `Confidence` tất định bằng phần nghìn (`u16`)

Thay vì lưu trực tiếp `f64` trong các cấu trúc cần so sánh bằng/sắp xếp (dễ gặp sai số biểu diễn dấu phẩy động hoặc `NaN`), `Confidence` lưu trường nội bộ `milli: u16` trong khoảng `0..=1000` kèm trường bắt buộc `rationale: String` không rỗng.

- `Confidence::new(score, rationale)` kiểm tra `score.is_finite()` và `(0.0..=1.0).contains(&score)`.
- Một nguồn đơn lẻ xác nhận giá trị cho độ tin cậy giới hạn tối đa `900` milli (`0.900`).
- Khi ≥ 2 `SourceKind` độc lập cùng đồng thuận một giá trị (ví dụ `Whois`, `GitHub` và `SocialMetadata` cùng trỏ `official_domain = corp.example`), điểm tin cậy tăng thêm phần thưởng đồng thuận nhưng luôn chặn trần tại `980` milli (`0.980`), không bao giờ tự gán `1.000` tuyệt đối cho dữ liệu OSINT.

### 6.2 Ba loại cạnh và bất biến khóa ngoại

`EvidenceGraph` kiểm chứng cấu trúc dữ liệu qua `graph.validate()`:

- Mọi `Evidence` phải trỏ tới `Observation` tồn tại và `SourceRecord` khớp `SourceKind`.
- Mọi `Observation` phải trỏ tới một thực thể hợp lệ (trực tiếp hoặc qua `merged_aliases`) và thực thể đó phải chứa `Evidence.id` tương ứng; không được tồn tại hai quan sát trùng bộ khóa `(source_id, resolved_entity_id, attribute, value, observed_at)`.
- Mọi `GraphEdge` (`Supports`, `Contradicts`, `SameAs`) phải có ít nhất một `evidence_id` tồn tại trong đồ thị và hai đầu mút đều hợp lệ.
- Mọi thuộc tính có nhiều hơn một giá trị ứng viên khác nhau bắt buộc ở trạng thái `Conflicted` và phải có cạnh `Contradicts` đi kèm.

---

## 7. Đường ống theo giai đoạn và khôi phục bằng `Checkpoint`

```text
Stage 1: ScopeValidation  ──► Stage 2: SeedCollection  ──► Stage 3: PassiveExpansion
                                                                   │
Stage 6: ReportGeneration ◄── Stage 5: GraphVerification ◄── Stage 4: EntityResolution
```

Mỗi `Checkpoint` lưu trạng thái ở ranh giới sạch ngay trước khi thực thi công cụ tiếp theo, bao gồm:

- `version`: bằng `CHECKPOINT_VERSION` (`1`).
- `engagement_id`: khóa phạm vi đợt kiểm tra.
- `input_hash`: băm SHA-256 của cấu hình `OsintScope`, `seed_targets`, `max_steps` và `max_tool_calls`.
- `stage_statuses`: bản đồ `BTreeMap<Stage, StageStatus>` (`Pending`, `InProgress`, `Completed`, `Halted`).
- `state`: `AgentState` lưu `seeds_remaining`, `pending_targets`, `visited_calls`, `steps_used`, `tool_calls_used`, `action_trace`.
- `completed_calls`: các `ToolResult` đã hoàn tất kèm `raw_digest` SHA-256.
- `graph`: bản sao `EvidenceGraph` tại thời điểm checkpoint.

Khi gọi `agent.resume(&checkpoint)`:

1. Xác thực `version`, `engagement_id`, `input_hash` khớp cấu hình hiện tại (nếu đổi scope hoặc budget giữa chừng, `resume` từ chối bằng `InvalidCheckpoint`).
2. Tính lại SHA-256 cho từng `ToolResult` trong `completed_calls`, kiểm tra lại từng bản ghi bằng `validate_raw_record` và gọi `checkpoint.graph.validate()`.
3. Bỏ qua mọi `(source, target)` đã nằm trong `completed_calls`: công cụ đã chạy thành công ở lần trước **không bao giờ** được gọi lại (`invocation_count` giữ nguyên) và không nạp lại vào đồ thị (`duplicate_ingest_skips == 0`).
4. Tiếp tục thực thi phần còn lại theo đúng thứ tự tất định, cho ra chuỗi `render_json()` và `render_markdown()` khớp từng byte với lượt chạy thông suốt từ đầu.

---

## 8. `OsintReport`: tách bạch `Fact`, `Inference`, `Conflict`, `Unknown` & chống ảo giác

Một báo cáo có trách nhiệm phân biệt rạch ròi bốn lớp thông tin (và gắn `generated_at = "2026-01-01T00:00:00Z"` cố định để tái lập từng byte):

| Mục báo cáo | Điều kiện được đưa vào | Kiểm tra bắt buộc trong `OsintReport::validate` |
|---|---|---|
| `facts` | Thuộc tính đơn trị đang `ResolutionStatus::Confirmed` | Phải có `evidence_ids` không rỗng, khớp đúng thuộc tính & giá trị trên `Entity`; **cấm** lấy từ `Conflicted` (`ConflictedFactClaim`) |
| `inferences` | Kết luận tổng hợp đa nguồn (VD: liên kết `org:corp-example` với `corp.example`, hoặc `portal` trỏ về `api`) | Bắt buộc có **≥ 2 `evidence_ids`** hợp lệ tồn tại trong đồ thị (`UnsupportedClaim`) |
| `conflicts` | Thuộc tính đang `ResolutionStatus::Conflicted` | Giữ đủ ứng viên hai phía, ≥ 2 `evidence_ids`, đối chiếu đồ thị đúng trạng thái `Conflicted` |
| `unknowns` | Thiếu dữ liệu hoặc nằm ngoài giới hạn passive | VD: `mail.corp.example` thiếu chứng chỉ trong CT logs, hoặc secret redacted chưa biết đã xoay vòng chưa |

Cả `render_json()` và `render_markdown()` đều xử lý thoát ký tự (escaping): dấu `"`, `\`, tab/newline và ký tự điều khiển `< 0x20` trong JSON; dấu `|`, `<`, `>`, `` ` `` và xuống dòng trong bảng Markdown để dữ liệu tổng hợp không thể phá vỡ cấu trúc báo cáo hay chèn thẻ HTML ngoài ý muốn.

---

## 9. Đánh giá định lượng & Kiểm soát phòng thủ bề mặt lộ lọt

### 9.1 Bộ chỉ số đánh giá (`EvaluationMetrics`)

`MockOSINTAgent::evaluate(&report)` đo lường chất lượng vận hành dựa trên golden fixture `corp.example`:

- `sources_covered`: số nguồn trong 6 `SourceKind` thực sự tham gia thu thập (`6/6`).
- `unsupported_claim_count`: luôn bằng `0` sau `OsintReport::validate`.
- `duplicate_entity_or_evidence_count`: số lần nạp trùng bị chặn; bằng `0` trên lượt chạy chuẩn hoặc sau `resume`.
- `completeness_milli` / `completeness()`: tỷ lệ bao phủ cả 6 nguồn, 6 thực thể golden và phát hiện được mâu thuẫn (`1.000`).
- `budget_exhausted`: đánh dấu rõ khi báo cáo đến từ phiên dừng sớm do giới hạn ngân sách.

### 9.2 Chuyển hóa phát hiện thành khắc phục cho Blue Team

Giá trị lớn nhất của OSINT thụ động trong một tổ chức là nhìn lại bề mặt lộ lọt của chính mình trước khi kẻ tấn công khai thác:

| Phát hiện trong fixture | Rủi ro vận hành ngoài đời | Biện pháp phòng thủ & kiểm tra lại (`DefensiveRemediation`) |
|---|---|---|
| `cert:staging-old.corp.example-2025` có `cert_status = expired-unrevoked` | Chứng chỉ hết hạn/cũ vẫn nằm trong CT logs công khai làm lộ tên dịch vụ nội bộ hoặc quy ước đặt tên staging | Thu hồi chứng chỉ nếu khóa có nguy cơ lộ, rà soát vòng đời cấp phát tự động (ACME) và cập nhật danh mục tài sản |
| `staging-old.corp.example` mâu thuẫn `Dns` (`decommissioned`, `10.20.30.20`) vs `Wayback` (`active`, `10.20.30.99`) | Bản ghi DNS cũ trỏ vào hạ tầng cũ/không còn kiểm soát dẫn tới nguy cơ subdomain takeover hoặc nhầm lẫn phạm vi | Đối chiếu sổ quản lý tài sản nội bộ được ủy quyền; xóa bản ghi A/CNAME treo nếu dịch vụ đã ngừng |
| `repo:corp-example-labs/public-sdk` chứa `config_secret = SYNTHETIC-REDACTED-NOT-A-SECRET` | Token hoặc khóa cấu hình từng xuất hiện trong lịch sử commit công khai không thể tự biến mất chỉ bằng xóa dòng mới nhất | Coi là chưa an toàn cho tới khi xoay vòng/thu hồi secret trong kho nội bộ, quét toàn bộ lịch sử git và bật pre-commit secret scanning |
| `mail.corp.example` thiếu chứng chỉ trên CT logs | Thiếu khả năng quan sát trạng thái mã hóa kênh hoặc dịch vụ chưa thống nhất chuẩn TLS | Kiểm tra nội bộ cấu hình STARTTLS/TLS trên hạ tầng được ủy quyền (không dùng active probe từ tác tử thụ động) |

---

## 10. Crate, demo và bảng lỗi biên dịch thường gặp

### 10.1 Public API và bất biến

Mã nguồn đầy đủ nằm tại [`code/ch109/src/main.rs`](code/ch109/src/main.rs), manifest tại [`code/ch109/Cargo.toml`](code/ch109/Cargo.toml). Các định danh trung tâm:

| API | Vai trò |
|---|---|
| `SourceKind`, `OsintTool`, `ToolRequest`, `ToolResult`, `ToolRegistry` | Giao thức định kiểu cho 6 nguồn passive, kiểm tra `ToolTargetKind`, allowlist và `SimulatedFault` |
| `Entity`, `EntityId`, `Observation`, `Evidence`, `EvidenceGraph`, `Confidence` | Đồ thị bằng chứng truy vết nguồn gốc, hợp nhất `SameAs`, bảo toàn `Conflicted`/`Contradicts` |
| `AgentState`, `AgentAction`, `Stage`, `Checkpoint`, `MockOSINTAgent` | Vòng lặp ReAct có phanh ngân sách kép, phát hiện chu trình chuẩn hóa và `resume` tất định |
| `OsintReport`, `EvaluationMetrics` | Tách `Fact` / `Inference` / `Conflict` / `Unknown` / `DefensiveRemediation`, chống `UnsupportedClaim`, xuất Markdown/JSON |

Mọi tập hợp và ánh xạ đều dùng `BTreeMap` / `BTreeSet`; thời gian sử dụng hằng số `FIXED_TIMESTAMP` (`2026-01-01T00:00:00Z`) và `ARCHIVE_TIMESTAMP` (`2025-06-15T00:00:00Z`). Hai lần chạy độc lập hoặc một lần chạy bị ngắt `TimeoutOnce` rồi `resume` đều cho kết quả giống nhau từng byte.

### 10.2 Chạy demo và kiểm thử

```bash
cd code
cargo run -p ch109 --locked
cargo test -p ch109 --locked
cargo clippy -p ch109 --all-targets --locked -- -D warnings
```

Output thực tế của demo:

```text
MockOSINTAgent — 100% synthetic, passive-only, std-only, không network/API/PII
Active probe guard: blocked(ActiveProbeProhibited)
Sources covered: 6/6; entities: 16; edges: 22
Merged alias (SameAs): org:corp-example-labs -> org:corp-example (aliases=1)
Conflicts preserved: 2; facts: 28; inferences: 2; unknowns: 4
Checkpoint/resume byte-identical: true; duplicate_count=0
Budget guard: stop_reason=BudgetExhausted
Evaluation: completeness=1.000; unsupported_claims=0; duplicates=0
```

Crate hiện có **51 unit test** bao phủ: vector SHA-256 (`"abc"` và chuỗi rỗng), chuẩn hóa tên miền `.example`/FQDN và org handle, ranh giới IPv4 RFC 1918, kiểm tra định dạng timestamp ISO 8601 UTC, chặn `ActiveProbe`/ngoài scope/sai engagement/disallowed source/unregistered tool/schema mismatch, lọc PII và secret thô (cả chữ thường), nạp kết quả nguyên tử (`ingest_result`), bất biến `Confidence`, toàn bộ 6 nguồn mock và đồng thuận đa nguồn, gộp `SameAs` lũy đẳng/khác kiểu/khử trùng quan sát cùng nguồn giữa các bí danh/giữ mâu thuẫn/xử lý bí danh trỏ tới thực thể chưa thu thập (`unresolved_alias_target`), phát hiện chu trình chuẩn hóa (`SkipVisited`), cạn ngân sách `max_steps`/`max_tool_calls`/ngân sách 0, `Checkpoint`/`resume` (cùng instance, sang instance mới, chống sửa bẩn bộ đếm/giai đoạn/hàng đợi/seed/đồ thị/nguồn bị cấm), `OsintReport::validate` (thiếu bằng chứng, bằng chứng giả, nâng khống mâu thuẫn kể cả ở inference, giá trị inference sai lệch, bỏ sót/sửa/trùng bản ghi `ConflictRecord`, sửa trùng claim ID, remediation thiếu evidence), JSON/Markdown escaping (kể cả ký tự `\`) và replay tất định.

### 10.3 Bảng tra cứu compiler error

| Lỗi | Ví dụ nguyên nhân trong bài tập | Cách sửa |
|---|---|---|
| `E0053` — method signature mismatch | Triển khai `OsintTool::query(&self, ...)` thiếu `mut` | Đổi chữ ký thành `fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError>` |
| `E0308` — mismatched types | Truyền `&str` vào `ToolResult::new` nơi hàm nhận `Vec<RawFindingRecord>`, hoặc quên `EntityId(...)` | Bọc chuỗi trong `EntityId("...".to_string())` và truyền `vec![...]` đúng kiểu |
| `E0502` — cannot borrow as mutable | Đang giữ `let g = agent.graph();` rồi gọi `agent.run()` cần `&mut agent` | Đọc `agent.graph()` trong phạm vi hẹp trước hoặc sau khi `run()`/`resume()` xong |
| `E0382` — borrow of moved value | Dùng lại `checkpoint` sau khi truyền theo giá trị, hoặc dùng `agent` sau move | `resume(&cp)` nhận tham chiếu `&Checkpoint`; truyền `&cp` thay vì move |
| `E0599` — no method named `len` | Gọi `confidence.value` như trường hoặc gọi `agent.graph.entities()` thiếu `()` | Dùng method `confidence.value()` / `confidence.milli()` và `agent.graph().entities()` |
| `E0603` — private field access | Đọc trực tiếp `graph.entities` hay `confidence.milli` từ module ngoài | Gọi accessor công khai `graph.entities()`, `graph.edges()`, `confidence.milli()` |
| `E0277` — `?` conversion error | Hàm bài tập trả `Result<(), OsintError>` nhưng lại dùng `.expect` hoặc lỗi khác | Đặt kiểu trả về `Result<(), Box<dyn std::error::Error>>` cho hàm bài tập |
| `E0716` — temporary value dropped | Lấy `&RawFindingRecord` từ `vec![...]` tạm thời | Gán `let rec = RawFindingRecord::new(...);` vào biến trước khi tham chiếu |
| `E0425` — cannot find value `FIXED_TIMESTAMP` | Quên `use` hằng số hoặc tự viết chuỗi ngày sai định dạng | Dùng trực tiếp hằng số `FIXED_TIMESTAMP` hoặc `ARCHIVE_TIMESTAMP` của crate |

---

## 11. Bài tập rèn luyện

### Bài 1 — Thêm custom mock `OsintTool` và kiểm chứng hàng rào scope/passive

Viết một kiểu `struct CustomPassiveDnsTool;` cài `OsintTool` cho `SourceKind::Dns` (nhận `ToolTargetKind::Domain`), chỉ trả về một `RawFindingRecord` cho `corp.example` với IP `10.20.30.50`. Đăng ký vào một `ToolRegistry` mới, chứng minh query `API.CORP.EXAMPLE.` thành công (và được chuẩn hóa về `api.corp.example` hoặc `corp.example`), query `other.example` trả `OutOfScope`, và yêu cầu `CollectionMode::ActiveProbe` bị chặn bằng `ActiveProbeProhibited`.

<details><summary>Gợi ý</summary>

Dùng `OsintScope::standard()` và `ToolRegistry::new(scope.allowed_sources.clone())`, rồi gọi `registry.register(CustomPassiveDnsTool)`. Sử dụng `ToolRequest::passive(...).with_mode(CollectionMode::ActiveProbe)` để kiểm tra chặn active probe.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_custom_tool_and_scope_guard() -> Result<(), Box<dyn std::error::Error>> {
    struct CustomPassiveDnsTool;
    impl OsintTool for CustomPassiveDnsTool {
        fn source_kind(&self) -> SourceKind {
            SourceKind::Dns
        }
        fn expected_target_kind(&self) -> ToolTargetKind {
            ToolTargetKind::Domain
        }
        fn query(&mut self, request: &ToolRequest) -> Result<Vec<RawFindingRecord>, OsintError> {
            Ok(vec![RawFindingRecord::new(
                &format!("domain:{}", request.target),
                EntityKind::Domain,
                &request.target,
                "a_record",
                "10.20.30.50",
                FIXED_TIMESTAMP,
                850,
            )])
        }
    }

    let scope = OsintScope::standard();
    let mut reg = ToolRegistry::new(scope.allowed_sources.clone());
    reg.register(CustomPassiveDnsTool);

    let ok_req = ToolRequest::passive(
        ENGAGEMENT_ID,
        SourceKind::Dns,
        ToolTargetKind::Domain,
        "API.CORP.EXAMPLE.",
    );
    let result = reg.execute(&scope, &ok_req, "custom-dns-api")?;
    assert_eq!(result.canonical_target, "api.corp.example");
    assert_eq!(result.records.len(), 1);

    let out_req = ToolRequest::passive(
        ENGAGEMENT_ID,
        SourceKind::Dns,
        ToolTargetKind::Domain,
        "other.example",
    );
    assert!(matches!(
        reg.execute(&scope, &out_req, "custom-out"),
        Err(OsintError::OutOfScope(_))
    ));

    let active_req = ToolRequest::passive(
        ENGAGEMENT_ID,
        SourceKind::Dns,
        ToolTargetKind::Domain,
        "corp.example",
    )
    .with_mode(CollectionMode::ActiveProbe);
    assert_eq!(
        reg.execute(&scope, &active_req, "custom-active"),
        Err(OsintError::ActiveProbeProhibited)
    );
    Ok(())
}
```

</details>

### Bài 2 — Chuẩn hóa định danh và gộp thực thể `SameAs` không nhân đôi bằng chứng

Chuẩn hóa `"PORTAL.CORP.EXAMPLE."` và `"Corp-Example-Labs"`. Sau đó chạy `Whois` trên `"corp.example"` và `GitHub` trên `"corp-example-labs"`, nạp vào `EvidenceGraph` (nạp lại kết quả `GitHub` thêm một lần nữa để kiểm chứng lũy đẳng), rồi gọi `merge_same_as_entities()` và xác minh `org:corp-example` chứa bí danh `org:corp-example-labs` mà không tạo bản sao bằng chứng.

<details><summary>Gợi ý</summary>

Dùng `normalize_example_domain`, `normalize_org_handle`, `EvidenceGraph::ingest_result` và `EvidenceGraph::merge_same_as_entities`. Gọi `graph.validate()?` sau khi gộp.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_normalize_and_same_as_merge() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        normalize_example_domain("PORTAL.CORP.EXAMPLE.")?,
        "portal.corp.example"
    );
    assert_eq!(normalize_org_handle("Corp-Example-Labs")?, "corp-example-labs");

    let scope = OsintScope::standard();
    let mut reg = ToolRegistry::standard();
    let whois = reg.execute(
        &scope,
        &ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::Whois,
            ToolTargetKind::Domain,
            "corp.example",
        ),
        "ex2-whois",
    )?;
    let github = reg.execute(
        &scope,
        &ToolRequest::passive(
            ENGAGEMENT_ID,
            SourceKind::GitHub,
            ToolTargetKind::OrgHandle,
            "corp-example-labs",
        ),
        "ex2-github",
    )?;

    let mut graph = EvidenceGraph::new();
    graph.ingest_result(&scope, &whois)?;
    graph.ingest_result(&scope, &github)?;
    graph.ingest_result(&scope, &github)?;
    assert_eq!(graph.duplicate_ingest_skips(), 1);

    let merged = graph.merge_same_as_entities()?;
    assert_eq!(merged, 1);
    assert_eq!(graph.merge_same_as_entities()?, 0);
    graph.validate()?;

    let org = graph
        .entities()
        .get(&EntityId("org:corp-example".to_string()))
        .expect("merged org");
    assert!(org
        .aliases
        .contains(&EntityId("org:corp-example-labs".to_string())));
    assert_eq!(
        org.attributes
            .get("role_contact")
            .and_then(|r| r.resolved_value.as_deref()),
        Some("security@corp.example")
    );
    Ok(())
}
```

</details>

### Bài 3 — Bảo toàn mâu thuẫn `Contradicts` và chặn nâng khống thành `Fact`

Chạy `MockOSINTAgent::standard()`, kiểm chứng thuộc tính `a_record` của `domain:staging-old.corp.example` đang ở trạng thái `ResolutionStatus::Conflicted` với `resolved_value == None` và có cạnh `EdgeKind::Contradicts`. Sau đó thử thêm một `ReportClaim` vào `report.facts` để tự chọn giá trị `"10.20.30.20"` cho thuộc tính đó và chứng minh `report.validate(agent.graph())` trả về `OsintError::ConflictedFactClaim`.

<details><summary>Gợi ý</summary>

Đọc `agent.graph().entities()` và `agent.graph().edges()`. Khi dựng `ReportClaim` giả, dùng một `evidence_id` có thật trên thuộc tính đó (ví dụ `"ev:dns:staging-old.corp.example:0"`) để chứng minh validator chặn đúng lỗi mâu thuẫn thay vì chỉ lỗi thiếu ID.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_preserve_conflict_and_reject_fact_promotion()
-> Result<(), Box<dyn std::error::Error>> {
    let mut agent = MockOSINTAgent::standard();
    let report = agent.run()?;

    let staging_id = EntityId("domain:staging-old.corp.example".to_string());
    let staging = agent
        .graph()
        .entities()
        .get(&staging_id)
        .expect("staging-old exists");
    let a_record = staging.attributes.get("a_record").expect("a_record");
    assert_eq!(a_record.status, ResolutionStatus::Conflicted);
    assert_eq!(a_record.resolved_value, None);
    assert_eq!(a_record.candidates.len(), 2);
    assert!(agent.graph().edges().iter().any(|e| {
        e.kind == EdgeKind::Contradicts && e.from == staging_id && e.attribute == "a_record"
    }));

    let dns_ev_id = a_record
        .candidates
        .iter()
        .find(|c| c.source_kind == SourceKind::Dns)
        .expect("dns candidate")
        .evidence_id
        .clone();

    let mut forged = report;
    forged.facts.push(ReportClaim {
        claim_id: "fact:promoted-conflict".to_string(),
        entity_id: staging_id,
        attribute: "a_record".to_string(),
        claimed_value: "10.20.30.20".to_string(),
        statement: "tự ý chọn IP mới bất chấp dữ liệu Wayback".to_string(),
        confidence: Confidence::from_milli(900, "tự khẳng định")?,
        evidence_ids: BTreeSet::from([dns_ev_id]),
    });
    assert!(matches!(
        forged.validate(agent.graph()),
        Err(OsintError::ConflictedFactClaim { .. })
    ));
    Ok(())
}
```

</details>

### Bài 4 — Ngắt giả bằng `TimeoutOnce`, phục hồi bằng `Checkpoint` và kiểm tra trùng khớp từng byte

Chạy một `MockOSINTAgent::standard()` hoàn chỉnh để lấy báo cáo chuẩn. Tạo một agent thứ hai tiêm `SimulatedFault::TimeoutOnce` vào `(SourceKind::Dns, "api.corp.example")`, xác nhận `run()` dừng và lưu `Checkpoint`. Dùng một agent thứ ba hoàn toàn mới gọi `resume(&checkpoint)` và chứng minh `Whois` trên `corp.example` không bị gọi lại (`invocation_count == 0`), `duplicate_ingest_skips() == 0`, đồng thời cả `render_json()` lẫn `render_markdown()` khớp từng byte với báo cáo chuẩn.

<details><summary>Gợi ý</summary>

Dùng `agent.registry.inject_fault(...)`, `agent.last_checkpoint()`, rồi `recovered.resume(&cp)?`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_checkpoint_resume_determinism() -> Result<(), Box<dyn std::error::Error>> {
    let mut baseline = MockOSINTAgent::standard();
    let baseline_report = baseline.run()?;

    let mut faulty = MockOSINTAgent::standard();
    faulty.registry.inject_fault(
        SourceKind::Dns,
        "api.corp.example",
        SimulatedFault::TimeoutOnce,
    );
    assert!(matches!(
        faulty.run(),
        Err(OsintError::SimulatedFaultTriggered {
            source: SourceKind::Dns,
            fault: SimulatedFault::TimeoutOnce,
        })
    ));
    let cp = faulty
        .last_checkpoint()
        .expect("checkpoint saved")
        .clone();

    let mut recovered = MockOSINTAgent::standard();
    let resumed_report = recovered.resume(&cp)?;
    assert_eq!(
        recovered
            .registry
            .invocation_count(SourceKind::Whois, "corp.example"),
        0
    );
    assert_eq!(recovered.graph().duplicate_ingest_skips(), 0);
    assert_eq!(baseline_report.render_json(), resumed_report.render_json());
    assert_eq!(
        baseline_report.render_markdown(),
        resumed_report.render_markdown()
    );
    Ok(())
}
```

</details>

### Bài 5 — Sinh báo cáo escaped an toàn và chặn `UnsupportedClaim`

Xác thực báo cáo từ `MockOSINTAgent::standard()` bằng `report.validate(agent.graph())`, sau đó chứng minh việc xóa `evidence_ids` trên một `Fact` khiến `validate` trả `UnsupportedClaim`. Cuối cùng, dựng một `OsintReport` chứa chuỗi `"ENG-\"|\"<script>\n1"` trong `engagement_id`, gọi `render_json()` và `render_markdown()` để xác nhận ký tự đặc biệt đã được escape an toàn.

<details><summary>Gợi ý</summary>

Clone `report`, gọi `.evidence_ids.clear()` trên `facts[0]` để kiểm tra `UnsupportedClaim`. Trong bản báo cáo kiểm tra escape, giữ `generated_at = FIXED_TIMESTAMP.to_string()` và kiểm tra kết quả chuỗi JSON/Markdown.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_report_escaping_and_citation_check() -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = MockOSINTAgent::standard();
    let report = agent.run()?;
    report.validate(agent.graph())?;

    let mut unsupported = report.clone();
    unsupported.facts[0].evidence_ids.clear();
    assert!(matches!(
        unsupported.validate(agent.graph()),
        Err(OsintError::UnsupportedClaim(_))
    ));

    let escaped_report = OsintReport {
        schema_version: REPORT_SCHEMA_VERSION.to_string(),
        engagement_id: "ENG-\"|\"<script>\n1".to_string(),
        generated_at: FIXED_TIMESTAMP.to_string(),
        input_hash: report.input_hash,
        stop_reason: StopReason::Completed,
        facts: Vec::new(),
        inferences: Vec::new(),
        conflicts: Vec::new(),
        unknowns: Vec::new(),
        remediations: Vec::new(),
    };

    let json = escaped_report.render_json();
    let md = escaped_report.render_markdown();
    assert!(json.contains("\\\"|\\\"<script>\\n1"));
    assert!(!md.contains("<script>"));
    assert!(md.contains("\\|"));
    assert!(md.contains("&lt;script&gt;"));
    Ok(())
}
```

</details>

---

## 12. Tóm tắt, checklist lab và nguồn chuẩn

- Tác tử OSINT thụ động phải khóa scope trước khi chạy công cụ: chỉ miền `.example` được ủy quyền, IP RFC 1918 trong fixture tổng hợp và role contact tổ chức; nghiêm cấm active probing, PII và secret thô.
- Vòng lặp ReAct cần phanh kép (`max_steps`, `max_tool_calls`) và kiểm tra chu trình trên định danh đã chuẩn hóa (`lowercase` + cắt dấu chấm cuối FQDN).
- `ToolRegistry` tách bạch schema từng nguồn (`Domain` vs `OrgHandle`), kiểm soát allowlist và hỗ trợ mô phỏng lỗi tạm thời/bền bỉ.
- Hợp nhất `SameAs` giữ nguyên bằng chứng không nhân đôi; xung đột dữ liệu giữa các nguồn (như `Dns` vs `Wayback`) được giữ nguyên hai phía bằng `Conflicted` và `Contradicts`.
- `Checkpoint` có `input_hash` SHA-256 và xác thực đồ thị giúp phục hồi sau sự cố không gọi lại công cụ đã commit và tái tạo kết quả khớp từng byte.
- `OsintReport::validate` tách rõ `Fact`, `Inference` (≥ 2 bằng chứng), `Conflict` và `Unknown`, chặn nâng khống mâu thuẫn thành sự thật và escape an toàn đầu ra Markdown/JSON.

Checklist khi áp dụng nguyên lý này vào **dự án phòng thủ hoặc lab được ủy quyền**:

- [ ] Có văn bản phạm vi rõ `root_domains`, `allowed_org_handles`, nguồn thu thập được phép và điều kiện dừng.
- [ ] Tắt và chặn hoàn toàn cơ chế active probing trong đường ống thu thập thụ động.
- [ ] Áp dụng privacy minimization: không lưu email/tên/điện thoại cá nhân khi mục tiêu chỉ là kiểm kê hạ tầng tổ chức.
- [ ] Không đưa secret/token thô vào log, đồ thị hay báo cáo; mọi phát hiện lộ secret cũ phải kiểm tra xoay vòng.
- [ ] Lưu provenance đầy đủ cho mọi quan sát (`Source -> Observation -> Evidence -> Entity`).
- [ ] Không bao giờ xóa lặng lẽ bản ghi lịch sử khi nguồn mới mâu thuẫn với nguồn cũ.
- [ ] Kiểm thử chế độ `Checkpoint`/`resume` với lỗi ngắt giữa chừng để bảo đảm không nhân đôi dữ liệu.
- [ ] Chạy `cargo test -p ch109 --locked`, Clippy và các kiểm tra tài liệu trước khi sửa giáo trình.

### Nguồn chuẩn và đọc tiếp

- [RFC 2606 — Reserved Top Level DNS Names](https://www.rfc-editor.org/rfc/rfc2606): TLD `.example` dành riêng cho tài liệu và kiểm thử.
- [RFC 1918 — Address Allocation for Private Internets](https://www.rfc-editor.org/rfc/rfc1918): các dải địa chỉ IPv4 riêng tư `10/8`, `172.16/12`, `192.168/16`.
- [RFC 9162 — Certificate Transparency Version 2.0](https://www.rfc-editor.org/rfc/rfc9162): cơ chế nhật ký chứng chỉ công khai.
- [RFC 7480 — HTTP Usage in Registration Data Access Protocol (RDAP)](https://www.rfc-editor.org/rfc/rfc7480): chuẩn truy vấn thông tin đăng ký miền hiện đại thay thế WHOIS văn bản thuần.
- [SYNINT](https://github.com/gs-ai/SYNINT), [SpiderFoot fork](https://github.com/poppopjmp/spiderfoot), [argus-os](https://github.com/AXRoux/argus-os) và [agentic-osint-agent](https://github.com/thunderstornX/agentic-osint-agent): các kho kiến trúc tham chiếu theo kỷ luật phòng sạch.

Chương trước: [Chương 108 — Active Directory Attack Paths & Detection](chuong_108.md). Tiếp tục theo [ROADMAP](ROADMAP.md) và outline Phần II; không có liên kết tới chương chưa được publish.
