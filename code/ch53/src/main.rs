#![allow(dead_code, unused_variables, unused_imports)]
/// Ba vai trò khả dĩ của một nút trong cụm đồng thuận Raft
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RaftRole {
    Follower,
    Candidate,
    Leader,
}

/// Một bản ghi nhật ký giao dịch trong sổ cái Raft
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
    println!("\n[3] Mo phong Client gui giao dich 'CHUYEN_TIEN_100K' toi Leader:");
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
