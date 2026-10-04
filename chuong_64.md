# Chương 64: Hệ điều hành từ bên trong — Lập lịch CPU, Bộ nhớ ảo & Bế tắc (Operating Systems Internals)

## Giới thiệu & Mục tiêu học tập

Suốt 63 chương, chúng ta luôn ngầm giả định có một thế lực vô hình lo giúp mọi việc: cấp CPU cho chương trình, cho `Vec` mượn RAM, mở tệp, gửi gói tin. Thế lực đó là **hệ điều hành**. Chương 37 đã hé mở bản đồ bộ nhớ ảo; chương này lật hẳn nắp ca-pô.

Vì sao lập trình viên Rust cần hiểu hệ điều hành?

- Bạn gọi `thread::spawn` — nhưng **ai quyết định** luồng nào chạy trước? Câu trả lời giải thích vì sao đo hiệu năng đa luồng hay cho kết quả thất thường.
- Bạn cấp phát một `Vec` lớn — nhưng RAM **chưa hề được cấp** cho tới lần ghi đầu tiên. Hiểu phân trang giải thích vì sao `Vec::with_capacity(10_000_000)` nhanh còn vòng lặp ghi vào nó thì chậm.
- Chương trình của bạn treo cứng, không tốn CPU, không báo lỗi. Đó là **bế tắc** — và có thuật toán phát hiện nó.

Mục tiêu học tập:
- Hiểu **tiến trình** và khối điều khiển tiến trình (PCB) — thứ nhân hệ điều hành lưu cho mỗi chương trình.
- Cài và so sánh ba thuật toán **lập lịch CPU**: FCFS, SJF, Round-Robin; đo thời gian chờ và thời gian quay vòng.
- Hiểu **bộ nhớ ảo**: lỗi trang, các thuật toán thay trang FIFO/LRU/Tối ưu.
- Tự tay chứng kiến **nghịch lý Bélády**: thêm bộ nhớ mà chương trình chạy *chậm đi*.
- Phát hiện **bế tắc** bằng cách tìm chu trình trong đồ thị chờ đợi.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌────────────────────────────────────────────────────────────────────────────────┐
│        HÌNH TƯỢNG: HỆ ĐIỀU HÀNH = BAN QUẢN LÝ MỘT PHÒNG KHÁM ĐÔNG BỆNH NHÂN    │
├────────────────────────────────────────────────────────────────────────────────┤
│                                                                                │
│  MỘT BÁC SĨ (CPU)  ·  RẤT NHIỀU BỆNH NHÂN (tiến trình)                         │
│                                                                                │
│  ┌─ FCFS: "AI ĐẾN TRƯỚC KHÁM TRƯỚC" ───────────────────────────────────────┐   │
│  │  Công bằng về thứ tự. Nhưng nếu người đầu tiên khám tổng quát 2 tiếng,  │   │
│  │  thì người chỉ cần xin chữ ký 30 giây cũng phải đợi đủ 2 tiếng.         │   │
│  │  → "HIỆU ỨNG ĐOÀN XE": một xe tải chậm chặn cả đoàn xe con phía sau.    │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
│  ┌─ SJF: "AI NHANH NHẤT VÀO TRƯỚC" ────────────────────────────────────────┐   │
│  │  Tổng thời gian chờ NHỎ NHẤT có thể — đây là định lý, không phải mẹo.   │   │
│  │  Nhưng người cần khám 2 tiếng có thể ngồi cả ngày nếu ca ngắn cứ tới.   │   │
│  │  → "ĐÓI" (starvation): công bằng tổng thể, bất công với cá nhân.        │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
│  ┌─ ROUND-ROBIN: "MỖI NGƯỜI 10 PHÚT, HẾT GIỜ RA XẾP HÀNG LẠI" ─────────────┐   │
│  │  Không ai bị bỏ quên. Ai cũng thấy "mình đang được phục vụ".            │   │
│  │  Giá phải trả: mất thời gian mỗi lần đổi người (CHUYỂN NGỮ CẢNH).       │   │
│  │  → Chia lát thời gian là nền của mọi bộ lập lịch HĐH tương tác.         │   │
│  └─────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
├────────────────────────────────────────────────────────────────────────────────┤
│        BỘ NHỚ ẢO = THƯ VIỆN CÓ KHO SÁCH LỚN NHƯNG BÀN ĐỌC NHỎ                  │
│                                                                                │
│   Kho (ổ cứng): 1 triệu cuốn        Bàn đọc (RAM): chỉ đặt được 3 cuốn         │
│                                                                                │
│   Cần cuốn chưa có trên bàn → LỖI TRANG → chạy vào kho lấy (CHẬM 100 000 lần)  │
│   Bàn đầy → phải cất bớt một cuốn. CẤT CUỐN NÀO?                               │
│                                                                                │
│     FIFO  : cất cuốn lấy ra sớm nhất       (đơn giản, đôi khi ngu ngốc)        │
│     LRU   : cất cuốn lâu nhất không đụng   (khớp thói quen người đọc)          │
│     TỐI ƯU: cất cuốn LÂU NHẤT MỚI CẦN LẠI  (cần biết trước tương lai!)         │
│                                                                                │
├────────────────────────────────────────────────────────────────────────────────┤
│        BẾ TẮC = HAI NGƯỜI LỊCH SỰ Ở CỬA HẸP                                    │
│                                                                                │
│   An giữ CÁI THANG, cần CÁI BÚA.   Bình giữ CÁI BÚA, cần CÁI THANG.            │
│   Cả hai cùng đợi. Không ai nhường. Không ai chết. Không ai xong việc.         │
│                                                                                │
│        An ──cần──► Bình                                                        │
│         ▲            │        ← CÓ VÒNG TRÒN trong đồ thị "ai chờ ai"          │
│         └────cần─────┘          = CÓ BẾ TẮC. Đó là toàn bộ thuật toán.         │
└────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Tiến trình và cái giá của việc chuyển ngữ cảnh

Với mỗi chương trình đang chạy, nhân hệ điều hành giữ một **khối điều khiển tiến trình** (PCB): mã số, trạng thái, con trỏ lệnh, toàn bộ thanh ghi, bảng trang, danh sách tệp đang mở. Chuyển từ tiến trình này sang tiến trình khác nghĩa là *cất* toàn bộ đống đó và *nạp* đống khác vào.

Chi phí trực tiếp chỉ khoảng 1–5 micro-giây. Nhưng chi phí **gián tiếp** lớn hơn nhiều: bộ nhớ đệm CPU (cache) vừa được sưởi ấm bằng dữ liệu của tiến trình cũ nay thành vô dụng. Đây là lý do lượng tử thời gian của Round-Robin không được quá nhỏ — chia quá vụn thì CPU dành phần lớn thời gian để... chuyển ngữ cảnh.

Một tiến trình đi qua năm trạng thái:

```
   tạo ra               được lập lịch →
  ────────► Mới ──────► Sẵn sàng ⇄ Đang chạy ──────► Kết thúc
                                 ← hết lượng tử   chạy xong
                            ▲            │
                            │            │ gọi I/O (đọc đĩa, chờ mạng)
                            └── Chờ ◄────┘
                              I/O xong
```

Điểm mấu chốt: khi tiến trình gọi I/O, nó **tự nguyện** nhả CPU. Đây là lý do một máy chủ web xử lý được hàng nghìn kết nối trên vài lõi — phần lớn thời gian chúng đang *chờ*, không phải *tính*.

### 2. Ba thuật toán lập lịch và ba đánh đổi

| Thuật toán | Có tiếm quyền? | Điểm mạnh | Điểm yếu chí mạng |
|---|---|---|---|
| **FCFS** | Không | Đơn giản tuyệt đối, công bằng theo thứ tự | Hiệu ứng đoàn xe |
| **SJF** | Không | **Tối ưu** thời gian chờ trung bình | Gây đói; cần biết trước độ dài |
| **Round-Robin** | Có | Không ai bị đói; phản hồi nhanh | Chi phí chuyển ngữ cảnh |

SJF tối ưu về thời gian chờ trung bình là một **định lý** chứng minh được (khi mọi việc đã sẵn sàng cùng lúc): nếu có hai việc kề nhau mà việc dài đứng trước, đổi chỗ chúng luôn làm giảm tổng thời gian chờ. Cứ đổi cho tới khi sắp xếp tăng dần — không cách xếp nào tốt hơn.

Nhưng SJF cần biết **trước** mỗi việc chạy bao lâu, điều bất khả trong thực tế. Nhiều hệ điều hành vì vậy dùng **hàng đợi phản hồi nhiều mức** (ý tưởng nền của bộ lập lịch Windows và các Unix cổ điển): tiến trình mới vào hàng ưu tiên cao với lượng tử ngắn; nếu dùng hết lượng tử (tức là việc dài), nó bị đẩy xuống hàng ưu tiên thấp hơn với lượng tử dài hơn. Hệ thống *học* độ dài việc bằng cách quan sát, chứ không cần biết trước. Linux đi đường khác: CFS (và từ 6.6 là EEVDF) chia CPU theo "thời gian chạy ảo" — vẫn là chia lát thời gian, chỉ là lát được tính theo trọng số.

### 3. Bộ nhớ ảo: lời nói dối tử tế nhất của máy tính

Mỗi tiến trình tin rằng nó sở hữu trọn không gian địa chỉ liên tục. Sự thật: địa chỉ ảo được đơn vị quản lý bộ nhớ (MMU) dịch sang địa chỉ vật lý theo từng **trang** (thường 4 KB).

```
   Địa chỉ ảo 0x7FFF_1234
   ┌──────────────┬────────┐
   │ số hiệu trang│ độ lệch│
   └──────┬───────┴───┬────┘
          │           │
          ▼           │      Bảng trang
     ┌─────────┐      │      (mỗi tiến trình một bảng)
     │ trang 12│──────┼────► khung nhớ vật lý 4507
     └─────────┘      │
                      ▼
       Địa chỉ vật lý = 4507 × 4096 + độ lệch
```

Nếu trang cần dùng không nằm trong RAM → **lỗi trang** (page fault). CPU dừng lại, nhân hệ điều hành nạp trang từ đĩa, rồi cho lệnh chạy tiếp. Một lỗi trang phải đọc đĩa tốn khoảng 10 mili-giây trên đĩa quay (khoảng 100 micro-giây trên SSD NVMe) — trong khi truy cập RAM tốn 100 nano-giây. **Chậm hơn 100 000 lần** với đĩa quay, vẫn ~1 000 lần với SSD. Đó là lý do thuật toán thay trang quan trọng đến vậy.

### 4. Nghịch lý Bélády — bài học về trực giác sai

Trực giác nói: nhiều RAM hơn thì ít lỗi trang hơn. Với FIFO, **điều đó sai**. Với chuỗi truy cập `1,2,3,4,1,2,5,1,2,3,4,5`, FIFO gây 9 lỗi trang với 3 khung nhớ nhưng **10 lỗi** với 4 khung nhớ.

Vì sao? FIFO đuổi trang theo *tuổi*, một tiêu chí chẳng liên quan gì tới việc trang đó có sắp được dùng lại hay không. Thêm khung nhớ làm thay đổi *thứ tự* đuổi theo cách có thể tệ hơn.

LRU **miễn nhiễm** với nghịch lý này vì nó thuộc lớp "thuật toán ngăn xếp": tập trang trong bộ nhớ với `n` khung luôn là **tập con** của tập trang với `n+1` khung. Đã là tập con thì thêm khung không bao giờ gây thêm lỗi.

### 5. Bốn điều kiện Coffman của bế tắc

Bế tắc **chỉ có thể** xảy ra khi cả bốn điều kiện đồng thời đúng (bốn điều kiện *cần*; khi mỗi loại tài nguyên chỉ có một thể hiện — như mutex — chờ vòng tròn cũng là *đủ*):

1. **Loại trừ lẫn nhau** — tài nguyên không chia sẻ được.
2. **Giữ và chờ** — đang giữ cái này mà đòi cái khác.
3. **Không tiếm quyền** — không ai giật được tài nguyên khỏi tay người giữ.
4. **Chờ vòng tròn** — tồn tại một vòng tròn các tiến trình chờ nhau.

Phá **bất kỳ** điều kiện nào là hết bế tắc. Trong Rust, cách phá phổ biến nhất là phá điều kiện 4: **luôn khóa các mutex theo cùng một thứ tự toàn cục**. Nếu mọi luồng đều khóa `a` trước `b`, vòng tròn không thể hình thành. Đây là quy tắc đơn giản mà cứu được vô số giờ gỡ lỗi.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chạy bằng `cargo run -p ch64`, kiểm thử bằng `cargo test -p ch64`.

```rust
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
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0369`: binary operation `==` cannot be applied to type `Vec<Process>` | `#[derive(PartialEq)]` trên `ScheduleResult` nhưng `Process` bên trong lại không có | Thêm `PartialEq` vào derive của **mọi** kiểu lồng bên trong |
| `E0502`: cannot borrow `procs` as mutable because it is also borrowed as immutable | Vòng `for p in procs.iter()` rồi lại `procs.push(...)` bên trong | Thu thập vào `Vec` mới, hoặc dùng chỉ số `for i in 0..tt.len()` |
| `E0382: use of moved value` | Truyền `Vec<Process>` vào `schedule_fcfs` rồi truyền tiếp vào `schedule_sjf` | Mỗi thuật toán một bản sao: dùng closure `let make_processes = \|\| vec![...]` |
| *Không phải lỗi*: tưởng sẽ panic `index out of bounds` | `refs[i + 1..]` khi `i` là phần tử cuối | Rust cho phép `refs[len..]` (lát cắt rỗng); chỉ `refs[len + 1..]` mới panic — đây là lý do `optimal_replacement` không panic |
| Đệ quy tràn ngăn xếp trong `dfs` (lúc chạy) | Đồ thị chờ có chu trình mà quên đánh dấu màu xám | Đúng ba màu: trắng → xám (đang thăm) → đen (xong) |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 5 điểm cốt lõi cần ghi nhớ

1. **Hệ điều hành là trọng tài phân phối tài nguyên có hạn.** Mọi thuật toán trong chương đều trả lời một câu hỏi: *ai được dùng trước?*
2. **Không có thuật toán lập lịch tốt nhất.** SJF tối ưu thời gian chờ nhưng gây đói; Round-Robin công bằng nhưng tốn chuyển ngữ cảnh. Chọn theo mục tiêu, không theo "cái nào hay hơn".
3. **Nghịch lý Bélády (Bélády's anomaly) là bằng chứng trực giác có thể sai.** Thêm tài nguyên không đảm bảo tốt hơn — phải đo, đừng đoán.
4. **Một lỗi trang đắt gấp 100 000 lần một lần truy cập RAM.** Đó là lý do "nguyên lý cục bộ" thống trị mọi thiết kế bộ nhớ đệm, từ CPU cache tới CDN.
5. **Bế tắc = chu trình trong đồ thị chờ.** Cách phòng đơn giản nhất trong Rust: luôn khóa mutex theo một thứ tự toàn cục cố định.

### Bài tập rèn luyện tự giải

**Bài 1.** Cài thuật toán **SJF có tiếm quyền** (còn gọi là "thời gian còn lại ngắn nhất trước" — SRTF): mỗi khi có tiến trình mới tới, nếu nó ngắn hơn *thời gian còn lại* của tiến trình đang chạy thì giành quyền ngay. So sánh thời gian chờ trung bình với SJF thường.

<details>
<summary><b>Gợi ý</b></summary>

Chạy mô phỏng theo từng đơn vị thời gian. Ở **mỗi** đơn vị, chọn lại tiến trình có `remaining` nhỏ nhất trong số đã tới. Ghi `start` lần đầu được chọn, `end` khi `remaining` về 0. Vì chọn lại mỗi bước, tiến trình đang chạy tự động bị tiếm quyền khi có ứng viên tốt hơn.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn schedule_srtf(mut procs: Vec<Process>) -> ScheduleResult {
    let n = procs.len();
    let mut clock = 0u64;
    let mut timeline = Vec::new();
    let mut done = 0;

    while done < n {
        // Chọn LẠI ở MỖI đơn vị thời gian — đó chính là "tiếm quyền"
        let pick = (0..n)
            .filter(|&i| procs[i].remaining > 0 && procs[i].arrives_at <= clock)
            .min_by_key(|&i| (procs[i].remaining, procs[i].pid));
        match pick {
            Some(i) => {
                if procs[i].start.is_none() {
                    procs[i].start = Some(clock);
                }
                timeline.push((clock, procs[i].pid));
                procs[i].remaining -= 1;
                clock += 1;
                if procs[i].remaining == 0 {
                    procs[i].end = Some(clock);
                    procs[i].state = ProcessState::Finished;
                    done += 1;
                }
            }
            None => clock += 1,
        }
    }
    summarize(procs, timeline) // dùng lại hàm tổng kết của chương
}

#[test]
fn srtf_never_waits_longer_than_sjf() {
    let procs = || {
        vec![
            Process::new(1, "A", 0, 8, 1),
            Process::new(2, "B", 1, 4, 1),
            Process::new(3, "C", 2, 2, 1),
        ]
    };
    let srtf = schedule_srtf(procs());
    let sjf = schedule_sjf(procs());
    assert!(srtf.avg_wait <= sjf.avg_wait);
    assert_eq!(srtf.timeline.len(), 14);
}
```

SRTF cho thời gian chờ trung bình **thấp hơn hoặc bằng** SJF thường, vì nó tận dụng được thông tin mới (tiến trình vừa tới) thay vì cam kết mù quáng tới hết việc. Cái giá: nhiều lần chuyển ngữ cảnh hơn, và nguy cơ đói còn nặng hơn SJF.
</details>

**Bài 2.** Cài thuật toán thay trang **Clock** (còn gọi là "cơ hội thứ hai"): mỗi trang có một bit tham chiếu; kim đồng hồ quét vòng, gặp bit 1 thì xóa về 0 và đi tiếp, gặp bit 0 thì đuổi trang đó. Kiểm chứng trên chuỗi truy cập của chương rằng số lỗi trang của nó nằm **giữa** LRU và FIFO (điều này thường đúng nhưng không phải định lý — Clock vẫn có thể bị nghịch lý Bélády).

<details>
<summary><b>Gợi ý</b></summary>

Dùng `Vec<(u64, bool)>` cho các khung và một biến `hand: usize` (kim đồng hồ). Khi cần đuổi: lặp `while frames[hand].1 { frames[hand].1 = false; hand = (hand + 1) % n; }` rồi đuổi `frames[hand]`. Khi truy cập trúng một trang đã có, chỉ cần đặt bit tham chiếu của nó về `true`.

Clock là **xấp xỉ LRU giá rẻ**: nó chỉ cần 1 bit mỗi trang thay vì một dấu thời gian đầy đủ. Linux dùng một họ hàng của ý tưởng này: xấp xỉ LRU bằng bit tham chiếu với hai danh sách active/inactive (và từ 6.1 có thêm MGLRU nhiều thế hệ).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn clock_replacement(refs: &[u64], num_frames: usize) -> ReplacementResult {
    let mut frames: Vec<(u64, bool)> = Vec::new(); // (số trang, bit tham chiếu)
    let mut hand = 0usize; // kim đồng hồ
    let mut faults = 0;
    let mut history = Vec::new();

    for &t in refs {
        match frames.iter().position(|&(p, _)| p == t) {
            Some(i) => frames[i].1 = true, // trúng: cho "cơ hội thứ hai"
            None => {
                faults += 1;
                if frames.len() < num_frames {
                    frames.push((t, true));
                } else {
                    // quét kim tới khi gặp bit tham chiếu = 0
                    while frames[hand].1 {
                        frames[hand].1 = false; // xóa bit, cho qua lần này
                        hand = (hand + 1) % num_frames;
                    }
                    frames[hand] = (t, true);
                    hand = (hand + 1) % num_frames;
                }
            }
        }
        history.push(frames.iter().map(|&(p, _)| p).collect());
    }
    ReplacementResult {
        page_faults: faults,
        frame_history: history,
    }
}

#[test]
fn clock_sits_between_lru_and_fifo_here() {
    let refs = [7u64, 0, 1, 2, 0, 3, 0, 4, 2, 3, 0, 3, 2, 1, 2, 0, 1, 7, 0, 1];
    let clock = clock_replacement(&refs, 3).page_faults;
    assert!(lru_replace(&refs, 3).page_faults <= clock);
    assert!(clock <= fifo_replace(&refs, 3).page_faults);
}
```

Clock đạt gần chất lượng LRU với chi phí gần bằng FIFO — một ví dụ đẹp của "đủ tốt thắng hoàn hảo" trong kỹ thuật hệ thống. Lưu ý: nếu **mọi** bit đều bằng 1, vòng `while` sẽ xóa hết một lượt rồi mới dừng ở kim ban đầu — nên nó luôn kết thúc, không lặp vô hạn.
</details>

**Bài 3.** Cài **thuật toán chủ nhà băng** (Banker's algorithm) để *tránh* bế tắc thay vì chỉ *phát hiện* nó: cho ma trận nhu cầu tối đa và phân bổ hiện tại, xác định trạng thái có "an toàn" không.

<details>
<summary><b>Gợi ý</b></summary>

Trạng thái an toàn = tồn tại một **thứ tự hoàn thành** cho mọi tiến trình. Thuật toán: lặp tìm một tiến trình chưa xong mà `nhu cầu còn lại <= tài nguyên khả dụng`; giả vờ cho nó chạy xong và **trả lại** toàn bộ tài nguyên nó giữ; lặp lại. Nếu xử lý được hết → an toàn. Nếu kẹt mà còn tiến trình chưa xong → không an toàn.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
/// `max[i][j]`        = tiến trình i có thể cần tối đa bao nhiêu tài nguyên loại j
/// `allocated[i][j]`  = đang giữ bao nhiêu
/// `available[j]`     = còn rảnh bao nhiêu
/// Trả về một thứ tự hoàn thành an toàn, hoặc `None` nếu trạng thái không an toàn.
pub fn safe_state(max: &[Vec<i32>], allocated: &[Vec<i32>], available: &[i32]) -> Option<Vec<usize>> {
    let n = max.len();
    let m = available.len();
    let mut free: Vec<i32> = available.to_vec();
    let mut done = vec![false; n];
    let mut order = Vec::new();

    for _ in 0..n {
        // tìm một tiến trình có thể hoàn thành với tài nguyên đang rảnh
        let candidate =
            (0..n).find(|&i| !done[i] && (0..m).all(|j| max[i][j] - allocated[i][j] <= free[j]));
        match candidate {
            Some(i) => {
                // cho nó chạy xong rồi TRẢ LẠI mọi thứ nó giữ
                for (f, a) in free.iter_mut().zip(&allocated[i]) {
                    *f += a;
                }
                done[i] = true;
                order.push(i);
            }
            None => return None, // kẹt → trạng thái KHÔNG an toàn
        }
    }
    Some(order)
}

#[test]
fn bankers_classic_example_is_safe() {
    // Ví dụ kinh điển (Silberschatz): 5 tiến trình, 3 loại tài nguyên
    let max = vec![vec![7, 5, 3], vec![3, 2, 2], vec![9, 0, 2], vec![2, 2, 2], vec![4, 3, 3]];
    let alloc = vec![vec![0, 1, 0], vec![2, 0, 0], vec![3, 0, 2], vec![2, 1, 1], vec![0, 0, 2]];
    assert_eq!(safe_state(&max, &alloc, &[3, 3, 2]), Some(vec![1, 3, 0, 2, 4]));
    assert_eq!(safe_state(&max, &alloc, &[0, 0, 0]), None);
}
```

Điểm tinh tế: thuật toán này **bi quan** — nó giả định mọi tiến trình đều có thể đòi tới mức tối đa. Vì thế nó từ chối cả một số trạng thái thực ra vẫn ổn. Đó là lý do các hệ điều hành hiện đại **không** dùng nó: phải khai báo trước nhu cầu tối đa là điều bất khả thi. Thực tế người ta chọn *phòng ngừa* (thứ tự khóa cố định) hoặc *phát hiện rồi khôi phục* (như phần đồ thị chờ trong chương).
</details>
