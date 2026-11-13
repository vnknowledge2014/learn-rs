# Phần II — OSWP, Active Directory, Agentic OSINT & Red Team Methodology

> Trạng thái: **outline đã chốt cho Phase 0**. Các ID `ch106`–`ch112` là chương dự kiến, chưa phải nội dung đã publish. Không thêm chúng vào `SUMMARY.md` hoặc Cargo workspace cho tới khi có PR chương hoàn chỉnh.

## 1. Mục tiêu và ranh giới

Phần II bổ sung các năng lực còn thiếu sau ch98–105: wireless/OSWP, Active Directory, OSINT có tác tử, hậu khai thác ở dạng mô phỏng/kiểm kê, báo cáo chuyên nghiệp và capstone tích hợp.

Nguyên tắc không thay đổi:

- Đi sau ch105; không chèn hoặc đánh số lại chương cũ.
- Mỗi chương là một crate Rust 2024 self-contained, ưu tiên `std`, chạy tất định.
- Mọi target, identity, credential, capture, ticket, log và báo cáo đều là dữ liệu tổng hợp.
- Không socket Wi-Fi, monitor mode, deauthentication, LDAP/AD thật, query OSINT thật, cài persistence, thu credential hoặc exfil thật.
- Không sao chép mã từ framework tấn công. Tham khảo kiến trúc theo hướng clean-room.
- Mỗi capability phải đi kèm preconditions, evidence, detection, remediation và cleanup.
- POSIX/file tests phải dùng `#[cfg(all(test, unix))]`; không test phụ thuộc thứ tự hash map, thời gian hoặc môi trường máy.
- Cloud pentest nằm ngoài Phần II.

## 2. Dependency graph và thứ tự PR

```text
ch98–105
   │
   ├─→ ch106 OSWP ─────────────────────────────┐
   │                                           │
   └─→ ch107 AD Fundamentals                   │
            │                                  │
            └─→ ch108 AD Attacks               │
                     │                         │
ch86–91 ──→ ch109 Agentic OSINT ────────────────┤
                                               │
ch102–103 ─→ ch110 Post-Exploitation Audit ─────┤
                                               │
                     ch111 Methodology ─────────┤
                                               ▼
                                    ch112 Red Team Capstone II
```

Thứ tự merge: **106 → 107 → 108 → 109 → 110 → 111 → 112**. Một PR chỉ viết một chương; không mở PR nội dung phụ thuộc trước khi PR tiên quyết đã merge.

Mỗi PR chương phải cập nhật cùng lúc: `chuong_N.md`, `code/chN`, workspace member, `Cargo.lock`, `SUMMARY.md`, số liệu README/ROADMAP và các liên kết trước/sau. Không để link chết hoặc crate rỗng trên `main`.

## 3. Khuôn bắt buộc cho mọi chương

1. Giới thiệu, phạm vi hợp pháp và mục tiêu học tập đo được.
2. Ví dụ đời sống và sơ đồ trước khi vào thuật ngữ.
3. Cơ chế kỹ thuật; công thức/giá trị phải có nguồn chuẩn hoặc test vector.
4. Mô hình dữ liệu và bất biến trước khi viết thuật toán.
5. Crate chạy được; output demo tất định và tái lập.
6. Unit tests cho đường thành công, thất bại, biên và dữ liệu xấu.
7. Detection/defensive controls và remediation tương ứng với mỗi kỹ thuật.
8. Bảng lỗi biên dịch thường gặp.
9. Tối thiểu bốn bài tập, mỗi bài có gợi ý và lời giải khớp API crate.
10. Tóm tắt, checklist thực hành trong lab, liên kết chương trước/sau.

Không dùng số lượng dòng làm tiêu chí hoàn thành. Chương chỉ hoàn thành khi toàn bộ mục trên, CI và review kỹ thuật đều đạt.

---

## 4. ch106 — Wireless Security & OSWP Lab

**Crate/mock:** `code/ch106`, `MockWiFiLab`.

### Mục tiêu

- Đọc được beacon/capture tổng hợp và giải thích quá trình client gia nhập BSS.
- Tính và diễn giải dBm, RSSI, noise floor, SNR; phân biệt channel và channel width.
- Giải thích PMK/PTK/GTK, 4-way handshake, PMKID và điểm khác WPA2–WPA3.
- Phân tích offline capture giả, đánh giá cấu hình và nhận diện evil-twin indicators.

### Mục nội dung

1. **RF/PHY:** sóng vô tuyến; 2.4/5/6 GHz; dBm là logarit; SNR = tín hiệu − nhiễu khi cùng đơn vị dB; attenuation/interference; 20/40/80/160 MHz; channel bonding; OFDM và OFDMA/Wi-Fi 6.
2. **802.11 MAC:** BSSID/SSID; management/control/data frames; beacon, probe, authentication, association; sequence và retry; giới hạn của parser mô phỏng.
3. **Recon workflow:** vai trò của adapter, Kismet và Wireshark trong lab được cấp phép; capture và filter ở mức khái niệm; không phát lệnh radio thật.
4. **Các thế hệ bảo mật:** WEP và IV/RC4 weakness ở mức lịch sử; WPA/WPA2; PSK so với Enterprise; PMF; WPA3-SAE và forward secrecy.
5. **WPA2 key hierarchy:** PBKDF2-HMAC-SHA1(password, SSID, 4096, 256 bit) cho PMK; nonces và địa chỉ trong PTK derivation; GTK; MIC và replay/integrity.
6. **Handshake và PMKID:** bốn message; dữ liệu tối thiểu để kiểm tra offline; PMKID khác 4-way handshake; không trình bày việc thu capture ngoài phạm vi lab.
7. **Offline assessment:** mô phỏng pipeline airodump/aircrack bằng typed stages; wordlist giả; wrong-password phải fail; không tối ưu thành công cụ bẻ khóa thật.
8. **Evil twin và phòng thủ:** SSID trùng không đủ để kết luận; so BSSID, security suite, PMF, certificate/policy và signal history; không captive portal thu credential.
9. **Hardening:** WPA3/PMF, mật khẩu mạnh, Enterprise/EAP policy, segmentation, monitoring và incident response.

### API và bất biến

- `AccessPoint`, `Client`, `WirelessFrame`, `Beacon`, `SignalMeasurement`.
- `HandshakeCapture`, `PmkidRecord`, `CaptureAnalyzer`, `SecurityAssessment`.
- `MockWiFiLab::standard()` tạo dữ liệu giống nhau ở mọi lần chạy.
- Parser từ chối frame thiếu field/trailing bytes; channel/width không hợp lệ bị báo lỗi.
- MIC chỉ verify với key đúng; thay nonce, MAC hoặc message phải làm verify fail.
- Không có API nhận interface name, phát packet hoặc mở socket.

### Bài tập và nghiệm thu

Tính SNR; parse beacon; reproduce PTK/MIC trên vector tổng hợp; phân loại AP nghi evil twin; viết hardening report. Tests phải chứng minh dữ liệu xấu không panic, kết quả scan deterministic và không code path nào gọi OS/network.

---

## 5. ch107 — Active Directory Fundamentals & Enumeration

**Crate/mock:** `code/ch107`, `MockADMapper`.

### Mục tiêu

- Mô hình hóa được directory và các quan hệ bảo mật, trước khi nói về attack.
- Giải thích authentication flow và ý nghĩa của SPN, ACL, trust và GPO.
- Tạo inventory và attack-path graph từ dữ liệu tổng hợp; không kết nối DC.

### Mục nội dung

1. Domain, tree, forest, DC; SID/DN/GUID; site và replication ở mức nền tảng.
2. User, group, computer, service account; group scope/nesting; OU khác group.
3. GPO: link, inheritance, enforcement và security filtering ở mức mô hình.
4. Trust: direction, transitivity và điều kiện để một path có ý nghĩa bảo mật.
5. LDAP: DN, attribute, scope, filter; parser cho subset filter giả; paged enumeration dưới dạng iterator.
6. Kerberos: AS-REQ/AS-REP, TGT, TGS-REQ/TGS-REP, service ticket, SPN; nối với ch101.
7. ACL: rights, object owner, ACE allow/deny và inheritance; không suy diễn quyền khi thiếu dữ liệu.
8. BloodHound-style graph: node/edge typed, nguồn evidence, unknown/missing data.
9. Attack-path analysis: DFS/stack, visited set, cycle handling, max depth và path explanation.

### API và bất biến

- `AdObject`, `ObjectId`, `ObjectKind`, `GroupMembership`, `Trust`, `Gpo`.
- `AclEntry`, `AccessRight`, `LdapFilter`, `DirectorySnapshot`.
- `AdNode`, `AdEdge`, `AdGraph`, `AttackPath`, `MockADMapper`.
- Object ID duy nhất; reference tới object thiếu phải báo lỗi, không tự tạo object “ma”.
- DFS không treo khi graph có chu trình; path trả về phải gồm edges thật và đúng thứ tự.
- Trust không tự động suy ra quyền admin; mọi path đều có provenance.
- Không LDAP query, RPC, SMB hoặc Kerberos network call thật.

### Bài tập và nghiệm thu

Parse DN/filter; tính effective group nesting; dựng SPN inventory; tìm path có chu trình; giải thích vì sao một path không hợp lệ. Fixture phải có forest/trust/ACL đa dạng nhưng hoàn toàn tổng hợp.

---

## 6. ch108 — Active Directory Attack Paths & Detection

**Crate/mock:** `code/ch108`, `MockADAttacks`. Tiên quyết: ch101 và ch107.

### Mục tiêu

- Mô phỏng điều kiện tiên quyết và bằng chứng của các AD attack phổ biến trong OSCP/lab.
- Phân biệt “có dữ liệu để thử offline” với “đã chiếm được account”.
- Gắn mỗi attack với telemetry, detection và remediation.

### Mục nội dung

1. **Kerberoasting:** service account/SPN; service ticket tổng hợp; offline dictionary check; điều kiện thành công và log phòng thủ.
2. **AS-REP roasting:** pre-authentication bị tắt là precondition; encrypted response giả; wrong password fail.
3. **Delegation:** unconstrained, constrained và resource-based constrained delegation; target allowlist và những path không hợp lệ.
4. **ACL abuse paths:** GenericAll, WriteDacl, WriteOwner, ForceChangePassword và replication rights dưới dạng graph actions mô phỏng.
5. **DCSync:** replication permissions là precondition; output chỉ là secret giả; không implementation RPC replication.
6. **PtH/OPtH:** mô hình identity/ticket và khái niệm; không process injection, credential dumping hay token manipulation thật.
7. **Attack-path composition:** nối finding ch107–108 nhưng kiểm tra precondition tại mỗi bước; không báo thành công theo chuỗi suy diễn.
8. **Detection & remediation:** event types tổng hợp, managed service accounts, strong password, tiering, least privilege, delegation hygiene và ACL review.

### API và bất biến

- `SyntheticTicket`, `SyntheticCredential`, `DelegationRule`, `AttackTechnique`.
- `AttackAttempt`, `AttackFinding`, `Evidence`, `DetectionEvent`, `Remediation`.
- `MockADAttacks::attempt(...)` phải kiểm tra scope và preconditions trước khi trả finding.
- Không secret thật; fixture dùng chuỗi rõ ràng như `SYNTHETIC-NOT-A-REAL-HASH`.
- Không finding khi thiếu quyền; constrained delegation chỉ cho phép target được cấu hình.
- DCSync fail nếu thiếu replication rights; mọi thay đổi ACL giả đều có rollback/cleanup record.

### Bài tập và nghiệm thu

Phân loại account roastable; chứng minh AS-REP precondition; kiểm tra constrained target; dựng ACL attack path và detection rule. Tests bao phủ cả đường bị từ chối, không chỉ thành công.

---

## 7. ch109 — Agentic OSINT & Evidence Graph

**Crate/mock:** `code/ch109`, `MockOSINTAgent`. Tiên quyết: ch86–91 cho agent harness.

### Mục tiêu

- Xây dựng ReAct agent có tool registry typed, budget, checkpoint và khả năng dừng.
- Kết hợp nhiều nguồn passive giả thành entity/evidence graph có provenance.
- Sinh report tái lập; không query dịch vụ thật hoặc thu dữ liệu cá nhân.

### Mục nội dung

1. Scope, authorization, passive/active boundary, privacy minimization và nguồn dữ liệu được phép.
2. ReAct loop: observe → decide → tool call → validate → checkpoint → report; max steps/tool calls và cycle detection.
3. Tool registry: typed request/result, timeout/error giả, schema validation và capability allowlist.
4. Sáu nguồn mock: WHOIS, DNS, CT logs, GitHub, social metadata, Wayback; dữ liệu chỉ dùng `.example`, RFC1918 và identity tổng hợp.
5. Entity resolution: normalize domain/name, exact identifiers, merge rules, conflict và unknown; không suy đoán danh tính cá nhân.
6. Evidence graph: entity/observation/source nodes; `supports`, `contradicts`, `same-as` edges; provenance và confidence có giải thích.
7. Staged/resumable pipeline: stage status, input hash, idempotency, checkpoint version và recovery sau lỗi giả.
8. Report: tách fact/inference/unknown; citation tới evidence ID; output Markdown/JSON deterministic và escape dữ liệu đúng.
9. Evaluation: golden fixture, completeness, unsupported-claim rate, duplicate rate, replay equality và budget exhaustion.

### API và bất biến

- `OsintTool`, `ToolRegistry`, `ToolRequest`, `ToolResult`, `SourceKind`.
- `Entity`, `EntityId`, `Observation`, `Evidence`, `EvidenceGraph`, `Confidence`.
- `AgentState`, `AgentAction`, `Stage`, `Checkpoint`, `MockOSINTAgent`, `OsintReport`.
- Timestamp test dùng ISO 8601 cố định, ví dụ `2026-01-01T00:00:00Z`.
- Mọi fact phải truy được tới source; confidence nằm trong `[0, 1]`; conflict không bị xóa lặng lẽ.
- Resume không nhân đôi entity/evidence; cùng input và checkpoint phải cho cùng output.
- Hết budget phải dừng có lý do; không vòng lặp vô hạn, không network/API key.

### Tham khảo kiến trúc clean-room

- [SYNINT](https://github.com/gs-ai/SYNINT): local-first agents và staged/resumable pipelines.
- [SpiderFoot fork](https://github.com/poppopjmp/spiderfoot): module registry và agent phân tích.
- [argus-os](https://github.com/AXRoux/argus-os): investigation/tool chaining.
- [agentic-osint-agent](https://github.com/thunderstornX/agentic-osint-agent): ReAct/LangGraph và evidence-based reporting.

### Bài tập và nghiệm thu

Thêm mock tool; merge hai entity cùng ID; biểu diễn mâu thuẫn; kill và resume agent; sinh report có citation. Tests phải bắt unsupported claim và duplicate sau resume.

---

## 8. ch110 — Post-Exploitation & Persistence Audit

**Crate/mock:** `code/ch110`, `MockPostExploit`.

### Mục tiêu

- Kiểm kê trạng thái hậu khai thác trên snapshot giả và nhận diện persistence indicators.
- Mô hình hóa access/lateral path và defensive telemetry mà không thao tác host thật.
- Tạo cleanup plan kiểm chứng được và idempotent.

### Mục nội dung

1. Baseline/diff của services, scheduled tasks, startup entries, autoruns, accounts và security policy.
2. Persistence audit: location, owner, privilege, evidence, persistence mechanism classification và remediation.
3. Access inventory: quyền hiện có và phiên truy cập giả; không credential dumping/harvesting.
4. Lateral-movement graph: source/target, required access, allowed scope, evidence và blocked transitions.
5. Security-control telemetry: log coverage, detection gaps và cảnh báo; trọng tâm là audit/detection, không hướng dẫn evasion thao tác.
6. Exfil-risk classification: simulated flow metadata, volume, destination category và sensitivity; không payload hoặc transfer thật.
7. Cleanup: xóa artifact trong mock state, verify baseline, xử lý artifact đã mất và chạy lại idempotent.
8. Detection engineering: map finding → telemetry → detection → response.

### API và bất biến

- `SystemSnapshot`, `PersistenceRecord`, `Artifact`, `AccessGrant`.
- `TelemetryEvent`, `DataFlow`, `AuditFinding`, `CleanupPlan`, `MockPostExploit`.
- Audit read-only trên snapshot; mọi mutation chỉ nằm trong mock cleanup state.
- Không finding khi diff không có evidence; severity không tự tăng nếu thiếu impact.
- Cleanup lần hai không lỗi/không đổi state; verify phát hiện artifact còn sót.
- Không path hệ thống bắt buộc, shell command, process, registry, service hoặc file write thật.

### Bài tập và nghiệm thu

Diff snapshot; xếp hạng persistence finding; chặn lateral path ngoài scope; phát hiện exfil-risk flow giả; hoàn tất cleanup. Tests chứng minh không chạm host và rollback đầy đủ.

---

## 9. ch111 — Methodology, Reporting, CVSS & Legal

**Crate/mock:** `code/ch111`, report engine trên finding tổng hợp.

### Mục tiêu

- Biến kết quả kỹ thuật thành engagement có scope, evidence và report chuyên nghiệp.
- Tính CVSS 3.1 Base Score đúng vector và kiểm thử bằng vector chuẩn.
- Áp dụng stop conditions, data handling và trách nhiệm pháp lý/người vận hành agent.

### Mục nội dung

1. Written authorization, Rules of Engagement, scope allow/deny, third-party dependency và stop conditions.
2. Lifecycle: prepare → recon → validate → exploit trong lab → post-exploitation audit → cleanup → report → retest.
3. Evidence chain: timestamp, source, hash/reference, integrity và redaction; không đưa secret vào report.
4. Finding lifecycle: draft, verified, duplicate, false positive, accepted risk, remediated, retested.
5. CVSS 3.1: Base metrics, impact/exploitability, Scope changed/unchanged, roundup; phân biệt severity và business priority.
6. Report structure: executive summary, scope, methodology, finding, evidence, reproduction trong lab, impact, remediation và cleanup.
7. Deterministic renderer: stable ordering, escaping, JSON/Markdown schema version và citation.
8. Legal/ethics: consent, data minimization, disclosure, retention/deletion, agent approval và audit log.

### API và bất biến

- `Scope`, `Engagement`, `RulesOfEngagement`, `EvidenceRef`, `Finding`, `FindingStatus`.
- `Cvss31Vector`, `Cvss31Score`, `Report`, `ReportRenderer`.
- Vector chuẩn `AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H` phải cho **9.8**.
- Out-of-scope bị từ chối trước khi chạy module; unverified finding không được ghi là đã khai thác.
- Cùng findings cho cùng bytes output; duplicate được gộp nhưng giữ mọi evidence references.
- Report không chứa raw credential/secret; schema version là bắt buộc.

### Bài tập và nghiệm thu

Viết RoE; tính thêm CVSS vectors; deduplicate findings; sinh executive/technical report; xử lý retest. Test vector CVSS và snapshot report là bắt buộc.

---

## 10. ch112 — Red Team Capstone II: Multi-Module Autonomous Agent

**Crate:** `code/ch112`; orchestrator self-contained trên mock adapters.

### Mục tiêu

- Tích hợp tri thức ch98–111 thành workflow có scope, evidence, checkpoint và report.
- Dùng một interface chung cho module mà không giả vờ import binary crates cũ.
- Đánh giá safety, replay và chất lượng report bằng tests end-to-end.

### Interface chung

```rust
pub trait PentestModule {
    fn id(&self) -> &'static str;
    fn run(
        &mut self,
        context: &EngagementContext,
        input: ModuleInput,
    ) -> Result<ModuleOutput, ModuleError>;
}
```

`ModuleInput`/`ModuleOutput` là typed enums/structs, không phải chuỗi lệnh tùy ý. Output chứa `findings`, `evidence`, `next_stage` và `checkpoint`. Adapter mô phỏng wireless, OSINT, AD enumeration/attack, post-exploitation audit và report trong crate capstone.

### Mục nội dung

1. Kiến trúc module registry, dependency graph, stage transitions và compatibility của typed input/output.
2. Scenario chính: OSINT → AD enumeration → AD finding có điều kiện → report; wireless và post-audit là nhánh fixture bổ sung.
3. Engagement safety: scope gate, approval gate, tool/cost/time budget, kill switch và prohibited action.
4. Evidence/provenance: stable IDs, integrity chain, finding deduplication và unsupported-claim rejection.
5. Checkpoint/resume: version, completed stage, deterministic replay và recovery khi module fail.
6. Red-vs-blue: deterministic policies, telemetry, blocked action và feedback; không tự lan truyền hay tác động host.
7. RL reward: dùng bảng penalty **magnitude dương** và trừ trong công thức; không nhầm dấu như lỗi đã sửa ở ch105.
8. Evaluation: golden scenario, safety violations = 0, false-positive handling, replay equality, resume và report completeness.
9. Final report: scope, module trace, evidence, findings, remediation, cleanup và limitations.

### Bất biến và nghiệm thu

- Out-of-scope hoặc thiếu approval dừng trước module đầu tiên.
- Mọi finding phải có evidence; mọi stage transition phải hợp lệ.
- Resume không chạy lại stage đã commit và không nhân đôi evidence.
- Red action bị Blue chặn không được tính là thành công.
- Cùng seed/input cho cùng trace, reward và report bytes.
- Demo và toàn bộ tests chỉ dùng synthetic fixtures; không network, shell, credential hoặc persistence thật.

---

## 11. Definition of Done toàn Phần II

- Bảy chương xuất hiện liên tục sau ch105, có Chủ đề 29 trong `SUMMARY.md` và đủ crates.
- Số liệu README/ROADMAP được đo lại, không copy ước lượng; `Cargo.lock` được commit.
- `check-docs.py`, `check-book-code.py`, `check-katex.py` quét cả bảy chương.
- `cargo fmt`, clippy `-D warnings`, test workspace, book-code, docs, mdBook, macOS, Windows và audit đều xanh; không bỏ qua job `continue-on-error`.
- Không có lỗi kỹ thuật đã được `AGENTS.md` liệt kê quay trở lại.
- Review độc lập cho từng chương về correctness, safety, cross-reference, exercises và API lời giải.
- Cloud vẫn nằm ngoài phạm vi; không âm thầm thêm vào capstone.
