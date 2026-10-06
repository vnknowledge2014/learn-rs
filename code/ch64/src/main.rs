//! Chương 64 — Hệ điều hành từ bên trong: Lập lịch CPU, Phân trang bộ nhớ ảo,
//! Phát hiện bế tắc. Mô phỏng tất định nên kiểm thử được.

use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// 1. TIẾN TRÌNH & KHỐI ĐIỀU KHIỂN TIẾN TRÌNH (PCB)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessState {
    New,     // vừa tạo
    Ready,   // chờ được cấp CPU
    Running, // đang giữ CPU
    Waiting, // chờ I/O
    Finished,
}

/// Khối điều khiển tiến trình — thứ mà nhân hệ điều hành lưu cho MỖI tiến trình.
#[derive(Debug, Clone, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub arrives_at: u64,  // thời điểm đến (arrival time)
    pub time_needed: u64, // burst time — tổng CPU cần
    pub remaining: u64,
    pub priority: u8, // số nhỏ = ưu tiên cao
    pub state: ProcessState,
    pub start: Option<u64>,
    pub end: Option<u64>,
}

impl Process {
    pub fn new(pid: u32, name: &str, arrives_at: u64, burst: u64, priority: u8) -> Self {
        Process {
            pid,
            name: name.to_string(),
            arrives_at,
            time_needed: burst,
            remaining: burst,
            priority,
            state: ProcessState::New,
            start: None,
            end: None,
        }
    }
    /// Thời gian hoàn thành = lúc xong - lúc đến.
    pub fn turnaround_time(&self) -> Option<u64> {
        self.end.map(|end| end - self.arrives_at)
    }
    /// Thời gian chờ = quay vòng - thời gian thực sự dùng CPU.
    pub fn waiting_time(&self) -> Option<u64> {
        self.turnaround_time().map(|t| t - self.time_needed)
    }
}

#[derive(Debug, PartialEq)]
pub struct ScheduleResult {
    pub timeline: Vec<(u64, u32)>, // (thời điểm, pid đang chạy)
    pub processes: Vec<Process>,
    pub avg_wait: f64,
    pub mean_turnaround: f64,
}

fn summarize(procs: Vec<Process>, timeline: Vec<(u64, u32)>) -> ScheduleResult {
    let n = procs.len() as f64;
    let total_wait: u64 = procs.iter().filter_map(|p| p.waiting_time()).sum();
    let total_turnaround: u64 = procs.iter().filter_map(|p| p.turnaround_time()).sum();
    ScheduleResult {
        timeline,
        avg_wait: total_wait as f64 / n,
        mean_turnaround: total_turnaround as f64 / n,
        processes: procs,
    }
}

// ============================================================================
// 2. BA THUẬT TOÁN LẬP LỊCH CPU
// ============================================================================

/// FCFS (First-Come First-Served): ai đến trước chạy trước, chạy tới xong.
/// Nhược điểm kinh điển: "hiệu ứng đoàn xe" — một tiến trình dài chặn tất cả.
pub fn schedule_fcfs(mut procs: Vec<Process>) -> ScheduleResult {
    procs.sort_by_key(|p| (p.arrives_at, p.pid));
    let mut clock = 0u64;
    let mut timeline = Vec::new();
    for p in procs.iter_mut() {
        if clock < p.arrives_at {
            clock = p.arrives_at; // CPU rảnh, chờ tiến trình tới
        }
        p.start = Some(clock);
        for _ in 0..p.time_needed {
            timeline.push((clock, p.pid));
            clock += 1;
        }
        p.remaining = 0;
        p.end = Some(clock);
        p.state = ProcessState::Finished;
    }
    summarize(procs, timeline)
}

/// SJF không tiếm quyền (Shortest Job First): luôn chọn việc NGẮN NHẤT đang chờ.
/// Tối ưu về thời gian chờ trung bình — nhưng có thể gây "đói" cho việc dài.
pub fn schedule_sjf(mut procs: Vec<Process>) -> ScheduleResult {
    let n = procs.len();
    let mut done = 0;
    let mut clock = 0u64;
    let mut timeline = Vec::new();
    let mut finished = vec![false; n];

    while done < n {
        // Trong số các tiến trình ĐÃ TỚI và chưa chạy, chọn cái ngắn nhất
        let pick = (0..n)
            .filter(|&i| !finished[i] && procs[i].arrives_at <= clock)
            .min_by_key(|&i| (procs[i].time_needed, procs[i].pid));
        match pick {
            Some(i) => {
                procs[i].start = Some(clock);
                for _ in 0..procs[i].time_needed {
                    timeline.push((clock, procs[i].pid));
                    clock += 1;
                }
                procs[i].remaining = 0;
                procs[i].end = Some(clock);
                procs[i].state = ProcessState::Finished;
                finished[i] = true;
                done += 1;
            }
            None => clock += 1, // chưa ai tới, CPU rảnh
        }
    }
    summarize(procs, timeline)
}

/// Round-Robin: mỗi tiến trình được một "lượng tử thời gian", hết thì nhường.
/// Đây là thuật toán của hệ điều hành tương tác — bảo đảm không ai bị đói.
pub fn schedule_round_robin(mut procs: Vec<Process>, quantum: u64) -> ScheduleResult {
    let n = procs.len();
    let mut clock = 0u64;
    let mut timeline = Vec::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut admitted = vec![false; n];
    let mut done = 0;

    // Đưa vào hàng đợi những tiến trình đã tới tại thời điểm 0
    let admit =
        |clock: u64, queue: &mut VecDeque<usize>, admitted: &mut [bool], procs: &[Process]| {
            let mut arrived: Vec<usize> = (0..procs.len())
                .filter(|&i| !admitted[i] && procs[i].arrives_at <= clock)
                .collect();
            arrived.sort_by_key(|&i| (procs[i].arrives_at, procs[i].pid));
            for i in arrived {
                admitted[i] = true;
                queue.push_back(i);
            }
        };
    admit(clock, &mut queue, &mut admitted, &procs);

    while done < n {
        match queue.pop_front() {
            Some(i) => {
                if procs[i].start.is_none() {
                    procs[i].start = Some(clock);
                }
                let run = quantum.min(procs[i].remaining);
                for _ in 0..run {
                    timeline.push((clock, procs[i].pid));
                    clock += 1;
                    admit(clock, &mut queue, &mut admitted, &procs); // tiến trình mới tới trong lúc chạy
                }
                procs[i].remaining -= run;
                if procs[i].remaining == 0 {
                    procs[i].end = Some(clock);
                    procs[i].state = ProcessState::Finished;
                    done += 1;
                } else {
                    queue.push_back(i); // chưa xong -> quay lại cuối hàng
                }
            }
            None => {
                clock += 1;
                admit(clock, &mut queue, &mut admitted, &procs);
            }
        }
    }
    summarize(procs, timeline)
}

// ============================================================================
// 3. BỘ NHỚ ẢO — PHÂN TRANG & THAY TRANG
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct ReplacementResult {
    pub page_faults: usize,           // số lỗi trang
    pub frame_history: Vec<Vec<u64>>, // nội dung các khung sau mỗi lần truy cập
}

/// FIFO: trang vào trước ra trước. Đơn giản nhưng có "nghịch lý Belady".
pub fn fifo_replace(refs: &[u64], num_frames: usize) -> ReplacementResult {
    let mut frames: VecDeque<u64> = VecDeque::new();
    let mut resident: HashSet<u64> = HashSet::new();
    let mut faults = 0;
    let mut history = Vec::new();
    for &t in refs {
        if !resident.contains(&t) {
            faults += 1;
            if frames.len() == num_frames
                && let Some(old) = frames.pop_front()
            {
                resident.remove(&old);
            }
            frames.push_back(t);
            resident.insert(t);
        }
        history.push(frames.iter().copied().collect());
    }
    ReplacementResult {
        page_faults: faults,
        frame_history: history,
    }
}

/// LRU (Least Recently Used): thay trang lâu không dùng nhất.
/// Xấp xỉ tốt cho "nguyên lý cục bộ" — chương trình hay dùng lại thứ vừa dùng.
pub fn lru_replace(refs: &[u64], num_frames: usize) -> ReplacementResult {
    let mut frames: Vec<u64> = Vec::new();
    let mut last_used: HashMap<u64, usize> = HashMap::new();
    let mut faults = 0;
    let mut history = Vec::new();
    for (timestamp, &t) in refs.iter().enumerate() {
        if !frames.contains(&t) {
            faults += 1;
            if frames.len() == num_frames {
                // tìm trang có lần dùng cuối XA NHẤT
                let victim = frames
                    .iter()
                    .copied()
                    .min_by_key(|p| *last_used.get(p).unwrap_or(&0))
                    .unwrap();
                frames.retain(|&p| p != victim);
                last_used.remove(&victim);
            }
            frames.push(t);
        }
        last_used.insert(t, timestamp);
        history.push(frames.clone());
    }
    ReplacementResult {
        page_faults: faults,
        frame_history: history,
    }
}

/// OPT (tối ưu, Bélády): thay trang sẽ được dùng XA NHẤT trong tương lai.
/// Không cài được thật (cần biết tương lai) nhưng là CHUẨN SO SÁNH lý thuyết.
pub fn optimal_replacement(refs: &[u64], num_frames: usize) -> ReplacementResult {
    let mut frames: Vec<u64> = Vec::new();
    let mut faults = 0;
    let mut history = Vec::new();
    for i in 0..refs.len() {
        let t = refs[i];
        if !frames.contains(&t) {
            faults += 1;
            if frames.len() == num_frames {
                // trang nào KHÔNG xuất hiện lại, hoặc xuất hiện muộn nhất -> loại
                let victim = frames
                    .iter()
                    .copied()
                    .max_by_key(|p| {
                        refs[i + 1..]
                            .iter()
                            .position(|x| x == p)
                            .unwrap_or(usize::MAX)
                    })
                    .unwrap();
                frames.retain(|&p| p != victim);
            }
            frames.push(t);
        }
        history.push(frames.clone());
    }
    ReplacementResult {
        page_faults: faults,
        frame_history: history,
    }
}

// ============================================================================
// 4. BẾ TẮC (Deadlock) — PHÁT HIỆN BẰNG ĐỒ THỊ CHỜ
// ============================================================================

/// Đồ thị "chờ đợi": tiến trình A -> B nghĩa là A đang chờ tài nguyên B giữ.
/// Có CHU TRÌNH trong đồ thị này = có BẾ TẮC.
#[derive(Default)]
pub struct WaitForGraph {
    edges: HashMap<u32, Vec<u32>>,
}

impl WaitForGraph {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add_wait(&mut self, waiter: u32, holder: u32) {
        self.edges.entry(waiter).or_default().push(holder);
    }

    /// Phát hiện bế tắc = tìm chu trình bằng DFS 3 màu.
    pub fn has_deadlock(&self) -> Option<Vec<u32>> {
        let mut color: HashMap<u32, u8> = HashMap::new(); // 0=trắng 1=xám 2=đen
        let mut path: Vec<u32> = Vec::new();
        let mut nodes: Vec<u32> = self.edges.keys().copied().collect();
        nodes.sort();
        for d in nodes {
            if color.get(&d).copied().unwrap_or(0) == 0
                && let Some(cycle) = self.dfs(d, &mut color, &mut path)
            {
                return Some(cycle);
            }
        }
        None
    }

    fn dfs(&self, d: u32, color: &mut HashMap<u32, u8>, path: &mut Vec<u32>) -> Option<Vec<u32>> {
        color.insert(d, 1); // xám = đang thăm
        path.push(d);
        if let Some(next) = self.edges.get(&d) {
            let mut next = next.clone();
            next.sort();
            for k in next {
                match color.get(&k).copied().unwrap_or(0) {
                    1 => {
                        // gặp lại đỉnh XÁM -> có chu trình
                        let start = path.iter().position(|&x| x == k).unwrap();
                        return Some(path[start..].to_vec());
                    }
                    0 => {
                        if let Some(c) = self.dfs(k, color, path) {
                            return Some(c);
                        }
                    }
                    _ => {}
                }
            }
        }
        path.pop();
        color.insert(d, 2); // đen = xong
        None
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   HỆ ĐIỀU HÀNH: LẬP LỊCH CPU · PHÂN TRANG · PHÁT HIỆN BẾ TẮC   ");
    println!("═══════════════════════════════════════════════════════════════");

    let make_processes = || {
        vec![
            Process::new(1, "browser", 0, 8, 2),
            Process::new(2, "editor", 1, 4, 1),
            Process::new(3, "video-encode", 2, 9, 3),
            Process::new(4, "cloud-sync", 3, 5, 2),
        ]
    };

    println!("\n1. LẬP LỊCH CPU — cùng 4 tiến trình, ba thuật toán");
    for (name, result) in [
        ("FCFS       ", schedule_fcfs(make_processes())),
        ("SJF        ", schedule_sjf(make_processes())),
        ("Round-Robin", schedule_round_robin(make_processes(), 3)),
    ] {
        println!(
            "   {} | chờ TB = {:>5.2} | quay vòng TB = {:>5.2}",
            name, result.avg_wait, result.mean_turnaround
        );
    }
    println!("   → SJF tối ưu thời gian chờ, nhưng Round-Robin công bằng hơn (không ai bị đói).");

    println!("\n2. THAY TRANG BỘ NHỚ ẢO (3 khung nhớ)");
    let refs = [
        7u64, 0, 1, 2, 0, 3, 0, 4, 2, 3, 0, 3, 2, 1, 2, 0, 1, 7, 0, 1,
    ];
    for (name, result) in [
        ("FIFO   ", fifo_replace(&refs, 3)),
        ("LRU    ", lru_replace(&refs, 3)),
        ("Tối ưu ", optimal_replacement(&refs, 3)),
    ] {
        println!("   {} | {} lỗi trang", name, result.page_faults);
    }
    println!("   → Tối ưu là CẬN DƯỚI lý thuyết (cần biết tương lai). LRU bám sát nó nhất.");

    println!("\n3. NGHỊCH LÝ BÉLÁDY — thêm khung nhớ mà LỖI TRANG TĂNG!");
    let belady = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
    println!(
        "   FIFO 3 khung: {} lỗi",
        fifo_replace(&belady, 3).page_faults
    );
    println!(
        "   FIFO 4 khung: {} lỗi  ← NHIỀU HƠN dù có thêm bộ nhớ!",
        fifo_replace(&belady, 4).page_faults
    );
    println!(
        "   LRU  3 khung: {} lỗi",
        lru_replace(&belady, 3).page_faults
    );
    println!(
        "   LRU  4 khung: {} lỗi  ← LRU không bị nghịch lý này",
        lru_replace(&belady, 4).page_faults
    );

    println!("\n4. PHÁT HIỆN BẾ TẮC");
    let mut g = WaitForGraph::new();
    g.add_wait(1, 2); // P1 chờ tài nguyên P2 giữ
    g.add_wait(2, 3);
    g.add_wait(3, 1); // ... và P3 chờ P1 -> VÒNG TRÒN
    println!("   Đồ thị P1→P2→P3→P1: {:?}", g.has_deadlock());
    let mut g2 = WaitForGraph::new();
    g2.add_wait(1, 2);
    g2.add_wait(2, 3);
    println!(
        "   Đồ thị P1→P2→P3   : {:?} (không bế tắc)",
        g2.has_deadlock()
    );

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   HỆ ĐIỀU HÀNH = TRỌNG TÀI PHÂN PHỐI TÀI NGUYÊN CÓ HẠN         ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<Process> {
        vec![
            Process::new(1, "A", 0, 5, 1),
            Process::new(2, "B", 1, 3, 2),
            Process::new(3, "C", 2, 1, 3),
        ]
    }

    #[test]
    fn fcfs_runs_in_arrival_order() {
        let result = schedule_fcfs(sample());
        // A(0-5), B(5-8), C(8-9)
        assert_eq!(result.processes[0].end, Some(5));
        assert_eq!(result.processes[1].end, Some(8));
        assert_eq!(result.processes[2].end, Some(9));
        assert_eq!(result.timeline.len(), 9); // tổng burst = 5+3+1
    }

    #[test]
    fn sjf_beats_fcfs_on_average_wait() {
        let f = schedule_fcfs(sample());
        let s = schedule_sjf(sample());
        // SJF tối ưu thời gian chờ trung bình (định lý kinh điển)
        assert!(
            s.avg_wait <= f.avg_wait,
            "SJF ({}) phải <= FCFS ({})",
            s.avg_wait,
            f.avg_wait
        );
    }

    #[test]
    fn round_robin_starves_nobody() {
        let result = schedule_round_robin(sample(), 2);
        // Mọi tiến trình đều hoàn thành
        assert!(result.processes.iter().all(|p| p.end.is_some()));
        assert!(result.processes.iter().all(|p| p.remaining == 0));
        // Tổng thời gian CPU đúng bằng tổng burst
        assert_eq!(result.timeline.len(), 9);
    }

    #[test]
    fn every_scheduler_runs_total_burst() {
        for result in [
            schedule_fcfs(sample()),
            schedule_sjf(sample()),
            schedule_round_robin(sample(), 3),
        ] {
            assert_eq!(result.timeline.len(), 9, "phải dùng đúng 9 đơn vị CPU");
        }
    }

    #[test]
    fn optimal_replacement_is_a_lower_bound() {
        let refs = [
            7u64, 0, 1, 2, 0, 3, 0, 4, 2, 3, 0, 3, 2, 1, 2, 0, 1, 7, 0, 1,
        ];
        let opt = optimal_replacement(&refs, 3).page_faults;
        let lru = lru_replace(&refs, 3).page_faults;
        let fifo = fifo_replace(&refs, 3).page_faults;
        // OPT là cận dưới lý thuyết — không thuật toán nào tốt hơn
        assert!(opt <= lru, "OPT({}) phải <= LRU({})", opt, lru);
        assert!(opt <= fifo, "OPT({}) phải <= FIFO({})", opt, fifo);
    }

    #[test]
    fn belady_anomaly_is_real_for_fifo() {
        let refs = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
        let three = fifo_replace(&refs, 3).page_faults;
        let four = fifo_replace(&refs, 4).page_faults;
        // NGHỊCH LÝ: thêm khung nhớ mà lỗi trang lại TĂNG
        assert!(
            four > three,
            "Bélády: FIFO 4 khung ({}) phải nhiều lỗi hơn 3 khung ({})",
            four,
            three
        );
    }

    #[test]
    fn lru_is_immune_to_belady() {
        let refs = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
        let three = lru_replace(&refs, 3).page_faults;
        let four = lru_replace(&refs, 4).page_faults;
        // LRU là thuật toán "ngăn xếp" -> thêm khung KHÔNG BAO GIỜ làm tệ hơn
        assert!(
            four <= three,
            "LRU 4 khung ({}) không được tệ hơn 3 khung ({})",
            four,
            three
        );
    }

    #[test]
    fn enough_frames_means_only_compulsory_faults() {
        let refs = [1u64, 2, 3, 1, 2, 3, 1, 2, 3];
        // 3 trang khác nhau, 5 khung -> chỉ 3 lỗi bắt buộc (compulsory miss)
        assert_eq!(lru_replace(&refs, 5).page_faults, 3);
        assert_eq!(fifo_replace(&refs, 5).page_faults, 3);
    }

    #[test]
    fn detects_deadlock_on_cycle() {
        let mut g = WaitForGraph::new();
        g.add_wait(1, 2);
        g.add_wait(2, 3);
        g.add_wait(3, 1);
        let cycle = g.has_deadlock().expect("phải phát hiện bế tắc");
        assert_eq!(cycle.len(), 3);
        assert!(cycle.contains(&1) && cycle.contains(&2) && cycle.contains(&3));
    }

    #[test]
    fn no_deadlock_on_acyclic_graph() {
        let mut g = WaitForGraph::new();
        g.add_wait(1, 2);
        g.add_wait(2, 3);
        g.add_wait(1, 3); // vẫn không có chu trình
        assert_eq!(g.has_deadlock(), None);
    }

    #[test]
    fn classic_two_process_deadlock() {
        // P1 giữ A chờ B; P2 giữ B chờ A — bế tắc đơn giản nhất
        let mut g = WaitForGraph::new();
        g.add_wait(1, 2);
        g.add_wait(2, 1);
        assert!(g.has_deadlock().is_some());
    }
}
