use std::collections::HashMap;

/// Ba vai trò khả dĩ của một nút trong cụm đồng thuận Raft
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RaftRole {
    Follower,
    Candidate,
    Leader,
}

/// Một bản ghi nhật ký giao dịch trong sổ cái Raft (index bắt đầu từ 1)
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
    /// (Chỉ Leader dùng) chỉ số bản ghi cao nhất đã biết là khớp trên từng Follower
    pub match_index: HashMap<u64, u64>,
}

/// Ngưỡng quá bán của CẢ CỤM (tính cả Leader): floor(N/2) + 1
pub fn quorum(total_nodes: usize) -> usize {
    total_nodes / 2 + 1
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
            match_index: HashMap::new(),
        }
    }

    pub fn last_log_index(&self) -> u64 {
        self.log.len() as u64
    }

    pub fn last_log_term(&self) -> u64 {
        self.log.last().map_or(0, |e| e.term)
    }

    /// Term của bản ghi tại `index` (index 0 = "trước bản ghi đầu tiên", term 0)
    fn term_at(&self, index: u64) -> Option<u64> {
        if index == 0 {
            Some(0)
        } else {
            self.log.get(index as usize - 1).map(|e| e.term)
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

        // Cụm 1 nút: phiếu của chính mình đã đủ quá bán
        if self.votes_received >= quorum(total_cluster_nodes) {
            self.become_leader();
        }
    }

    /// Ứng viên nhận thêm một phiếu; đủ quá bán thì đắc cử
    pub fn receive_vote(&mut self, granted: bool, total_cluster_nodes: usize) {
        if self.role != RaftRole::Candidate || !granted {
            return;
        }
        self.votes_received += 1;
        if self.votes_received >= quorum(total_cluster_nodes) {
            println!(
                "    [Node {}] Nhận đủ {}/{} phiếu quá bán: ĐẮC CỬ LÀM LEADER của Term {}!",
                self.node_id, self.votes_received, total_cluster_nodes, self.current_term
            );
            self.become_leader();
        }
    }

    fn become_leader(&mut self) {
        self.role = RaftRole::Leader;
        self.match_index.clear();
    }

    /// Xử lý yêu cầu xin phiếu bầu từ một ứng viên khác (RequestVote RPC)
    pub fn handle_request_vote(
        &mut self,
        candidate_id: u64,
        candidate_term: u64,
        candidate_last_log_index: u64,
        candidate_last_log_term: u64,
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

        // 3. HẠN CHẾ BẦU CỬ (Election Restriction): chỉ bầu cho ứng viên có nhật ký
        //    "mới ít nhất bằng" của mình — so term bản ghi cuối trước, rồi tới độ dài.
        //    Nhờ vậy Leader mới luôn chứa mọi bản ghi đã được cam kết.
        let candidate_up_to_date = (candidate_last_log_term, candidate_last_log_index)
            >= (self.last_log_term(), self.last_log_index());
        if !candidate_up_to_date {
            println!(
                "    [Node {}] Từ chối bầu cho Node {}: nhật ký của ứng viên cũ hơn (term cuối {}, index cuối {})",
                self.node_id, candidate_id, candidate_last_log_term, candidate_last_log_index
            );
            return false;
        }

        // 4. Nếu chưa bỏ phiếu cho ai trong nhiệm kỳ này: Đồng ý bỏ phiếu!
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

        let new_index = self.last_log_index() + 1;
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

    /// Follower xử lý AppendEntries RPC. Trả `true` nếu nhận thành công.
    pub fn handle_append_entries(
        &mut self,
        leader_term: u64,
        prev_log_index: u64,
        prev_log_term: u64,
        entries: &[LogEntry],
        leader_commit: u64,
    ) -> bool {
        if leader_term < self.current_term {
            return false; // Leader cũ đã bị lật đổ
        }
        if leader_term > self.current_term {
            // Sang term mới: lá phiếu của term cũ hết hiệu lực. Không xoá thì nút có thể
            // từ chối nhầm ứng viên hợp lệ ở term mới (hại tính sống, dù vẫn an toàn).
            self.current_term = leader_term;
            self.voted_for = None;
        }
        self.role = RaftRole::Follower;

        // Kiểm tra tính nhất quán: phải có bản ghi prev_log_index với đúng term
        if self.term_at(prev_log_index) != Some(prev_log_term) {
            return false; // Leader sẽ lùi prev_log_index và thử lại
        }

        // CHỈ cắt nhật ký khi gặp XUNG ĐỘT (cùng index, khác term). Bản ghi đã có và
        // khớp thì bỏ qua — nếu cắt vô điều kiện, một AppendEntries cũ/đến trễ sẽ xóa
        // mất các bản ghi phía sau, kể cả bản ghi đã được cam kết.
        for entry in entries {
            match self.term_at(entry.index) {
                Some(term) if term == entry.term => continue,
                Some(_) => {
                    self.log.truncate(entry.index as usize - 1);
                    self.log.push(entry.clone());
                }
                None => self.log.push(entry.clone()),
            }
        }

        // Cập nhật commit_index theo Leader (không vượt quá bản ghi mới nhất vừa nhận)
        let last_new_index = prev_log_index + entries.len() as u64;
        if leader_commit > self.commit_index {
            self.commit_index = leader_commit.min(last_new_index);
        }
        true
    }

    /// Leader ghi nhận một Follower đã khớp nhật ký tới `match_index`
    pub fn record_replication(&mut self, follower_id: u64, match_index: u64) {
        let current = self.match_index.entry(follower_id).or_insert(0);
        *current = (*current).max(match_index);
    }

    /// Leader nâng commit_index tới chỉ số N CAO NHẤT thỏa:
    ///   (a) N đã nằm trên đa số CẢ CỤM (tính cả Leader), và
    ///   (b) bản ghi N thuộc term HIỆN TẠI của Leader.
    /// Bản ghi của term cũ không được cam kết bằng cách đếm bản sao (xem Hình 8 trong
    /// bài báo Raft); chúng được cam kết gián tiếp khi một bản ghi phía sau thuộc term
    /// hiện tại được cam kết.
    pub fn advance_commit_index(&mut self, total_nodes: usize) {
        if self.role != RaftRole::Leader {
            return;
        }
        for n in (self.commit_index + 1..=self.last_log_index()).rev() {
            if self.term_at(n) != Some(self.current_term) {
                continue;
            }
            let replicas = 1 + self.match_index.values().filter(|&&m| m >= n).count();
            if replicas >= quorum(total_nodes) {
                self.commit_index = n;
                println!(
                    "    [Leader Node {}] Index #{} có trên {}/{} nút: CAM KẾT tới index #{}!",
                    self.node_id, n, replicas, total_nodes, n
                );
                return;
            }
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   ĐỒNG THUẬN PHÂN TÁN RAFT & CAP THEOREM SIMULATION TRONG RUST   ");
    println!("==================================================================");

    // 1. Khởi tạo một cụm gồm 3 nút mạng phân tán (Node 1, Node 2, Node 3)
    let total_nodes = 3;
    let mut node1 = RaftNode::new(1);
    let mut node2 = RaftNode::new(2);
    let mut node3 = RaftNode::new(3);

    println!("\n[1] Khởi tạo cụm 3 nút mạng (Tất cả đều là Follower ban đầu):");
    for node in [&node1, &node2, &node3] {
        println!(
            "    - Node {} Role: {:?} | Term: {}",
            node.node_id, node.role, node.current_term
        );
    }

    // 2. Mô phỏng Node 1 bị hết hạn chờ (Election Timeout) và phát động tranh cử
    println!("\n[2] Node 1 bị Timeout và khởi động tranh cử lãnh đạo (Election):");
    node1.handle_election_timeout(total_nodes);
    let (term, last_idx, last_term) = (
        node1.current_term,
        node1.last_log_index(),
        node1.last_log_term(),
    );
    let vote_from_2 = node2.handle_request_vote(1, term, last_idx, last_term);
    node1.receive_vote(vote_from_2, total_nodes);
    let vote_from_3 = node3.handle_request_vote(1, term, last_idx, last_term);
    node1.receive_vote(vote_from_3, total_nodes); // đã là Leader: phiếu thừa bị bỏ qua
    assert_eq!(node1.role, RaftRole::Leader);

    // 3. Client gửi lệnh ghi; Leader sao chép sang Node 2 (Node 3 tạm mất mạng)
    println!("\n[3] Client gửi giao dịch 'CHUYEN_TIEN_ALICE_TO_BOB_100K' tới Leader:");
    let log_idx = node1
        .append_client_command("CHUYEN_TIEN_ALICE_TO_BOB_100K")
        .unwrap();
    let entry = node1.log[log_idx as usize - 1].clone();
    println!("    - Leader Node 1 gửi AppendEntries sang Node 2 (Node 3 mất kết nối)...");
    let ok = node2.handle_append_entries(node1.current_term, 0, 0, &[entry], node1.commit_index);
    if ok {
        node1.record_replication(2, log_idx);
    }

    // Leader kiểm tra Quorum để quyết định Commit (Node 1 + Node 2 = 2/3)
    node1.advance_commit_index(total_nodes);
    assert_eq!(node1.commit_index, log_idx);

    // 4. Node 3 (nhật ký rỗng, tụt hậu) hết hạn chờ và đòi làm Leader của Term 2
    println!("\n[4] Node 3 (nhật ký rỗng) tranh cử — Hạn chế bầu cử phải chặn nó:");
    node3.handle_election_timeout(total_nodes);
    let vote = node2.handle_request_vote(
        3,
        node3.current_term,
        node3.last_log_index(),
        node3.last_log_term(),
    );
    assert!(
        !vote,
        "Node 2 giữ bản ghi đã cam kết, không được bầu cho Node 3"
    );
    println!("    => Node 3 không thể đắc cử, bản ghi đã cam kết không bị mất.");

    println!("\n==================================================================");
    println!("   XÁC NHẬN: BẦU CỬ, SAO CHÉP VÀ CAM KẾT ĐÚNG QUY TẮC AN TOÀN RAFT ");
    println!("==================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(term: u64, index: u64) -> LogEntry {
        LogEntry {
            term,
            index,
            command: format!("cmd{index}"),
        }
    }

    fn leader_with_log(terms: &[u64], current_term: u64) -> RaftNode {
        let mut node = RaftNode::new(1);
        node.log = terms
            .iter()
            .enumerate()
            .map(|(i, &t)| entry(t, i as u64 + 1))
            .collect();
        node.current_term = current_term;
        node.role = RaftRole::Leader;
        node
    }

    #[test]
    fn rejects_candidate_with_stale_log() {
        let mut voter = RaftNode::new(2);
        voter.log = vec![entry(1, 1), entry(2, 2)];
        voter.current_term = 2;
        // Term ứng viên cao hơn nhưng nhật ký thiếu bản ghi term 2
        assert!(!voter.handle_request_vote(3, 3, 1, 1));
        // Nhật ký dài bằng nhưng term cuối thấp hơn cũng bị từ chối
        assert!(!voter.handle_request_vote(3, 3, 5, 1));
    }

    #[test]
    fn heartbeat_from_newer_term_clears_old_vote() {
        let mut node = RaftNode::new(2);
        // Term 1: đã bầu cho nút 1
        assert!(node.handle_request_vote(1, 1, 0, 0));
        // Leader term 2 (không phải nút ta đã bầu) gửi nhịp tim
        assert!(node.handle_append_entries(2, 0, 0, &[], 0));
        // Term 3: nút 3 ứng cử — lá phiếu term 1 không được chặn lá phiếu term 3
        assert!(node.handle_request_vote(3, 3, 0, 0));
    }

    #[test]
    fn grants_vote_to_more_up_to_date_candidate() {
        let mut voter = RaftNode::new(2);
        voter.log = vec![entry(1, 1), entry(1, 2), entry(1, 3)];
        // Ngắn hơn nhưng term cuối cao hơn -> mới hơn
        assert!(voter.handle_request_vote(3, 4, 2, 2));
    }

    #[test]
    fn commits_per_index_not_whole_log() {
        let mut leader = leader_with_log(&[1, 1, 1], 1);
        leader.record_replication(2, 1); // Follower 2 mới chỉ có index 1
        leader.advance_commit_index(3);
        // Bản cũ đặt commit_index = log.len() = 3 dù index 2, 3 chỉ có trên Leader
        assert_eq!(leader.commit_index, 1);
        leader.record_replication(3, 3);
        leader.advance_commit_index(3);
        assert_eq!(leader.commit_index, 3);
    }

    #[test]
    fn old_term_entry_not_committed_by_counting_replicas() {
        // Leader term 4 mang bản ghi index 2 từ term 2 (Hình 8 bài báo Raft)
        let mut leader = leader_with_log(&[1, 2], 4);
        leader.record_replication(2, 2);
        leader.advance_commit_index(3);
        assert_eq!(leader.commit_index, 0, "không được cam kết bản ghi term cũ");
        // Khi một bản ghi term 4 được sao chép quá bán, mọi thứ trước nó cũng cam kết
        leader.append_client_command("new").unwrap();
        leader.record_replication(2, 3);
        leader.advance_commit_index(3);
        assert_eq!(leader.commit_index, 3);
    }

    #[test]
    fn stale_append_entries_does_not_truncate_committed_entries() {
        let mut follower = RaftNode::new(2);
        let log = vec![entry(1, 1), entry(1, 2), entry(1, 3)];
        assert!(follower.handle_append_entries(1, 0, 0, &log, 3));
        assert_eq!(follower.commit_index, 3);
        // Một AppendEntries cũ đến trễ chỉ chứa bản ghi 1
        assert!(follower.handle_append_entries(1, 0, 0, &log[..1], 1));
        assert_eq!(follower.log.len(), 3);
        assert_eq!(follower.commit_index, 3);
    }

    #[test]
    fn conflicting_suffix_is_replaced() {
        let mut follower = RaftNode::new(2);
        follower.log = vec![entry(1, 1), entry(2, 2), entry(2, 3)]; // từ Leader cũ term 2
        follower.current_term = 2;
        let fresh = [entry(3, 2)];
        assert!(follower.handle_append_entries(3, 1, 1, &fresh, 0));
        assert_eq!(follower.log, vec![entry(1, 1), entry(3, 2)]);
    }

    #[test]
    fn rejects_when_prev_entry_missing() {
        let mut follower = RaftNode::new(2);
        assert!(!follower.handle_append_entries(1, 5, 1, &[entry(1, 6)], 0));
        assert!(follower.log.is_empty());
    }
}
