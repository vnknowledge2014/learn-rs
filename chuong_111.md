# Chương 111: Methodology, Reporting, CVSS 3.1 & Legal — Rules of Engagement, Chuỗi Bằng Chứng, Tái Kiểm Thử & Báo Cáo Tất Định

> **Phạm vi hợp pháp và giới hạn bắt buộc:** chương này dạy quy trình chuẩn mực, pháp lý và kỹ nghệ báo cáo an toàn thông tin phục vụ phòng thủ và diễn tập được ủy quyền. Toàn bộ crate `code/ch111` (`MockReportingLab`) chạy hoàn toàn **tất định trên dữ liệu tổng hợp** bằng thư viện chuẩn Rust 2024 (`std`), `#![forbid(unsafe_code)]`, dùng tên miền dành riêng `.example` (RFC 2606), địa chỉ RFC 1918 và nguồn bằng chứng ảo `virtual://` hoặc `lab-sensor://`. Crate **không** mở socket mạng, **không** quét mục tiêu thật, **không** thu thập credential thật và **cấm tuyệt đối** đưa secret/token/email cá nhân chưa che vào báo cáo.

## 1. Từ biên bản bàn giao phòng thí nghiệm tới quy trình Methodology & Reporting

Hãy tưởng tượng một đoàn kiểm toán an toàn thực phẩm được mời vào kiểm tra quy trình sản xuất tại một nhà máy. Trước khi chạm vào bất kỳ dây chuyền nào, đoàn phải tuân thủ bốn nguyên tắc không thể thương lượng:

1. **Giấy ủy quyền ký tên & biên bản phạm vi (`RulesOfEngagement` + `Scope`)**: ghi rõ khu nhà nào được phép vào (`allowed_domains`, `allowed_cidrs`), khu vô trùng nào tuyệt đối cấm bước vào (`denied_domains`, `denied_ips`), kho phụ trợ của nhà thầu nào phải có giấy đồng ý riêng (`third_party_assets`) và khi nào nhấn nút dừng khẩn cấp (`StopConditionTrigger`).
2. **Mẫu vật niêm phong có nhãn truy xuất (`EvidenceRef` + `EvidenceChain`)**: mọi mẫu thử đều ghi rõ thời gian UTC, vị trí lấy mẫu, mã băm SHA-256 niêm phong và **che kín thông tin cá nhân / bí mật kinh doanh** ngay khi thu thập, trước khi đưa vào hồ sơ chung.
3. **Thước đo mức độ tách khỏi thứ tự xử lý kinh doanh (`Cvss31Score` vs `BusinessPriority`)**: mức độ độc hại thuần kỹ thuật (CVSS v3.1 Base Score) cho biết bản chất kỹ thuật của lỗ hổng, nhưng độ ưu tiên xử lý thực tế còn phụ thuộc tài sản đó là bồn thử nghiệm cô lập (`DevSandbox`) hay dây chuyền lõi phục vụ khách hàng (`CoreIdentityOrRevenue`).
4. **Biên bản nghiệm thu sau khắc phục (`CleanupAttestation` + `Retest`)**: mọi dụng cụ thử nghiệm đều phải thu dọn sạch trước khi phát hành báo cáo, phát hiện nào chưa xác minh (`Draft`) hoặc báo động giả (`FalsePositive`) tuyệt đối không được phóng đại thành "đã khai thác", và phải có chu kỳ tái kiểm thử (`Retest`) để xác nhận bản vá thật sự hiệu quả.

```text
  [Written Consent + RulesOfEngagement + Scope (Deny > 3P > Allow)]
                                 │
                                 ▼
  Prepare ─► Recon ─► Validate ─► LabExploit (HITL Approval) ─► PostAudit
                                                                     │
  AuditLog (Hash-Chained) ◄──── mọi bước hợp lệ hay bị chặn ─────────┤
                                                                     ▼
  Retest ◄── Report (Schema v1.0.0 · Dedup · SHA-256) ◄── Cleanup (verified_clean)
```

### Mục tiêu học tập

Sau chương này, bạn có thể:

- Thiết kế `Scope` và `RulesOfEngagement` có hiệu lực thực thi **trước khi chạy module**, bảo đảm quy tắc Deny thắng Allow, kiểm tra chấp thuận bên thứ ba và kích hoạt `StopConditionTrigger` tức thì.
- Mô hình hóa vòng đời engagement 8 pha (`Prepare` → `Recon` → `Validate` → `LabExploit` → `PostExploitationAudit` → `Cleanup` → `Report` → `Retest`) kèm cổng `CleanupAttestation` và chuỗi nhật ký kiểm toán SHA-256 (`AuditLogEntry`).
- Cài đặt chuỗi bằng chứng `EvidenceRef` / `EvidenceChain` có auto-redaction secret/PII khi nạp và cơ chế `validate_no_raw_secret` chặn rò rỉ trên toàn bộ báo cáo.
- Thực thi bất biến vòng đời `Finding` / `FindingStatus`: cấm gán `ControlledLabExploited` cho `Draft` hoặc `FalsePositive`, khử trùng lặp độc lập hoán vị và giữ nguyên toàn bộ `EvidenceRef`.
- Tính chính xác **CVSS v3.1 Base Score** theo phương trình chuẩn FIRST.org (gồm hàm `Roundup` số nguyên nhân 100.000 ở Appendix A), tách bạch `CvssSeverity` với `BusinessPriority`.
- Xây dựng `ReportRenderer` tất định cho JSON, Executive Markdown và Technical Markdown, bắt buộc `schema_version`, trích dẫn chuẩn và escape chống chèn Markdown/JSON.
- Kiểm soát trách nhiệm pháp lý, giảm thiểu dữ liệu, lưu trữ/xóa theo hạn (`DataHandlingPolicy`), công bố phối hợp (`DisclosurePolicy`) và tái kiểm thử (`RetestRecord`).

Tiên quyết nên đọc trước: Chương 86–90 cho HITL và tính bền vững phiên, Chương 104–105 cho xác minh bằng chứng PoC và hiến pháp an toàn, Chương 109–110 cho đồ thị bằng chứng và dọn dẹp hậu khai thác.

---

## 2. Ủy quyền bằng văn bản, `Scope` và điều kiện dừng khẩn cấp

Mọi hoạt động kiểm thử hợp pháp đều bắt đầu từ **văn bản ủy quyền có hiệu lực** (`signed_written_consent == true`, `authorization_ref` rõ ràng) và cửa sổ thời gian UTC đóng khung (`window_start_utc < window_end_utc`). Nếu bất kỳ điều kiện nào thiếu hoặc hết hạn, bộ điều phối phải từ chối **trước khi** chạm vào logic của module kiểm thử (`module_invocations == 0`).

### 2.1 Quy tắc ưu tiên trong `Scope::check_target`

Một sai sót hay gặp là kiểm tra danh sách cho phép (`allow_subdomains_of`) trước khi kiểm tra danh sách loại trừ (`denied_domains`) — dẫn tới hệ quả một vùng nhạy cảm như `payroll.corp.example` bị cho qua chỉ vì nằm dưới `corp.example`. `ch111` khóa thứ tự kiểm tra chặt chẽ:

| Thứ tự | Quy tắc kiểm tra | Hành vi khi vi phạm |
|---|---|---|
| 1 | Kiểm tra cú pháp tên miền `.example` (RFC 2606), IPv4 nghiêm ngặt (chặn octet `010`) và ISO 8601 UTC | Trả `InvalidDomain` / `InvalidIpOrCidr` / `InvalidTimestamp` |
| 2 | **Deny luôn thắng Allow**: kiểm tra `denied_domains` (khớp chính xác hoặc tên miền con) và `denied_ips` | Trả `OutOfScope` |
| 3 | **Phụ thuộc bên thứ ba (`third_party_assets`)**: kiểm tra `written_consent_ref` không rỗng và `at_utc <= approved_until_utc` | Trả `ThirdPartyConsentMissing` |
| 4 | **Allowlist + RFC 1918**: đối chiếu `allowed_domains`/`allow_subdomains_of` hoặc IPv4 thuộc `allowed_cidrs` (RFC 1918 hợp lệ, không bit host khác 0) | Trả `OutOfScope` |

### 2.2 Tác nhân tự hành (Agentic Safety), Phê duyệt Con người & `StopConditionTrigger`

Khi một hệ thống tác tử (như vòng kiểm soát ở Chương 105) đề xuất chạy phương pháp có rủi ro tác động trạng thái phòng thí nghiệm (`ControlledLabExploit`), `RulesOfEngagement::agent_human_approval_required_for` đòi hỏi phải có một `AgentApprovalRecord` còn hạn, khớp đúng `method` và `target`. Song song đó, `stop_conditions` (như `ProductionAvailabilityDegradation`, `UnintendedDataExposure`, `ActiveIncidentResponseDetected`, `EmergencyClientAbort`) đóng vai trò chốt ngắt cứng: một khi đã kích hoạt qua `trigger_stop_condition`, mọi lời gọi chuyển pha hoặc chạy bước mới đều bị chặn vĩnh viễn và ghi vết vào `AuditLogEntry`.

---

## 3. Vòng đời Engagement 8 pha và Nhật ký Kiểm toán Chuỗi Băm

`EngagementPhase` chuẩn hóa luồng công việc khép kín gồm tám trạng thái nối tiếp hợp lệ:

```text
Prepare ──► Recon ──► Validate ──┬──► LabExploit ──┬──► PostExploitationAudit ──┐
                                 │                 └────────────────────────────┼──► Cleanup ──► Report ──► Retest
                                 └──────────────────────────────────────────────┘
```

- Mỗi pha chỉ chấp nhận đúng tập `TestingMethod` tương thích qua `EngagementPhase::allows_method`: ví dụ `ControlledLabExploit` chỉ chạy được ở pha `LabExploit`, còn `RetestVerification` chỉ chạy ở pha `Retest`.
- Không thể nhảy từ `Cleanup` sang `Report` nếu thiếu `CleanupAttestation` chứng nhận `verified_clean == true && remaining_artifacts == 0`.
- Mọi sự kiện (bước được phép, bước bị chặn ngoài scope, chuyển pha, kích hoạt stop condition, và cập nhật retest) đều nối vào `audit_log: Vec<AuditLogEntry>` với trường `prev_hash` và `entry_hash = sha256_hex(...)` tính từ số thứ tự, timestamp, pha, tác nhân, hành động, mục tiêu và quyết định. Sửa một ký tự trong quá khứ lập tức làm `verify_audit_log()` trả lỗi `AuditLogTampered`.

---

## 4. Chuỗi bằng chứng (`EvidenceRef`, `EvidenceChain`) và Chính sách Dữ liệu

Một báo cáo có giá trị pháp lý phải bảo đảm hai yêu cầu tưởng chừng trái ngược: **chứng minh không chối cãi được phát hiện** nhưng **tuyệt đối không để rò rỉ bí mật hoặc dữ liệu cá nhân**.

1. **Tạo và làm sạch ngay đầu vào (`EvidenceRef::from_raw_capture`)**: hàm nhận bản ghi thô từ phòng thí nghiệm giả, tự động dùng `redact_sensitive_text` thay thế mẫu `Bearer ...`, `password=...`, `api_key=...`, `secret=...`, khóa riêng PEM, token định danh tổng hợp (`syn_tok_...`, `ghp_...`, `eyj...`) thành `[REDACTED-SECRET]` và email ngoài `.example` thành `[REDACTED-PII]` **trước khi** tính `artifact_sha256` và lưu `redacted_excerpt`.
2. **Kiểm tra hai lớp (`validate_no_raw_secret`)**: mọi trường văn bản của `RulesOfEngagement`, `Finding`, `EvidenceRef`, `AuditLogEntry`, `StandardCitation` và `Report` đều đi qua `validate_no_raw_secret`. Ngay cả khi người dùng dựng `Report` thủ công rồi chèn chuỗi bí mật chưa che vào `executive_narrative` hay `remediation`, cả `Report::validate()` lẫn `ReportRenderer` đều chặn bằng `Err(ReportingError::RawSecretProhibited(_))`.
3. **Liên kết chống sửa lén (`EvidenceChain`)**: kiểm tra tuần tự `sequence`, đơn điệu không giảm của `captured_at_utc`, không trùng `evidence_id`, `prev_hash` nối khớp mắt xích trước và `source_uri` chỉ chấp nhận `virtual://` hoặc `lab-sensor://`.
4. **Hạn lưu giữ và xóa dữ liệu (`DataHandlingPolicy`)**: bắt buộc `data_minimization_enforced == true`, `redact_secrets_and_pii == true`, `retention_days` nằm trong `1..=365` và `deletion_deadline_utc >= window_end_utc`. Hàm `evaluate_retention(check_at_utc)` phân định rõ `ActiveWindow`, `WithinRetentionPeriod`, `DeletionRequiredNow` và `PurgedConfirmed`.

---

## 5. Vòng đời `Finding`, Chống Thổi Phồng & Khử Trùng Lặp Độc Lập Hoán Vị

### 5.1 Bảy trạng thái `FindingStatus` và Ranh giới Khai thác

`FindingStatus` gồm bảy trạng thái rõ ràng:

| Trạng thái | Ý nghĩa | Đưa vào mục Verified Findings? | Cho phép claim `ControlledLabExploited`? |
|---|---|---|---|
| `Draft` | Nghi vấn sơ bộ chưa kiểm chứng đủ | Không (tách sang `excluded`) | **Cấm tuyệt đối** (`UnverifiedExploitationClaim`) |
| `FalsePositive` | Đã xác nhận báo động giả | Không (tách sang `excluded`) | **Cấm tuyệt đối** (`UnverifiedExploitationClaim`) |
| `Duplicate` | Bản ghi trùng lặp trỏ tới bản chính | Gộp bằng chứng vào bản chính | Chỉ gộp bằng chứng sang bản chính |
| `Verified` | Đã tái hiện chắc chắn trong lab | Có | Chỉ khi có `lab_proof_evidence_id` nằm trong `evidence_refs` |
| `AcceptedRisk` | Chủ sở hữu chấp nhận rủi ro bằng văn bản | Có (không tính là rủi ro mở) | Nếu trước đó đã xác minh có proof |
| `Remediated` | Chủ hệ thống đã áp dụng bản vá | Có (chờ kiểm chứng retest) | Nếu trước đó đã xác minh có proof |
| `Retested` | Đã tái kiểm thử hậu khắc phục | Có (`FixedVerified` đóng rủi ro) | Giữ lịch sử bằng chứng |

### 5.2 Khử trùng lặp độc lập hoán vị (`deduplicate_findings`)

Nhiều công cụ quét khác nhau thường báo cùng một nguyên nhân gốc trên nhiều URL hoặc IP. Nếu chỉ lấy "bản ghi đầu tiên gặp trong mảng", thứ tự quét thay đổi sẽ làm đổi kết quả báo cáo. `deduplicate_findings` loại bỏ hoàn toàn sự phụ thuộc thứ tự đó:

1. Xác thực từng `Finding` đối với `EvidenceChain` (đảm bảo mọi `evidence_id` có thật và khớp nội dung/digest).
2. Kiểm tra liên kết `duplicate_of` tường minh: cấm tự trỏ chính nó, cấm trỏ tới ID không tồn tại, cấm dây chuyền `A -> B -> C` và cấm trỏ tới bản ghi `Draft`/`FalsePositive` chưa xác minh.
3. Gom nhóm các bản ghi hợp lệ cùng chữ ký kỹ thuật `(rule_id, root_cause_key, primary_component)` (đã chuẩn hóa chữ thường) và sắp xếp nội bộ theo `(verification_rank giảm dần, cvss_score giảm dần, finding_id tăng dần)` để chọn đúng một bản ghi đại diện.
4. Gộp toàn bộ `evidence_refs` (báo lỗi `DeduplicationConflict` nếu cùng `evidence_id` nhưng sai lệch nội dung/digest), `affected_assets`, `citation_ids`, `merged_duplicate_ids`, bảo toàn `ControlledLabExploited`, lấy mức criticality/exposure/CVSS cao nhất rồi tính lại `BusinessPriority`.

---

## 6. CVSS v3.1 Base Score Chuẩn FIRST.org & `BusinessPriority`

Tiêu chuẩn **FIRST.org CVSS v3.1** định nghĩa điểm Base từ tám metric cơ sở theo đúng thứ tự chuẩn:

$$\text{AV} \to \text{AC} \to \text{PR} \to \text{UI} \to \text{S} \to \text{C} \to \text{I} \to \text{A}$$

| Metric | Mã hợp lệ & Trọng số chuẩn FIRST.org CVSS v3.1 |
|---|---|
| Attack Vector (`AV`) | `N` = `0.85` · `A` = `0.62` · `L` = `0.55` · `P` = `0.20` |
| Attack Complexity (`AC`) | `L` = `0.77` · `H` = `0.44` |
| Privileges Required (`PR`) | `N` = `0.85` · `L` = `0.62` (`S:U`) / `0.68` (`S:C`) · `H` = `0.27` (`S:U`) / `0.50` (`S:C`) |
| User Interaction (`UI`) | `N` = `0.85` · `R` = `0.62` |
| Confidentiality / Integrity / Availability (`C`/`I`/`A`) | `H` = `0.56` · `L` = `0.22` · `N` = `0.00` |

### 6.1 Bộ phương trình CVSS v3.1 và mẹo tránh lỗi dấu phẩy động ở Appendix A

1. **Impact Sub-Score (ISS)**:
   $$\text{ISS} = 1 - (1 - \text{C}) \times (1 - \text{I}) \times (1 - \text{A})$$
2. **Impact**:
   - Nếu `Scope` là Unchanged (`S:U`):
     $$\text{Impact} = 6.42 \times \text{ISS}$$
   - Nếu `Scope` là Changed (`S:C`):
     $$\text{Impact} = 7.52 \times (\text{ISS} - 0.029) - 3.25 \times (\text{ISS} - 0.02)^{15}$$
3. **Exploitability**:
   $$\text{Exploitability} = 8.22 \times \text{AV} \times \text{AC} \times \text{PR} \times \text{UI}$$
4. **Base Score**: nếu $\text{Impact} \le 0$ thì điểm là `0.0`; ngược lại với `S:U` là $\text{Roundup}(\min(\text{Impact} + \text{Exploitability}, 10))$ và với `S:C` là $\text{Roundup}(\min(1.08 \times (\text{Impact} + \text{Exploitability}), 10))$.

> **Cạm bẫy float trong Appendix A:** nếu cài `Roundup(x)` bằng `(x * 10.0).ceil() / 10.0`, lỗi biểu diễn IEEE 754 khiến `0.1 + 0.2 = 0.30000000000000004` bị làm tròn vống lên `0.4` thay vì `0.3`! Vì vậy Appendix A của FIRST.org yêu cầu nhân `input * 100_000.0`, làm tròn thành số nguyên gần nhất (`int_input`), chỉ tăng bậc khi `int_input % 10_000 != 0` và lưu `score_tenths: u8` (`0..=100`) để mọi phép so sánh và sắp xếp hoàn toàn chính xác.

| Vector CVSS v3.1 kiểm chứng | Điểm chuẩn (`score_tenths`) | Severity chuẩn | Ghi chú |
|---|---:|---|---|
| `AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H` | `9.8` (`98`) | `Critical` | Vector chuẩn RCE/AuthZ qua mạng không xác thực |
| `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H` | `10.0` (`100`) | `Critical` | Scope Changed đẩy kịch trần |
| `CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:C/C:H/I:H/A:H` | `9.1` (`91`) | `Critical` | So sánh trực tiếp tác động tăng trọng số `PR:H` khi `S:C` |
| `CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:U/C:H/I:H/A:H` | `7.2` (`72`) | `High` | Cùng vector trên nhưng `S:U` làm `PR:H = 0.27` |
| `CVSS:3.1/AV:L/AC:L/PR:N/UI:R/S:U/C:H/I:H/A:H` | `7.8` (`78`) | `High` | Cục bộ + tương tác người dùng |
| `CVSS:3.1/AV:N/AC:L/PR:N/UI:R/S:C/C:L/I:L/A:N` | `6.1` (`61`) | `Medium` | Điển hình Stored/Reflected XSS vượt origin |
| `CVSS:3.1/AV:P/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N` | `1.6` (`16`) | `Low` | Đòi hỏi tiếp cận vật lý + nhiều điều kiện |
| `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N` | `0.0` (`0`) | `None` | Không tác động C/I/A |

### 6.2 Phân biệt `CvssSeverity` với `BusinessPriority`

CVSS đo mức độ nghiêm trọng kỹ thuật **độc lập với tài sản**. Một lỗ hổng `9.8 Critical` nằm trong container sandbox cô lập không dữ liệu thật (`DevSandbox` + `IsolatedLabOnly`) có thể chỉ được xếp lịch `P3-Scheduled`, trong khi một lỗ hổng `6.1 Medium` trên hệ thống lõi định danh công khai Internet (`CoreIdentityOrRevenue` + `InternetFacing`) được `compute_business_priority` đẩy lên `P1-Immediate`. `Finding::validate()` và `Report::validate()` buộc trường `business_priority` luôn khớp công thức tính để không ai tự ý sửa nhãn ưu tiên.

---

## 7. Trình kết xuất Báo cáo Tất định (`Report`, `ReportRenderer`)

Một báo cáo chuyên nghiệp cần phục vụ cả hai nhóm người đọc:

- **`render_executive_markdown`**: tóm tắt điều hành (metadata ủy quyền, hạn lưu giữ/xóa, bảng đếm theo mức độ, danh sách xếp hạng ưu tiên) mà không làm loãng người đọc quản lý bằng chi tiết kỹ thuật thô.
- **`render_technical_markdown`**: đầy đủ phạm vi, chứng nhận dọn dẹp (`CleanupAttestation`), từng phát hiện kèm tài sản ảnh hưởng, danh sách duplicate đã gộp, bước tái hiện trong lab, impact, remediation, cleanup, detection, bằng chứng SHA-256, lịch sử retest và trích dẫn chuẩn (`citations`).
- **`render_json`**: đầu ra JSON tất định có cấu trúc đầy đủ (`schema_version = "1.0.0"`, `input_digest_sha256` bao phủ toàn bộ nội dung báo cáo, `excluded_non_actionable`, `findings`, `retest`, `citations`).

Cả bốn hàm render (`render_json`, `render_executive_markdown`, `render_technical_markdown`, `render_markdown`) đều gọi `report.validate()?` trước khi xuất chuỗi, và tách riêng `escape_json_string`, `escape_markdown_inline` (cho văn xuôi/bảng biểu) và `escape_markdown_code` (cho chuỗi nằm trong cặp backtick) để dữ liệu đầu vào chứa `|`, `<`, `>`, `*`, `\n` hay `\x07` không thể chèn tiêu đề giả hay phá cấu trúc tài liệu.

---

## 8. Quy trình Tái Kiểm Thử (`Retest`), Pháp lý & Đạo đức Vận hành

- **Tái kiểm thử có bằng chứng (`Engagement::apply_retest`)**: chỉ chạy khi engagement đã chuyển sang pha `Retest`, yêu cầu bằng chứng mới `captured_at_utc >=` mốc bằng chứng mới nhất trước đó của finding, không chấp nhận ghi chú trống hoặc secret thô, và phân rõ ba kết quả: `FixedVerified` (đóng rủi ro mở), `PartiallyMitigated { residual_vector }` (cập nhật CVSS tồn dư và tính lại `BusinessPriority`) hoặc `StillVulnerable` (vẫn giữ trạng thái rủi ro mở).
- **Công bố có trách nhiệm & Hủy dữ liệu (`DisclosurePolicy` + `evaluate_retention`)**: mọi liên hệ đều dùng địa chỉ vai trò tổ chức thuộc `.example` (không thu thập PII cá nhân), tôn trọng cửa sổ phối hợp khắc phục (`coordination_window_days` / `embargo_until_utc`) và kiểm tra trạng thái lưu giữ để bảo đảm xóa sạch dữ liệu kiểm thử đúng hạn.

---

## 9. Mã nguồn minh họa thực chiến (`code/ch111`)

### 9.1 Tóm tắt kiến trúc crate `code/ch111`

Crate `code/ch111` tập hợp trọn vẹn các kiểu công khai cốt lõi theo thiết kế:

- **Quản trị & thực thi có kiểm soát**: `Scope`, `RulesOfEngagement`, `Engagement`, `EngagementPhase`, `ThirdPartyAuthorization`, `TestingMethod`, `ProhibitedAction`, `StopConditionTrigger`, `AgentApprovalRecord`, `AuditLogEntry`, `CleanupAttestation`, `DataHandlingPolicy`, `DisclosurePolicy`.
- **Bằng chứng & phát hiện**: `EvidenceRef`, `EvidenceChain`, `Finding`, `FindingStatus`, `ExploitationClaim`, `DeduplicationResult`, `deduplicate_findings`, `RetestRecord`, `RetestOutcome`, `StandardCitation`.
- **Tính điểm & xuất báo cáo**: `Cvss31Vector`, `Cvss31Score`, `roundup_cvss31_tenths`, `CvssSeverity`, `AssetCriticality`, `ExposureContext`, `BusinessPriority`, `compute_business_priority`, `Report`, `ReportRenderer`, `MockReportingLab`.

### 9.2 Chạy thử demo và bộ kiểm thử

```bash
cd code
cargo run -p ch111 --locked
cargo test -p ch111 --locked
cargo clippy -p ch111 --all-targets --locked -- -D warnings
```

Output thực tế của demo:

```text
MockReportingLab — 100% synthetic governance, evidence chain, CVSS 3.1 & deterministic reporting
Out-of-scope gate: blocked=true; module_invocations=0
CVSS 3.1 check: std=9.8 (Critical) | scope_changed_high_pr=9.1 (Critical)
Report verified=3 excluded=1 f101_evidence=2 f101_priority=P1-Immediate f103_priority=P3-Scheduled replay_identical=true
Retest verified fixed=1 digest_prefix=124f45b5cecae2fb
```

Crate hiện có **46 unit test** bao phủ toàn bộ đường thành công, đường lỗi và trường hợp biên: băm SHA-256 khớp vector NIST, chuẩn hóa miền `.example` và CIDR RFC 1918 chặt, cô lập URI `virtual://`/`lab-sensor://`, 12 vector CVSS v3.1 chuẩn cùng `Roundup` Appendix A (`0.1 + 0.2 -> 0.3`), từ chối vector `CVSS:3.0` hoặc sai thứ tự metric, tách bạch `CvssSeverity` với `BusinessPriority`, ưu tiên Deny > 3P > Allow, chặn mọi bước ngoài scope/hết hạn/chưa phê duyệt HITL với `0` lần gọi module, tính nguyên tử của `advance_phase` / `StopConditionTrigger` và `AuditLogEntry` framing theo độ dài chống giả mạo, cổng `CleanupAttestation` chặn xuất báo cáo khi còn tàn dư, tự động redaction secret (kể cả JSON key/value, dấu cách trong ngoặc kép ở cả assignment lẫn `Bearer`, khoảng trắng/xuống dòng sau Bearer/=, nhiều khối PEM liên tiếp, JWT, email ngoài `.example`), từ chối mọi assertion `ControlledLabExploited` ở `Draft`/`FalsePositive`, bất biến hoán vị và tách nhóm mở/đóng của `deduplicate_findings`, cập nhật `Retest` bảo toàn điểm tồn dư khi gộp trùng lặp, `evaluate_retention`, và kiểm chứng toàn vẹn + escape block/inline/code + tất định trên cả JSON lẫn Markdown.

### 9.3 Bảng tra cứu compiler error thường gặp

| Lỗi | Nguyên nhân hay gặp trong bài tập | Cách sửa |
|---|---|---|
| `E0308` — mismatched types | Truyền `&str` thay vì `TargetAsset::Domain("...".to_string())` vào `check_target` | Bọc chuỗi bằng `TargetAsset::Domain(...)` hoặc `TargetAsset::IpV4(...)` |
| `E0382` — use of moved value | Truyền `ev` vào `eng.add_evidence(ev)` rồi dùng tiếp `ev` trong `Finding` hoặc biến khác | Clone `ev.clone()` ở điểm chuyển sở hữu đầu tiên |
| `E0502` / `E0499` — cannot borrow as mutable | Đang mượn bất biến `&eng.findings` rồi gọi `&mut eng.apply_retest(...)` | Kết thúc phạm vi mượn đọc (hoặc clone trường cần kiểm tra) trước khi gọi phương thức `&mut self` |
| `E0599` — no method named `score` | Gọi `.score()` trên `&str` thay vì phân tích thành `Cvss31Vector` trước | Gọi `Cvss31Vector::parse(...)?.score()` |
| `E0603` — private field access | Cố truy cập trực tiếp `links` bên trong `EvidenceChain` từ bên ngoài crate/module | Dùng phương thức công khai `chain.links()` và `chain.get(id)` |
| `E0061` — wrong number of arguments | Quên tham số `at_utc: &str` khi gọi `scope.check_target(&target, at_utc)` | Bổ sung mốc thời gian ISO 8601 UTC hợp lệ cho kiểm tra chấp thuận bên thứ ba |
| `E0277` — `?` conversion error | Quên kiểu trả về `Result<(), Box<dyn std::error::Error>>` trong hàm bài tập dùng `?` | Khai báo chữ ký hàm trả `Result<(), Box<dyn std::error::Error>>` |
| `E0716` — temporary value dropped | Lấy lát cắt `&str` từ kết quả `render_json(&report)?` tạm không gán biến | Gán `let json = renderer.render_json(&report)?;` trước khi mượn tiếp |

---

## 10. Bài tập rèn luyện

### Bài 1 — Thực thi cổng `Scope`, văn bản ủy quyền và `StopConditionTrigger`

Viết hàm `pub fn exercise_roe_scope_and_stop_gate() -> Result<(), Box<dyn std::error::Error>>`:
1. Khởi tạo `MockReportingLab::standard_engagement()?` và chứng minh `check_target` cho phép `auth.corp.example` nhưng từ chối `payroll.corp.example` (`OutOfScope`) và `legacy-sso.vendor.example` (`ThirdPartyConsentMissing`).
2. Chuyển bản sao sang pha `Recon`, thử chạy `execute_module_step` nhắm `payroll.corp.example`, xác nhận trả `Err(ReportingError::OutOfScope(_))` và số lần gọi closure thực tế bằng `0`.
3. Gọi `trigger_stop_condition` với `StopConditionTrigger::EmergencyClientAbort`, sau đó chứng minh cả `advance_phase(EngagementPhase::Retest, ...)` lẫn `verify_audit_log()` hoạt động đúng thiết kế (bước mới bị chặn ngay lập tức, nhật ký audit vẫn nguyên vẹn).

<details><summary>Gợi ý</summary>

Dùng `eng.scope.check_target(&TargetAsset::Domain(...), "2026-02-05T10:00:00Z")` và biến đếm `let mut module_calls = 0usize;` bên trong closure truyền vào `execute_module_step`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_roe_scope_and_stop_gate() -> Result<(), Box<dyn std::error::Error>> {
    let mut eng = MockReportingLab::standard_engagement()?;
    let at = "2026-02-05T10:00:00Z";

    assert!(
        eng.scope
            .check_target(&TargetAsset::Domain("auth.corp.example".to_string()), at)
            .is_ok()
    );
    assert!(matches!(
        eng.scope
            .check_target(&TargetAsset::Domain("payroll.corp.example".to_string()), at),
        Err(ReportingError::OutOfScope(_))
    ));
    assert!(matches!(
        eng.scope
            .check_target(&TargetAsset::Domain("legacy-sso.vendor.example".to_string()), at),
        Err(ReportingError::ThirdPartyConsentMissing(_))
    ));

    let mut oos_eng = eng.clone();
    oos_eng.phase = EngagementPhase::Recon;
    let mut module_calls = 0usize;
    let step_res = oos_eng.execute_module_step(
        &ModuleStepRequest {
            step_id: "ex1-oos".to_string(),
            actor: "recon-agent".to_string(),
            method: TestingMethod::PassiveRecon,
            target: TargetAsset::Domain("payroll.corp.example".to_string()),
            timestamp_utc: "2026-02-05T10:05:00Z".to_string(),
            prohibited_action: None,
        },
        |_| {
            module_calls += 1;
            Ok("unreachable".to_string())
        },
    );
    assert!(matches!(step_res, Err(ReportingError::OutOfScope(_))));
    assert_eq!(module_calls, 0);

    eng.trigger_stop_condition(
        StopConditionTrigger::EmergencyClientAbort,
        "2026-02-12T18:00:00Z",
        "client-ciso",
        "Halt requested during drill",
    )?;
    assert!(matches!(
        eng.advance_phase(EngagementPhase::Retest, "2026-02-12T18:01:00Z", "operator"),
        Err(ReportingError::StopConditionTriggered(
            StopConditionTrigger::EmergencyClientAbort
        ))
    ));
    eng.verify_audit_log()?;
    Ok(())
}
```

</details>

---

### Bài 2 — Kiểm chứng vector CVSS 3.1, Appendix A `Roundup` và `BusinessPriority`

Viết hàm `pub fn exercise_cvss31_vectors_and_priority() -> Result<(), Box<dyn std::error::Error>>`:
1. Parse và kiểm tra đúng `score_tenths` cho các vector chuẩn:
   - `AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H` → `98` (`Critical`)
   - `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H` → `100` (`Critical`)
   - `CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:C/C:H/I:H/A:H` → `91` (`Critical`)
   - `CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:U/C:H/I:H/A:H` → `72` (`High`)
   - `CVSS:3.1/AV:N/AC:L/PR:N/UI:R/S:C/C:L/I:L/A:N` → `61` (`Medium`)
   - `CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N` → `0` (`None`)
2. Kiểm tra `roundup_cvss31_tenths(0.1 + 0.2) == 3`, `4.00 -> 40`, `4.02 -> 41`, và từ chối `CVSS:3.0/...` lẫn vector sai thứ tự metric.
3. Chứng minh hai chiều tách biệt: `Critical` trong `DevSandbox + IsolatedLabOnly` trả `P3Scheduled`, còn `Medium` trong `CoreIdentityOrRevenue + InternetFacing` trả `P1Immediate`.

<details><summary>Gợi ý</summary>

Dùng `Cvss31Vector::parse(...)`, `.score()`, `roundup_cvss31_tenths(...)` và `compute_business_priority(...)`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_cvss31_vectors_and_priority() -> Result<(), Box<dyn std::error::Error>> {
    let expected = [
        ("AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H", 98u8, CvssSeverity::Critical),
        ("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:C/C:H/I:H/A:H", 100, CvssSeverity::Critical),
        ("CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:C/C:H/I:H/A:H", 91, CvssSeverity::Critical),
        ("CVSS:3.1/AV:N/AC:L/PR:H/UI:N/S:U/C:H/I:H/A:H", 72, CvssSeverity::High),
        ("CVSS:3.1/AV:N/AC:L/PR:N/UI:R/S:C/C:L/I:L/A:N", 61, CvssSeverity::Medium),
        ("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:N/I:N/A:N", 0, CvssSeverity::None),
    ];
    for (vec_str, tenths, sev) in expected {
        let score = Cvss31Vector::parse(vec_str)?.score();
        assert_eq!(score.score_tenths, tenths);
        assert_eq!(score.severity, sev);
    }

    assert_eq!(roundup_cvss31_tenths(0.1 + 0.2), 3);
    assert_eq!(roundup_cvss31_tenths(4.00), 40);
    assert_eq!(roundup_cvss31_tenths(4.02), 41);
    assert!(matches!(
        Cvss31Vector::parse("CVSS:3.0/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"),
        Err(ReportingError::InvalidCvssVector(_))
    ));
    assert!(matches!(
        Cvss31Vector::parse("AC:L/AV:N/PR:N/UI:N/S:U/C:H/I:H/A:H"),
        Err(ReportingError::InvalidCvssVector(_))
    ));

    assert_eq!(
        compute_business_priority(
            CvssSeverity::Critical,
            AssetCriticality::DevSandbox,
            ExposureContext::IsolatedLabOnly
        ),
        BusinessPriority::P3Scheduled
    );
    assert_eq!(
        compute_business_priority(
            CvssSeverity::Medium,
            AssetCriticality::CoreIdentityOrRevenue,
            ExposureContext::InternetFacing
        ),
        BusinessPriority::P1Immediate
    );
    Ok(())
}
```

</details>

---

### Bài 3 — Khử trùng lặp độc lập hoán vị và chặn claim khai thác chưa xác minh

Viết hàm `pub fn exercise_dedup_permutation_and_claim_guard() -> Result<(), Box<dyn std::error::Error>>`:
1. Từ `MockReportingLab::standard_engagement()?`, lấy `eng.findings` rồi đảo ngược thứ tự vào một `Vec<Finding>` mới, gọi `deduplicate_findings` trên cả hai thứ tự và chứng minh `canonical_findings` + `excluded_non_actionable` khớp hoàn toàn nhau (gồm cả `F-101` giữ đủ 2 `EvidenceRef` từ `F-102`).
2. Clone một `Finding` ở trạng thái `Draft` và thử gắn `ControlledLabExploited`, xác nhận bị từ chối bằng `Err(ReportingError::UnverifiedExploitationClaim(_))`.

<details><summary>Gợi ý</summary>

Dùng `eng.findings.iter().cloned().rev().collect::<Vec<_>>()` và `set_exploitation_claim`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_dedup_permutation_and_claim_guard() -> Result<(), Box<dyn std::error::Error>> {
    let eng = MockReportingLab::standard_engagement()?;
    let forward = eng.findings.clone();
    let reversed: Vec<Finding> = eng.findings.iter().cloned().rev().collect();

    let res_fwd = deduplicate_findings(&forward, &eng.evidence_chain)?;
    let res_rev = deduplicate_findings(&reversed, &eng.evidence_chain)?;
    assert_eq!(res_fwd, res_rev);
    assert_eq!(res_fwd.canonical_findings.len(), 3);
    assert_eq!(res_fwd.excluded_non_actionable.len(), 1);

    let f101 = res_fwd
        .canonical_findings
        .iter()
        .find(|f| f.finding_id == "F-101")
        .ok_or("thiếu F-101")?;
    assert_eq!(f101.evidence_refs.len(), 2);
    assert!(f101.merged_duplicate_ids.contains("F-102"));

    let mut draft = res_fwd.excluded_non_actionable[0].clone();
    assert_eq!(draft.status, FindingStatus::Draft);
    assert!(matches!(
        draft.set_exploitation_claim(ExploitationClaim::ControlledLabExploited {
            lab_proof_evidence_id: draft.evidence_refs[0].evidence_id.clone()
        }),
        Err(ReportingError::UnverifiedExploitationClaim(_))
    ));
    Ok(())
}
```

</details>

---

### Bài 4 — Tự động che secret/PII, cổng `schema_version` và kết xuất tất định

Viết hàm `pub fn exercise_redaction_and_deterministic_render() -> Result<(), Box<dyn std::error::Error>>`:
1. Nạp một bản ghi giả bằng `EvidenceRef::from_raw_capture` chứa cả `password="DemoLabSecret99"`, `Authorization: Bearer syn_tok_alpha001122` và `user=tester@other-org.test`, xác minh chuỗi bí mật và email ngoài `.example` đều bị che sạch (`redaction_count == 3`) và `validate()` đạt.
2. Tạo `Report` từ `MockReportingLab::standard_engagement()?`, render qua `ReportRenderer` (JSON, Executive Markdown, Technical Markdown), chứng minh render 2 lần cho cùng byte và không rò chuỗi bí mật thô nào.
3. Sửa `report.schema_version = "0.9.0".to_string()` và chứng minh `render_json` từ chối trước khi xuất.

<details><summary>Gợi ý</summary>

Dùng `EvidenceRef::from_raw_capture`, `eng.build_report(...)`, `ReportRenderer::render_json`, `render_executive_markdown`, `render_technical_markdown`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_redaction_and_deterministic_render() -> Result<(), Box<dyn std::error::Error>> {
    let ev = EvidenceRef::from_raw_capture(
        "EV-EX4",
        "2026-02-05T12:00:00Z",
        "virtual://portal.corp.example/auth/dump",
        "Header Bearer syn_tok_alpha001122 | config password=\"DemoLabSecret99\" | user=tester@other-org.test",
    )?;
    assert_eq!(ev.redaction_count, 3);
    assert!(!ev.redacted_excerpt.contains("syn_tok_alpha001122"));
    assert!(!ev.redacted_excerpt.contains("DemoLabSecret99"));
    assert!(!ev.redacted_excerpt.contains("other-org.test"));
    ev.validate()?;

    let eng = MockReportingLab::standard_engagement()?;
    let report = eng.build_report(REPORT_GENERATED_UTC, "Exercise 4 Report")?;
    let renderer = ReportRenderer;

    let j1 = renderer.render_json(&report)?;
    let j2 = renderer.render_json(&report)?;
    let exec = renderer.render_executive_markdown(&report)?;
    let tech = renderer.render_technical_markdown(&report)?;
    assert_eq!(j1, j2);
    assert!(j1.contains("\"schema_version\": \"1.0.0\""));
    assert!(exec.contains("# Executive Security Report"));
    assert!(tech.contains("## Verified Findings"));

    let mut wrong_schema = report;
    wrong_schema.schema_version = "0.9.0".to_string();
    assert!(matches!(
        renderer.render_json(&wrong_schema),
        Err(ReportingError::ReportSchemaInvalid(_))
    ));
    Ok(())
}
```

</details>

---

### Bài 5 — Tái kiểm thử `Retest`, đánh giá lưu giữ/xóa và chống sửa lén báo cáo

Viết hàm `pub fn exercise_retest_retention_and_tamper_guard() -> Result<(), Box<dyn std::error::Error>>`:
1. Chuyển `MockReportingLab::standard_engagement()?` sang pha `Retest`, áp dụng `RetestOutcome::FixedVerified` cho `F-101` và `RetestOutcome::PartiallyMitigated` với vector thấp hơn cho `F-104`, chứng minh `retested_fixed_count == 1` và `F-104` tự động hạ `BusinessPriority` tương ứng.
2. Kiểm tra `evaluate_retention` ở ba mốc thời gian (`WithinRetentionPeriod`, `DeletionRequiredNow`, rồi sau khi bật `data_purged = true` → `PurgedConfirmed`).
3. Sửa một trường nội dung trong `Report` (`remediation`) và chứng minh `report.validate()` trả lỗi `ReportSchemaInvalid`.

<details><summary>Gợi ý</summary>

Cần tạo `EvidenceRef::from_raw_capture` có timestamp không sớm hơn bằng chứng trước của finding rồi gọi `eng.apply_retest(...)`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_retest_retention_and_tamper_guard() -> Result<(), Box<dyn std::error::Error>> {
    let mut eng = MockReportingLab::standard_engagement()?;
    eng.advance_phase(EngagementPhase::Retest, "2026-02-13T09:00:00Z", "operator")?;

    let ev_fixed = EvidenceRef::from_raw_capture(
        "EV-EX5-101",
        "2026-02-13T10:00:00Z",
        "virtual://api.corp.example/http/retest-fixed",
        "Verified HTTP 403 returned on /admin/export without admin claim",
    )?;
    eng.apply_retest(
        "F-101",
        ev_fixed,
        RetestOutcome::FixedVerified,
        "Confirmed fail-closed RBAC fix",
        "verifier",
    )?;

    let ev_partial = EvidenceRef::from_raw_capture(
        "EV-EX5-104",
        "2026-02-13T10:15:00Z",
        "virtual://portal.corp.example/headers/retest-csp",
        "CSP enforced with minor report-only exception",
    )?;
    let low_vec = Cvss31Vector::parse("CVSS:3.1/AV:L/AC:H/PR:H/UI:R/S:U/C:L/I:N/A:N")?;
    eng.apply_retest(
        "F-104",
        ev_partial,
        RetestOutcome::PartiallyMitigated {
            residual_vector: low_vec,
        },
        "Residual impact lowered to Low",
        "verifier",
    )?;

    let report = eng.build_report("2026-02-13T11:00:00Z", "Retest Verification Report")?;
    assert_eq!(report.executive_summary.retested_fixed_count, 1);
    let f104 = report
        .findings
        .iter()
        .find(|f| f.finding_id == "F-104")
        .ok_or("thiếu F-104")?;
    assert_eq!(f104.business_priority, BusinessPriority::P2Accelerated);

    assert_eq!(
        eng.evaluate_retention("2026-03-01T00:00:00Z")?,
        RetentionDisposition::WithinRetentionPeriod
    );
    assert_eq!(
        eng.evaluate_retention("2026-06-01T00:00:00Z")?,
        RetentionDisposition::DeletionRequiredNow
    );
    eng.data_purged = true;
    assert_eq!(
        eng.evaluate_retention("2026-06-01T00:00:00Z")?,
        RetentionDisposition::PurgedConfirmed
    );

    let mut tampered = report;
    tampered.findings[0].remediation.push_str(" tampered");
    assert!(matches!(
        tampered.validate(),
        Err(ReportingError::ReportSchemaInvalid(_))
    ));
    Ok(())
}
```

</details>

---

## 11. Tóm tắt, checklist lab và nguồn chuẩn

- **Ủy quyền & phạm vi (`RulesOfEngagement` + `Scope`)** luôn đứng trước mọi thao tác kỹ thuật: luật Deny thắng Allow, phụ thuộc bên thứ ba bắt buộc có văn bản còn hạn, thao tác rủi ro của tác tử cần `AgentApprovalRecord` và `StopConditionTrigger` ngắt ngay khi kích hoạt.
- **Chuỗi bằng chứng (`EvidenceRef` + `EvidenceChain`)** tự động che secret/PII, đóng dấu SHA-256 trước khi nạp và từ chối xuất báo cáo nếu bất kỳ trường nào chứa chuỗi bí mật thô.
- **Chất lượng phát hiện & điểm CVSS 3.1**: cấm gắn `ControlledLabExploited` cho `Draft`/`FalsePositive`; `deduplicate_findings` độc lập hoán vị và giữ trọn mọi `EvidenceRef`; CVSS v3.1 dùng công thức FIRST.org cùng `Roundup` nhân `100.000` (cho `9.8` với `AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H`) và tách khỏi `BusinessPriority`.
- **Báo cáo tất định & vòng đời sau kiểm thử**: bắt buộc `CleanupAttestation` sạch trước khi sang `Report`, `schema_version` chuẩn, escape chống chèn JSON/Markdown, `Retest` có bằng chứng mới hơn và theo dõi `DataHandlingPolicy` tới thời điểm hủy dữ liệu.

Checklist trước khi bàn giao một engagement mô phỏng hoặc kiểm thử trong **lab được ủy quyền**:

- [ ] Có văn bản chấp thuận bằng chữ ký (`signed_written_consent`), danh sách Allow/Deny đầy đủ và chấp thuận riêng cho mọi phụ thuộc bên thứ ba.
- [ ] Gắn cổng kiểm tra phạm vi, khung giờ và phê duyệt con người (HITL) **trước khi** module kiểm thử được gọi.
- [ ] Chuẩn hóa và che kín toàn bộ secret, token, hash và email cá nhân khỏi mọi bằng chứng, log audit và báo cáo.
- [ ] Kiểm thử toàn bộ vector CVSS v3.1 với `Roundup` Appendix A; trình bày riêng `CvssSeverity` và `BusinessPriority`.
- [ ] Xác nhận `Draft`/`FalsePositive` bị loại khỏi danh sách đã xác minh và mọi duplicate sau khử trùng lặp giữ nguyên đủ `EvidenceRef`.
- [ ] Hoàn thành `CleanupAttestation` sạch trước khi xuất báo cáo, theo dõi hạn xóa dữ liệu và chạy `cargo test -p ch111 --locked`, Clippy cùng các kiểm tra tài liệu.

### Nguồn chuẩn và đọc tiếp

- [FIRST.org — Common Vulnerability Scoring System v3.1 Specification & Appendix A](https://www.first.org/cvss/v3.1/specification-document): phương trình Base Score, trọng số `PR` theo `Scope` và thuật toán `Roundup` số nguyên.
- [NIST SP 800-115 — Technical Guide to Information Security Testing and Assessment](https://csrc.nist.gov/publications/detail/sp/800-115/final): lập kế hoạch, Rules of Engagement, thực thi kiểm thử và quy trình báo cáo.
- [ISO/IEC 29147 — Vulnerability Disclosure](https://www.iso.org/standard/72311.html): chuẩn mực tiếp nhận và công bố lỗ hổng có phối hợp.
- [OWASP Web Security Testing Guide (WSTG)](https://owasp.org/www-project-web-security-testing-guide/): chuẩn danh mục kiểm thử ứng dụng web và cấu trúc trích dẫn phát hiện.
- [RFC 2606 — Reserved Top Level DNS Names](https://www.rfc-editor.org/rfc/rfc2606) và [FIPS 180-4 — Secure Hash Standard](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf): tên miền giả định `.example` và SHA-256 dùng cho bằng chứng & audit log.

Chương trước: [Chương 110 — Post-Exploitation & Persistence Audit](chuong_110.md). Tiếp tục theo [ROADMAP](ROADMAP.md) và outline Phần II; không có liên kết tới chương chưa được publish.
