# Chương 108: Active Directory Attack Paths & Detection — Roasting, Delegation, ACL, DCSync, PtH/OPtH & Precondition Graph

> **Phạm vi hợp pháp và giới hạn bắt buộc:** chương này phục vụ học OSCP/Active Directory security trong lab riêng có ủy quyền. `MockADAttacks` chỉ xử lý fixture tổng hợp trong bộ nhớ. Không có credential dump, password/hash/ticket thật, process injection, token manipulation, replication RPC, LDAP/SMB/Kerberos network call, socket hay lệnh hệ điều hành. Không có lệnh tấn công vận hành được. `SYNTHETIC-NOT-A-REAL-HASH`, `SYNTHETIC-CANDIDATE-*`, ticket label và mọi identity `.example` đều không dùng được ngoài mô hình. Đây là clean-room state machine cho precondition, evidence, detection và remediation, không phải công cụ tấn công AD.

## 1. Từ chuỗi cửa trong tòa nhà tới precondition graph

Hãy tưởng tượng một tòa nhà có phòng lễ tân, phòng máy chủ và kho hồ sơ. Một người có thể:

1. Nhìn thấy biển tên phòng máy chủ.
2. Xin một phiếu tiếp nhận có mã giả.
3. Thử đối chiếu mã giả với danh sách mã trong cuốn sổ.
4. Được một nhân viên khác ủy quyền đi qua cửa.
5. Có quyền sửa danh sách người được vào kho hồ sơ.

Năm sự kiện này **không tương đương nhau**. Nhìn thấy biển tên không phải mở được cửa. Có phiếu không chứng minh đã biết mã chủ phòng. Đối chiếu đúng một mã trong cuốn sổ giả cũng không chứng minh đã chiếm tài khoản thật. Muốn đi từ sảnh tới kho hồ sơ, mỗi cánh cửa cần chìa khóa, giờ mở cửa, người được phép và bằng chứng riêng; cửa trước mở không làm cửa sau tự mở.

```text
Hành lang đời thực                         Mô hình AD trong chương

Biển tên phòng dịch vụ        ─────────►   SPN + service account
Phiếu tiếp nhận mã giả        ─────────►   synthetic service/AS-REP artifact
Cuốn sổ mã để đối chiếu       ─────────►   offline dictionary candidate giả
Quy tắc ủy quyền theo cửa     ─────────►   delegation rule + target allowlist
Quyền sửa sổ kiểm soát cửa    ─────────►   ACL right trên object
Nhật ký bảo vệ + biên bản     ─────────►   DetectionEvent + CleanupRecord

Path chỉ tồn tại khi:
[đúng scope] AND [evidence] AND [P edge 1] AND [P edge 2] AND ... AND [liên tiếp]
```

Active Directory cũng vậy: một attack path là chuỗi hành động có **điều kiện tiên quyết (precondition)**, không phải hình vẽ nối hai tên bằng mũi tên. Chương này dùng graph từ [Chương 107](chuong_107.md) làm bản đồ quan hệ, rồi thêm một lớp thực thi mô phỏng: mỗi edge phải được `attempt` kiểm tra lại trên trạng thái hiện tại.

### Mục tiêu học tập

Sau chương này, bạn có thể:

- Giải thích Kerberoasting và AS-REP roasting mà không trộn “có roastable material” với “đã chiếm account”.
- Phân biệt unconstrained, constrained và resource-based constrained delegation; kiểm tra target allowlist và chiều của RBCD.
- Mô hình `GenericAll`, `WriteDacl`, `WriteOwner`, `ForceChangePassword` và replication rights bằng typed actions.
- Chứng minh mọi ACL mutation giả đều có snapshot trước/sau, rollback và cleanup evidence.
- Giải thích vì sao DCSync cần đồng thời `ReplicationGetChanges` và `ReplicationGetChangesAll` trên domain object.
- Phân biệt Pass-the-Hash (PtH) với Overpass-the-Hash (OPtH) ở mức identity/ticket state transition.
- Ghép attack path chỉ khi mọi edge có evidence, đúng scope và thỏa toàn bộ precondition.
- Đọc typed telemetry, thiết kế remediation và retest bằng cùng attempt sau khi control được áp dụng.
- Xử lý wrong candidate, dữ liệu malformed, unknown evidence, chu trình, path rời rạc và giới hạn depth bằng lỗi tường minh.

Nên đọc Chương 25–30 cho graph, Chương 101 cho Kerberos/mật mã và Chương 107 cho AD object, ACL, evidence và typed graph. Crate `ch108` dùng Rust 2024, thuần `std`, `#![forbid(unsafe_code)]`, không dependency ngoài.

---

## 2. Kiến trúc dữ liệu: không thể tự khai báo thành công

### 2.1 Ba lớp phải tách nhau

| Lớp | Type chính | Trả lời câu hỏi |
|---|---|---|
| Sự thật của fixture | `Account`, `AclState`, `DelegationRule` | Directory snapshot hiện chứa gì? |
| Yêu cầu mô phỏng | `AttackAttempt`, `AttackTechnique` | Ai muốn thử kỹ thuật nào trên target nào? |
| Kết quả có thể kiểm toán | `AttackFinding`, `Evidence`, `DetectionEvent`, `Remediation` | Điều gì xảy ra, dựa trên đâu, phát hiện và sửa thế nào? |

Caller không truyền credential bytes hay ticket bytes tùy ý để tự tạo success. `AttackAttempt` chỉ chứa **ID** như `cred-kerb-web` hoặc `ticket-service-bob`; engine resolve ID trong registry của `MockADAttacks::standard()`. Nếu ID không tồn tại, artifact không đúng nhãn tổng hợp, owner/use sai, evidence không khớp hoặc precondition không còn, attempt bị từ chối.

Một nhãn `SYNTHETIC-*` tự chèn **không đủ** để trở thành artifact của fixture. Engine giữ issuance snapshot riêng cho credential, ticket và evidence ngay khi tạo `standard()`; mỗi attempt đối chiếu registry public với snapshot đó. Caller không thể thêm ID mới, đổi owner/use/verifier của credential, rebind ticket hay thay nội dung evidence rồi tự tạo finding. Riêng `credential.usable` là live state được phép đổi để mô phỏng revocation. Snapshot này là bất biến của mô hình dữ liệu trong bộ nhớ, không phải chữ ký số hay cơ chế xác thực nguồn cho hệ thống thật.

### 2.2 Scope được kiểm tra trước technique

`EngagementScope` gồm bốn tập: engagement ID, technique, actor và target. Một account tồn tại trong fixture vẫn có thể nằm ngoài phạm vi; ví dụ `outside.example` tồn tại nhưng không thuộc target scope. Engine kiểm tra:

```text
attempt.engagement == scope.id
AND technique ∈ scope.techniques
AND actor       ∈ scope.actors
AND target      ∈ scope.targets
AND actor/target là known identities
AND mọi evidence ID đã đăng ký, hợp lệ và đúng loại
AND mọi precondition riêng của technique
```

Thứ tự này tránh việc probe một target ngoài scope để suy ra cấu hình của nó. Lỗi scope cũng tạo `DetectionEvent` với `TelemetryKind::ScopeEnforcement`, nhưng không tạo `AttackFinding`.

### 2.3 Evidence là điều kiện, không phải trang trí

Mỗi `Evidence` có `id`, `source`, `detail`. Source hợp lệ trong fixture bắt đầu bằng `synthetic-`; ID rỗng, source giả mạo hoặc detail rỗng bị từ chối. Ngoài kiểm tra hình thức, toàn bộ record phải bằng issuance record của fixture; thay `detail` bằng một claim khác cũng làm ID trở thành `UnknownEvidence`. Attempt phải có evidence chung của technique và evidence gắn với credential, ticket, ACL grant hoặc delegation rule được dùng.

Mỗi technique có allowlist evidence type: delegation nhận `EVIDENCE_DELEGATION` và `EVIDENCE_TICKET`; các technique khác chỉ nhận evidence loại tương ứng. Thêm evidence đúng registry nhưng không liên quan vẫn trả `UnrelatedEvidence`. Delegation rule phải gắn `EVIDENCE_DELEGATION`, ACL action gắn `EVIDENCE_ACL`, còn cả hai replication grant của DCSync phải gắn `EVIDENCE_REPLICATION`; không thể rebind một grant/rule sang evidence khác rồi thêm ID đó vào attempt để qua validator.

Các hằng số công khai gồm `EVIDENCE_KERBEROS`, `EVIDENCE_ASREP`, `EVIDENCE_DELEGATION`, `EVIDENCE_TICKET`, `EVIDENCE_ACL`, `EVIDENCE_REPLICATION` và `EVIDENCE_IDENTITY`. Chúng chỉ trỏ tới dữ liệu tổng hợp; không phải cơ chế xác thực nguồn log ngoài đời.

### 2.4 Bốn trạng thái cần nói khác nhau

| Phát biểu | `FindingOutcome`/kết quả | Có chứng minh account compromise? |
|---|---|---|
| Có SPN và synthetic service/AS-REP material | `RoastableMaterial` | **Không**, `NotEstablished` |
| Một candidate khớp verifier tổng hợp | `OfflineDictionaryMatch` | **Không**, vẫn `NotEstablished` |
| Không candidate nào khớp | `WrongSyntheticCandidate` | Không; đây là error, không phải finding |
| Một ACL/delegation/PtH/OPtH action được mô phỏng | Outcome tương ứng | Chỉ `SyntheticStateOnly`, không phải hệ thống thật |

`offline_verifier` chỉ là chuỗi để test so khớp exact. Không có password, salt, hash computation, encryption, key derivation hay mật mã thật trong crate. Một match giả không cung cấp credential mới cho edge kế tiếp.

---

## 3. Kerberoasting: service ticket và offline hypothesis

### 3.1 Cơ chế cần hiểu

Trong Kerberos, service principal name (SPN) liên kết một service instance với service account. Client có TGT có thể yêu cầu service ticket cho SPN. Ở mức khái niệm, một phần service ticket được bảo vệ bằng key liên quan tới service account; nếu secret gốc yếu, dữ liệu ticket có thể trở thành vật liệu cho thử nghiệm offline. Hoạt động này thường được gọi là **Kerberoasting**.

Điều đó không có nghĩa mọi SPN đều dễ bị chiếm. Secret dài ngẫu nhiên, managed service account, cấu hình mật mã hiện đại, đặc quyền tối thiểu và monitoring làm thay đổi hoàn toàn rủi ro. Chương không triển khai thuật toán bẻ khóa hay cấu trúc ticket thật.

Trong mô hình:

- target phải là `ServiceAccount`, có SPN dạng `service/instance` không rỗng, không whitespace hay `/` thừa, và `service_ticket_available=true`;
- `SyntheticCredential.intended_use` phải là `ServiceTicketVerifier`;
- credential phải thuộc đúng target, chưa revoke và có `artifact == SYNTHETIC-NOT-A-REAL-HASH`;
- attempt phải có `EVIDENCE_KERBEROS`;
- dictionary rỗng chỉ trả `RoastableMaterial`;
- dictionary có `SYNTHETIC-CANDIDATE-WEB` trả `OfflineDictionaryMatch` cho fixture `svc-web@lab.example`;
- dictionary chỉ chứa candidate sai trả `WrongSyntheticCandidate`.

Service ticket không phải TGT, và Kerberoasting không phải AS-REP roasting. PA-TGS-REQ trong trao đổi TGS chứa AP-REQ với TGT và authenticator; không được gọi phần đó là AS-REP.

### 3.2 Detection, remediation và retest

- **Detection:** trên lab có audit Kerberos, gom event 4769 theo account/client/SPN/encryption type và tìm chuỗi request bất thường so với baseline. Một 4769 là hoạt động bình thường; không đủ kết luận roasting hay account compromise. Event 4769 trong crate chỉ là `synthetic_event_id` minh họa.
- **Remediation:** chuyển sang gMSA/secret dài theo chính sách, ưu tiên AES, bỏ SPN/account stale, giảm quyền service account. Trong mock, `apply_remediation(Kerberoasting)` tắt `service_ticket_available` và revoke verifier tương ứng.
- **Retest:** chạy lại **cùng** `AttackAttempt`; phải nhận `MissingPrecondition` trước offline dictionary check, không phải đổi candidate cho tới khi test xanh.

---

## 4. AS-REP roasting: pre-authentication là ranh giới

Kerberos pre-authentication yêu cầu client chứng minh biết credential trước khi nhận phản hồi cấp TGT. Nếu account được cấu hình không bắt buộc pre-authentication, phản hồi có thể chứa vật liệu cho thử nghiệm offline; đây là ý tưởng của **AS-REP roasting**.

Precondition trong mock rất cố ý:

1. target là `User`;
2. `preauthentication_required=false`;
3. credential giả có `CredentialUse::AsRepVerifier`, đúng owner và evidence AS-REP;
4. chỉ sau đó mới đến bước dictionary.

Fixture dùng `legacy@lab.example`. Bật pre-authentication cho account này làm attempt thất bại ngay cả khi dictionary có candidate đúng. Ngược lại, tắt pre-authentication không bảo đảm dictionary match: candidate sai vẫn trả `WrongSyntheticCandidate`.

- **Detection:** theo dõi thay đổi account policy và chuỗi AS-REQ/AS-REP bất thường trên lab; event 4768 có thể hỗ trợ phân tích TGT request nhưng cần ngữ cảnh. Không có telemetry nào được crate gửi tới Windows.
- **Remediation:** bật lại pre-authentication, xóa ngoại lệ legacy không còn lý do, review account tương tự. Mock đặt `preauthentication_required=true` và revoke AS-REP verifier.
- **Retest:** attempt cũ phải fail vì precondition, không phải chỉ vì wrong candidate.

---

## 5. Delegation: ủy quyền không phải “đi đâu cũng được”

### 5.1 Ba loại delegation

| Loại | Ý nghĩa khái niệm | Điều mock kiểm tra |
|---|---|---|
| **Unconstrained delegation** | Hệ thống được cấu hình ở mức rộng có thể giữ material phục vụ ủy quyền của identity kết nối tới nó | Rule thuộc principal, TGT tổng hợp của client khác, realm và service `krbtgt/lab.example` |
| **Constrained delegation** | Principal chỉ được ủy quyền tới tập service/resource được liệt kê; các cơ chế S4U liên quan cần hiểu theo cấu hình thật | Rule + service ticket gắn principal + `allowed_targets` chứa target |
| **Resource-based constrained delegation (RBCD)** | Rule nằm trên **resource/target** cho phép principal được ủy quyền tới resource đó | `rule.resource == attempt.target`, principal khớp và service ticket đúng binding |

Với RBCD, chiều rất quan trọng: không phải principal tự khai báo “tôi được đi tới mọi resource”; resource chứa rule cho phép principal. Một rule giống hệt nhau đặt trên `ws01.lab.example` không cấp quyền tới `server01.lab.example`.

`SyntheticTicket` có `id`, `kind`, `realm`, `client`, `service`, `label` và `evidence_id`. Không có bytes ticket, session key hay mã hóa. Ngoài binding với issuance snapshot, validator từ chối label không bắt đầu `SYNTHETIC-TICKET-`, client trùng actor, realm khác `LAB.EXAMPLE`, TGT/service kind sai và service binding sai.

Fixture `svc-front@lab.example` có ba rule riêng để test độc lập. Constrained allowlist chỉ chứa `server01.lab.example`, nên attempt tới `ws01.lab.example` bị từ chối dù có cùng ticket và evidence.

### 5.2 Giới hạn mô hình

Crate không triển khai S4U2Self/S4U2Proxy, KDC, ticket forwarding, protected user policy hay cách Windows lưu ticket. Nó không mô phỏng mọi điều kiện cấu hình delegation của hệ thống thật. Thành công chỉ nói rằng **rule giả và ticket giả thỏa validator**.

### 5.3 Detection, remediation và retest

- **Unconstrained:** review Kerberos ticket/logon và delegation configuration; loại bỏ rule rộng, áp dụng tiering và policy chống delegation phù hợp trong lab. Retest cùng TGT giả phải fail khi rule bị xóa.
- **Constrained:** correlate 4769 với principal/resource và review allowlist. Thu nhỏ/xóa allowed target và protocol transition không cần thiết. Retest target cũ phải fail nếu không còn rule/allowlist.
- **RBCD:** theo dõi thay đổi security descriptor trên resource cùng Kerberos telemetry; event thay đổi directory như 5136 chỉ xuất hiện khi audit phù hợp. Gỡ principal khỏi rule trên resource, review ai có quyền sửa resource ACL. Retest phải chứng minh target không còn chấp nhận principal.

`remediate_and_retest` trước tiên chạy baseline `attempt` và chỉ áp dụng control nếu baseline thành công. Sau đó nó chỉ coi `MissingPrecondition` là retest thành công. Nếu baseline fail vì sai scope, thiếu evidence/credential hay precondition vốn đã không tồn tại, hàm trả lỗi ngay và không thay đổi control; không được biến lỗi đầu vào thành bằng chứng remediation hiệu quả.

---

## 6. ACL abuse actions và giao dịch rollback

### 6.1 Bốn action, bốn hiệu ứng khác nhau

| Right/action | Ý nghĩa cần nhận diện | Mutation giả trong `AclState` |
|---|---|---|
| `GenericAll` | Quyền kiểm soát rộng trên object; phải xem effective ACL và ngữ cảnh object | Thêm target vào `controlled_objects` |
| `WriteDacl` | Có thể thay đổi DACL; không tự động đồng nghĩa mọi quyền trước khi có mutation | Thêm grant `GenericAll` giả cho actor |
| `WriteOwner` | Có thể thay owner; ownership có thể dẫn tới thay đổi kiểm soát nhưng không phải kết quả tức thời ngoài mô hình | Đổi `owners[target]` thành actor |
| `ForceChangePassword` | Quyền reset secret của account trong phạm vi object; không chứng minh biết secret cũ | Thêm synthetic reset marker |

Ngoài bốn action trên, `AclRight` còn có hai replication rights cho DCSync. Engine yêu cầu exact tuple `(actor, target, right)` trong `grants`; có `WriteOwner` không suy ra `WriteDacl`, có grant trên object A không suy ra grant trên object B. `ForceChangePassword` chỉ nhận target `User`, `ServiceAccount` hoặc `Computer`; domain root không phải account có password để reset dù caller tự thêm exact grant. Effective group, deny ACE và inheritance của Windows thật nằm ngoài subset này; chương 107 đã giải thích `Unknown` khi dữ liệu chưa đủ.

### 6.2 Mutation phải có before/after và cleanup

Mỗi ACL success đi qua giao dịch trong bộ nhớ:

```text
validate scope/evidence/exact grant
        │
        ▼
clone before ──► mutate state ──► clone after
        │                              │
        └────── restore before ◄───────┘
                       │
                       ▼
       AclMutation.rolled_back = true
       CleanupRecord.restored = true
       AttackFinding(AclActionRolledBack)
```

`AclMutation` giữ toàn bộ `before` và `after`; vì vậy test không chỉ tin một cờ tự khai báo. Test kiểm tra `before != after`, mutation đúng loại, `rolled_back`, `CleanupRecord`, rồi so `lab.acl == before` về grants, owners, controlled objects và reset markers. Nếu thiếu exact grant, không có mutation hay cleanup record giả được tạo.

Nếu target đã ở đúng trạng thái đích — ví dụ đã có controlled marker, owner đã là actor hoặc grant kết quả đã tồn tại — thì `before == after`. Engine trả `AttackError::NoStateChange`, ghi rejection telemetry và **không** tạo `AclMutation`, `CleanupRecord` hay finding `AclActionRolledBack`. Một no-op không được phép giả vờ là thay đổi đã được rollback.

Rollback tự động là quy ước của lab để mỗi bài test độc lập. Nó không phải lời hứa rằng một thay đổi directory thật có thể hoàn nguyên an toàn mà không điều tra phiên, replication và hệ quả phụ.

### 6.3 Detection, remediation và retest

- **Detection:** trên object lab được chọn, cấu hình SACL rồi correlate thay đổi owner/DACL/reset với principal, nguồn và change ticket. Event 5136/4670/4724 có thể liên quan tùy action và cấu hình; không phải event nào cũng tự xuất hiện. Crate dùng 5136 cho ba action control/DACL/owner và 4724 cho reset như metadata giả.
- **Remediation:** gỡ exact ACE không cần thiết, khôi phục DACL/owner theo change record, giới hạn delegated administration. Với reset, xử lý secret/phiên trong lab theo quy trình thay vì chỉ xóa log.
- **Retest:** chạy cùng actor/target/right; phải nhận missing precondition. So snapshot trước/sau cleanup và xác nhận không còn marker hay grant phát sinh. Không xóa edge khỏi graph để giả vờ directory đã được sửa.

---

## 7. DCSync: hai replication rights, không có replication RPC

DCSync là tên gọi cho cách lợi dụng quyền replication trên domain object để yêu cầu dữ liệu directory nhạy cảm thông qua cơ chế replication. Về mô hình quyền trong chương này, actor cần **đồng thời**:

- `ReplicationGetChanges`;
- `ReplicationGetChangesAll`;
- cả hai trên cùng target là domain root `lab.example`, có evidence replication.

Chỉ có một quyền, có quyền trên user thay vì domain, hoặc có hai grant cho hai actor khác nhau đều không đủ. Engine không suy ra quyền từ tên “admin”, group name hay adjacency.

Success trả `AttackFinding.synthetic_secret = Some("SYNTHETIC-NOT-A-REAL-HASH")`. Đây là một string literal rõ ràng vô dụng, không phải NT hash, Kerberos key hay credential của bất kỳ account nào. Crate không có DRSUAPI, RPC, serialization replication, SMB hay network I/O.

- **Detection:** audit replication access trên domain/DC trong lab, theo principal và nguồn được phê duyệt. 4662 có thể hữu ích khi SACL và object access audit được cấu hình đúng; cần theo dõi replication rights/GUID chứ không chỉ đếm event. Một event tổng hợp trong `DetectionEvent` không chứng minh telemetry thật đã bật.
- **Remediation:** gỡ một hoặc cả hai replication right không cần thiết trên domain root, review group/trust/delegation làm quyền hiệu lực thay đổi. Trong mock, remediation gỡ cả hai quyền khỏi fixture.
- **Retest:** hai test độc lập lần lượt bỏ từng right đều phải fail và không có secret output. Sau remediation, cùng attempt phải nhận `MissingPrecondition`.

---

## 8. PtH và OPtH: state transition, không phải credential tooling

### 8.1 Khái niệm

**Pass-the-Hash (PtH)** là khái niệm dùng dữ liệu hash trong xác thực thay vì password plaintext ở những giao thức/cấu hình chấp nhận cơ chế tương ứng. **Overpass-the-Hash (OPtH)** mô tả dùng material đó để có trạng thái Kerberos/TGT thay vì chỉ xác thực NTLM. Tên gọi không được làm ta quên rằng hệ thống thật còn phụ thuộc giao thức, policy, hardening và credential protection.

Trong crate:

- cả hai yêu cầu `SyntheticCredential` có `CredentialUse::NtHashLabel`, đúng actor, chưa revoke, artifact tổng hợp và `EVIDENCE_IDENTITY`;
- target phải là `Computer` hoặc `ServiceAccount`;
- PtH tạo `IdentityStateTransition` từ `SYNTHETIC-UNAUTHENTICATED` tới actor trên resource, `ticket=None`;
- OPtH tạo transition tương tự nhưng kèm `SyntheticTicketKind::Tgt` có label `SYNTHETIC-TICKET-OPTH`;
- cả hai đều có `CompromiseStatus::SyntheticStateOnly`.

Mỗi success tạo session key từ logical sequence của detection event vừa ghi, không chỉ từ actor. Vì vậy PtH/OPtH lặp cùng actor hoặc chuyển giữa nhiều resource không ghi đè transition cũ; `sessions()` giữ đủ audit state theo thứ tự tất định.

Không có LSASS access, credential dumping, process injection, token manipulation, logon API hay Kerberos request thật. OPtH không tạo ticket dùng được và cũng không chứng minh đã chiếm resource thật.

### 8.2 Detection, remediation và retest

- **PtH detection:** trên lab, correlate logon event như 4624 với logon type, account, source, process và baseline; xem NTLM telemetry nơi có sẵn. Không có một event ID đơn lẻ nào tự chứng minh PtH.
- **OPtH detection:** correlate TGT/TGS events như 4768/4769 với logon và cùng identity/source. Sự thay đổi từ NTLM sang Kerberos là manh mối cần ngữ cảnh, không phải verdict độc lập.
- **Remediation:** revoke material trong fixture, áp dụng credential hygiene, tiering, least privilege, giảm reuse và bảo vệ service account theo chính sách lab. Ngoài mock, credential protection và rotate sau khi có bằng chứng lộ phải theo quy trình ủy quyền.
- **Retest:** credential trở thành `usable=false`; cùng attempt fail và `sessions` không có transition mới. PtH và OPtH dùng cùng loại credential giả nên control revoke ảnh hưởng cả hai, nhưng mỗi retest vẫn chạy riêng.

---

## 9. Attack-path composition: nối điều kiện, không nối mũi tên

### 9.1 Precondition của một edge và của cả path

Với edge `e`, gọi `P(e)` là toàn bộ scope, evidence và technique preconditions. Một path hợp lệ cần:

1. attempt không rỗng và không quá 8 edge;
2. actor của edge đầu là `start`;
3. target edge trước bằng actor edge sau;
4. không lặp identity, nên cycle bị từ chối;
5. mỗi edge có evidence đã đăng ký;
6. `P(e)` đúng khi engine gọi `attempt` cho edge đó;
7. target cuối bằng goal.

```text
analyst@lab.example
  │  Kerberoasting + dictionary match giả
  │  evidence: ev-kerberos
  ▼
svc-web@lab.example
  │  PtH với cred-pth-web đã đăng ký riêng
  │  evidence: ev-identity
  ▼
server01.lab.example
```

Hai edge trên nối được trong fixture vì cả hai attempt độc lập đều hợp lệ. **Không** có suy diễn “roasting match nên tự có `cred-pth-web`”. Nếu xóa credential này, `compose_path` dừng ở step 2 với `PathFailed`, dù hai node vẫn kề nhau trên hình.

Một path không được dùng output roast làm input credential thật. Đây là điểm khác với nhiều graph chỉ vẽ quan hệ tiềm năng: `ComposedPath` chứa các `AttackFinding` đã qua engine, không chứa chuỗi tên node tự tuyên bố thành công.

### 9.2 Lỗi không được gộp thành “không có path”

| Lỗi | Ý nghĩa |
|---|---|
| `EmptyPath` | Không có attempt để kiểm tra |
| `PathTooLong` | Vượt boundary 8 step |
| `PathDisconnected` | Actor/target không liên tiếp |
| `PathCycle` | Edge quay lại identity đã thăm; bị chặn trước khi execute edge đó |
| `PathFailed { step, error }` | Đúng hình nhưng scope/evidence/precondition của một edge không đạt |
| `GoalMismatch` | Chuỗi thực thi xong nhưng không tới goal yêu cầu |

`ComposedPath::validate()` kiểm tra lại tính liên tiếp, evidence, chu trình và độ dài của kết quả công khai. Nó không thay thế `MockADAttacks::compose_path`, vì chỉ engine mới có state để retest preconditions.

### 9.3 Detection/remediation cho cả path

Không chỉ chặn edge cuối. Liệt kê telemetry và control theo từng step, chọn remediation phá precondition thực tế với ít tác động nghiệp vụ nhất, rồi chạy lại toàn path. Nếu một edge chuyển thành `Unknown` do thiếu dữ liệu thì không được gọi là an toàn; phải bổ sung evidence. `DetectionEvent` của mỗi finding giữ logical `sequence` tất định để tái lập thứ tự mà không phụ thuộc wall clock.

---

## 10. Typed telemetry và ma trận phòng thủ

### 10.1 Telemetry là dữ liệu có cấu trúc

Mỗi `DetectionEvent` có:

- `sequence: u64` tăng từ 1, không đọc đồng hồ;
- `kind: TelemetryKind` như `ServiceTicket`, `Delegation`, `DirectoryMutation`, `Replication`, `IdentityLogon`;
- `technique`, `actor`, `target`, `outcome`, `detail`;
- `evidence: Vec<Evidence>`;
- `synthetic_event_id` để liên hệ với bài học Windows auditing.

Wrong candidate có một event quan sát material và một event `OfflineDictionary/Rejected`. ACL success có event quan sát và event `RolledBack`. Mọi success đều kèm ít nhất một detection event và một `Remediation` không rỗng. Engine ghi cả rejection để test không biến im lặng thành success.

### 10.2 Ma trận từng kỹ thuật

Các event dưới đây là **gợi ý detection cho lab được cấu hình phù hợp**, không phải lời bảo đảm hệ thống mặc định luôn log. Cột retest mô tả điều kiện đo lại, không phải lệnh vận hành.

| Kỹ thuật | Detection cần correlate | Remediation cụ thể | Retest bắt buộc |
|---|---|---|---|
| Kerberoasting | 4769 theo account/source/SPN/etype và chuỗi bất thường | gMSA/secret dài, AES, gỡ SPN stale, giảm privilege | `service_ticket_available=false`; cùng attempt fail trước offline check |
| AS-REP roasting | thay đổi pre-auth policy + 4768/AS exchange theo ngữ cảnh | bật pre-authentication, xóa ngoại lệ legacy | account bắt pre-auth; attempt cũ là `MissingPrecondition` |
| Unconstrained delegation | ticket/logon theo principal + delegation configuration | gỡ rule rộng, tiering và chính sách chống delegation phù hợp | rule biến mất; TGT giả không tạo finding |
| Constrained delegation | 4769 principal/resource + thay đổi allowlist | thu nhỏ/xóa `allowed_targets`, review protocol transition | target cũ ngoài allowlist/rule và bị từ chối |
| RBCD | thay đổi security descriptor resource + Kerberos telemetry | gỡ principal trên resource, review quyền sửa resource ACL | `rule.resource` không còn cho phép actor/target |
| GenericAll | directory/owner/access events theo SACL và change ticket | gỡ ACE GenericAll, khôi phục quyền hợp lệ | exact grant biến mất; không controlled marker sau rollback |
| WriteDacl | 5136/ACL change có audit + principal/change ticket | gỡ WriteDacl và ACE phát sinh, review DACL | grant cũ fail; grant set sau cleanup bằng before |
| WriteOwner | owner/ACL change như 5136/4670 khi được log | gỡ WriteOwner, khôi phục owner theo change record | owner bằng baseline và attempt cũ fail |
| ForceChangePassword | reset events như 4724 + account/session context | gỡ reset right, xử lý secret/phiên lab theo quy trình | grant biến mất; reset marker không tồn tại sau cleanup |
| DCSync | 4662/replication access với SACL, principal và DC baseline | gỡ replication right không cần thiết trên domain root | bỏ từng right đều fail; không secret output |
| PtH | 4624/logon type/NTLM/account/source theo baseline | revoke material, tiering, least privilege, credential protection | credential unusable; không session transition mới |
| OPtH | 4768/4769 + logon cùng identity/source | revoke material, credential hygiene và protect service account | credential unusable; không TGT label/session mới |

### 10.3 Không tối ưu alert bằng cách bỏ ngữ cảnh

Detection tốt không phải “có 4769 là tấn công”. Cần baseline dịch vụ, giờ/nguồn được phép, change ticket, account sensitivity và chuỗi hành vi. Ngược lại, không có event cũng không chứng minh không có hành vi nếu audit chưa bật. `TelemetryKind` giúp rule không phải parse chuỗi tự do, nhưng không thay thế dữ liệu thu thập đầy đủ.

`Remediation` trong crate là mô tả actions/retest và thay đổi fixture tương ứng. Nó không phát PowerShell, không sửa AD, không rotate password thật và không tự xác nhận hệ quả ngoài mock.

---

## 11. Crate, demo và lỗi biên dịch thường gặp

### 11.1 Public API và bất biến

Mã đầy đủ nằm tại [`code/ch108/src/main.rs`](code/ch108/src/main.rs), manifest tại [`code/ch108/Cargo.toml`](code/ch108/Cargo.toml). Các type trung tâm:

| API | Vai trò |
|---|---|
| `MockADAttacks` | Registry + scope + validator + transaction + telemetry |
| `SyntheticTicket`, `SyntheticCredential` | Artifact labels do fixture sở hữu, không credential material |
| `DelegationRule` | Ba loại delegation, allowlist hoặc resource rõ ràng |
| `AttackTechnique`, `AttackAttempt` | Yêu cầu typed và các evidence/credential/ticket IDs |
| `AttackFinding` | Outcome, compromise status, evidence, detection, remediation, state |
| `Evidence`, `DetectionEvent` | Provenance và telemetry deterministic |
| `Remediation` | Actions cùng tiêu chí retest cho từng technique |

BTreeMap/BTreeSet giữ iteration ổn định. Không có HashMap iteration, RNG, environment hay system time. Hai lần chạy cùng chuỗi attempt trên hai fixture mới phải tạo findings, errors, events, ACL mutations và cleanup records bằng nhau từng phần.

`attempt_template` chỉ là cách lấy attempt hợp lệ của fixture để demo/test ngắn gọn. Nó không bypass validator; caller có thể sửa public fields để mô phỏng malformed data, nhưng `attempt` vẫn kiểm tra lại toàn bộ. Các map public giúp test thay đổi trạng thái phòng thủ, không phải API để đăng ký artifact mới: issuance snapshot vẫn giữ riêng và không thể ghi đè từ caller.

### 11.2 Chạy demo và kiểm thử

```bash
cd code
cargo run -p ch108 --locked
cargo test -p ch108 --locked
cargo clippy -p ch108 --all-targets --locked -- -D warnings
```

Demo tất định in các mốc sau:

```text
MockADAttacks — 100% synthetic, std-only, không network/OS/RPC
Kerberoasting material: RoastableMaterial; compromise=NotEstablished
Offline dictionary: OfflineDictionaryMatch; compromise=NotEstablished
AS-REP wrong candidate: không candidate tổng hợp nào khớp; không có offline match
Constrained allowlist: svc-front@lab.example -> server01.lab.example (DelegationApproved)
Constrained invalid target: ConstrainedDelegation: thiếu precondition: target không nằm trong constrained allowlist
WriteDacl: AclActionRolledBack; mutation_changed=true; restored=true
DCSync: SyntheticSecretDisclosed; output=SYNTHETIC-NOT-A-REAL-HASH
OPtH state: SYNTHETIC-UNAUTHENTICATED -> svc-web@lab.example; ticket_kind=Some(Tgt)
Composed path: analyst@lab.example -> server01.lab.example; validated_steps=2
Telemetry events: 11; ACL cleanup records: 1
```

Crate hiện có 41 unit test: positive/negative cho mọi technique, wrong candidate, missing/unknown/malformed/replaced/unrelated evidence, rebind rule/grant evidence, caller-inserted/rebound credential và ticket, malformed SPN, target allowlist, từng replication right, domain-root reset rejection, bốn rollback và bốn no-op ACL, unique identity sessions, path liên tiếp/cycle/disconnected/depth/unknown evidence, detection, invalid remediation baseline và determinism. Test không chỉ kiểm tra `success=true`: chúng so state snapshots, exact error, event outcome, owner/use binding và từng finding.

### 11.3 Bảng tra cứu compiler error

| Lỗi | Ví dụ nguyên nhân trong bài tập | Cách sửa |
|---|---|---|
| `E0382` — use of moved value | Dùng lại `finding.state_transition` sau khi `and_then` đã move field | Clone/kiểm tra bằng reference, hoặc destructure một lần rồi dùng giá trị đó |
| `E0502` — conflicting borrows | Giữ mutable borrow tới `lab.credentials` rồi gọi `lab.attempt` cần `&mut lab` | Kết thúc borrow trong block riêng trước khi attempt |
| `E0308` — mismatched types | Truyền `&String` nơi builder nhận `&str`, hoặc quên `Some` cho `Option` | Dùng `as_str()`/string literal; gói giá trị đúng `Option` |
| `E0599` — no variant/method | Gọi `CredentialUse::ServiceTicketVerifier` nhưng nhầm enum `AttackTechnique` | Đọc signature; technique và credential use là hai enum khác nhau |
| `E0277` — `?` không convert được | Hàm bài tập trả `Result<(), AttackError>` nhưng dùng nhiều loại lỗi | Trả `Result<(), Box<dyn std::error::Error>>` hoặc map lỗi tường minh |
| `E0603`/`E0624` — private access | Đọc trực tiếp `events`, `mutations`, `cleanup`, `sessions` từ module ngoài | Dùng `events()`, `acl_mutations()`, `cleanup_records()`, `sessions()` |
| `E0061` — wrong argument count | Gọi `DelegationRule::constrained` thiếu allowlist/evidence | Truyền principal, `BTreeSet<String>` targets và evidence ID |
| `E0716` — temporary dropped | Giữ reference tới `attempt_template(...)` hoặc `with_dictionary(...)` trong statement tạm | Tạo binding `let attempt = ...;` rồi truyền `&attempt` |
| Borrow của `Vec` sau `clone` | Tưởng `attempt.with_dictionary(...)` mutate tại chỗ | Builder nhận `self` và trả `Self`; gán `let attempt = attempt.with_dictionary(...)` |

---

## 12. Bài tập rèn luyện

### Bài 1 — Phân loại roastable account và offline match

Chứng minh `svc-web@lab.example` có Kerberoastable material, `legacy@lab.example` có AS-REP material. Với Kerberoasting, chứng minh candidate đúng tạo offline match nhưng vẫn chưa chứng minh account compromise, còn candidate sai phải fail.

<details><summary>Gợi ý</summary>

Dùng `attempt_template` cho hai technique. Dictionary rỗng và dictionary khác rỗng là hai trạng thái khác nhau; đọc cả `outcome` lẫn `compromise`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_roastable_material() -> Result<(), Box<dyn std::error::Error>> {
    let mut lab = MockADAttacks::standard();
    let kerb = lab.attempt_template(AttackTechnique::Kerberoasting);
    let material = lab.attempt(&kerb)?;
    assert_eq!(material.outcome, FindingOutcome::RoastableMaterial);
    assert_eq!(material.compromise, CompromiseStatus::NotEstablished);

    let matched = lab.attempt(
        &kerb.clone().with_dictionary(&["SYNTHETIC-CANDIDATE-WEB"]),
    )?;
    assert_eq!(matched.outcome, FindingOutcome::OfflineDictionaryMatch);
    assert_eq!(matched.compromise, CompromiseStatus::NotEstablished);
    assert_eq!(
        lab.attempt(&kerb.with_dictionary(&["SYNTHETIC-CANDIDATE-WRONG"])),
        Err(AttackError::WrongSyntheticCandidate)
    );

    let asrep = lab.attempt(&lab.attempt_template(AttackTechnique::AsRepRoasting))?;
    assert_eq!(asrep.outcome, FindingOutcome::RoastableMaterial);
    assert_eq!(asrep.compromise, CompromiseStatus::NotEstablished);
    Ok(())
}
```

</details>

### Bài 2 — Chặn constrained delegation tới target không được phép

Chứng minh `svc-front@lab.example` được phép tới `server01.lab.example` nhưng không được phép tới `ws01.lab.example` bằng constrained delegation. Sau đó áp dụng remediation và retest target hợp lệ ban đầu.

<details><summary>Gợi ý</summary>

Target ngoài allowlist cần cùng ticket và evidence để bạn biết lỗi thật sự đến từ allowlist. Dùng `remediate_and_retest` cho target hợp lệ.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_constrained_target() -> Result<(), Box<dyn std::error::Error>> {
    let mut lab = MockADAttacks::standard();
    let allowed = lab.attempt_template(AttackTechnique::ConstrainedDelegation);
    assert_eq!(
        lab.attempt(&allowed)?.outcome,
        FindingOutcome::DelegationApproved
    );

    let denied = AttackAttempt::new(
        AttackTechnique::ConstrainedDelegation,
        "svc-front@lab.example",
        "ws01.lab.example",
    )
    .with_ticket("ticket-service-bob")
    .with_evidence(EVIDENCE_DELEGATION)
    .with_evidence(EVIDENCE_TICKET);
    assert!(matches!(
        lab.attempt(&denied),
        Err(AttackError::MissingPrecondition { .. })
    ));

    lab.remediate_and_retest(&allowed)?;
    assert!(matches!(
        lab.attempt(&allowed),
        Err(AttackError::MissingPrecondition { .. })
    ));
    Ok(())
}
```

</details>

### Bài 3 — Chứng minh bốn ACL mutation đều rollback

Với `GenericAll`, `WriteDacl`, `WriteOwner` và `ForceChangePassword`, chứng minh snapshot `after` khác `before`, live state được khôi phục và có đúng một cleanup record. Rồi xóa exact grant của `WriteDacl` và chứng minh attempt thất bại mà không tạo mutation.

<details><summary>Gợi ý</summary>

Chạy mỗi action trên fixture mới để không lẫn cleanup. So toàn bộ `AclState`, không chỉ đọc cờ `restored`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_acl_rollback() -> Result<(), Box<dyn std::error::Error>> {
    for technique in [
        AttackTechnique::GenericAll,
        AttackTechnique::WriteDacl,
        AttackTechnique::WriteOwner,
        AttackTechnique::ForceChangePassword,
    ] {
        let mut lab = MockADAttacks::standard();
        let before = lab.acl.clone();
        let finding = lab.attempt(&lab.attempt_template(technique))?;
        assert_eq!(finding.outcome, FindingOutcome::AclActionRolledBack);
        let mutation = lab.acl_mutations().last().expect("mutation");
        assert_ne!(mutation.before, mutation.after);
        assert!(mutation.rolled_back);
        assert_eq!(lab.acl, before);
        assert_eq!(lab.cleanup_records().len(), 1);
        assert!(lab.cleanup_records()[0].restored);
    }

    let mut denied_lab = MockADAttacks::standard();
    denied_lab.acl.grants.retain(|grant| grant.right != AclRight::WriteDacl);
    let before = denied_lab.acl.clone();
    assert!(denied_lab
        .attempt(&denied_lab.attempt_template(AttackTechnique::WriteDacl))
        .is_err());
    assert_eq!(denied_lab.acl, before);
    assert!(denied_lab.acl_mutations().is_empty());
    Ok(())
}
```

</details>

### Bài 4 — DCSync boundary và PtH/OPtH state

Chứng minh DCSync thành công với hai replication rights nhưng fail khi thiếu từng right. Sau đó kiểm tra PtH không tạo ticket còn OPtH chỉ tạo TGT label tổng hợp.

<details><summary>Gợi ý</summary>

Dùng fixture mới cho mỗi right bị xóa. Secret DCSync phải bằng `SYNTHETIC_HASH`, và state transition không được gọi là compromise thật.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_dcsync_and_identity_states() -> Result<(), Box<dyn std::error::Error>> {
    let mut lab = MockADAttacks::standard();
    let finding = lab.attempt(&lab.attempt_template(AttackTechnique::DcSync))?;
    assert_eq!(finding.synthetic_secret.as_deref(), Some(SYNTHETIC_HASH));

    for right in [
        AclRight::ReplicationGetChanges,
        AclRight::ReplicationGetChangesAll,
    ] {
        let mut partial = MockADAttacks::standard();
        partial.acl.grants.retain(|grant| grant.right != right);
        assert!(partial
            .attempt(&partial.attempt_template(AttackTechnique::DcSync))
            .is_err());
    }

    let pth = lab.attempt(&lab.attempt_template(AttackTechnique::PassTheHash))?;
    let pth_state = pth.state_transition.expect("PtH state");
    assert_eq!(pth_state.ticket, None);
    assert_eq!(pth.compromise, CompromiseStatus::SyntheticStateOnly);

    let opth = lab.attempt(&lab.attempt_template(AttackTechnique::OverPassTheHash))?;
    let ticket = opth
        .state_transition
        .and_then(|state| state.ticket)
        .expect("synthetic TGT");
    assert_eq!(ticket.kind, SyntheticTicketKind::Tgt);
    assert!(ticket.label.starts_with("SYNTHETIC-TICKET-"));
    Ok(())
}
```

</details>

### Bài 5 — Compose path và viết detection rule typed

Ghép path hai edge từ analyst qua service account tới server. Chứng minh xóa PtH credential làm path fail ở step 2 dù topology không đổi. Trên fixture thành công, viết kiểm tra typed telemetry cho Kerberoasting/PtH và xác nhận mọi event đều có evidence.

<details><summary>Gợi ý</summary>

Roasting edge cần dictionary candidate đúng để tạo offline match, nhưng match không cấp PtH credential. `compose_path` trả `PathFailed { step: 2, .. }` khi registry mất credential.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_precondition_path_and_detection() -> Result<(), Box<dyn std::error::Error>> {
    let mut lab = MockADAttacks::standard();
    let roast = lab
        .attempt_template(AttackTechnique::Kerberoasting)
        .with_dictionary(&["SYNTHETIC-CANDIDATE-WEB"]);
    let pth = lab.attempt_template(AttackTechnique::PassTheHash);
    let path = lab.compose_path("server01.lab.example", &[roast.clone(), pth.clone()])?;
    assert!(path.validate());
    assert_eq!(path.findings.len(), 2);

    assert!(path.findings[0]
        .detection
        .iter()
        .any(|event| event.kind == TelemetryKind::OfflineDictionary
            && event.outcome == EventOutcome::Matched));
    assert!(path.findings[1]
        .detection
        .iter()
        .any(|event| event.kind == TelemetryKind::IdentityLogon));
    assert!(path
        .findings
        .iter()
        .flat_map(|finding| &finding.detection)
        .all(|event| !event.evidence.is_empty()));

    let mut broken_lab = MockADAttacks::standard();
    broken_lab.credentials.remove("cred-pth-web");
    assert!(matches!(
        broken_lab.compose_path("server01.lab.example", &[roast, pth]),
        Err(AttackError::PathFailed { step: 2, .. })
    ));
    Ok(())
}
```

</details>

---

## 13. Tóm tắt và checklist lab

- Roastable material, offline dictionary match và account compromise là ba phát biểu khác nhau; mock không bao giờ chứng minh compromise thật.
- Kerberoasting gắn với service account/SPN và service-ticket verifier; AS-REP roasting bắt buộc pre-authentication bị tắt.
- Constrained delegation phải có target allowlist; RBCD thuộc về resource/target và chỉ cho principal được cấu hình.
- ACL action cần exact grant; no-op trả `NoStateChange`, còn mọi mutation giả phải thay đổi state trong transaction rồi khôi phục, lưu before/after và cleanup evidence.
- DCSync cần đồng thời hai replication rights trên domain root và chỉ trả `SYNTHETIC-NOT-A-REAL-HASH`.
- PtH/OPtH là typed identity/ticket transitions, không phải credential dumping, process injection hay token manipulation.
- Path hợp lệ phải liên tiếp, không cycle, trong giới hạn depth, có evidence và vượt toàn bộ precondition ở từng step.
- Typed event là dữ liệu tổng hợp; detection thật phụ thuộc audit configuration và correlation.
- Remediation chỉ chạy trên baseline attempt thành công, sửa precondition ở nguồn rồi chạy lại cùng attempt; không xóa graph edge hoặc thay input để test xanh.

Checklist khi chuyển kiến thức vào **lab riêng được ủy quyền**:

- [ ] Ghi engagement, actor, target, technique, thời gian, điều kiện dừng và quyền thay đổi/rollback trong Rules of Engagement.
- [ ] Không đưa credential, hash, ticket, hostname hoặc log thật vào source/fixture của giáo trình.
- [ ] Phân biệt material, offline match, synthetic state và account compromise trong từng finding.
- [ ] Giữ evidence ID/source/detail cho từng artifact, ACL grant, delegation rule và edge.
- [ ] Review SPN, pre-authentication, delegation rule, DACL/owner và replication rights theo snapshot mới.
- [ ] Bật audit/SACL có chọn lọc trên lab và xác nhận telemetry thật sự tồn tại trước khi viết detection.
- [ ] Với mọi mutation lab, có change record, owner, cleanup, restoration proof và xử lý hệ quả phiên/secret.
- [ ] Chạy retest với cùng attempt; phân biệt `MissingPrecondition`, `UnknownEvidence`, ngoài scope và wrong candidate.
- [ ] Retest toàn path sau remediation; không suy ra path an toàn khi dữ liệu chỉ đang `Unknown`.
- [ ] Chạy `cargo test -p ch108 --locked`, Clippy và các documentation checks trước khi sửa giáo trình.

### Nguồn chuẩn và đọc tiếp

- [RFC 4120 — Kerberos Network Authentication Service (V5)](https://www.rfc-editor.org/rfc/rfc4120): AS/TGS exchange, pre-authentication và ticket framework.
- [RFC 4757 — RC4-HMAC Kerberos Encryption Types](https://www.rfc-editor.org/rfc/rfc4757): etype RC4-HMAC lịch sử; chương này không triển khai cracking.
- [Microsoft Learn — Kerberos Constrained Delegation](https://learn.microsoft.com/windows-server/identity/ad-ds/manage/kerberos-constrained-delegation-overview): khái niệm constrained delegation và S4U.
- [Microsoft Learn — Resource-based Constrained Delegation](https://learn.microsoft.com/windows-server/identity/ad-ds/manage/kerberos-constrained-delegation-overview): delegation theo resource và giới hạn ủy quyền.
- [Microsoft Learn — Active Directory Domain Services](https://learn.microsoft.com/windows-server/identity/ad-ds/active-directory-domain-services): directory, replication và administration.
- [Microsoft Learn — Audit Directory Service Changes](https://learn.microsoft.com/windows/security/threat-protection/auditing/audit-directory-service-changes): điều kiện và ý nghĩa auditing thay đổi directory.
- [Microsoft Learn — Audit Directory Service Access](https://learn.microsoft.com/windows/security/threat-protection/auditing/audit-directory-service-access): directory access auditing và event 4662 trong hệ thống được cấu hình.

Chương trước: [Chương 107 — Active Directory Fundamentals & Enumeration](chuong_107.md). Tiếp tục theo [ROADMAP](ROADMAP.md) và outline Phần II; không có liên kết tới chương chưa được publish.
