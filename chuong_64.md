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
│        HÌNH TƯỢNG: HỆ ĐIỀU HÀNH = BAN QUẢN LÝ MỘT PHÒNG KHÁM ĐÔNG BỆNH NHÂN     │
├────────────────────────────────────────────────────────────────────────────────┤
│                                                                                │
│  MỘT BÁC SĨ (CPU)  ·  RẤT NHIỀU BỆNH NHÂN (tiến trình)                        │
│                                                                                │
│  ┌─ FCFS: "AI ĐẾN TRƯỚC KHÁM TRƯỚC" ──────────────────────────────────────┐   │
│  │  Công bằng về thứ tự. Nhưng nếu người đầu tiên khám tổng quát 2 tiếng,  │   │
│  │  thì người chỉ cần xin chữ ký 30 giây cũng phải đợi đủ 2 tiếng.         │   │
│  │  → "HIỆU ỨNG ĐOÀN XE": một xe tải chậm chặn cả đoàn xe con phía sau.    │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
│  ┌─ SJF: "AI NHANH NHẤT VÀO TRƯỚC" ───────────────────────────────────────┐   │
│  │  Tổng thời gian chờ NHỎ NHẤT có thể — đây là định lý, không phải mẹo.   │   │
│  │  Nhưng người cần khám 2 tiếng có thể ngồi cả ngày nếu ca ngắn cứ tới.   │   │
│  │  → "ĐÓI" (starvation): công bằng tổng thể, bất công với cá nhân.        │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
│  ┌─ ROUND-ROBIN: "MỖI NGƯỜI 10 PHÚT, HẾT GIỜ RA XẾP HÀNG LẠI" ────────────┐   │
│  │  Không ai bị bỏ quên. Ai cũng thấy "mình đang được phục vụ".            │   │
│  │  Giá phải trả: mất thời gian mỗi lần đổi người (CHUYỂN NGỮ CẢNH).       │   │
│  │  → Đây là thuật toán của mọi hệ điều hành có giao diện người dùng.      │   │
│  └────────────────────────────────────────────────────────────────────────┘   │
│                                                                                │
├────────────────────────────────────────────────────────────────────────────────┤
│        BỘ NHỚ ẢO = THƯ VIỆN CÓ KHO SÁCH LỚN NHƯNG BÀN ĐỌC NHỎ                  │
│                                                                                │
│   Kho (ổ cứng): 1 triệu cuốn        Bàn đọc (RAM): chỉ đặt được 3 cuốn         │
│                                                                                │
│   Cần cuốn chưa có trên bàn → LỖI TRANG → chạy vào kho lấy (CHẬM 100 000 lần)  │
│   Bàn đầy → phải cất bớt một cuốn. CẤT CUỐN NÀO?                              │
│                                                                                │
│     FIFO  : cất cuốn lấy ra sớm nhất       (đơn giản, đôi khi ngu ngốc)        │
│     LRU   : cất cuốn lâu nhất không đụng   (khớp thói quen người đọc)          │
│     TỐI ƯU: cất cuốn LÂU NHẤT MỚI CẦN LẠI  (cần biết trước tương lai!)         │
│                                                                                │
├────────────────────────────────────────────────────────────────────────────────┤
│        BẾ TẮC = HAI NGƯỜI LỊCH SỰ Ở CỬA HẸP                                    │
│                                                                                │
│   An giữ CÁI THANG, cần CÁI BÚA.   Bình giữ CÁI BÚA, cần CÁI THANG.           │
│   Cả hai cùng đợi. Không ai nhường. Không ai chết. Không ai xong việc.        │
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
   tạo ra          được lập lịch         hết lượng tử
  ────────► Mới ──────► Sẵn sàng ⇄ Đang chạy ──────► Kết thúc
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

SJF tối ưu về thời gian chờ trung bình là một **định lý** chứng minh được: nếu có hai việc kề nhau mà việc dài đứng trước, đổi chỗ chúng luôn làm giảm tổng thời gian chờ. Cứ đổi cho tới khi sắp xếp tăng dần — không cách xếp nào tốt hơn.

Nhưng SJF cần biết **trước** mỗi việc chạy bao lâu, điều bất khả trong thực tế. Hệ điều hành thật vì vậy dùng **hàng đợi phản hồi nhiều mức**: tiến trình mới vào hàng ưu tiên cao với lượng tử ngắn; nếu dùng hết lượng tử (tức là việc dài), nó bị đẩy xuống hàng ưu tiên thấp hơn với lượng tử dài hơn. Hệ thống *học* độ dài việc bằng cách quan sát, chứ không cần biết trước.

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

Nếu trang cần dùng không nằm trong RAM → **lỗi trang** (page fault). CPU dừng lại, nhân hệ điều hành nạp trang từ đĩa, rồi cho lệnh chạy tiếp. Một lỗi trang tốn khoảng 10 mili-giây trên đĩa quay — trong khi truy cập RAM tốn 100 nano-giây. **Chậm hơn 100 000 lần.** Đó là lý do thuật toán thay trang quan trọng đến vậy.

### 4. Nghịch lý Bélády — bài học về trực giác sai

Trực giác nói: nhiều RAM hơn thì ít lỗi trang hơn. Với FIFO, **điều đó sai**. Với chuỗi truy cập `1,2,3,4,1,2,5,1,2,3,4,5`, FIFO gây 9 lỗi trang với 3 khung nhớ nhưng **10 lỗi** với 4 khung nhớ.

Vì sao? FIFO đuổi trang theo *tuổi*, một tiêu chí chẳng liên quan gì tới việc trang đó có sắp được dùng lại hay không. Thêm khung nhớ làm thay đổi *thứ tự* đuổi theo cách có thể tệ hơn.

LRU **miễn nhiễm** với nghịch lý này vì nó thuộc lớp "thuật toán ngăn xếp": tập trang trong bộ nhớ với `n` khung luôn là **tập con** của tập trang với `n+1` khung. Đã là tập con thì thêm khung không bao giờ gây thêm lỗi.

### 5. Bốn điều kiện Coffman của bế tắc

Bế tắc xảy ra **khi và chỉ khi** cả bốn điều kiện đồng thời đúng:

1. **Loại trừ lẫn nhau** — tài nguyên không chia sẻ được.
2. **Giữ và chờ** — đang giữ cái này mà đòi cái khác.
3. **Không tiếm quyền** — không ai giật được tài nguyên khỏi tay người giữ.
4. **Chờ vòng tròn** — tồn tại một vòng tròn các tiến trình chờ nhau.

Phá **bất kỳ** điều kiện nào là hết bế tắc. Trong Rust, cách phá phổ biến nhất là phá điều kiện 4: **luôn khóa các mutex theo cùng một thứ tự toàn cục**. Nếu mọi luồng đều khóa `a` trước `b`, vòng tròn không thể hình thành. Đây là quy tắc đơn giản mà cứu được vô số giờ gỡ lỗi.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chạy bằng `cargo run -p ch64`, kiểm thử bằng `cargo test -p ch64`.

```rust
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
    KetThuc,
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
        p.state = StateProcess::KetThuc;
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
                tt[i].state = StateProcess::KetThuc;
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
                    tt[i].state = StateProcess::KetThuc;
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

    /// Phát hiện bế tắc = tìm owner trình bằng DFS 3 màu.
    pub fn has_deadlock(&self) -> Option<Vec<u32>> {
        let mut mau: HashMap<u32, u8> = HashMap::new(); // 0=trắng 1=xám 2=đen
        let mut duong: Vec<u32> = Vec::new();
        let mut peak: Vec<u32> = self.edge.keys().copied().collect();
        peak.sort();
        for d in peak {
            if mau.get(&d).copied().unwrap_or(0) == 0 {
                if let Some(chu_trinh) = self.dfs(d, &mut mau, &mut duong) {
                    return Some(chu_trinh);
                }
            }
        }
        None
    }

    fn dfs(&self, d: u32, mau: &mut HashMap<u32, u8>, duong: &mut Vec<u32>) -> Option<Vec<u32>> {
        mau.insert(d, 1); // xám = đang thăm
        duong.push(d);
        if let Some(ke) = self.edge.get(&d) {
            let mut ke = ke.clone();
            ke.sort();
            for k in ke {
                match mau.get(&k).copied().unwrap_or(0) {
                    1 => {
                        // gặp lại đỉnh XÁM -> có owner trình
                        let start = duong.iter().position(|&x| x == k).unwrap();
                        return Some(duong[start..].to_vec());
                    }
                    0 => {
                        if let Some(c) = self.dfs(k, mau, duong) { return Some(c); }
                    }
                    _ => {}
                }
            }
        }
        duong.pop();
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
        g.them_cho(1, 3); // vẫn không có owner trình
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
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0369: binary operation == cannot be applied` | `#[derive(PartialEq)]` trên `KetQuaLapLich` nhưng `Process` bên trong lại không có | Thêm `PartialEq` vào derive của **mọi** kiểu lồng bên trong |
| `E0502: cannot borrow as mutable ... also borrowed as immutable` | Vòng `for p in tt.iter()` rồi lại `tt.push(...)` bên trong | Thu thập vào `Vec` mới, hoặc dùng chỉ số `for i in 0..tt.len()` |
| `E0382: use of moved value` | Truyền `Vec<Process>` vào `lap_lich_fcfs` rồi dùng lại | Mỗi thuật toán một bản sao: dùng closure `let tao = \|\| vec![...]` |
| `index out of bounds` (lúc chạy) | `chuoi[i + 1..]` khi `i` là phần tử cuối | Rust cho phép `chuoi[len..]` (lát cắt rỗng) — đây là lý do `optimal_replacement` không panic |
| Đệ quy tràn ngăn xếp trong `dfs` | Đồ thị chờ có chu trình mà quên đánh dấu màu xám | Đúng ba màu: trắng → xám (đang thăm) → đen (xong) |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 5 điểm cốt lõi cần ghi nhớ

1. **Hệ điều hành là trọng tài phân phối tài nguyên có hạn.** Mọi thuật toán trong chương đều trả lời một câu hỏi: *ai được dùng trước?*
2. **Không có thuật toán lập lịch tốt nhất.** SJF tối ưu thời gian chờ nhưng gây đói; Round-Robin công bằng nhưng tốn chuyển ngữ cảnh. Chọn theo mục tiêu, không theo "cái nào hay hơn".
3. **Nghịch lý Bélády là bằng chứng trực giác có thể sai.** Thêm tài nguyên không đảm bảo tốt hơn — phải đo, đừng đoán.
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
pub fn lap_lich_srtf(mut tt: Vec<Process>) -> KetQuaLapLich {
    let n = tt.len();
    let mut clock = 0u64;
    let mut dtg = Vec::new();
    let mut done = 0;

    while done < n {
        // Chọn LẠI ở MỖI đơn vị thời gian — đó chính là "tiếm quyền"
        let pick = (0..n)
            .filter(|&i| tt[i].remaining > 0 && tt[i].arrives_at <= clock)
            .min_by_key(|&i| (tt[i].remaining, tt[i].pid));
        match pick {
            Some(i) => {
                if tt[i].start.is_none() { tt[i].start = Some(clock); }
                dtg.push((clock, tt[i].pid));
                tt[i].remaining -= 1;
                clock += 1;
                if tt[i].remaining == 0 {
                    tt[i].end = Some(clock);
                    tt[i].state = StateProcess::KetThuc;
                    done += 1;
                }
            }
            None => clock += 1,
        }
    }
    // (dùng lại hàm tong_ket của chương)
    let tong_cho: u64 = tt.iter().filter_map(|p| p.time_time_wait()).sum();
    let tong_qv: u64 = tt.iter().filter_map(|p| p.turnaround_time()).sum();
    KetQuaLapLich {
        timeline: dtg,
        wait_mean: tong_cho as f64 / n as f64,
        mean_turnaround: tong_qv as f64 / n as f64,
        process: tt,
    }
}
```

SRTF cho thời gian chờ trung bình **thấp hơn hoặc bằng** SJF thường, vì nó tận dụng được thông tin mới (tiến trình vừa tới) thay vì cam kết mù quáng tới hết việc. Cái giá: nhiều lần chuyển ngữ cảnh hơn, và nguy cơ đói còn nặng hơn SJF.
</details>

**Bài 2.** Cài thuật toán thay trang **Clock** (còn gọi là "cơ hội thứ hai"): mỗi trang có một bit tham chiếu; kim đồng hồ quét vòng, gặp bit 1 thì xóa về 0 và đi tiếp, gặp bit 0 thì đuổi trang đó. Kiểm chứng nó cho số lỗi trang nằm **giữa** FIFO và LRU.

<details>
<summary><b>Gợi ý</b></summary>

Dùng `Vec<(u64, bool)>` cho các khung và một biến `kim: usize`. Khi cần đuổi: lặp `while khung[kim].1 { khung[kim].1 = false; kim = (kim+1) % n; }` rồi đuổi `khung[kim]`. Khi truy cập trúng một trang đã có, chỉ cần đặt bit tham chiếu của nó về `true`.

Clock là **xấp xỉ LRU giá rẻ**: nó chỉ cần 1 bit mỗi trang thay vì một dấu thời gian đầy đủ. Đây chính là thuật toán mà Linux dùng (dưới dạng biến thể hai danh sách active/inactive).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn thay_trang_clock(series: &[u64], num_frame: usize) -> StateChange {
    let mut frame: Vec<(u64, bool)> = Vec::new(); // (số trang, bit tham chiếu)
    let mut kim = 0usize;
    let mut error = 0;
    let mut history = Vec::new();

    for &t in series {
        match frame.iter().position(|&(p, _)| p == t) {
            Some(i) => frame[i].1 = true,            // trúng: cho "cơ hội thứ hai"
            None => {
                error += 1;
                if frame.len() < num_frame {
                    frame.push((t, true));
                } else {
                    // quét kim tới khi gặp bit tham chiếu = 0
                    while frame[kim].1 {
                        frame[kim].1 = false;        // xóa bit, cho qua lần này
                        kim = (kim + 1) % num_frame;
                    }
                    frame[kim] = (t, true);
                    kim = (kim + 1) % num_frame;
                }
            }
        }
        history.push(frame.iter().map(|&(p, _)| p).collect());
    }
    StateChange { num_error_state: error, series_frame: history }
}
```

Clock đạt gần chất lượng LRU với chi phí gần bằng FIFO — một ví dụ đẹp của "đủ tốt thắng hoàn hảo" trong kỹ thuật hệ thống. Lưu ý: nếu **mọi** bit đều bằng 1, vòng `while` sẽ xóa hết một lượt rồi mới dừng ở kim ban đầu — nên nó luôn kết thúc, không lặp vô hạn.
</details>

**Bài 3.** Cài **thuật toán chủ nhà băng** (Banker's algorithm) để *tránh* bế tắc thay vì chỉ *phát hiện* nó: cho ma trận nhu cầu tối đa và phân bổ hiện tại, xác định trạng thái có "an toàn" không.

<details>
<summary><b>Gợi ý</b></summary>

Trạng thái an toàn = tồn tại một **thứ tự hoàn thành** cho mọi tiến trình. Thuật toán: lặp tìm một tiến trình chưa xong mà `nhu_cau_con_lai <= tai_nguyen_kha_dung`; giả vờ cho nó chạy xong và **trả lại** toàn bộ tài nguyên nó giữ; lặp lại. Nếu xử lý được hết → an toàn. Nếu kẹt mà còn tiến trình chưa xong → không an toàn.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
/// `toi_da[i][j]`  = tiến trình i có thể cần tối đa bao nhiêu tài nguyên loại j
/// `da_cap[i][j]`  = đang giữ bao nhiêu
/// `kha_dung[j]`   = còn rảnh bao nhiêu
pub fn trang_thai_an_toan(
    toi_da: &[Vec<i32>], da_cap: &[Vec<i32>], kha_dung: &[i32],
) -> Option<Vec<usize>> {
    let n = toi_da.len();
    let m = kha_dung.len();
    let mut ranh: Vec<i32> = kha_dung.to_vec();
    let mut done = vec![false; n];
    let mut thu_tu = Vec::new();

    for _ in 0..n {
        // tìm một tiến trình có thể hoàn thành với tài nguyên đang rảnh
        let candidates = (0..n).find(|&i| {
            !done[i] && (0..m).all(|j| toi_da[i][j] - da_cap[i][j] <= ranh[j])
        });
        match candidates {
            Some(i) => {
                // cho nó chạy xong rồi TRẢ LẠI mọi thứ nó giữ
                for j in 0..m { ranh[j] += da_cap[i][j]; }
                done[i] = true;
                thu_tu.push(i);
            }
            None => return None, // kẹt → trạng thái KHÔNG an toàn
        }
    }
    Some(thu_tu)
}
```

Điểm tinh tế: thuật toán này **bi quan** — nó giả định mọi tiến trình đều có thể đòi tới mức tối đa. Vì thế nó từ chối cả một số trạng thái thực ra vẫn ổn. Đó là lý do các hệ điều hành hiện đại **không** dùng nó: phải khai báo trước nhu cầu tối đa là điều bất khả thi. Thực tế người ta chọn *phòng ngừa* (thứ tự khóa cố định) hoặc *phát hiện rồi khôi phục* (như phần đồ thị chờ trong chương).
</details>
