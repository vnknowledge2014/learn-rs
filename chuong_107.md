# Chương 107: Active Directory Fundamentals & Enumeration — Domain, LDAP, Kerberos, GPO, ACL & Typed Attack-Path Graph

> **Phạm vi:** chương này phục vụ học tập trong lab có ủy quyền. `MockADMapper` chỉ đọc một directory snapshot tổng hợp trong bộ nhớ. Không có LDAP/RPC/SMB query, Kerberos network call, credential thật, collector hay lệnh hệ điều hành. Các thuật toán được xây clean-room từ tài liệu công khai; chúng mô hình hóa dữ liệu và bất biến, không phải công cụ thu thập hay tấn công AD thật.

## 1. Từ sổ danh bạ tới bản đồ quan hệ có bằng chứng

Hãy tưởng tượng một tòa nhà nhiều công ty. **Danh bạ** cho biết ai ở phòng nào; **thẻ ra vào** cho biết họ được mở cửa nào; **tổ chức phòng ban** cho biết ai thuộc nhóm nào. Ba dữ liệu đó liên quan nhưng không thể thay thế nhau. Biết một người ngồi ở tầng ba không chứng minh họ có quyền mở két sắt tầng ba.

Active Directory cũng cần sự tách biệt ấy:

1. **Directory object** mô tả danh tính và vị trí trong cây.
2. **Authentication** kiểm chứng danh tính và cấp ticket.
3. **Authorization** quyết định principal được làm gì trên object.
4. **Trust, OU và GPO** mô tả quan hệ hoặc chính sách, không tự động là quyền admin.
5. **Graph** nối các quan hệ đã quan sát; mỗi edge phải giữ nguồn evidence.

```text
Snapshot tổng hợp
  ├─ objects: user · group · computer · service account · DC
  ├─ relationships: memberOf · owner · ACL · trust · GPO link
  ├─ protocol models: LDAP filter · Kerberos AS/TGS exchange
  └─ typed graph → DFS có depth/visited → path + provenance
```

### Mục tiêu học tập

Sau chương này, bạn có thể:

- Phân biệt domain, tree, forest, DC, site và replication.
- Đọc SID, DN, GUID; phân biệt OU với group và giải thích group nesting.
- Mô tả GPO link, inheritance, enforcement và security filtering ở mức mô hình.
- Đọc chiều trust mà không suy ra quyền quản trị.
- Parse một subset DN/LDAP filter và duyệt kết quả bằng paged iterator.
- Vẽ AS-REQ/AS-REP và TGS-REQ/TGS-REP; giải thích TGT, service ticket và SPN.
- Đánh giá ACL trên snapshot, giữ trạng thái `Unknown` khi dữ liệu chưa đủ.
- Dựng typed graph và tìm path bằng DFS không treo trên chu trình.
- Kiểm chứng rằng mọi edge trong path đều tồn tại, liên tiếp và có provenance.

Nên đọc Chương 25–30 cho cấu trúc dữ liệu, Chương 65 cho giao thức mạng và Chương 101 cho Kerberos/mật mã. Crate `ch107` thuần `std`, Rust 2024, tất định và không có dependency ngoài.

---

## 2. Directory: domain, tree, forest và Domain Controller

### 2.1 Bốn khái niệm không được trộn lẫn

- **Domain** là một phân vùng directory có cơ sở dữ liệu, chính sách và các security principal riêng. Domain không phải lớp vỏ bảo mật tuyệt đối nếu forest hoặc quan hệ quản trị bên ngoài bị ảnh hưởng.
- **Tree** là một hoặc nhiều domain dùng không gian tên DNS liền nhau, ví dụ `lab.example` và `child.lab.example`.
- **Forest** gồm một hoặc nhiều tree dùng chung schema, configuration và forest root. Forest thường được xem là security boundary quan trọng hơn domain.
- **Domain Controller (DC)** là server lưu và phục vụ directory, xác thực và các dịch vụ liên quan. Writable DC khác read-only DC về khả năng thay đổi dữ liệu; cả hai đều cần được bảo vệ và giám sát.

Một forest có thể chứa nhiều domain mà DNS của chúng không tạo thành một tree. Ngược lại, cùng hậu tố DNS không chứng minh hai hệ thống cùng forest.

Fixture biểu diễn inventory tối thiểu của một forest tổng hợp: `lab.example` là forest root, `child.lab.example` là child domain cùng tree, còn `other.example` là root của một tree khác trong cùng forest. `partner.example` và `vendor.example` là các domain bên ngoài chỉ xuất hiện trong trust record. Ba domain root được kiểm tra trong unit test; crate không có forest-topology engine và không tự suy ra quan hệ forest chỉ từ chuỗi DNS.

### 2.2 SID, DN và GUID trả lời ba câu hỏi khác nhau

| Định danh | Ví dụ tổng hợp | Ý nghĩa |
|---|---|---|
| SID | `S-1-5-21-1000000001-1000000002-1000000003-1101` | Security identifier của principal trong ngữ cảnh Windows/AD |
| DN | `CN=alice,OU=Users,DC=lab,DC=example` | Đường dẫn object trong cây LDAP |
| GUID | `00000000-0000-4000-8000-000000001101` | Định danh ổn định theo object trong mô hình fixture |

SID của user thường kết thúc bằng RID; `512` là RID quen thuộc của Domain Admins trong domain. DN có thể đổi khi object được di chuyển hoặc đổi tên. GUID không phải SID và DN không phải đường dẫn filesystem. Trong code, `ObjectId` là newtype riêng để không truyền nhầm tên hiển thị vào một tham chiếu đồ thị.

Fixture dùng chuỗi SID giả trong miền `S-1-5-21-…` được chọn cho lab và ID `OU-…`/`GPO-…` riêng cho container/policy. Không dùng chúng để nhận diện object trên hệ thống thật.

### 2.3 Site và replication

**Site** mô tả cấu trúc mạng, thường liên kết subnet và vị trí vật lý, giúp client chọn DC phù hợp và giúp tối ưu replication. Site không phải OU, không phải security group và không phải trust.

**Replication** đồng bộ dữ liệu directory giữa các DC. Ở mức nền tảng:

- replication trong site và giữa site có topology/schedule khác nhau;
- một thay đổi có thể chưa xuất hiện đồng đều tại mọi DC ngay lập tức;
- độ trễ replication có thể làm hai snapshot khác nhau mà không ai đang “nói sai”;
- quyền replication trên object domain là dữ liệu ACL nhạy cảm, nhưng chương này không triển khai replication RPC.

Crate chỉ gắn nhãn `SYNTHETIC-SITE-A` cho object để minh họa inventory. Nó không mô phỏng KCC, topology, USN hay replication thật.

---

## 3. Object, group và OU

### 3.1 Các loại object trong fixture

| `ObjectKind` | Vai trò | Điểm cần nhớ |
|---|---|---|
| `User` | Danh tính người dùng tổng hợp | Group membership ảnh hưởng quyền hiệu lực |
| `Group` | Tập hợp principal | Có scope và có thể nesting |
| `Computer` | Danh tính máy trạm/máy chủ | Cũng là security principal, không chỉ là thiết bị |
| `ServiceAccount` | Account gắn dịch vụ | SPN nối dịch vụ với account trong Kerberos |
| `DomainController` | DC tổng hợp | Là computer đặc biệt trong directory |
| `OrganizationalUnit` | Container cây directory | Không phải security group |
| `Domain` | Gốc phân vùng domain | Có ACL và các quan hệ mức domain |

### 3.2 Group scope và nesting

Ba scope nền tảng là **Domain Local**, **Global** và **Universal**. Nói ở mức mô hình: Global thường gom principal cùng domain, Domain Local thường được gán quyền tại tài nguyên, Universal có thể phục vụ ngữ cảnh nhiều domain trong forest. Quy tắc nesting và phạm vi sử dụng chi tiết còn phụ thuộc functional level và loại thành viên; không nén toàn bộ thành khẩu quyết “scope nào cũng như nhau”.

Nesting tạo membership gián tiếp:

```text
alice → Helpdesk → IT Operations
```

Nếu ACL cho `IT Operations` một right thì phân tích quyền của `alice` phải mở rộng cả nhóm gián tiếp. `effective_groups` dùng stack và `BTreeSet` đã visited. Một chu trình `Cycle A ↔ Cycle B` được cố ý đưa vào fixture như **dữ liệu xấu để kiểm thử thuật toán**, không phải cấu hình được khuyến nghị hay khẳng định rằng mọi AD đều chấp nhận chu trình như vậy.

### 3.3 OU khác group

- **OU** là container để tổ chức object và gắn GPO.
- **Group** là security principal để gom membership và nhận quyền.
- Đặt user trong `OU=Users` không làm user thành viên của group `Users`.
- DN suffix giúp xác định object nằm dưới container nào, nhưng không tự tạo `MemberOf` edge.

Đây là lý do `DirectorySnapshot` giữ `memberships` tách khỏi cây DN.

---

## 4. GPO: chính sách theo vị trí và điều kiện áp dụng

Một Group Policy Object có thể được link tới site, domain hoặc OU. Trong subset của crate, `GpoLink` gắn policy với DN của container và có hai cờ:

- `enabled`: link có được bật không;
- `enforced`: link có vượt qua block inheritance trong mô hình không.

Security filter được snapshot resolve thành danh sách `ObjectId` được phép áp dụng. Danh sách rỗng trong fixture nghĩa là không có giới hạn thêm trong mô hình. Hệ thống thật còn liên quan quyền **Read/Apply Group Policy**, WMI filter, thứ tự xử lý và precedence giữa nhiều policy; crate không giả vờ mô phỏng hết.

```text
Domain policy ─┐
               ├─ kế thừa xuống OU Workstations ─► WS01
OU policy ─────┘         ▲
                         └─ block inheritance chặn policy thường từ trên xuống,
                            nhưng không chặn link trực tiếp tại OU hoặc enforced link
```

`effective_gpos` trả về policy theo thứ tự `ObjectId` ổn định để test tái lập. Đây **không phải** thứ tự ưu tiên GPO của Windows. Muốn kết luận policy cuối cùng trên host thật cần Resultant Set of Policy và dữ liệu triển khai tương ứng.

---

## 5. Trust: xác thực qua ranh giới không đồng nghĩa quyền quản trị

Một trust có hai domain, chiều và tính transitive. Cách dễ nhầm nhất là đọc ngược chiều:

- **Outbound trust từ `lab.example` tới `partner.example`**: `lab.example` tin `partner.example`; identity phía partner có thể xác thực vào tài nguyên phía lab, tùy cấu hình và authorization tiếp theo.
- **Inbound trust** mô tả chiều ngược lại theo góc nhìn của domain đang xét.
- **Bidirectional** gồm cả hai chiều.
- **Transitive** cho phép xét khả năng đi qua nhiều chặng trust theo loại trust; không phải phép cộng quyền.

Fixture chứa bốn record: outbound non-transitive tới `partner.example`, inbound non-transitive từ góc nhìn `lab.example` với `vendor.example`, và hai trust bidirectional/transitive cho child domain `child.lab.example` cùng tree root `other.example`. Các test kiểm tra cả hai chiều xác thực; chúng không tạo edge quyền quản trị.

`Trust::permits_authentication(identity_domain, resource_domain)` chỉ trả lời một câu hỏi về chiều xác thực trên record tổng hợp. Nó không trả lời:

- identity có được log on vào máy cụ thể không;
- identity có thuộc group có quyền trên resource không;
- SID filtering hoặc các điều kiện triển khai khác ra sao;
- có path admin hay không.

Vì vậy `AdGraph::from_snapshot` không phát sinh edge `AdminTo` từ trust. Muốn có path bảo mật, phải có group/ACL/ownership evidence riêng và phải kiểm tra precondition ở từng bước.

---

## 6. LDAP subset: DN, attribute, scope, filter và paging

### 6.1 DN parser phải từ chối dữ liệu sai

DN của fixture dùng các RDN `CN`, `OU`, `DC`, ví dụ:

```text
CN=svc-web,OU=Services,DC=lab,DC=example
```

`DistinguishedName::parse` nhận thứ tự `CN → OU → DC`, bắt buộc ít nhất một `DC`, giữ nguyên chữ hoa/thường khi hiển thị nhưng chuẩn hóa domain về lowercase. Nó từ chối giá trị rỗng, key lạ, trailing comma và escape chưa được hỗ trợ.

Đây là **subset có chủ đích**, không phải parser RFC 4514 hoàn chỉnh: không multi-valued RDN, quoted value, UTF-8 escape đầy đủ hay mọi attribute type. Từ chối dữ liệu ngoài subset an toàn hơn parse đại thành một DN khác.

### 6.2 Attribute là đa giá trị

`AdObject::attributes` tạo map deterministic từ snapshot. Ví dụ `servicePrincipalName` có thể có nhiều giá trị và `memberOf` chứa DN của các group trực tiếp. Matching trong mock so sánh ASCII case-insensitive để bài tập dễ đọc; LDAP thật dùng matching rule theo schema, không phải mọi attribute đều cùng quy tắc.

Một filter tìm user có SPN:

```text
(&(objectClass=user)(servicePrincipalName=*))
```

AST được biểu diễn bằng enum:

```rust
#[derive(Debug, PartialEq, Eq)]
enum Filter {
    Present(String),
    Equality(String, String),
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Box<Filter>),
}

fn main() {
    let service_users = Filter::And(vec![
        Filter::Equality("objectClass".into(), "user".into()),
        Filter::Present("servicePrincipalName".into()),
    ]);
    let Filter::And(children) = &service_users else {
        unreachable!();
    };
    assert_eq!(children.len(), 2);
}
```

Parser recursive descent hỗ trợ `&`, `|`, `!`, equality và presence. Dấu `*` chỉ được chấp nhận khi đứng một mình cho presence; substring filter nằm ngoài subset. `\xx` được decode theo byte cho filter value, còn escape DN là cú pháp riêng không được đem sang áp dụng máy móc.

### 6.3 Scope và paged iterator

| Scope | Điều kiện trong snapshot |
|---|---|
| `Base` | Chỉ object có DN bằng base |
| `OneLevel` | Object có parent DN bằng base |
| `Subtree` | Base và mọi object có suffix DN dưới base |

Base lookup, `Base`, `OneLevel` và `Subtree` dùng cùng phép so sánh RDN theo ASCII case-insensitive. Vì vậy `ou=users,dc=LAB,dc=Example` vẫn chọn đúng `OU=Users,DC=lab,DC=example`; test kiểm tra cả ba scope với base đổi hoa/thường. Đây là quy ước của subset, không phải tuyên bố mọi attribute LDAP đều có cùng matching rule.

`paged_search` trả `PagedEnumeration`, mỗi lần `next()` cho một `Vec<&AdObject>`. Page size bằng 0 là lỗi; page cuối được trả đúng phần còn lại và iterator sau đó kết thúc. Vì snapshot không đổi trong lúc duyệt, mock không cần cookie. Paging LDAP thật cần cookie và phải xử lý dữ liệu thay đổi giữa các trang.

---

## 7. Kerberos: ticket là dữ kiện của flow, không phải “chìa khóa admin”

### 7.1 Hai exchange nền tảng

1. **AS-REQ → AS-REP:** client yêu cầu Authentication Service; pre-authentication được kiểm tra theo policy; response chứa TGT và phần dành cho client.
2. **TGS-REQ → TGS-REP:** client trình TGT để yêu cầu service ticket cho một SPN; response chứa ticket dành cho service và phần phiên dành cho client.
3. Client sau đó xác thực tới service bằng AP exchange phù hợp. Chương này dừng ở mô hình cấp service ticket.

Trong TGS-REQ, **PA-TGS-REQ chứa AP-REQ**, và AP-REQ gồm TGT cùng authenticator. Nó không chứa AS-REP. Crate thể hiện trực tiếp bằng `PaTgsReq { ap_req: ApReq }` để cấu trúc kiểu giúp tránh nhầm message.

```text
AS-REQ(client, realm, pre-auth giả)
       │ MockKerberosKdc kiểm tra client + proof tổng hợp
       ▼
AS-REP(TGT tổng hợp)
       │
       ▼
TGS-REQ(PA-TGS-REQ(AP-REQ(TGT, authenticator)), SPN)
       │ kiểm tra realm, client, TicketKind::Tgt và SPN tồn tại
       ▼
TGS-REP(service ticket tổng hợp)
```

### 7.2 SPN và service ticket

**Service Principal Name (SPN)** như `HTTP/web.lab.example` giúp chọn service account tương ứng. Một account có thể có nhiều SPN; inventory phải giữ một row cho mỗi SPN thay vì gộp thành chuỗi khó kiểm tra. Duplicate SPN trên deployment thật là dữ liệu cần điều tra; fixture dùng map deterministic và không triển khai duplicate detection cho directory thật.

Service ticket không chứng minh client là Domain Admin. Nó là kết quả của một exchange cho service cụ thể. Ngược lại, có SPN cũng không tự động nghĩa account có password yếu, quyền cao hay đang chạy dịch vụ đáng tin.

`EncryptionType::protocol_id` giữ các giá trị protocol cần phân biệt: DES-CBC-CRC = **1**, AES256-CTS-HMAC-SHA1-96 = **18**, RC4-HMAC = **23**, RC4-HMAC-EXP = **24**. RC4-HMAC thuộc RFC **4757**. Các số này là metadata, không phải lựa chọn khuyến nghị triển khai mới.

### 7.3 Mock không phải Kerberos implementation

`SyntheticTicket` chỉ có kind, realm, client, service, SPN, etype, nhãn phiên bắt đầu `SYNTHETIC-` và evidence. Không có ASN.1 encoding, key derivation, encryption, password, hash, nonce crypto hoặc ticket bytes. `SYNTHETIC-PREAUTH-OK` là marker kiểm thử, không phải credential. Chương 101 mới là nơi đối chiếu cơ chế mật mã; lab này tập trung vào quan hệ và validation.

---

## 8. ACL, object owner và trạng thái Unknown

### 8.1 ACE trong mô hình

Mỗi `AclEntry` gồm principal, target, `AccessRight`, `Allow`/`Deny`, cờ inherited và evidence. Các right gồm:

- `GenericRead`, `GenericAll`;
- `WriteDacl`, `WriteOwner`;
- `ForceChangePassword`;
- `ReplicationGetChanges`, `ReplicationGetChangesAll`.

Tên right mô tả dữ liệu đầu vào. Chương này không thực hiện thao tác đổi password, sửa DACL, chiếm ownership hay replication.

### 8.2 Evaluation có giới hạn rõ ràng

`access_decision` mở rộng principal bằng effective groups rồi tìm ACE khớp cả target và right:

1. Có `Deny` phù hợp → `Denied`.
2. Không có deny nhưng có `Allow` phù hợp → `Allowed`.
3. Không có ACE phù hợp → `Unknown`, kèm evidence rỗng.

Quy tắc deny-ưu-tiên là quy tắc của subset. Windows security descriptor thật còn có thứ tự ACE, inheritance/protection, object-specific rights, mandatory integrity và nhiều điều kiện khác. Snapshot của crate giả định ACL đã được resolve; cờ `inherited` là provenance, không phải engine tự tính inheritance từ container.

Fixture có cả ACE inherited `IT Operations → WS01` cho `GenericAll`, explicit deny của `Helpdesk` trên service account và `ReplicationGetChangesAll` của `Domain Admins` trên domain root. Test kiểm tra inherited evidence và các loại ACE; replication right được giữ trong inventory nhưng không tự biến thành graph action hay thao tác replication.

**Object owner không đồng nghĩa GenericAll.** Ownership có ý nghĩa bảo mật riêng và cần phân tích đúng cơ chế Windows, nhưng mock không suy từ `owner` sang mọi `AccessRight`. Nó chỉ tạo `Owns` edge có nguồn.

`Unknown` cũng không đồng nghĩa `Denied`: nó nói rằng snapshot thiếu dữ liệu để kết luận. Báo cáo phải ghi cần bổ sung ACL/effective token/group membership nào thay vì tô màu một giả định thành fact.

---

## 9. BloodHound-style typed graph và DFS có phanh

### 9.1 Graph là bản trình bày của evidence

`AdGraph` có `BTreeMap<ObjectId, AdNode>` và `Vec<AdEdge>`. Node giữ loại object; edge có `from`, `to`, `EdgeKind` và `Evidence`. Các edge trong fixture:

| Edge | Nguồn quan hệ |
|---|---|
| `MemberOf` | `GroupMembership` trực tiếp |
| `Owns` | owner attribute |
| `GenericAll` | effective allow ACL tương ứng |
| `WriteDacl`, `WriteOwner`, `ForceChangePassword` | effective allow ACL tương ứng |

`GenericRead` và replication right được giữ trong ACL/inventory nhưng không tự biến thành graph action ở chương nền tảng này. Snapshot được validate trước: ID phải duy nhất, DN không trùng, reference thiếu phải báo lỗi và evidence không được rỗng. Parser không tự tạo object “ma” để làm path đẹp hơn.

Graph cũng giữ các deny ACE gốc. Khi bắt đầu từ một user, DFS mở rộng effective principals của user đó và không cho phép một group `Allow` khác vô hiệu hóa `Deny` phù hợp. Nếu không, chuỗi `user → group có Allow → target` sẽ tạo ra một con đường vòng sai chỉ để lách deny. `AttackPath::validate` áp dụng cùng kiểm tra này thay vì chỉ so khớp hình dạng edge.

Đây là mô hình lấy cảm hứng từ graph enumeration, không phải BloodHound collector, schema tương thích hay công cụ truy vấn AD.

### 9.2 DFS dùng stack tường minh

`find_path(start, goal, max_depth)` dùng `Vec` làm stack với `pop()`. Mỗi candidate mang:

- node hiện tại;
- path các edge đã đi;
- visited set của **nhánh đó**.

Visited theo nhánh ngăn chu trình mà không để một nhánh thất bại che mất node ở nhánh khác. `max_depth` giới hạn số edge; identity path `start == goal` có 0 edge. DFS không hứa trả path ngắn nhất; nếu cần shortest path, hãy dùng BFS với queue và quy tắc riêng.

```text
alice
  │ MemberOf [synthetic-directory-export]
  ▼
Helpdesk
  │ MemberOf [synthetic-directory-export]
  ▼
IT Operations
  │ GenericAll [synthetic ACE inherited from Workstations OU]
  ▼
WS01
```

`AdGraph::from_snapshot` gọi lại validation trước khi dựng graph, kể cả khi caller chưa tự gọi `validated()`. DFS chỉ đi qua edge có cả hai endpoint là node thật. `AttackPath::validate` đi từ `start`, kiểm tra mọi endpoint trung gian, đối chiếu từng edge với graph và nối `edge.to` thành node kế tiếp. Nó từ chối node “ma”, evidence rỗng, edge bị xóa, edge không liên tiếp và goal sai. `explain()` in quan hệ cùng nguồn; `explain_no_path()` nói rõ có thể do thiếu edge, depth quá nhỏ hoặc dữ liệu chưa biết.

Một path hợp lệ về cấu trúc vẫn chưa phải hành động đã được thực thi. Nó là giả thuyết có evidence để review precondition, detection và remediation.

---

## 10. Chạy lab và đọc mã nguồn

```bash
cd code
cargo run -p ch107
cargo test -p ch107
cargo clippy -p ch107 --all-targets --locked -- -D warnings
```

Chương trình in số object, effective groups của `alice`, inventory SPN và path `alice → WS01`. Mọi kết quả được sắp xếp bằng `BTreeMap`/`BTreeSet` hoặc key tường minh; không dùng thứ tự `HashMap`, đồng hồ thật, RNG hay dữ liệu môi trường.

Bố cục `code/ch107/src/main.rs`:

| Khu vực | API chính |
|---|---|
| Định danh/object | `ObjectId`, `DistinguishedName`, `AdObject`, `ObjectKind` |
| Quan hệ/policy | `GroupMembership`, `Trust`, `Gpo`, `AclEntry` |
| LDAP | `LdapFilter`, `SearchScope`, `PagedEnumeration` |
| Snapshot | `DirectorySnapshot`, `DirectoryError`, `AccessDecision` |
| Graph | `AdNode`, `AdEdge`, `AdGraph`, `AttackPath` |
| Kerberos offline | `AsReq`, `AsRep`, `PaTgsReq`, `TgsReq`, `TgsRep`, `MockKerberosKdc` |
| Fixture | `MockADMapper::standard()` |

### Bảng lỗi thường gặp

| Triệu chứng | Nguyên nhân | Cách sửa |
|---|---|---|
| `E0308`: expected `ObjectId`, found `String` | Truyền tên hiển thị vào API định danh | Tạo/kiểm tra `ObjectId` bằng smart constructor |
| `E0599`: method không có trên `MockADMapper` | Gọi nhầm method chỉ có trên `AdGraph` | Dùng `lab.graph`, hoặc forwarding method được crate cung cấp |
| `binary_search` trả sai dù object tồn tại | Vec chưa được sort theo đúng `ObjectId` | Validate/sort snapshot trước khi index |
| `E0502`: mutable và immutable borrow chồng nhau | Sửa snapshot trong lúc iterator đang mượn object | Thu thập ID cần đổi, kết thúc borrow rồi mới mutate |
| DFS chạy mãi trên chu trình | Không có visited hoặc chỉ đệ quy không giới hạn | Dùng stack, visited theo nhánh và `max_depth` |
| Path in ra đẹp nhưng validate fail | Tự chế edge hoặc mất thứ tự `from → to` | Chỉ dựng path từ `graph.edges` và giữ evidence |
| Filter hợp lệ ngoài subset bị từ chối | Parser không hỗ trợ substring/extensible/RDN đầy đủ | Mở rộng grammar và tests, không bỏ qua lỗi parse |
| Quyền trả `Unknown` | Snapshot không có ACE phù hợp | Bổ sung evidence; không đổi thành `Allowed` để demo thành công |

---

## 11. Bài tập

### Bài 1 — Parse DN và filter có điều kiện

Parse `CN=svc-web,OU=Services,DC=lab,DC=example`, kiểm tra domain là `lab.example`. Sau đó tìm subtree bằng `(&(objectClass=user)(servicePrincipalName=*))` và chứng minh chỉ có `svc-web`.

<details><summary>Gợi ý</summary>

Dùng `DistinguishedName::parse`, `LdapFilter::parse` và `DirectorySnapshot::search`. Mọi lỗi của lab đều implement `std::error::Error`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_parse_dn_and_filter() -> Result<(), Box<dyn std::error::Error>> {
    let service_dn = DistinguishedName::parse(
        "CN=svc-web,OU=Services,DC=lab,DC=example",
    )?;
    assert_eq!(service_dn.domain(), "lab.example");
    let lab = MockADMapper::standard();
    let filter = LdapFilter::parse(
        "(&(objectClass=user)(servicePrincipalName=*))",
    )?;
    let found = lab.snapshot.search(
        &DistinguishedName::parse("DC=lab,DC=example")?,
        SearchScope::Subtree,
        &filter,
    )?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].dn, service_dn);
    Ok(())
}
```

</details>

### Bài 2 — Effective group nesting và chu trình

Chứng minh `alice` là member gián tiếp của `IT Operations` nhưng không thuộc `Domain Admins`. Với chu trình, kiểm tra `Cycle A` có thể đi tới `Cycle B` mà kết quả không tự thêm `Cycle A` như một group cha mới.

<details><summary>Gợi ý</summary>

Dùng `is_member_of` cho quan hệ gián tiếp và `effective_groups` để quan sát tập kết quả. `ObjectId::new` trả `Result`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_group_nesting() -> Result<(), Box<dyn std::error::Error>> {
    let object_id = |value: &str| ObjectId::new(value);
    let lab = MockADMapper::standard();
    let alice = object_id("S-1-5-21-1000000001-1000000002-1000000003-1101")?;
    let operations = object_id("S-1-5-21-1000000001-1000000002-1000000003-2102")?;
    let admins = object_id("S-1-5-21-1000000001-1000000002-1000000003-512")?;
    assert!(lab.snapshot.is_member_of(&alice, &operations));
    assert!(!lab.snapshot.is_member_of(&alice, &admins));

    let cycle_a = object_id("S-1-5-21-1000000001-1000000002-1000000003-2103")?;
    let cycle_b = object_id("S-1-5-21-1000000001-1000000002-1000000003-2104")?;
    let groups = lab.snapshot.effective_groups(&cycle_a);
    assert!(groups.contains(&cycle_b));
    assert!(!groups.contains(&cycle_a));
    Ok(())
}
```

</details>

### Bài 3 — Dựng SPN inventory và cấp ticket tổng hợp

Kiểm tra inventory có hai row cho cùng `svc-web`. Dùng mock KDC xin service ticket cho `HTTP/web.lab.example`; ticket phải có `TicketKind::Service`, đúng client và evidence. SPN không tồn tại phải bị từ chối.

<details><summary>Gợi ý</summary>

Tạo `MockKerberosKdc::from_snapshot(&lab.snapshot)`, rồi gọi `request_service_ticket`. Không cần và không được thêm password vào fixture.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_spn_and_kerberos() -> Result<(), Box<dyn std::error::Error>> {
    let lab = MockADMapper::standard();
    let inventory = lab.snapshot.spn_inventory();
    assert_eq!(inventory.len(), 2);
    assert!(inventory.iter().all(|row| row.account == "svc-web"));

    let alice = ObjectId::new(
        "S-1-5-21-1000000001-1000000002-1000000003-1101",
    )?;
    let kdc = MockKerberosKdc::from_snapshot(&lab.snapshot);
    let reply = kdc.request_service_ticket(&alice, "HTTP/web.lab.example")?;
    assert_eq!(reply.service_ticket.kind, TicketKind::Service);
    assert_eq!(reply.service_ticket.client, alice);
    assert!(!reply.service_ticket.evidence.source.is_empty());
    assert!(kdc.request_service_ticket(&alice, "HTTP/missing").is_err());
    Ok(())
}
```

</details>

### Bài 4 — Tìm path qua dữ liệu có chu trình

Tìm path từ `bob` tới `Domain Admins` trong depth 5. Path phải validate, có `MemberOf` và `GenericAll`, mọi edge phải liên tiếp. Xóa một edge giữa path và chứng minh validation thất bại.

<details><summary>Gợi ý</summary>

`bob` đi qua `Cycle A` và `Cycle B`; chu trình không được làm DFS treo. `AttackPath::validate(&lab.graph)` kiểm tra cả sự tồn tại của edge và provenance.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_cyclic_path() -> Result<(), Box<dyn std::error::Error>> {
    let lab = MockADMapper::standard();
    let bob = ObjectId::new("S-1-5-21-1000000001-1000000002-1000000003-1102")?;
    let admins = ObjectId::new("S-1-5-21-1000000001-1000000002-1000000003-512")?;
    let path = lab.find_path(&bob, &admins, 5).expect("path tổng hợp");
    assert!(path.validate(&lab.graph));
    assert_eq!(path.edges.first().map(|edge| edge.kind), Some(EdgeKind::MemberOf));
    assert_eq!(path.edges.last().map(|edge| edge.kind), Some(EdgeKind::GenericAll));
    assert!(path.edges.iter().all(|edge| !edge.evidence.detail.is_empty()));

    let mut broken = path;
    broken.edges.remove(1);
    assert!(!broken.validate(&lab.graph));
    Ok(())
}
```

</details>

### Bài 5 — Giải thích vì sao path không hợp lệ hoặc chưa tìm thấy

Chứng minh không có path từ `WS01` tới `alice` trong snapshot, rồi chứng minh path ba edge `alice → WS01` không thể xuất hiện với `max_depth = 2`. Hai lời giải thích phải phân biệt thiếu dữ liệu/quan hệ với giới hạn tìm kiếm; không được kết luận từ trust hoặc OU.

<details><summary>Gợi ý</summary>

Dùng `find_path` và `explain_no_path`. Với trường hợp thiếu quyền, `access_decision` phải giữ `Unknown` khi không có ACE phù hợp.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_invalid_path_explanation() -> Result<(), Box<dyn std::error::Error>> {
    let lab = MockADMapper::standard();
    let alice = ObjectId::new("S-1-5-21-1000000001-1000000002-1000000003-1101")?;
    let ws01 = ObjectId::new("S-1-5-21-1000000001-1000000002-1000000003-3101")?;
    let admins = ObjectId::new("S-1-5-21-1000000001-1000000002-1000000003-512")?;

    assert!(lab.find_path(&ws01, &alice, 10).is_none());
    assert!(lab.explain_no_path(&ws01, &alice, 10).contains("Không có typed path"));
    assert!(lab.find_path(&alice, &ws01, 2).is_none());
    assert!(lab.explain_no_path(&alice, &ws01, 2).contains("depth 2"));
    assert!(lab.find_path(&alice, &ws01, 3).is_some());

    let decision = lab.snapshot.access_decision(&alice, &admins, AccessRight::WriteDacl);
    assert_eq!(decision.status, AccessStatus::Unknown);
    Ok(())
}
```

</details>

---

## 12. Detection, defensive controls và remediation

Một query hay một edge không tự chứng minh hành vi xấu. Detection phải gắn account, nguồn, thời điểm, phạm vi, baseline và change ticket; remediation phải sửa nguồn tạo quan hệ rồi đo lại, không chỉ xóa một dòng khỏi graph.

### 12.1 LDAP enumeration

- **Detection:** trên lab có audit phù hợp, theo dõi DN base, scope, filter, số trang, account và máy nguồn. Event 1644 có thể giúp nhận diện query LDAP đắt đỡ khi logging được cấu hình; không phải mọi query đều sinh event này. Đánh giá query phủ nhiều OU/SPN hoặc paging bất thường so với baseline thay vì coi mọi lần đọc directory là tấn công.
- **Controls/remediation:** giới hạn quyền đọc attribute nhạy cảm, bảo vệ tài khoản và máy trạm có quyền export, loại bỏ quyền truy cập không còn nghiệp vụ. LDAP signing/channel binding bảo vệ chất lượng xác thực và kênh, không phải biện pháp chặn mọi enumeration đã được authorize.
- **Retest:** chạy lại cùng filter trên snapshot mới, đối chiếu số object và attribute trước/sau; giữ truy vấn ngoài phạm vi ở trạng thái bị RoE từ chối.

### 12.2 Kerberos và SPN inventory

- **Detection:** xem event 4768/4769 cùng account, client, SPN và etype; alert theo chuỗi request bất thường hoặc downgrade yếu phải có baseline. Một 4769 đơn lẻ là hoạt động Kerberos bình thường, không đủ để kết luận Kerberoasting hay account đã bị chiếm.
- **Controls/remediation:** ưu tiên AES, dùng service account được quản lý và secret dài theo chính sách, bỏ SPN/account dịch vụ không còn dùng, giới hạn đặc quyền của service account. Nếu có bằng chứng lộ credential trong lab, rotate theo quy trình và điều tra phiên liên quan.
- **Retest:** dựng lại `spn_inventory`, xác nhận SPN mồi/stale đã biến mất và service ticket chỉ được cấp cho SPN còn tồn tại. Phần II tiếp theo sẽ phân tích điều kiện roasting trên dữ liệu giả.

### 12.3 ACL và ownership

- **Detection:** bật SACL có chọn lọc trên object nhạy cảm; đối chiếu event truy cập/thay đổi object như 4662, 5136 hoặc 4670 khi hệ thống thực sự tạo chúng. Ưu tiên thay đổi `GenericAll`, `WriteDacl`, `WriteOwner`, owner và replication right; ghi cả principal lẫn group gốc.
- **Controls/remediation:** áp dụng least privilege, delegated administration và change control cho DACL/owner. Gỡ ACE hoặc membership trái phép ở object thật, xem xét cả explicit deny/inheritance và kiểm tra không phá vỡ access hợp lệ.
- **Retest:** tính lại effective groups và `access_decision`. Quyền vừa gỡ phải trở thành `Denied` hoặc `Unknown` tùy dữ liệu còn lại; không cố biến `Unknown` thành `Allowed` để làm test xanh.

### 12.4 GPO link và inheritance

- **Detection:** giữ baseline GPO ID, version, link, trạng thái enabled/enforced và security filter; cảnh báo thay đổi ngoài change ticket và đối chiếu metadata directory với nội dung SYSVOL trên lab.
- **Controls/remediation:** giới hạn delegation tạo/sửa/link GPO và quản trị OU; phê duyệt thay đổi qua change control. Hoàn nguyên link/filter hoặc policy trái phép, xử lý cả cache và thời điểm áp dụng trên client.
- **Retest:** dùng RSoP trên hệ thống lab và chạy lại `effective_gpos` trên export mới. Kiểm tra block inheritance vẫn giữ link trực tiếp/enforced đúng như thiết kế, không chỉ nhìn tên policy.

### 12.5 Typed graph và trust

Graph là lớp phân tích, không phải log nguồn. Detection phải quay về telemetry của membership, ACL, owner, GPO và trust đã tạo edge; riêng việc chạy graph có thể chỉ sinh dấu vết ở quyền đọc/export. Giữ provenance và thời điểm snapshot để phân biệt thay đổi thật với replication latency.

Review trust theo chiều, transitivity, SID filtering/selective authentication và mục đích nghiệp vụ; trust không tự tạo admin. Khi remediation một path, sửa membership/ACE/trust gây ra quan hệ trên hệ thống lab, thu snapshot mới rồi chạy lại DFS. Path mục tiêu phải biến mất vì nguồn đã thay đổi, không phải vì xóa tay edge trong `AdGraph`. Nếu vẫn thiếu evidence, báo `Unknown` và nêu dữ liệu cần thu thập thay vì kết luận an toàn.

---

## 13. Tóm tắt và checklist lab

- Domain, tree, forest, DC, site và replication trả lời những câu hỏi khác nhau.
- SID, DN và GUID không thay thế nhau; `ObjectId` giúp compiler chặn tham chiếu nhầm.
- OU là container; group là security principal. Nesting cần thuật toán chịu được chu trình/dữ liệu xấu.
- GPO hiệu lực cần link, inheritance, enforcement và filtering; mock không phải RSoP engine.
- Trust có chiều và transitivity, nhưng không tự tạo quyền admin.
- LDAP subset phải từ chối cú pháp ngoài phạm vi; paging phải kết thúc đúng và từ chối page size 0.
- AS-REQ/AS-REP cấp TGT; TGS dùng PA-TGS-REQ chứa AP-REQ và cấp service ticket theo SPN.
- Thiếu dữ liệu ACL là `Unknown`; owner không đồng nghĩa mọi quyền.
- DFS dùng stack, visited theo nhánh và max depth; path phải chứa edge thật đúng thứ tự.
- Mọi quan hệ bảo mật đều cần provenance và sự phân biệt giữa fact, inference và unknown.

Checklist khi chuyển kiến thức vào **lab riêng được ủy quyền**:

- [ ] Có phạm vi domain/forest, account, thời gian, điều kiện dừng và quyền được phép đọc.
- [ ] Chỉ dùng snapshot/credential của lab; không đưa secret thật vào source hoặc fixture.
- [ ] Ghi nguồn, DC, thời điểm và phiên bản cho từng export; lưu ý replication latency.
- [ ] Không suy ra quyền từ tên OU, trust, SPN hoặc group name.
- [ ] Tách dữ liệu trực tiếp, quan hệ hiệu lực và điều chưa biết.
- [ ] Review từng precondition của path trước khi ghi finding.
- [ ] Gắn detection/remediation và thực hiện cleanup/retest theo Rules of Engagement.

### Nguồn chuẩn và đọc tiếp

- [RFC 4120 — Kerberos Network Authentication Service (V5)](https://www.rfc-editor.org/rfc/rfc4120): AS/TGS exchange và message framework.
- [RFC 4757 — RC4-HMAC Kerberos Encryption Types](https://www.rfc-editor.org/rfc/rfc4757): đặc tả etype RC4-HMAC lịch sử.
- [RFC 4514 — LDAP Distinguished Names](https://www.rfc-editor.org/rfc/rfc4514) và [RFC 4515 — LDAP Search Filters](https://www.rfc-editor.org/rfc/rfc4515): cú pháp đầy đủ mà subset của chương không thay thế.
- [RFC 2696 — LDAP Simple Paged Results](https://www.rfc-editor.org/rfc/rfc2696): cơ chế paging có cookie trên LDAP thật.
- [Microsoft Learn — Active Directory Domain Services](https://learn.microsoft.com/windows-server/identity/ad-ds/active-directory-domain-services): domain, forest, DC, replication và administration.
- [Microsoft Learn — Group Policy](https://learn.microsoft.com/windows-server/identity/ad-ds/manage/group-policy/group-policy-overview): link, processing và policy management.
- [Microsoft Learn — Kerberos SPN](https://learn.microsoft.com/windows/win32/ad/service-principal-names): SPN trong AD.

Chương trước: [Chương 106 — Wireless Security & OSWP Lab](chuong_106.md). Tiếp tục theo [ROADMAP](ROADMAP.md) và outline Phần II; không có liên kết tới chương chưa được publish.
