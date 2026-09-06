#![allow(dead_code, unused_variables)]
//! Chương 64 — Hệ điều hành từ bên trong: Lập lịch CPU, Phân trang bộ nhớ ảo,
//! Phát hiện bế tắc. Mô phỏng tất định nên kiểm thử được.

use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// 1. TIẾN TRÌNH & KHỐI ĐIỀU KHIỂN TIẾN TRÌNH (PCB)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateProcess {
    Moi,        // vừa tạo
    SanSang,    // chờ được cấp CPU
    DangChay,   // đang giữ CPU
    Cho,        // chờ I/O
    Finished,
}

/// Khối điều khiển tiến trình — thứ mà nhân hệ điều hành lưu cho MỖI tiến trình.
#[derive(Debug, Clone, PartialEq)]
pub struct Process {
    pub pid: u32,
    pub name: String,
    pub arrives_at: u64,   // arrival time
    pub time_time_can: u64,   // burst time — tổng CPU cần
    pub remaining: u64,
    pub uu_tien: u8,          // số nhỏ = ưu tiên cao
    pub state: StateProcess,
    pub start: Option<u64>,
    pub end: Option<u64>,
}

impl Process {
    pub fn new(pid: u32, name: &str, den: u64, can: u64, uu_tien: u8) -> Self {
        Process {
            pid, name: name.to_string(), arrives_at: den,
            time_time_can: can, remaining: can, uu_tien,
            state: StateProcess::Moi, start: None, end: None,
        }
    }
    /// Thời gian hoàn thành = lúc xong - lúc đến.
    pub fn turnaround_time(&self) -> Option<u64> {
        self.end.map(|k| k - self.arrives_at)
    }
    /// Thời gian chờ = quay vòng - thời gian thực sự dùng CPU.
    pub fn time_time_wait(&self) -> Option<u64> {
        self.turnaround_time().map(|q| q - self.time_time_can)
    }
}

#[derive(Debug, PartialEq)]
pub struct KetQuaLapLich {
    pub timeline: Vec<(u64, u32)>, // (thời điểm, pid đang chạy)
    pub process: Vec<Process>,
    pub wait_mean: f64,
    pub mean_turnaround: f64,
}

fn tong_ket(tt: Vec<Process>, dtg: Vec<(u64, u32)>) -> KetQuaLapLich {
    let n = tt.len() as f64;
    let tong_cho: u64 = tt.iter().filter_map(|p| p.time_time_wait()).sum();
    let tong_qv: u64 = tt.iter().filter_map(|p| p.turnaround_time()).sum();
    KetQuaLapLich {
        timeline: dtg,
        wait_mean: tong_cho as f64 / n,
        mean_turnaround: tong_qv as f64 / n,
        process: tt,
    }
}

// ============================================================================
// 2. BA THUẬT TOÁN LẬP LỊCH CPU
// ============================================================================

/// FCFS (First-Come First-Served): ai đến trước chạy trước, chạy tới xong.
/// Nhược điểm kinh điển: "hiệu ứng đoàn xe" — một tiến trình dài chặn tất cả.
pub fn lap_lich_fcfs(mut tt: Vec<Process>) -> KetQuaLapLich {
    tt.sort_by_key(|p| (p.arrives_at, p.pid));
    let mut clock = 0u64;
    let mut dtg = Vec::new();
    for p in tt.iter_mut() {
        if clock < p.arrives_at {
            clock = p.arrives_at; // CPU rảnh, chờ tiến trình tới
        }
        p.start = Some(clock);
        for _ in 0..p.time_time_can {
            dtg.push((clock, p.pid));
            clock += 1;
        }
        p.remaining = 0;
        p.end = Some(clock);
        p.state = StateProcess::Finished;
    }
    tong_ket(tt, dtg)
}

/// SJF không tiếm quyền (Shortest Job First): luôn chọn việc NGẮN NHẤT đang chờ.
/// Tối ưu về thời gian chờ trung bình — nhưng có thể gây "đói" cho việc dài.
pub fn lap_lich_sjf(mut tt: Vec<Process>) -> KetQuaLapLich {
    let n = tt.len();
    let mut done = 0;
    let mut clock = 0u64;
    let mut dtg = Vec::new();
    let mut da_chay = vec![false; n];

    while done < n {
        // Trong số các tiến trình ĐÃ TỚI và chưa chạy, chọn cái ngắn nhất
        let pick = (0..n)
            .filter(|&i| !da_chay[i] && tt[i].arrives_at <= clock)
            .min_by_key(|&i| (tt[i].time_time_can, tt[i].pid));
        match pick {
            Some(i) => {
                tt[i].start = Some(clock);
                for _ in 0..tt[i].time_time_can {
                    dtg.push((clock, tt[i].pid));
                    clock += 1;
                }
                tt[i].remaining = 0;
                tt[i].end = Some(clock);
                tt[i].state = StateProcess::Finished;
                da_chay[i] = true;
                done += 1;
            }
            None => clock += 1, // chưa ai tới, CPU rảnh
        }
    }
    tong_ket(tt, dtg)
}

/// Round-Robin: mỗi tiến trình được một "lượng tử thời gian", hết thì nhường.
/// Đây là thuật toán của hệ điều hành tương tác — bảo đảm không ai bị đói.
pub fn lap_lich_round_robin(mut tt: Vec<Process>, luong_tu: u64) -> KetQuaLapLich {
    let n = tt.len();
    let mut clock = 0u64;
    let mut dtg = Vec::new();
    let mut queue: VecDeque<usize> = VecDeque::new();
    let mut da_in = vec![false; n];
    let mut done = 0;

    // Đưa vào hàng đợi những tiến trình đã tới tại thời điểm 0
    let nap = |clock: u64, queue: &mut VecDeque<usize>, da_in: &mut Vec<bool>, tt: &Vec<Process>| {
        let mut new: Vec<usize> = (0..tt.len())
            .filter(|&i| !da_in[i] && tt[i].arrives_at <= clock)
            .collect();
        new.sort_by_key(|&i| (tt[i].arrives_at, tt[i].pid));
        for i in new { da_in[i] = true; queue.push_back(i); }
    };
    nap(clock, &mut queue, &mut da_in, &tt);

    while done < n {
        match queue.pop_front() {
            Some(i) => {
                if tt[i].start.is_none() { tt[i].start = Some(clock); }
                let run = luong_tu.min(tt[i].remaining);
                for _ in 0..run {
                    dtg.push((clock, tt[i].pid));
                    clock += 1;
                    nap(clock, &mut queue, &mut da_in, &tt); // tiến trình mới tới trong lúc chạy
                }
                tt[i].remaining -= run;
                if tt[i].remaining == 0 {
                    tt[i].end = Some(clock);
                    tt[i].state = StateProcess::Finished;
                    done += 1;
                } else {
                    queue.push_back(i); // chưa xong -> quay lại cuối hàng
                }
            }
            None => {
                clock += 1;
                nap(clock, &mut queue, &mut da_in, &tt);
            }
        }
    }
    tong_ket(tt, dtg)
}

// ============================================================================
// 3. BỘ NHỚ ẢO — PHÂN TRANG & THAY TRANG
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct StateChange {
    pub num_error_state: usize, // page faults
    pub series_frame: Vec<Vec<u64>>,
}

/// FIFO: trang vào trước ra trước. Đơn giản nhưng có "nghịch lý Belady".
pub fn fifo_replace(series: &[u64], num_frame: usize) -> StateChange {
    let mut frame: VecDeque<u64> = VecDeque::new();
    let mut visited: HashSet<u64> = HashSet::new();
    let mut error = 0;
    let mut history = Vec::new();
    for &t in series {
        if !visited.contains(&t) {
            error += 1;
            if frame.len() == num_frame {
                if let Some(cu) = frame.pop_front() { visited.remove(&cu); }
            }
            frame.push_back(t);
            visited.insert(t);
        }
        history.push(frame.iter().copied().collect());
    }
    StateChange { num_error_state: error, series_frame: history }
}

/// LRU (Least Recently Used): thay trang lâu không dùng nhất.
/// Xấp xỉ tốt cho "nguyên lý cục bộ" — chương trình hay dùng lại thứ vừa dùng.
pub fn lru_replace(series: &[u64], num_frame: usize) -> StateChange {
    let mut frame: Vec<u64> = Vec::new();
    let mut last_lan: HashMap<u64, usize> = HashMap::new();
    let mut error = 0;
    let mut history = Vec::new();
    for (timestamp, &t) in series.iter().enumerate() {
        if !frame.contains(&t) {
            error += 1;
            if frame.len() == num_frame {
                // tìm trang có lần dùng cuối XA NHẤT
                let nan_nhan = frame.iter().copied()
                    .min_by_key(|p| *last_lan.get(p).unwrap_or(&0)).unwrap();
                frame.retain(|&p| p != nan_nhan);
                last_lan.remove(&nan_nhan);
            }
            frame.push(t);
        }
        last_lan.insert(t, timestamp);
        history.push(frame.clone());
    }
    StateChange { num_error_state: error, series_frame: history }
}

/// OPT (tối ưu, Bélády): thay trang sẽ được dùng XA NHẤT trong tương lai.
/// Không cài được thật (cần biết tương lai) nhưng là CHUẨN SO SÁNH lý thuyết.
pub fn optimal_replacement(series: &[u64], num_frame: usize) -> StateChange {
    let mut frame: Vec<u64> = Vec::new();
    let mut error = 0;
    let mut history = Vec::new();
    for i in 0..series.len() {
        let t = series[i];
        if !frame.contains(&t) {
            error += 1;
            if frame.len() == num_frame {
                // trang nào KHÔNG xuất hiện lại, hoặc xuất hiện muộn nhất -> loại
                let nan_nhan = frame.iter().copied().max_by_key(|p| {
                    series[i + 1..].iter().position(|x| x == p).unwrap_or(usize::MAX)
                }).unwrap();
                frame.retain(|&p| p != nan_nhan);
            }
            frame.push(t);
        }
        history.push(frame.clone());
    }
    StateChange { num_error_state: error, series_frame: history }
}

// ============================================================================
// 4. BẾ TẮC (Deadlock) — PHÁT HIỆN BẰNG ĐỒ THỊ CHỜ
// ============================================================================

/// Đồ thị "chờ đợi": tiến trình A -> B nghĩa là A đang chờ tài nguyên B giữ.
/// Có CHU TRÌNH trong đồ thị này = có BẾ TẮC.
pub struct WaitForGraph {
    edge: HashMap<u32, Vec<u32>>,
}

impl WaitForGraph {
    pub fn new() -> Self { WaitForGraph { edge: HashMap::new() } }
    pub fn them_cho(&mut self, ai_cho: u32, cho_ai: u32) {
        self.edge.entry(ai_cho).or_default().push(cho_ai);
    }

    /// Phát hiện bế tắc = tìm chu trình bằng DFS 3 màu.
    pub fn has_deadlock(&self) -> Option<Vec<u32>> {
        let mut mau: HashMap<u32, u8> = HashMap::new(); // 0=trắng 1=xám 2=đen
        let mut positive: Vec<u32> = Vec::new();
        let mut peak: Vec<u32> = self.edge.keys().copied().collect();
        peak.sort();
        for d in peak {
            if mau.get(&d).copied().unwrap_or(0) == 0 {
                if let Some(chu_trinh) = self.dfs(d, &mut mau, &mut positive) {
                    return Some(chu_trinh);
                }
            }
        }
        None
    }

    fn dfs(&self, d: u32, mau: &mut HashMap<u32, u8>, positive: &mut Vec<u32>) -> Option<Vec<u32>> {
        mau.insert(d, 1); // xám = đang thăm
        positive.push(d);
        if let Some(ke) = self.edge.get(&d) {
            let mut ke = ke.clone();
            ke.sort();
            for k in ke {
                match mau.get(&k).copied().unwrap_or(0) {
                    1 => {
                        // gặp lại đỉnh XÁM -> có chu trình
                        let start = positive.iter().position(|&x| x == k).unwrap();
                        return Some(positive[start..].to_vec());
                    }
                    0 => {
                        if let Some(c) = self.dfs(k, mau, positive) { return Some(c); }
                    }
                    _ => {}
                }
            }
        }
        positive.pop();
        mau.insert(d, 2); // đen = xong
        None
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   HỆ ĐIỀU HÀNH: LẬP LỊCH CPU · PHÂN TRANG · PHÁT HIỆN BẾ TẮC   ");
    println!("═══════════════════════════════════════════════════════════════");

    let tao = || vec![
        Process::new(1, "trinh-duyet", 0, 8, 2),
        Process::new(2, "trinh-soan-thao", 1, 4, 1),
        Process::new(3, "nen-video", 2, 9, 3),
        Process::new(4, "dong-bo-may", 3, 5, 2),
    ];

    println!("\n1. LẬP LỊCH CPU — cùng 4 tiến trình, ba thuật toán");
    for (name, kq) in [
        ("FCFS       ", lap_lich_fcfs(tao())),
        ("SJF        ", lap_lich_sjf(tao())),
        ("Round-Robin", lap_lich_round_robin(tao(), 3)),
    ] {
        println!("   {} | chờ TB = {:>5.2} | quay vòng TB = {:>5.2}",
                 name, kq.wait_mean, kq.mean_turnaround);
    }
    println!("   → SJF tối ưu thời gian chờ, nhưng Round-Robin công bằng hơn (không ai bị đói).");

    println!("\n2. THAY TRANG BỘ NHỚ ẢO (3 khung nhớ)");
    let series = [7u64, 0, 1, 2, 0, 3, 0, 4, 2, 3, 0, 3, 2, 1, 2, 0, 1, 7, 0, 1];
    for (name, kq) in [
        ("FIFO   ", fifo_replace(&series, 3)),
        ("LRU    ", lru_replace(&series, 3)),
        ("Tối ưu ", optimal_replacement(&series, 3)),
    ] {
        println!("   {} | {} lỗi trang", name, kq.num_error_state);
    }
    println!("   → Tối ưu là CẬN DƯỚI lý thuyết (cần biết tương lai). LRU bám sát nó nhất.");

    println!("\n3. NGHỊCH LÝ BÉLÁDY — thêm khung nhớ mà LỖI TRANG TĂNG!");
    let belady = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
    println!("   FIFO 3 khung: {} lỗi", fifo_replace(&belady, 3).num_error_state);
    println!("   FIFO 4 khung: {} lỗi  ← NHIỀU HƠN dù có thêm bộ nhớ!", fifo_replace(&belady, 4).num_error_state);
    println!("   LRU  3 khung: {} lỗi", lru_replace(&belady, 3).num_error_state);
    println!("   LRU  4 khung: {} lỗi  ← LRU không bị nghịch lý này", lru_replace(&belady, 4).num_error_state);

    println!("\n4. PHÁT HIỆN BẾ TẮC");
    let mut g = WaitForGraph::new();
    g.them_cho(1, 2); // P1 chờ tài nguyên P2 giữ
    g.them_cho(2, 3);
    g.them_cho(3, 1); // ... và P3 chờ P1 -> VÒNG TRÒN
    println!("   Đồ thị P1→P2→P3→P1: {:?}", g.has_deadlock());
    let mut g2 = WaitForGraph::new();
    g2.them_cho(1, 2);
    g2.them_cho(2, 3);
    println!("   Đồ thị P1→P2→P3   : {:?} (không bế tắc)", g2.has_deadlock());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   HỆ ĐIỀU HÀNH = TRỌNG TÀI PHÂN PHỐI TÀI NGUYÊN CÓ HẠN         ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mau() -> Vec<Process> {
        vec![
            Process::new(1, "A", 0, 5, 1),
            Process::new(2, "B", 1, 3, 2),
            Process::new(3, "C", 2, 1, 3),
        ]
    }

    #[test]
    fn fcfs_runs_in_arrival_order() {
        let kq = lap_lich_fcfs(mau());
        // A(0-5), B(5-8), C(8-9)
        assert_eq!(kq.process[0].end, Some(5));
        assert_eq!(kq.process[1].end, Some(8));
        assert_eq!(kq.process[2].end, Some(9));
        assert_eq!(kq.timeline.len(), 9); // tổng burst = 5+3+1
    }

    #[test]
    fn sjf_beats_fcfs_on_average_wait() {
        let f = lap_lich_fcfs(mau());
        let s = lap_lich_sjf(mau());
        // SJF tối ưu thời gian chờ trung bình (định lý kinh điển)
        assert!(s.wait_mean <= f.wait_mean,
                "SJF ({}) phải <= FCFS ({})", s.wait_mean, f.wait_mean);
    }

    #[test]
    fn round_robin_starves_nobody() {
        let kq = lap_lich_round_robin(mau(), 2);
        // Mọi tiến trình đều hoàn thành
        assert!(kq.process.iter().all(|p| p.end.is_some()));
        assert!(kq.process.iter().all(|p| p.remaining == 0));
        // Tổng thời gian CPU đúng bằng tổng burst
        assert_eq!(kq.timeline.len(), 9);
    }

    #[test]
    fn every_scheduler_runs_total_burst() {
        for kq in [lap_lich_fcfs(mau()), lap_lich_sjf(mau()), lap_lich_round_robin(mau(), 3)] {
            assert_eq!(kq.timeline.len(), 9, "phải dùng đúng 9 đơn vị CPU");
        }
    }

    #[test]
    fn optimal_replacement_is_a_lower_bound() {
        let series = [7u64, 0, 1, 2, 0, 3, 0, 4, 2, 3, 0, 3, 2, 1, 2, 0, 1, 7, 0, 1];
        let opt = optimal_replacement(&series, 3).num_error_state;
        let lru = lru_replace(&series, 3).num_error_state;
        let fifo = fifo_replace(&series, 3).num_error_state;
        // OPT là cận dưới lý thuyết — không thuật toán nào tốt hơn
        assert!(opt <= lru, "OPT({}) phải <= LRU({})", opt, lru);
        assert!(opt <= fifo, "OPT({}) phải <= FIFO({})", opt, fifo);
    }

    #[test]
    fn belady_anomaly_is_real_for_fifo() {
        let series = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
        let ba = fifo_replace(&series, 3).num_error_state;
        let bon = fifo_replace(&series, 4).num_error_state;
        // NGHỊCH LÝ: thêm khung nhớ mà lỗi trang lại TĂNG
        assert!(bon > ba, "Bélády: FIFO 4 khung ({}) phải nhiều lỗi hơn 3 khung ({})", bon, ba);
    }

    #[test]
    fn lru_is_immune_to_belady() {
        let series = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
        let ba = lru_replace(&series, 3).num_error_state;
        let bon = lru_replace(&series, 4).num_error_state;
        // LRU là thuật toán "ngăn xếp" -> thêm khung KHÔNG BAO GIỜ làm tệ hơn
        assert!(bon <= ba, "LRU 4 khung ({}) không được tệ hơn 3 khung ({})", bon, ba);
    }

    #[test]
    fn enough_frames_means_only_compulsory_faults() {
        let series = [1u64, 2, 3, 1, 2, 3, 1, 2, 3];
        // 3 trang khác nhau, 5 khung -> chỉ 3 lỗi bắt buộc (compulsory miss)
        assert_eq!(lru_replace(&series, 5).num_error_state, 3);
        assert_eq!(fifo_replace(&series, 5).num_error_state, 3);
    }

    #[test]
    fn detects_deadlock_on_cycle() {
        let mut g = WaitForGraph::new();
        g.them_cho(1, 2);
        g.them_cho(2, 3);
        g.them_cho(3, 1);
        let ct = g.has_deadlock().expect("phải phát hiện bế tắc");
        assert_eq!(ct.len(), 3);
        assert!(ct.contains(&1) && ct.contains(&2) && ct.contains(&3));
    }

    #[test]
    fn no_deadlock_on_acyclic_graph() {
        let mut g = WaitForGraph::new();
        g.them_cho(1, 2);
        g.them_cho(2, 3);
        g.them_cho(1, 3); // vẫn không có chu trình
        assert_eq!(g.has_deadlock(), None);
    }

    #[test]
    fn classic_two_process_deadlock() {
        // P1 giữ A chờ B; P2 giữ B chờ A — bế tắc đơn giản nhất
        let mut g = WaitForGraph::new();
        g.them_cho(1, 2);
        g.them_cho(2, 1);
        assert!(g.has_deadlock().is_some());
    }
}
