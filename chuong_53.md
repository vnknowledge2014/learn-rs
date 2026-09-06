# Chương 53: Nền tảng hệ phân tán, Định lý CAP & Thuật toán đồng thuận Raft (Distributed Consensus, CAP Theorem & Raft Protocol)

## Giới thiệu & Mục tiêu học tập

Trong một máy tính đơn lẻ, việc xác định "sự thật" rất đơn giản: Bạn đọc một ô nhớ trên RAM hoặc một khối dữ liệu trên đĩa cứng, dữ liệu đó là duy nhất và nhất quán. Nhưng khi hệ thống của bạn mở rộng thành một cụm gồm 10, 100 hay 1,000 máy chủ đặt rải rác ở Tokyo, Frankfurt và California, thế giới trở nên vô cùng hỗn loạn: **Cáp quang dưới biển có thể bị đứt (Network Partition), máy chủ có thể bị sập đột ngột (Crash Fault), và đồng hồ nguyên tử giữa các trung tâm dữ liệu không bao giờ khớp nhau từng phần tỉ giây.**

Làm thế nào hàng chục cỗ máy độc lập có thể cùng nhau thống nhất được một trạng thái duy nhất — ví dụ: *"Tài khoản của Alice còn 1 triệu đồng hay đã chuyển tiền cho Bob?"* — mà không bị xung đột dữ liệu? Câu trả lời nằm ở trái tim của toàn bộ ngành khoa học hệ thống phân tán: **Định lý CAP** và **Thuật toán Đồng thuận Raft (Raft Consensus Protocol)**.

Trong chương này, chúng ta sẽ chinh phục:
- Bản chất khắc nghiệt của môi trường phân tán: Sự cố mạng là điều tất yếu chứ không phải ngoại lệ.
- **Định lý CAP (Consistency, Availability, Partition Tolerance)**: Phân tích vì sao không một hệ thống nào trên thế giới có thể đồng thời đạt được cả tính Nhất quán tuyệt đối và Tính sẵn sàng 100% khi xảy ra đứt gãy mạng (Sự lựa chọn giữa CP và AP).
- **Thuật toán đồng thuận Raft trực quan**: Ba trạng thái của Node (Follower, Candidate, Leader), vòng đời nhiệm kỳ (Term), và cơ chế bỏ phiếu bầu cử quá bán (Quorum).
- Cơ chế **Sao chép Sổ nhật ký (Log Replication)** và Cam kết giao dịch (Commitment): Bảo đảm mọi nút trong mạng đều thực thi cùng một chuỗi câu lệnh theo đúng thứ tự.
- Tự tay lập trình một mô hình mô phỏng cụm Raft Node hoàn chỉnh bằng Rust an toàn, không có điều kiện tranh chấp dữ liệu.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu định lý CAP và thuật toán Raft mà không cần bất kỳ công thức toán học ma trận nào, hãy quan sát hai câu chuyện đời thường:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA: ĐOÀN DU LỊCH 5 NGƯỜI BẦU TRƯỞNG ĐOÀN (RAFT)         │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. ĐOÀN XE 5 NGƯỜI VÀ BẦU CỬ QUÁ BÁN QUORUM (LEADER ELECTION)]                  │
│ 5 người bạn lái 5 chiếc xe đi xuyên Việt: An, Bình, Cường, Dũng, Em.            │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Bình thường: An làm Trưởng đoàn (Leader). An định kỳ bấm bộ đàm:    │         │
│ │   "Alo alo, tôi vẫn khỏe, đoàn cứ chạy thẳng nhé!" (Heartbeat).      │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Biến cố: Xe của An bị nổ lốp, mất liên lạc quá 5 phút (Timeout)!     │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ 1. Bình thấy mất dấu An ──► Bình giơ tay ứng cử (Candidate, Term 2)! │         │
│ │ 2. Bình phát loa: "Ai đồng ý bầu tôi làm Trưởng đoàn mới thì bấm còi!│         │
│ │ 3. Cường và Dũng bấm còi đồng ý.                                     │         │
│ │    ===> 3/5 người đồng ý (ĐỦ QUÁ BÁN QUORUM = 5/2 + 1 = 3 PHIẾU)!    │         │
│ │ 4. Bình chính thức đắc cử làm TRƯỞNG ĐOÀN MỚI (LEADER)!              │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│                                                                                  │
│ [2. ĐỊNH LÝ CAP: CƠN BÃO LÀM ĐỨT ĐƯỜNG DÂY ĐIỆN THOẠI (NETWORK PARTITION)]       │
│ Một hòn đảo bị bão cắt đứt liên lạc với đất liền (Phân rã mạng P):               │
│ - LỰA CHỌN CP (Nhất quán): Đảo từ chối cho rút tiền vì không thể xác nhận số dư  │
│   với trụ sở chính (Từ chối phục vụ để bảo vệ tiền).                           │
│ - LỰA CHỌN AP (Sẵn sàng): Đảo vẫn cho người dân rút tiền thoải mái, nhưng chấp   │
│   nhận rủi ro tài khoản bị âm tiền khi nối lại cáp mạng (Bảo đảm phục vụ liên tục).│
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Bầu trưởng đoàn đi xuyên Việt (Raft Leader Election)
- Một cụm máy chủ Raft thường có số lượng nút lẻ: `3`, `5` hoặc `7` máy chủ.
- **Quy tắc Quá bán (Quorum = $N/2 + 1$)**: Để đưa ra bất kỳ quyết định nào (bầu cử Trưởng đoàn hoặc ghi thêm dữ liệu vào sổ cái), hệ thống bắt buộc phải nhận được sự đồng thuận của hơn một nửa số thành viên (ít nhất 3/5 nút).
- Nhờ quy tắc này, dù có 2 máy chủ bị sét đánh cháy nguồn, 3 máy chủ còn lại vẫn chiếm đa số và duy trì hệ thống hoạt động bình thường!

### 2. Định lý CAP: Hòn đảo bị bão cô lập
- Giả sử bạn gửi 10 triệu đồng vào ngân hàng ở Hà Nội. Cùng lúc đó bạn của bạn ở Đảo Phú Quốc cầm thẻ ATM ra cây rút tiền.
- Bất ngờ một cơn bão biển giật đứt đường cáp quang nối Phú Quốc với đất liền (**Sự cố Phân tách mạng - Network Partitioning**). Cây ATM ở Phú Quốc không thể gọi điện về Hà Nội để hỏi: *"Tài khoản này còn tiền không?"*.
- Ngân hàng buộc phải chọn một trong hai con đường:
  - **Con đường 1: Chọn Tính Nhất quán (C - Consistency)**: Cây ATM báo lỗi: *"Mạng bị gián đoạn, xin quý khách quay lại sau"*. Ngân hàng bảo vệ số dư tuyệt đối chính xác, nhưng hy sinh Tính sẵn sàng (Availability). Hệ thống này gọi là hệ thống **CP**.
  - **Con đường 2: Chọn Tính Sẵn sàng (A - Availability)**: Cây ATM vẫn nhả tiền cho bạn rút, nhưng chấp nhận rủi ro: Nếu ở Hà Nội bạn vừa rút hết tiền rồi, thì ở Phú Quốc người bạn rút thêm sẽ làm tài khoản bị âm. Hệ thống này gọi là hệ thống **AP**.
- Bạn không thể chọn cả hai. Sự cố đứt mạng là quy luật vật lý, bạn bắt buộc phải đánh đổi!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Phân tích Chi tiết Định lý CAP (Brewer's CAP Theorem)

```
        Tính Nhất quán (Consistency)
               /\
              /  \
             /    \
            /  CP  \
           /        \
          /__________\
Tính Sẵn sàng       Tính Chịu Phân Rã
(Partition Tolerance)
```

- **Tính Nhất quán (Consistency - C)**: Mọi thao tác đọc đều nhận được dữ liệu của lần ghi mới nhất hoặc trả về lỗi. Tuyệt đối không bao giờ trả về dữ liệu cũ đã lỗi thời.
- **Tính Sẵn sàng (Availability - A)**: Mọi yêu cầu gửi tới các nút còn sống đều nhận được phản hồi thành công (không bị lỗi hoặc timeout), nhưng không đảm bảo đó là dữ liệu mới nhất.
- **Tính Chịu đựng Phân rã mạng (Partition Tolerance - P)**: Hệ thống vẫn tiếp tục vận hành ngay cả khi các gói tin mạng giữa các máy chủ bị rơi rớt hoặc chậm trễ vô hạn.
- **Kết luận thực tiễn**: Vì trong thế giới thực, mạng viễn thông chắc chắn sẽ có lúc bị đứt hoặc trễ (P luôn xảy ra), mọi kiến trúc sư phân tán chỉ có thể chọn: **Hệ thống CP** (như Raft, etcd, ZooKeeper, CockroachDB) hoặc **Hệ thống AP** (như DynamoDB, Cassandra, CouchDB).

### 2. Ba Trạng Thái & Nhiệm Kỳ của Thuật toán Raft

Mỗi máy chủ (Node) trong cụm Raft luôn nằm ở 1 trong 3 trạng thái:
1. **Follower (Người phục tùng)**: Trạng thái mặc định ban đầu. Chỉ thụ động lắng nghe yêu cầu từ Leader và Candidate. Định kỳ nhận thông điệp nhịp tim (`Heartbeat / AppendEntries`).
2. **Candidate (Ứng viên tranh cử)**: Khi không nhận được Heartbeat trong khoảng thời gian **Election Timeout** (ngẫu nhiên từ 150ms đến 300ms), Follower tự nâng cấp mình lên thành Candidate, tăng số thứ tự Nhiệm kỳ (`current_term += 1`), tự bỏ phiếu cho mình và gửi RPC `RequestVote` tới tất cả các nút khác.
3. **Leader (Trưởng đoàn điều hành)**: Nếu Candidate nhận được đủ số phiếu quá bán ($N/2 + 1$), nó chính thức trở thành Leader. Leader bắt đầu gửi Heartbeat định kỳ để duy trì quyền lực và tiếp nhận mọi yêu cầu ghi từ Client.

### 3. Quy trình Sao chép Sổ nhật ký (Log Replication)

```
Client ──► [LEADER (Node 1)] ──(AppendEntries)──► [FOLLOWER (Node 2)] (Ghi đệm)
                 │
                 └──(AppendEntries)──────────────► [FOLLOWER (Node 3)] (Ghi đệm)
                 │
           (Đủ 2/3 nút xác nhận!)
                 ▼
        [LEADER COMMIT!] ──► Cập nhật State Machine ──► Trả kết quả về Client
```
1. Client gửi lệnh: `set("account", "1000k")` tới Leader.
2. Leader ghi lệnh vào cuối cuốn Sổ nhật ký (Log) của mình ở trạng thái Chưa cam kết (Uncommitted).
3. Leader gửi bản sao lệnh đó tới tất cả các Follower thông qua RPC `AppendEntries`.
4. Khi đa số các Follower (Quorum) đã ghi nhận bản ghi vào đĩa của họ và phản hồi thành công, Leader chính thức **Cam kết bản ghi (Commit)** và nạp vào máy trạng thái (State Machine).
5. Leader trả lời Client: *"Ghi dữ liệu thành công!"*. Ở nhịp Heartbeat tiếp theo, Leader thông báo cho các Follower biết chỉ số Commit Index để các Follower cũng cam kết bản ghi vào máy trạng thái của họ. Dữ liệu trên toàn cụm đạt trạng thái nhất quán tuyệt đối!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn Rust hoàn chỉnh hiện thực hóa một **Động cơ Đồng thuận Raft thu nhỏ (Educational Raft Consensus Simulation)**: Mô phỏng đầy đủ 3 vai trò (Follower, Candidate, Leader), cơ chế tính toán nhiệm kỳ (Term), quy tắc bầu cử đạt ngưỡng quá bán Quorum ($N/2 + 1$), và tiến trình sao chép sổ nhật ký an toàn:

```rust
/// Ba vai trò khả dĩ của một nút trong cụm đồng thuận Raft
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RaftRole {
    Follower,
    Candidate,
    Leader,
}

/// Một bản ghi nhật ký deliver dịch trong sổ cái Raft
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    pub term: u64,
    pub index: u64,
    pub command: String,
}

/// Thực thể một nút mạng trong cụm Raft (Raft Cluster Node)
pub struct RaftNode {
    pub node_id: u64,
    pub current_term: u64,
    pub voted_for: Option<u64>,
    pub role: RaftRole,
    pub log: Vec<LogEntry>,
    pub commit_index: u64,
    pub votes_received: usize,
}

impl RaftNode {
    pub fn new(node_id: u64) -> Self {
        Self {
            node_id,
            current_term: 0,
            voted_for: None,
            role: RaftRole::Follower,
            log: Vec::new(),
            commit_index: 0,
            votes_received: 0,
        }
    }

    /// Kích hoạt khi hết thời gian chờ bầu cử (Election Timeout)
    pub fn handle_election_timeout(&mut self, total_cluster_nodes: usize) {
        self.role = RaftRole::Candidate;
        self.current_term += 1;
        self.voted_for = Some(self.node_id); // Tự bỏ 1 phiếu cho chính mình
        self.votes_received = 1;

        println!(
            "    [Node {}] Hết hạn chờ! Chuyển thành CANDIDATE ở Nhiệm kỳ (Term) #{}",
            self.node_id, self.current_term
        );

        // Tính toán ngưỡng quá bán Quorum: (N / 2) + 1
        let quorum = (total_cluster_nodes / 2) + 1;
        if self.votes_received >= quorum {
            self.role = RaftRole::Leader;
            println!(
                "    [Node {}] Nhận đủ {}/{} phiếu quá bán: ĐẮC CỬ LÀM LEADER!",
                self.node_id, self.votes_received, total_cluster_nodes
            );
        }
    }

    /// Xử lý yêu cầu xin phiếu bầu từ một ứng viên khác (RequestVote RPC)
    pub fn handle_request_vote(
        &mut self,
        candidate_id: u64,
        candidate_term: u64,
    ) -> bool {
        // 1. Nếu nhiệm kỳ của ứng viên thấp hơn nhiệm kỳ hiện tại: Từ chối ngay
        if candidate_term < self.current_term {
            println!(
                "    [Node {}] Từ chối bầu cho Node {}: Term {} < Term hiện tại {}",
                self.node_id, candidate_id, candidate_term, self.current_term
            );
            return false;
        }

        // 2. Nếu nhiệm kỳ của ứng viên cao hơn: Cập nhật nhiệm kỳ và quay về làm Follower
        if candidate_term > self.current_term {
            self.current_term = candidate_term;
            self.role = RaftRole::Follower;
            self.voted_for = None;
        }

        // 3. Nếu chưa bỏ phiếu cho ai trong nhiệm kỳ này: Đồng ý bỏ phiếu!
        if self.voted_for.is_none() || self.voted_for == Some(candidate_id) {
            self.voted_for = Some(candidate_id);
            println!(
                "    [Node {}] ĐÃ BỎ PHIẾU ĐỒNG Ý cho Ứng viên Node {} ở Term {}",
                self.node_id, candidate_id, self.current_term
            );
            true
        } else {
            false
        }
    }

    /// Tiếp nhận lệnh ghi từ Client (Chỉ Leader mới có quyền tiếp nhận)
    pub fn append_client_command(&mut self, command: &str) -> Result<u64, &'static str> {
        if self.role != RaftRole::Leader {
            return Err("Nút này không phải Leader: Từ chối tiếp nhận lệnh ghi!");
        }

        let new_index = (self.log.len() as u64) + 1;
        let entry = LogEntry {
            term: self.current_term,
            index: new_index,
            command: command.to_string(),
        };

        self.log.push(entry);
        println!(
            "    [Leader Node {}] Đã thêm lệnh mới vào Log ở index #{}: '{}'",
            self.node_id, new_index, command
        );

        Ok(new_index)
    }

    /// Kiểm tra và xác nhận cam kết bản ghi khi đủ số nút sao chép (Quorum Commit)
    pub fn check_and_commit(&mut self, successful_replications: usize, total_nodes: usize) {
        let quorum = (total_nodes / 2) + 1;
        if successful_replications >= quorum {
            self.commit_index = self.log.len() as u64;
            println!(
                "    [Leader Node {}] Đạt Quorum ({}/{} nút): CAM KẾT LOG INDEX #{} VÀO MÁY TRẠNG THÁI!",
                self.node_id, successful_replications, total_nodes, self.commit_index
            );
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   DONG THUAN PHAN TAN RAFT & CAP THEOREM SIMULATION TRONG RUST   ");
    println!("==================================================================");

    // 1. Khởi tạo một cụm gồm 3 nút mạng phân tán (Node 1, Node 2, Node 3)
    let total_nodes = 3;
    let mut node1 = RaftNode::new(1);
    let mut node2 = RaftNode::new(2);
    let mut node3 = RaftNode::new(3);

    println!("\n[1] Khoi tao cum 3 nut mang (Tat ca deu la Follower ban dau):");
    println!("    - Node 1 Role: {:?} | Term: {}", node1.role, node1.current_term);
    println!("    - Node 2 Role: {:?} | Term: {}", node2.role, node2.current_term);
    println!("    - Node 3 Role: {:?} | Term: {}", node3.role, node3.current_term);

    // 2. Mô phỏng Node 1 bị hết hạn chờ (Election Timeout) và phát động tranh cử
    println!("\n[2] Node 1 bi Timeout va khoi dong tranh cu lanh dao (Election):");
    node1.handle_election_timeout(total_nodes);

    // Node 1 gửi RequestVote tới Node 2 và Node 3
    let vote_from_2 = node2.handle_request_vote(node1.node_id, node1.current_term);
    let vote_from_3 = node3.handle_request_vote(node1.node_id, node1.current_term);

    if vote_from_2 {
        node1.votes_received += 1;
    }
    if vote_from_3 {
        node1.votes_received += 1;
    }

    let quorum = (total_nodes / 2) + 1;
    if node1.votes_received >= quorum {
        node1.role = RaftRole::Leader;
        println!(
            "\n    [+] Chuc mung Node 1 da tro thanh LEADER hop phap cua Term {} voi {}/{} phieu!",
            node1.current_term, node1.votes_received, total_nodes
        );
    }
    assert_eq!(node1.role, RaftRole::Leader);

    // 3. Mô phỏng Client gửi lệnh ghi dữ liệu tới Leader
    println!("\n[3] Mo phong Client gui deliver dich 'CHUYEN_TIEN_100K' toi Leader:");
    let log_idx = node1.append_client_command("CHUYEN_TIEN_ALICE_TO_BOB_100K").unwrap();

    // Leader sao chép sang Node 2 thành công
    println!("    - Leader Node 1 sao chep ban ghi sang Node 2...");
    let replication_success_count = 2; // Node 1 (chính nó) + Node 2 đồng ý

    // Leader kiểm tra Quorum để quyết định Commit
    node1.check_and_commit(replication_success_count, total_nodes);
    assert_eq!(node1.commit_index, log_idx);

    println!("\n==================================================================");
    println!("   XAC NHAN: THUAT TOAN RAFT HOAT DONG DUNG QUY CHUAN DONG THUAN! ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi lập trình các thuật toán đồng thuận và quản lý trạng thái phân tán trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0506** | `cannot assign to 'self.current_term' because it is borrowed` | Cố gắng thay đổi trường dữ liệu trong khi đang mượn tham chiếu đọc trường khác trong cùng struct. | Tách nhỏ các phương thức hoặc sao chép giá trị số nguyên (`u64`) ra biến cục bộ độc lập trên Stack. |
| **E0382** | `use of moved value: 'node1'` | Di chuyển quyền sở hữu (ownership) của nút mạng vào luồng khác mà không dùng con trỏ đếm tham chiếu đa luồng. | Sử dụng con trỏ thông minh (smart pointer) `Arc<Mutex<RaftNode>>` khi chia sẻ một Node qua nhiều luồng mạng. |
| **E0277** | `the trait 'Clone' is not implemented for 'LogEntry'` | Cố gắng nhân bản danh sách nhật ký `log.clone()` mà struct chưa triển khai trait `Clone`. | Bổ sung derive tự động: `#[derive(Clone, Debug, PartialEq, Eq)]` lên trên định nghĩa `LogEntry`. |
| **E0599** | `no method named 'len' found for struct 'LogEntry'` | Gọi nhầm phương thức `.len()` trên một phần tử đơn lẻ thay vì trên mảng vector `self.log`. | Kiểm tra lại cú pháp: Dùng `self.log.len()` để đếm số lượng bản ghi nhật ký. |

### Ví dụ phân tích lỗi `E0506` khi vừa duyệt mảng log vừa thay đổi term:

```rust
struct ViDuNode {
    term: u64,
    log: Vec<String>,
}

// Đoạn mã lỗi minh họa E0506:
fn update_broken(node: &mut ViDuNode) {
    // let first_cmd = node.log.first(); // Mượn bất biến node.log
    // node.term += 1;                   // LỖI E0506: Mượn khả biến node để sửa term!
    // println!("Lệnh: {:?}", first_cmd);
}

// Cách sửa chữa đúng chuẩn: Sao chép dữ liệu hoặc attempt hẹp phạm vi mượn
fn update_correct(node: &mut ViDuNode) {
    let first_cmd = node.log.first().cloned(); // Sao chép giá trị ra biến riêng
    node.term += 1;                            // Sửa term an toàn 100%
    println!("Lệnh đã trích xuất: {:?}", first_cmd);
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Định lý CAP không thể phá vỡ**: Khi mạng bị phân tách (Partition), hệ thống phân tán bắt buộc phải lựa chọn giữa Tính Nhất quán (CP) hoặc Tính Sẵn sàng (AP).
2. **Quy tắc Quá bán Quorum**: Bất kỳ quyết định bầu cử hay cam kết dữ liệu nào trong Raft cũng đòi hỏi sự đồng thuận của hơn một nửa số nút ($N/2 + 1$), loại bỏ triệt để xung đột não đôi (Split-Brain).
3. **Sao chép Sổ nhật ký (Log Replication)**: Dữ liệu chỉ được cam kết chính thức vào máy trạng thái khi đã được ghi nhận trên đa số các nút trong cụm.
4. **Độ an toàn Nhất quán trong Rust**: Sự phối hợp giữa quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) giúp mô phỏng và vận hành cỗ máy đồng thuận Raft mà không bao giờ gặp lỗi rò rỉ dữ liệu hay xung đột luồng.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Mô phỏng Phân chia Não đôi - Split-Brain Prevention)**:  
   Tạo một cụm 5 nút mạng. Giả lập tình huống mạng bị chia cắt thành 2 cụm con: Cụm A gồm 2 nút (Node 1, 2) và Cụm B gồm 3 nút (Node 3, 4, 5). Chứng minh bằng mã nguồn rằng Cụm A sẽ không bao giờ có thể bầu được Leader mới vì không thể đạt đủ 3 phiếu Quorum ($5/2 + 1 = 3$).
2. **Bài tập 2 (Phục hồi Đồng bộ Nhật ký khi Node sống lại)**:  
   Giả sử Node 3 bị sập nguồn trong 1 tiếng và bị thiếu mất 10 bản ghi nhật ký. Hãy viết hàm `sync_follower_log` cho phép Leader tự động phát hiện vị trí bản ghi không khớp và gửi lại các bản ghi còn thiếu để đưa Node 3 về trạng thái nhất quán với toàn cụm.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Tại sao Raft lại thay thế Paxos?)**:  
   Trước khi Raft ra đời vào năm 2014, Paxos là thuật toán đồng thuận thống trị thế giới. Tại sao tác giả Diego Ongaro lại sáng tạo ra Raft với mục tiêu hàng đầu là "Tính dễ hiểu (Understandability)"? Hãy phân tích sự khác biệt giữa cấu trúc có Leader độc tôn của Raft so với tính đối xứng phức tạp của Multi-Paxos.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Quorum = quá bán: với 5 nút cần floor(5/2)+1 = 3 phiếu để bầu Leader. Cụm A chỉ có 2 nút -> tối đa 2 phiếu -> không bao giờ đủ. Đây là cách Raft chặn 'não đôi'.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
/// Kiểm một cụm con có đủ quorum để bầu Leader không.
/// Quorum của cụm n nút = n/2 + 1 (quá bán).
fn co_du_quorum(so_phieu: usize, tong_cum: usize) -> bool {
    so_phieu >= tong_cum / 2 + 1
}

/// Mô phỏng: cụm 5 nút bị chia thành A (2 nút) và B (3 nút).
/// Chứng minh chỉ cụm B bầu được Leader.
#[test]
fn chong_nao_doi_split_brain() {
    let tong = 5;
    let quorum_can = tong / 2 + 1; // = 3
    assert_eq!(quorum_can, 3);

    // Cụm A: Node 1, 2 -> tối đa 2 phiếu (tự bầu cho nhau).
    let phieu_cum_a = 2;
    assert!(!co_du_quorum(phieu_cum_a, tong),
        "Cụm A chỉ 2 phiếu < 3 -> KHÔNG được bầu Leader");

    // Cụm B: Node 3, 4, 5 -> 3 phiếu.
    let phieu_cum_b = 3;
    assert!(co_du_quorum(phieu_cum_b, tong),
        "Cụm B đủ 3 phiếu >= 3 -> ĐƯỢC bầu Leader");

    // Điểm mấu chốt: hai cụm KHÔNG THỂ cùng đạt quorum.
    // Vì nếu cả hai cùng đủ quá bán thì tổng phiếu > tổng nút — vô lý.
    assert!(!(co_du_quorum(phieu_cum_a, tong) && co_du_quorum(phieu_cum_b, tong)),
        "Không bao giờ có HAI Leader cùng lúc");
}
```

**"Não đôi" (split-brain)** là ác mộng của hệ phân tán: mạng bị chia cắt, mỗi phía tưởng phía kia đã chết và **tự bầu Leader riêng** — giờ có *hai* Leader cùng nhận lệnh ghi, dữ liệu phân kỳ không thể hòa giải. Raft chặn triệt để bằng luật quorum quá bán: một Leader chỉ hợp lệ khi được **hơn một nửa toàn cụm** bầu. Chứng minh toán học vì sao điều này an toàn: nếu cụm A và cụm B *cùng* đạt quá bán, thì tổng số phiếu >= (n/2+1) × 2 > n — nhưng mỗi nút chỉ bầu một lần, nên tổng phiếu <= n. Mâu thuẫn. Do đó **tối đa một cụm con đạt quorum** — phía thiểu số (cụm A) tự động mất khả năng bầu Leader và chỉ chờ, còn phía đa số (cụm B) tiếp tục hoạt động. Đây là lý do các cụm đồng thuận luôn dùng **số nút lẻ** (3, 5, 7): để một lần chia cắt luôn tạo ra một phía đa số rõ ràng.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Leader so nhật ký của mình với follower, lùi dần tìm điểm bắt đầu khớp, rồi gửi lại mọi bản ghi từ đó trở đi. Follower cắt bỏ phần lệch và nối phần Leader gửi.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
#[derive(Clone, PartialEq, Debug)]
pub struct LogEntry { pub term: u64, pub command: String }

/// Leader đồng bộ nhật ký cho một follower bị tụt lại.
/// Trả về các bản ghi follower CÒN THIẾU (từ điểm khớp cuối trở đi).
pub fn sync_follower_log(leader_log: &[LogEntry], follower_log: &[LogEntry]) -> Vec<LogEntry> {
    // Tìm điểm khớp cuối cùng: quét tới khi hai nhật ký bắt đầu lệch.
    let mut match_len = 0;
    while match_len < leader_log.len()
        && match_len < follower_log.len()
        && leader_log[match_len] == follower_log[match_len]
    {
        match_len += 1;
    }
    // Mọi bản ghi của Leader từ điểm lệch trở đi là phần cần gửi bù.
    leader_log[match_len..].to_vec()
}

/// Follower áp bản vá: cắt phần lệch của mình, nối phần Leader gửi.
pub fn apply_sync(follower_log: &mut Vec<LogEntry>, tu_vi_tri: usize, bu: &[LogEntry]) {
    follower_log.truncate(tu_vi_tri); // bỏ phần lệch (nếu có)
    follower_log.extend_from_slice(bu);
}

#[test]
fn dong_bo_follower_bi_tut_lai() {
    let e = |t, c: &str| LogEntry { term: t, command: c.to_string() };
    // Leader có 5 bản ghi; follower (Node 3) mới có 2 -> thiếu 3 cái cuối.
    let leader = vec![e(1,"a"), e(1,"b"), e(2,"c"), e(2,"d"), e(3,"e")];
    let mut follower = vec![e(1,"a"), e(1,"b")];

    let bu = sync_follower_log(&leader, &follower);
    assert_eq!(bu.len(), 3); // c, d, e
    apply_sync(&mut follower, 2, &bu);
    assert_eq!(follower, leader); // đã nhất quán với Leader

    // Follower có bản ghi LỆCH (term sai) phải bị cắt bỏ, không giữ lại.
    let mut lech = vec![e(1,"a"), e(1,"b"), e(9,"X")]; // "X" ở term 9 là rác cần bỏ
    let bu2 = sync_follower_log(&leader, &lech);
    apply_sync(&mut lech, 2, &bu2); // cắt từ vị trí khớp cuối (2)
    assert_eq!(lech, leader);
}
```

Đây là trái tim của cơ chế **nhất quán nhật ký (log matching)** trong Raft. Nguyên tắc: Leader là **nguồn chân lý duy nhất** — mọi follower phải khớp nhật ký của Leader tới từng bản ghi. Khi một nút sống lại sau sự cố và bị thiếu (hoặc tệ hơn, có bản ghi *lệch* do từng nhận lệnh từ một Leader cũ đã bị lật đổ), Leader **lùi dần tìm điểm khớp cuối cùng**, rồi buộc follower **cắt bỏ mọi thứ sau điểm đó và chép lại từ Leader**. Chi tiết an toàn quan trọng: follower *không* được giữ lại bản ghi lệch của mình (bản ghi `X` ở term 9 trong test) — nó phải bị ghi đè, vì chỉ nhật ký của Leader mới được thừa nhận. Nhờ luật này, sau đồng bộ mọi nút hội tụ về *đúng một* lịch sử thống nhất, dù trước đó chúng phân kỳ thế nào.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Raft và Paxos giải cùng bài toán đồng thuận, nhưng Raft đặt 'tính dễ hiểu' làm mục tiêu số một. Khác biệt cốt lõi: Raft có một Leader độc tôn dẫn dắt, Paxos đối xứng và phi tập trung hơn.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Vì sao Diego Ongaro tạo Raft (2014) với mục tiêu hàng đầu là *tính dễ hiểu* — dù Paxos đã tồn tại và được chứng minh đúng:**

Câu trả lời nằm ở một sự thật phũ phàng của ngành: **Paxos đúng về mặt toán học nhưng nổi tiếng là khó hiểu và khó cài đặt đúng.** Chính Leslie Lamport (tác giả Paxos) viết bài báo gốc dưới dạng một truyện ngụ ngôn về nghị viện Hy Lạp khiến nó càng khó nắm. Hệ quả thực tế: các kỹ sư đọc Paxos, gật gù rằng nó đúng, rồi *không cài nổi* — và những bản cài "Paxos" ngoài đời thường là các biến thể chắp vá (Multi-Paxos) mà không ai chắc còn đúng không. Ongaro lập luận: **một thuật toán đồng thuận mà con người không hiểu nổi thì không thể cài đúng, không thể vận hành, không thể dạy** — nên *tính dễ hiểu* tự nó là một mục tiêu kỹ thuật chính đáng, ngang hàng với tính đúng đắn.

**Khác biệt cấu trúc cốt lõi — Leader độc tôn (Raft) so với đối xứng (Multi-Paxos):**

| | Raft | Multi-Paxos |
|---|---|---|
| **Vai trò** | Leader độc tôn rõ ràng; mọi ghi đi qua Leader | Đối xứng — nút nào cũng có thể đề xuất, vai trò mờ |
| **Luồng dữ liệu** | Một chiều: Leader -> follower | Nhiều bên thương lượng qua lại |
| **Cách hiểu** | Tách thành 3 bài toán con rời: bầu Leader, sao chép nhật ký, an toàn | Trộn lẫn, khó tách để suy luận từng phần |
| **Khi Leader chết** | Bầu lại rõ ràng theo term tăng dần | Có thể có nhiều đề xuất cạnh tranh, phức tạp hơn |

**Raft đơn giản hóa bằng cách *áp đặt cấu trúc*:** thay vì để mọi nút bình đẳng thương lượng (như Paxos), Raft **bầu ra một Leader độc tôn** và quy định *mọi* thay đổi phải đi qua Leader theo một chiều. Điều này thu hẹp không gian trạng thái phải suy luận: bạn chỉ cần hiểu "Leader nói, follower nghe theo và khớp nhật ký". Ongaro còn cố ý **chia Raft thành ba bài toán con độc lập** — (1) bầu Leader, (2) sao chép nhật ký, (3) đảm bảo an toàn — để người học nắm từng mảnh riêng rồi ghép lại, thay vì nuốt cả khối như Paxos.

Cái giá của sự đơn giản: Leader độc tôn là **điểm nghẽn** (mọi ghi qua một nút) và tạo một khoảng ngừng khi Leader chết (phải bầu lại). Paxos đối xứng về lý thuyết mềm dẻo hơn. Nhưng Ongaro đặt cược đúng: **với đa số hệ thống thực tế, một thuật toán *dễ hiểu và cài đúng* giá trị hơn một thuật toán *tối ưu lý thuyết nhưng không ai cài nổi*.** Kết quả lịch sử chứng minh điều đó — Raft nay là nền tảng của etcd, Consul, TiKV, CockroachDB và vô số hệ thống production, trong khi Paxos thuần phần lớn vẫn nằm trong các bài báo.
</details>
