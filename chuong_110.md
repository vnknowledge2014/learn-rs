# Chương 110: Post-Exploitation & Persistence Audit — Baseline Diff, Kiểm Kê Persistence, Lateral Graph, Telemetry & Cleanup Lũy Đẳng

> **Phạm vi hợp pháp và giới hạn bắt buộc:** chương này phục vụ kiểm toán phòng thủ và xác minh hậu khai thác trong phòng thí nghiệm được ủy quyền. Toàn bộ crate `ch110` (`MockPostExploit`) hoạt động hoàn toàn trên **snapshot chỉ đọc trong bộ nhớ**: mọi host đều là tên miền giả định `.example` (RFC 2606), mọi địa chỉ IP đều thuộc RFC 1918, mọi vị trí đều dùng scheme ảo `virtual://<host.example>/<subsystem>/<entry>` (chặn đường dẫn OS thật), và mọi thay đổi khi cleanup chỉ diễn ra trong `MockCleanupState` cô lập. Crate **không** đọc file hệ thống, **không** chạy tiến trình hay lệnh shell, **không** mở socket, **không** thu thập/dump credential thật (`LsassDump`, `SamDump`, `ShadowRead`, `TokenTheft` đều bị chặn bởi `CredentialHarvestingProhibited`) và **không** trình bày kỹ thuật né tránh hệ thống phát hiện (evasion).

## 1. Từ kiểm toán bàn giao kho bạc tới kiểm kê hậu khai thác

Hãy tưởng tượng sau một cuộc diễn tập kiểm tra an ninh tại một tòa nhà trụ sở giả định, đội kiểm toán bước vào với một nguyên tắc sắt đá: **không tự ý đục tường, không mang theo khóa vạn năng và không mở két sắt thật**. Thay vào đó, họ mang hai bộ hồ sơ niêm phong:

1. **Hồ sơ niêm phong ban đầu (`baseline_snapshots`)**: danh sách mọi ổ khóa, camera, cửa ra vào, tài khoản bảo vệ và quy định bật đèn hành lang trước giờ diễn tập.
2. **Hồ sơ hiện trạng (`current_snapshots`)**: ảnh chụp toàn vẹn sau giờ diễn tập.

Công việc của kiểm toán viên hậu khai thác gồm bốn nhiệm vụ rành mạch:

- **Đối chiếu sai lệch (Baseline Diff)**: phát hiện xem có chìa khóa dự phòng nào mới gắn thêm (`PersistenceRecord`), tệp vật chứng lạ nào xuất hiện (`Artifact`), tài khoản nào tự ý kích hoạt (`AccountState`) hay camera giám sát nào bị ai đó gạt tắt công tắc (`SecurityPolicyRecord` → `TelemetryGap`).
- **Đo tầm lan truyền (Lateral Graph)**: nếu một phòng đã mở tạm thời trong diễn tập, luồng di chuyển thực tế đi xa tới đâu trước khi đụng cửa chống cháy (segmentation), thẻ hết hạn (`SessionExpired`), quyền chưa đủ (`InsufficientPrivilege`) hay ranh giới tòa nhà khác (`OutOfScope`)?
- **Kiểm soát luồng ra (DataFlow Metadata)**: rà soát nhật ký ra vào cửa ở mức siêu dữ liệu (thời gian, đích đến, dung lượng, giao thức) để phát hiện dấu hiệu rò rỉ dữ liệu nhạy cảm mà không cần mở thùng hàng thật.
- **Thu dọn có kiểm chứng (`CleanupPlan`)**: tháo dỡ đúng các dấu vết diễn tập trong phạm vi cho phép mà tuyệt đối không xóa nhầm thiết bị gốc của tòa nhà (`BaselineProtected`), chạy lại lần hai vẫn an toàn (`idempotent`) và nghiệm thu sạch hoàn toàn về trạng thái ban đầu (`verify_baseline`).

```text
  [Snapshot Baseline] + [Snapshot Hiện Trạng]  (bất biến / read-only)
           │
           ├──► Baseline Diff (Services · Tasks · Startup · Autorun · Accounts · Policy · Artifacts)
           │           │
           │           ▼
           ├──► Persistence & Policy Audit ──► Đối chiếu TelemetryEvent ──► TelemetryGap
           │
           ├──► Lateral Graph (Stack DFS + AccessGrant + Scope + Segmentation + Cycle guard)
           │
           ├──► DataFlow Metadata Classifier (Destination · Protocol · Sensitivity · Volume)
           │
           └──► AuditReport (Fact + Evidence + Severity<=Impact + Detection + Remediation)
                       │
                       ▼
                 CleanupPlan ──► MockCleanupState cô lập (Idempotent · AlreadyAbsent · VerifyBaseline)
```

### Mục tiêu học tập

Sau chương này, bạn có thể:

- Phân biệt rạch ròi giữa kiểm toán trạng thái hậu khai thác có ủy quyền với thao tác khai thác thật hay thu thập credential.
- Thiết kế bộ so sánh `diff_snapshots` read-only bảo đảm bất biến mọi sai lệch đều phải gắn bằng chứng hợp lệ và `Severity <= max_severity_for_impact(ImpactLevel)`.
- Mô hình hóa kiểm kê `PersistenceRecord` across service, scheduled task, startup entry và autorun bằng đường dẫn ảo `virtual://` an toàn đa nền tảng.
- Dựng đồ thị `AccessGrant` và `LateralEdge` bằng DFS chống chu trình, phân tách chính xác `OutOfScope`, `NetworkSegmented`, `MissingAccessGrant`, `InsufficientPrivilege`, `SessionExpired` và `MaxDepthReached`.
- Ánh xạ từng phát hiện với `TelemetryChannel`, phát hiện điểm mù log (`TelemetryGap`) khi chính sách bị chỉnh sửa, và chuyển thành quy tắc phát hiện–ứng phó phòng thủ.
- Phân loại rủi ro rò rỉ dữ liệu qua `DataFlow` chỉ dựa trên siêu dữ liệu, không xử lý payload thật.
- Xây dựng và thực thi `CleanupPlan` nguyên tử, bảo vệ tài nguyên baseline, xử lý artifact đã mất và chứng minh tính lũy đẳng khi chạy lặp.

Tiên quyết nên đọc trước: Chương 64–65 cho hệ điều hành và mạng nền tảng, Chương 102 cho leo thang đặc quyền & sự thật nền, Chương 103 cho mô hình phiên hậu khai thác. Crate `ch110` dùng Rust 2024 edition, thuần `std`, `#![forbid(unsafe_code)]`, không dependency ngoài.

---

## 2. Phạm vi, cô lập `virtual://` và ranh giới cấm thu thập credential

Một sai lầm phổ biến khi viết công cụ hậu khai thác mô phỏng là trộn lẫn đường dẫn hệ điều hành thật (`C:\Windows\System32\...` hay `/etc/systemd/...`) hoặc giữ các hàm "tạm thời" dính tới credential dumping. Điều đó vừa mất an toàn, vừa khiến test chạy khác nhau giữa Windows và macOS/Linux.

`code/ch110` khóa chặt bốn rào chắn ngay ở tầng kiểu và validator:

| Hàng rào | Luật kiểm tra trong `ch110` | Lỗi trả về khi vi phạm |
|---|---|---|
| Phạm vi host (`AuditScope`) | Mọi host phải chuẩn hóa `.example` (RFC 2606) và nằm trong `allowed_hosts` | `OutOfScope` / `InvalidDomain` |
| Cô lập vị trí (`normalize_virtual_location`) | Bắt buộc prefix `virtual://<host.example>/<subsystem>/<entry>`, cấm `\`, `..`, khoảng trắng, ký tự shell (`;`, `&`, `\|`, ...) | `InvalidLocation` |
| Cấm thu thập credential (`HarvestingAction`) | `allow_credential_harvesting` bị khóa `false`; gọi `request_harvesting(...)` luôn bị từ chối | `CredentialHarvestingProhibited` |
| Lọc secret thô (`validate_no_raw_secret`) | Chặn mẫu token/key/hash không phân biệt hoa thường; mọi trường nhạy cảm chỉ được mang marker `SYNTHETIC-REDACTED-CREDENTIAL` | `RawSecretProhibited` |

Ngoài ra, mọi địa chỉ IP nội bộ trong siêu dữ liệu mạng đều được kiểm bằng `is_rfc1918_ipv4` (`10/8`, `172.16/12`, `192.168/16`, chặn octal dẫn đầu kiểu `010.0.0.1`) và mọi mốc thời gian đều kiểm nghiêm ngặt bằng `is_valid_iso8601_utc` (kể cả kiểm tra năm nhuận ngày `29/02`).

---

## 3. Baseline Diff chỉ đọc trên `SystemSnapshot`

Trước khi kết luận một hệ thống có dấu hiệu bị cài cắm hậu khai thác, kiểm toán viên cần trả lời một câu hỏi: *"Thứ này có từ ảnh chuẩn (golden baseline) hay mới xuất hiện/biến đổi?"*

`SystemSnapshot` mô hình hóa toàn bộ trạng thái một host tại một thời điểm bằng các `BTreeMap` tất định trên 6 hạng mục cốt lõi:

1. **Services (`PersistenceMechanism::Service`)**: dịch vụ nền kiểu systemd hoặc Windows Service.
2. **Scheduled Tasks (`PersistenceMechanism::ScheduledTask`)**: tác vụ định kỳ (Task Scheduler hoặc cron/timer).
3. **Startup Entries (`PersistenceMechanism::StartupEntry`)**: mục khởi chạy theo người dùng/phiên đăng nhập.
4. **Autoruns (`PersistenceMechanism::Autorun`)**: khóa tự khởi động lúc boot hoặc logon.
5. **Tài khoản (`accounts: BTreeMap<String, AccountState>`)**: trạng thái bật/tắt, mức `PrivilegeLevel` và tập nhóm quyền.
6. **Chính sách bảo mật (`security_policies: BTreeMap<String, SecurityPolicyRecord>`) & Kiểm soát log (`telemetry_controls`)**: giá trị an toàn kỳ vọng so với giá trị hiện hành, kèm liên kết `disables_channel` nếu chính sách tắt kênh telemetry.

Hàm `diff_snapshots(baseline: &SystemSnapshot, current: &SystemSnapshot, evidence_store)` thực thi ba bất biến nghiêm ngặt:

- **Chỉ đọc tuyệt đối**: tham số đầu vào đều là tham chiếu bất biến `&SystemSnapshot`; bản thân hàm và `run_audit` không thể làm sai lệch dữ liệu gốc.
- **Khớp ngữ cảnh**: từ chối ngay (`SnapshotMismatch`) nếu `host` hoặc `platform` lệch nhau, hoặc `current.captured_at < baseline.captured_at` (`InvalidTimestamp`).
- **Không bằng chứng = báo lỗi (`MissingEvidence`)**: nếu bất kỳ persistence record, artifact, account hoặc security policy nào thiếu `evidence_id` hoặc trỏ tới ID không tồn tại trong `evidence_store`, bộ kiểm toán dừng với lỗi thay vì tự bịa finding. Nếu hai snapshot giống hệt nhau, `diff.is_clean()` trả về `true` và không sinh finding nào.

---

## 4. Kiểm kê Persistence và bất biến giới hạn Severity theo Impact

Mỗi `PersistenceRecord` không chỉ lưu tên mục khởi động mà ghi đủ năm thông tin để đánh giá rủi ro và phục hồi: vị trí ảo `virtual_location`, chủ sở hữu `owner`, ngữ cảnh đặc quyền `privilege: PrivilegeLevel` (`StandardUser` < `ServiceAccount` < `LocalAdmin` < `SystemOrRoot`), `autostart` / `signature_trusted`, và liên kết tùy chọn `artifact_id` tới tệp vật chứng kèm hash SHA-256 (`Artifact`).

Một lỗi kinh điển trong báo cáo bảo mật là gán mức độ nghiêm trọng (`Severity`) tùy tiện — ví dụ nâng một phát hiện không có tác động thật lên thành `High` hoặc `Critical`. `ch110` khóa quy tắc bất biến cứng ngay tại hàm `max_severity_for_impact`:

| Mức tác động (`ImpactLevel`) | Mức `Severity` tối đa được phép |
|---|---|
| `ImpactLevel::None` | `Severity::Low` |
| `ImpactLevel::Low` | `Severity::Low` |
| `ImpactLevel::Medium` | `Severity::Medium` |
| `ImpactLevel::High` | `Severity::High` |
| `ImpactLevel::Critical` | `Severity::Critical` |

Hàm `AuditFinding::validate_invariants` và `AuditReport::validate` kiểm tra `finding.severity <= max_severity_for_impact(finding.impact)` cho từng finding lẫn từng kết quả đánh giá luồng dữ liệu. Nếu vượt ngưỡng, hệ thống trả về `PostExploitError::SeverityExceedsImpact`. Hàm tiện ích `rank_persistence_findings` sắp xếp các finding persistence theo thứ tự `Severity` giảm dần, `ImpactLevel` giảm dần và `finding_id` tăng dần giúp phân tầng xử lý nhanh chóng và hoàn toàn tất định.

---

## 5. Access Inventory và Đồ thị Lateral Movement

Trong kiểm toán hậu khai thác, câu hỏi tiếp theo sau khi liệt kê quyền truy cập (`AccessGrant`) là: *"Từ điểm đặt chân ban đầu `start_host`, các phiên/grant hiện có cho phép di chuyển ngang tới những host nào, và những cánh cửa nào chặn thành công?"*

`MockPostExploit::analyze_lateral_paths(start_host, max_depth)` sử dụng DFS dựa trên stack (`Vec::pop`) — nhất quán với bài học chống đệ quy vô hạn từ Chương 102 — nhưng bổ sung tầng kiểm chứng bốn lớp cho từng cạnh `LateralEdge`:

```text
Cạnh (from_host -> to_host, required_access, min_privilege)
  ├─► to_host không thuộc AuditScope.allowed_hosts   ──► BlockedTransition(OutOfScope)
  ├─► hops.contains(&to_host)                         ──► BlockedTransition(CycleDetected)
  ├─► edge_ids.len() >= max_depth                     ──► BlockedTransition(MaxDepthReached)
  ├─► !edge.network_reachable                         ──► BlockedTransition(NetworkSegmented)
  ├─► Không có AccessGrant khớp loại truy cập         ──► BlockedTransition(MissingAccessGrant)
  ├─► Mọi grant có expires_at <= audit_time           ──► BlockedTransition(SessionExpired)
  ├─► Grant còn hạn nhưng privilege < min_privilege   ──► BlockedTransition(InsufficientPrivilege)
  └─► Hợp lệ ──► Ghi nhận LateralPath kèm min_effective_privilege & evidence_ids
```

Đặc biệt chú ý: khi kiểm tra thời hạn phiên `AccessGrant::is_active_at(&self.audit_time)`, mốc `expires_at == audit_time` bị coi là **đã hết hạn** (`>` thay vì `>=`), tránh nhầm ranh giới phiên vừa đóng cửa thành phiên còn sống.

---

## 6. Telemetry Coverage, `TelemetryGap` và Kỹ nghệ Phát hiện

Thay vì liệt kê mẹo tắt/tránh log (điều bị nghiêm cấm trong khung đào tạo phòng thủ của giáo trình), Phần 6 của mô hình `MockPostExploit` tập trung hoàn toàn vào **tính toàn vẹn của tầm nhìn giám sát (telemetry audit)**.

Mỗi bản ghi persistence và thay đổi chính sách được ánh xạ sang `TelemetryChannel` kỳ vọng theo hệ điều hành:

- Windows `Service` → `ServiceControlManager`; Windows `ScheduledTask` → `TaskScheduler`; Windows `Autorun`/`StartupEntry` → `ProcessAudit`.
- Linux `Service` → `SystemdJournal`; Linux `StartupEntry` → `CronAudit`; Linux khác → `ProcessAudit`.
- Thay đổi chính sách → `PolicyChangeAudit`; tài khoản → `AuthAudit`; lưu lượng → `FirewallFlow`.

Khi một chính sách như `pol:task-audit` bị sửa từ `"Enabled"` sang `"Disabled"` và trỏ `disables_channel: Some(TelemetryChannel::TaskScheduler)`:

1. Engine phát hiện `SecurityPolicyTamper` cho `pol:task-audit` kèm `PolicyChangeAudit` event tương ứng (`tel:win-policy`).
2. Engine phát hiện thêm finding `TelemetryGap` riêng cho kênh `TaskScheduler` bị tắt so với baseline.
3. Trên finding `task:updater-sync`, trường `detection_gap` tự động gắn ghi chú *"Kênh TaskScheduler bị tắt trên win-srv01.corp.example nên không thu được event"* — bảo đảm **không finding nào thiếu cả `telemetry_ids` lẫn `detection_gap`**, đồng thời đi kèm `detection_rule`, `response_action` và `remediation` cụ thể.

---

## 7. Phân loại rủi ro rò rỉ dữ liệu qua `DataFlow` metadata

Để mô hình hóa đánh giá nguy cơ exfiltration mà không tạo hay vận chuyển bất kỳ payload thật nào, `DataFlow` chỉ chứa siêu dữ liệu thuần túy:

- `source_host`: host nguồn thuộc phạm vi kiểm toán.
- `destination`: tên miền `.example` hoặc địa chỉ IPv4 RFC 1918 đã chuẩn hóa.
- `destination_category`: `InternalCollector`, `AuthorizedBackup`, `UnapprovedExternal` hoặc `UnknownExternal`.
- `protocol`: `HttpsMeta`, `DnsTxtMeta`, `SmbInternalMeta`, `SftpMeta`.
- `bytes_sent`, `duration_secs` (bắt buộc `> 0`), `sensitivity` (`Public`..`RestrictedRegulated`) và `off_hours`.

Hàm `classify_data_flow` phân biệt rành mạch bốn kịch bản:

- **Backup hợp lệ (`AuthorizedBackup`)** gửi đúng tới `scope.authorized_backup_destinations` với dung lượng trong ngưỡng (`<= 50 MB`) được xếp `ExfilRiskLevel::Benign` (`ImpactLevel::None`) — không tạo báo động giả.
- **Kênh bất thường nội bộ (`DnsTxtMeta`)** vẫn bị gắn `Medium` hoặc `High` tùy độ nhạy dữ liệu vì DNS TXT không phải kênh truyền dữ liệu chuẩn.
- **Luồng ra đích `Public` ngoài hệ thống** bị khóa trần rủi ro ở `Low` / `ImpactLevel::Low` — đúng bất biến *không tự phóng đại severity khi thiếu impact*.
- **Luồng ngoài giờ hoặc kênh `DnsTxtMeta` tới `UnapprovedExternal` mang dữ liệu `Confidential`/`RestrictedRegulated`** được phân hạng `High` hoặc `Critical` kèm danh sách lý do cụ thể.

---

## 8. `CleanupPlan` nguyên tử và Kiểm chứng Baseline Lũy Đẳng

Đây là điểm phân biệt một bài tập hậu khai thác có trách nhiệm với mã kịch bản chạy bừa: mọi thay đổi thu dọn chỉ chạy trên `MockCleanupState` cô lập (clone từ hiện trạng), **không bao giờ chạm `baseline_snapshots` hay `current_snapshots`**, và phải thỏa mãn năm tính chất:

1. **Bảo vệ baseline (`BaselineProtected`)**: mọi bước cố xóa persistence, artifact hoặc `AccessGrant` có nguồn gốc từ baseline (`created_in_lab == false` hoặc có khóa trong baseline) đều bị từ chối lập tức.
2. **Kiểm chứng digest (`ArtifactDigestMismatch`)**: khi `RemoveArtifact` mang `expected_sha256: Some(...)`, hash SHA-256 phải khớp 100% trước khi xóa artifact đó.
3. **Nguyên tử toàn kế hoạch (`apply_cleanup`)**: toàn bộ thay đổi được diễn tập trước trên `let mut staged = self.cleanup_state.clone();`. Nếu bước thứ 2 trong plan thất bại vì vi phạm `BaselineProtected` hay sai hash, bước thứ 1 cũng **không** được commit.
4. **Chấp nhận artifact đã mất trước đó (`AlreadyAbsent`)**: trong thực tế kiểm toán, một tệp tạm hoặc phiên lab có thể đã hết hạn hoặc bị dọn trước khi chạy `CleanupPlan`. Nhánh `AlreadyAbsent` ghi nhận trạng thái sạch mà không làm sập toàn bộ kế hoạch.
5. **Lũy đẳng và nghiệm thu (`verify_baseline`)**: chạy `apply_cleanup(&plan)` lần thứ hai trả về `applied_mutations == 0`, trạng thái `cleanup_state` giống hệt lần đầu, và `RestorePolicy` đồng thời khôi phục các `telemetry_controls` bị tắt về trạng thái baseline. Sau cleanup, `verify_baseline()` đối chiếu từng host với `baseline_snapshots` và liệt kê mọi `residual_drifts` nếu còn sót dù chỉ một artifact hay một grant thử nghiệm.

---

## 9. Thực hành với crate `code/ch110`

### 9.1 Tóm tắt kiến trúc crate `ch110`

Crate nằm tại `code/ch110`, gồm các API công khai chính đúng theo đặc tả kiểm toán:

- **Kiểu dữ liệu cốt lõi**: `SystemSnapshot`, `PersistenceRecord`, `Artifact`, `AccessGrant`, `TelemetryEvent`, `DataFlow`, `AuditFinding`, `CleanupPlan`, `MockPostExploit`.
- **Bộ hàm phân tích & kiểm chứng**: `diff_snapshots`, `classify_data_flow`, `rank_persistence_findings`, `max_severity_for_impact`, `normalize_example_domain`, `normalize_virtual_location`, `is_rfc1918_ipv4`, `is_valid_iso8601_utc`, `sha256_hex`.
- **Động cơ kiểm toán**: `MockPostExploit::standard()`, `run_audit`, `diff_all_hosts`, `analyze_lateral_paths`, `build_cleanup_plan`, `apply_cleanup`, `verify_baseline` và `AuditReport` (`validate`, `render_json`, `render_markdown`).

### 9.2 Chạy thử demo và bộ kiểm thử

```bash
cd code
cargo run -p ch110 --locked
cargo test -p ch110 --locked
cargo clippy -p ch110 --all-targets --locked -- -D warnings
```

Output thực tế của demo:

```text
MockPostExploit — 100% synthetic, read-only snapshot audit, std-only, không chạm host/OS
Credential guard: blocked(CredentialHarvestingProhibited(từ chối tuyệt đối thao tác LsassDump: ngoài ranh giới audit hậu khai thác an toàn))
Hosts audited: 5; Findings: 12 (Persistence=4, PolicyTamper=1, TelemetryGap=1, ExfilRisk=3)
Lateral paths: reachable=2; blocked=5
Cleanup pass 1: mutations=13; pass 2: mutations=0, already_absent=13, verify_clean=true
Deterministic replay byte-identical: true
```

Crate hiện có **56 unit test** bao phủ: hai test vector SHA-256 NIST (`""` và `"abc"`), chuẩn hóa miền `.example` và ranh giới IPv4 RFC 1918, lọc nghiêm ngặt `virtual://` chống OS path/`..`/ký tự shell, validator ISO 8601 UTC năm nhuận và chặn dấu `+`, chặn secret thô (cả trong descriptor/group/precondition/detection gap) và toàn bộ hành vi `HarvestingAction`, diff sạch không sinh finding, lỗi lệch host/timestamp/thiếu evidence/dangling `artifact_id`, đủ 6 hạng mục snapshot + modified/removed persistence + modified/removed artifact + added/removed account & policy + xóa hoặc giảm `retention_days` của telemetry control, xếp hạng `rank_persistence_findings` và trần `Severity <= max_severity_for_impact(ImpactLevel)`, xác thực telemetry khớp host & subject/detection gap/precondition/remediation trên `AuditFinding`, DFS lateral chuẩn hóa `start_host` và chặn chu trình/out-of-scope/network segmentation/missing grant/ranh giới hết hạn phiên/insufficient privilege/`max_depth`, phân loại `DataFlow` cho backup chuẩn/RFC1918 không allowlist/InternalCollector giả đích/oversized/DNS TXT/Public cap/RestrictedRegulated, từ chối artifact baseline đánh dấu `created_in_lab` và artifact post-baseline gắn `created_in_lab=false`, thực thi `CleanupPlan` nguyên tử (rollback khi lỗi giữa chừng, từ chối sai SHA-256, giữ nguyên baseline & grant baseline, khôi phục modified persistence/artifact/policy/telemetry control trực tiếp, chấp nhận `AlreadyAbsent`, lũy đẳng pass 2, phát hiện tàn dư ở `verify_baseline`), kiểm chứng `AuditReport::validate` toàn vẹn (title, thứ tự, `input_hash`, `generated_at`, diffs, lateral, exfil), escape `&`/`\`/`|`/backtick an toàn trong Markdown/JSON và replay khớp từng byte.

### 9.3 Bảng tra cứu compiler error thường gặp

| Lỗi | Nguyên nhân hay gặp trong bài tập | Cách sửa |
|---|---|---|
| `E0308` — mismatched types | Truyền `&str` thay vì `Option<&str>` vào `PersistenceRecord::new(..., artifact_id, ...)` | Bọc tên artifact bằng `Some("art:id")` hoặc truyền `None` |
| `E0596` / `E0502` — cannot borrow as mutable | Đang giữ tham chiếu `&SystemSnapshot` từ `engine.baseline_snapshots()` rồi gọi `engine.apply_cleanup(&plan)` cần `&mut engine` | Kết thúc vòng đời đọc trước khi gọi `apply_cleanup`, hoặc `.clone()` snapshot sang biến riêng |
| `E0382` — use of moved value | Truyền `plan` theo giá trị vào hàm khác trước khi gọi `engine.apply_cleanup(&plan)` lần hai | `apply_cleanup` nhận `&CleanupPlan`; luôn truyền tham chiếu `&plan` |
| `E0599` — no method named `collect` hoặc `push` | Gọi `.collect()` trực tiếp khi quên `.iter()`, hoặc gọi `push` trên `BTreeSet<String>` (như `evidence_ids`) | Dùng `.iter()...collect()` hoặc `.insert(...)` trên `BTreeSet` |
| `E0603` — private field access | Cố truy cập trực tiếp trường riêng tư `engine.baseline_snapshots` hay `engine.cleanup_state` từ bên ngoài | Gọi accessor công khai `engine.baseline_snapshots()`, `engine.current_snapshots()`, `engine.cleanup_snapshot(host)` |
| `E0061` — wrong number of arguments | Thiếu tham số `artifact_id` hoặc `duration_secs` trong `PersistenceRecord::new` / `DataFlow::new` | Đối chiếu đúng danh sách tham số trong mục 9.1 hoặc lời giải mẫu |
| `E0277` — `?` conversion error | Trả `Result<(), Box<dyn std::error::Error>>` hoặc quên `.map_err` | Hàm bài tập trả `Result<(), Box<dyn std::error::Error>>` sẽ tự động chuyển `PostExploitError` qua `?` |
| `E0716` — temporary value dropped | Lấy `&SystemSnapshot` từ một giá trị `MockPostExploit::standard()` tạm thời không gán biến | Gán `let engine = MockPostExploit::standard();` trước khi mượn snapshot |

---

## 10. Bài tập rèn luyện

### Bài 1 — Đối chiếu snapshot sạch, diff có drift và phát hiện thiếu bằng chứng

Viết hàm `pub fn exercise_snapshot_diff_and_evidence_guard() -> Result<(), Box<dyn std::error::Error>>`:
1. Khởi tạo `MockPostExploit::standard()` và gọi `diff_snapshots` giữa `baseline` và `baseline` của `win-srv01.corp.example`, kiểm tra `is_clean()`.
2. Gọi `diff_snapshots` giữa `baseline` và `current` của `win-srv01.corp.example`, kiểm tra có đúng `2` persistence mới, `1` tài khoản đổi quyền và `1` policy drift.
3. Clone `current`, thay `evidence_id` của `task:updater-sync` thành `"ev:forged"` và chứng minh `diff_snapshots` trả về `Err(PostExploitError::MissingEvidence(_))`.

<details><summary>Gợi ý</summary>

Dùng `engine.baseline_snapshots().get("win-srv01.corp.example")` và `engine.current_snapshots().get("win-srv01.corp.example")`. Hàm `diff_snapshots` nhận `(&base, &cur, engine.evidence_store())`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_snapshot_diff_and_evidence_guard() -> Result<(), Box<dyn std::error::Error>> {
    let engine = MockPostExploit::standard();
    let base_win = engine
        .baseline_snapshots()
        .get("win-srv01.corp.example")
        .ok_or("thiếu baseline win-srv01")?;
    let cur_win = engine
        .current_snapshots()
        .get("win-srv01.corp.example")
        .ok_or("thiếu current win-srv01")?;

    let clean_diff = diff_snapshots(base_win, base_win, engine.evidence_store())?;
    assert!(clean_diff.is_clean());

    let actual_diff = diff_snapshots(base_win, cur_win, engine.evidence_store())?;
    assert_eq!(actual_diff.added_persistence.len(), 2);
    assert_eq!(actual_diff.modified_accounts.len(), 1);
    assert_eq!(actual_diff.policy_drifts.len(), 1);

    let mut forged = cur_win.clone();
    if let Some(task) = forged.persistence_records.get_mut("task:updater-sync") {
        task.evidence_id = "ev:forged".to_string();
    }
    assert!(matches!(
        diff_snapshots(base_win, &forged, engine.evidence_store()),
        Err(PostExploitError::MissingEvidence(_))
    ));
    Ok(())
}
```

</details>

---

### Bài 2 — Kiểm chứng trần `Severity <= ImpactLevel` và xếp hạng persistence findings

Viết hàm `pub fn exercise_rank_persistence_and_impact_cap() -> Result<(), Box<dyn std::error::Error>>`:
1. Chạy `engine.run_audit("win-srv01.corp.example", 4)?` và dùng `rank_persistence_findings(&report.findings)` để xếp hạng 4 phát hiện persistence.
2. Kiểm tra thứ tự trả về bảo đảm phần tử trước có `severity >=` phần tử sau và mục có `autostart` nhưng thuộc `StandardUser` nằm sau các mục `LocalAdmin`/`SystemOrRoot`.
3. Clone một `AuditFinding`, hạ `impact = ImpactLevel::Low` nhưng ép `severity = Severity::Critical`, rồi kiểm chứng `validate_invariants` trả về `Err(PostExploitError::SeverityExceedsImpact { .. })`.

<details><summary>Gợi ý</summary>

Kiểm tra các cặp liên tiếp bằng `ranked.windows(2)` và dùng `matches!(..., Err(PostExploitError::SeverityExceedsImpact { .. }))`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_rank_persistence_and_impact_cap() -> Result<(), Box<dyn std::error::Error>> {
    let engine = MockPostExploit::standard();
    let report = engine.run_audit("win-srv01.corp.example", 4)?;

    let ranked = rank_persistence_findings(&report.findings);
    assert_eq!(ranked.len(), 4);
    for pair in ranked.windows(2) {
        assert!(pair[0].severity >= pair[1].severity);
    }
    assert_eq!(ranked[0].severity, Severity::High);
    assert_eq!(ranked.last().unwrap().severity, Severity::Medium);

    let mut invalid = ranked[0].clone();
    invalid.impact = ImpactLevel::Low;
    invalid.severity = Severity::Critical;
    assert!(matches!(
        invalid.validate_invariants(engine.evidence_store(), engine.telemetry_events()),
        Err(PostExploitError::SeverityExceedsImpact { .. })
    ));
    Ok(())
}
```

</details>

---

### Bài 3 — Phân tích đồ thị Lateral Movement và các lý do bị chặn

Viết hàm `pub fn exercise_lateral_graph_boundaries() -> Result<(), Box<dyn std::error::Error>>`:
1. Gọi `engine.analyze_lateral_paths("win-srv01.corp.example", 4)?` để tìm các đường di chuyển ngang khả dụng (bao gồm đường 2 hop tới `files-int.corp.example`).
2. Thu thập các `LateralBlockReason` trong `blocked_transitions` và xác nhận có đủ `OutOfScope`, `NetworkSegmented`, `SessionExpired`, `InsufficientPrivilege` và `CycleDetected`.
3. Chứng minh gọi `analyze_lateral_paths("payroll-prod.corp.example", 3)` từ host ngoài scope bị chặn ngay bằng `Err(PostExploitError::OutOfScope(_))`.

<details><summary>Gợi ý</summary>

Thu thập `blocked_transitions.iter().map(|b| b.reason)` vào `BTreeSet<LateralBlockReason>`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_lateral_graph_boundaries() -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::BTreeSet;

    let engine = MockPostExploit::standard();
    let lat = engine.analyze_lateral_paths("win-srv01.corp.example", 4)?;
    assert_eq!(lat.reachable_paths.len(), 2);
    assert!(lat.reachable_paths.iter().any(|p| {
        p.hops
            == [
                "win-srv01.corp.example",
                "linux-app01.corp.example",
                "files-int.corp.example",
            ]
    }));

    let reasons: BTreeSet<LateralBlockReason> =
        lat.blocked_transitions.iter().map(|b| b.reason).collect();
    assert!(reasons.contains(&LateralBlockReason::OutOfScope));
    assert!(reasons.contains(&LateralBlockReason::NetworkSegmented));
    assert!(reasons.contains(&LateralBlockReason::SessionExpired));
    assert!(reasons.contains(&LateralBlockReason::InsufficientPrivilege));
    assert!(reasons.contains(&LateralBlockReason::CycleDetected));

    assert!(matches!(
        engine.analyze_lateral_paths("payroll-prod.corp.example", 3),
        Err(PostExploitError::OutOfScope(_))
    ));
    Ok(())
}
```

</details>

---

### Bài 4 — Đánh giá rủi ro `DataFlow` metadata thuần túy

Viết hàm `pub fn exercise_data_flow_classification() -> Result<(), Box<dyn std::error::Error>>`:
Dùng `classify_data_flow` trên `MockPostExploit::standard()` để kiểm tra ba luồng dữ liệu tự tạo:
1. Backup hợp lệ về `backup01.corp.example` qua `SftpMeta` (dưới 50 MB) → `ExfilRiskLevel::Benign`, `ImpactLevel::None`.
2. Luồng ra `UnknownExternal` (`mirror.external.example`) nhưng chỉ chứa dữ liệu `DataSensitivity::Public` → `ExfilRiskLevel::Low`, `ImpactLevel::Low` (không tự nâng khống severity).
3. Luồng ngoài giờ qua `DnsTxtMeta` tới `tunnel.unapproved.example` mang dữ liệu `DataSensitivity::RestrictedRegulated` → `ExfilRiskLevel::Critical`, `ImpactLevel::Critical`.

<details><summary>Gợi ý</summary>

Có thể tái sử dụng `evidence_id` có sẵn như `"ev:flow-backup"`, `"ev:flow-public"`, `"ev:flow-dns"` từ `MockPostExploit::standard()`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_data_flow_classification() -> Result<(), Box<dyn std::error::Error>> {
    let engine = MockPostExploit::standard();

    let ok_backup = DataFlow::new(
        "flow:ex-backup",
        "linux-app01.corp.example",
        "backup01.corp.example",
        DestinationCategory::AuthorizedBackup,
        FlowProtocol::SftpMeta,
        8_000_000,
        90,
        DataSensitivity::Internal,
        false,
        "ev:flow-backup",
    );
    let a1 = classify_data_flow(&engine.scope, &ok_backup, engine.evidence_store())?;
    assert_eq!(a1.risk, ExfilRiskLevel::Benign);
    assert_eq!(a1.impact, ImpactLevel::None);

    let public_fetch = DataFlow::new(
        "flow:ex-public",
        "win-srv01.corp.example",
        "mirror.external.example",
        DestinationCategory::UnknownExternal,
        FlowProtocol::HttpsMeta,
        16_384,
        10,
        DataSensitivity::Public,
        false,
        "ev:flow-public",
    );
    let a2 = classify_data_flow(&engine.scope, &public_fetch, engine.evidence_store())?;
    assert_eq!(a2.risk, ExfilRiskLevel::Low);
    assert_eq!(a2.impact, ImpactLevel::Low);

    let dns_reg = DataFlow::new(
        "flow:ex-dns-reg",
        "win-srv01.corp.example",
        "tunnel.unapproved.example",
        DestinationCategory::UnapprovedExternal,
        FlowProtocol::DnsTxtMeta,
        250_000,
        120,
        DataSensitivity::RestrictedRegulated,
        true,
        "ev:flow-dns",
    );
    let a3 = classify_data_flow(&engine.scope, &dns_reg, engine.evidence_store())?;
    assert_eq!(a3.risk, ExfilRiskLevel::Critical);
    assert_eq!(a3.impact, ImpactLevel::Critical);
    Ok(())
}
```

</details>

---

### Bài 5 — Thực thi `CleanupPlan` nguyên tử, xử lý artifact đã mất và kiểm chứng lũy đẳng

Viết hàm `pub fn exercise_atomic_idempotent_cleanup() -> Result<(), Box<dyn std::error::Error>>`:
1. Tạo một `CleanupPlan` xấu chứa một bước hợp lệ (`task:updater-sync`) rồi tới bước xóa `svc:baseline-monitoring` (thuộc baseline), chứng minh `apply_cleanup` trả `Err(PostExploitError::BaselineProtected(_))` và **không** xóa nhầm bước đầu tiên (tính nguyên tử).
2. Gọi `engine.build_cleanup_plan("ex5-full")?`, chèn thêm một bước `RemoveArtifact` trỏ tới `"art:already-gone"` không tồn tại để kiểm chứng xử lý `AlreadyAbsent`.
3. Chạy `apply_cleanup(&plan)` lần 1 → `verification.clean == true`, `already_absent_steps == 1`; chạy `apply_cleanup(&plan)` lần 2 → `applied_mutations == 0`, `verification.clean == true`, đồng thời `baseline_snapshots()` và `current_snapshots()` ban đầu không hề thay đổi.

<details><summary>Gợi ý</summary>

Dùng `engine.cleanup_snapshot("win-srv01.corp.example")` để kiểm tra `task:updater-sync` vẫn còn sau lỗi nguyên tử của `bad_plan`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_atomic_idempotent_cleanup() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = MockPostExploit::standard();
    let baseline_before = engine.baseline_snapshots().clone();
    let current_before = engine.current_snapshots().clone();

    let bad_plan = CleanupPlan {
        plan_id: "ex5-bad".to_string(),
        engagement_id: ENGAGEMENT_ID.to_string(),
        steps: vec![
            CleanupStep::RemovePersistence {
                host: "win-srv01.corp.example".to_string(),
                record_id: "task:updater-sync".to_string(),
            },
            CleanupStep::RemovePersistence {
                host: "win-srv01.corp.example".to_string(),
                record_id: "svc:baseline-monitoring".to_string(),
            },
        ],
    };
    assert!(matches!(
        engine.apply_cleanup(&bad_plan),
        Err(PostExploitError::BaselineProtected(_))
    ));
    assert!(
        engine
            .cleanup_snapshot("win-srv01.corp.example")
            .unwrap()
            .persistence_records
            .contains_key("task:updater-sync")
    );

    let mut plan = engine.build_cleanup_plan("ex5-full")?;
    plan.steps.push(CleanupStep::RemoveArtifact {
        host: "linux-app01.corp.example".to_string(),
        artifact_id: "art:already-gone".to_string(),
        expected_sha256: None,
    });

    let pass1 = engine.apply_cleanup(&plan)?;
    assert!(pass1.verification.clean);
    assert_eq!(pass1.already_absent_steps, 1);

    let pass2 = engine.apply_cleanup(&plan)?;
    assert_eq!(pass2.applied_mutations, 0);
    assert!(pass2.verification.clean);
    assert_eq!(engine.baseline_snapshots(), &baseline_before);
    assert_eq!(engine.current_snapshots(), &current_before);
    Ok(())
}
```

</details>

---

## 11. Tóm tắt, checklist lab và nguồn chuẩn

- Kiểm kê hậu khai thác an toàn bắt đầu bằng **baseline diff chỉ đọc**: tách bạch ảnh chuẩn và ảnh hiện trạng, mọi drift đều phải gắn `evidence_id` và `Severity` bị chặn bởi `ImpactLevel`.
- Các chỉ báo persistence (Service, ScheduledTask, StartupEntry, Autorun), thay đổi tài khoản và chỉnh sửa chính sách phải dùng định danh/vị trí ảo `virtual://`, tuyệt đối không chạm OS thật và không thu thập credential (`LsassDump`, `SamDump`, `ShadowRead`, `TokenTheft`).
- Đồ thị lateral movement phải xử lý đủ rào phạm vi (`OutOfScope`), phân vùng mạng (`NetworkSegmented`), hết hạn phiên (`SessionExpired`), thiếu đặc quyền (`InsufficientPrivilege`) và vòng lặp chu trình (`CycleDetected`).
- Thay đổi cấu hình tắt log phải hiện rõ thành `SecurityPolicyTamper` + `TelemetryGap` và gắn kèm rule phát hiện + hành động phản ứng phòng thủ thay vì hướng dẫn né tránh.
- `CleanupPlan` phải bảo vệ tài nguyên baseline (`BaselineProtected`), nguyên tử khi gặp lỗi giữa chừng, chấp nhận an toàn khi artifact đã mất (`AlreadyAbsent`), lũy đẳng khi chạy lặp và kết thúc bằng bước `verify_baseline`.

Checklist khi thực hiện audit hậu khai thác trong **lab được ủy quyền**:

- [ ] Lưu niêm phong golden baseline (hashes, services, scheduled tasks, autoruns, accounts, audit policy) trước khi chạy kịch bản kiểm thử.
- [ ] Khóa `AuditScope` rõ `allowed_hosts`, `authorized_backup_destinations` và cấm credential dumping hoặc thay đổi cấu hình hệ thống thật ngoài phạm vi lab.
- [ ] Gắn bằng chứng cụ thể và kiểm tra `Severity <= max_severity_for_impact(ImpactLevel)` cho từng finding.
- [ ] Đối chiếu từng kỹ thuật với kênh telemetry tương ứng; mọi lỗ hổng log do policy bị sửa đều phải mở mục `TelemetryGap`.
- [ ] Kiểm thử `CleanupPlan` trên snapshot giả/lab với hai lần chạy liên tiếp và chứng minh `verify_baseline` báo sạch hoàn toàn trước khi đóng phiên.
- [ ] Chạy `cargo test -p ch110 --locked`, Clippy và các công cụ kiểm tra tài liệu trước khi tích hợp thay đổi.

### Nguồn chuẩn và đọc tiếp

- [NIST SP 800-61r2 — Computer Security Incident Handling Guide](https://csrc.nist.gov/publications/detail/sp/800-61/rev-2/final): quy trình điều tra bằng chứng, containment, eradication và recovery.
- [NIST SP 800-92 — Guide to Computer Security Log Management](https://csrc.nist.gov/publications/detail/sp/800-92/final): bảo toàn nhật ký kiểm toán và phát hiện khoảng trống telemetry.
- [RFC 2606 — Reserved Top Level DNS Names](https://www.rfc-editor.org/rfc/rfc2606) và [RFC 1918 — Address Allocation for Private Internets](https://www.rfc-editor.org/rfc/rfc1918): TLD `.example` và dải địa chỉ riêng tư trong fixture tổng hợp.
- [FIPS 180-4 — Secure Hash Standard](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf): SHA-256 dùng kiểm chứng tính toàn vẹn của `Artifact` và `input_hash`.

Chương trước: [Chương 109 — Agentic OSINT & Evidence Graph](chuong_109.md). Tiếp tục theo [ROADMAP](ROADMAP.md) và outline Phần II; không có liên kết tới chương chưa được publish.
