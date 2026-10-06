# Chương 49: Động cơ bất đồng bộ Tokio Runtime, Vòng lặp sự kiện & Cơ chế Epoll (Asynchronous Tokio Runtime, Event Loops & Epoll)

## Giới thiệu & Mục tiêu học tập

Trong thập niên 2000, thế giới điện toán đối mặt với một bức tường giới hạn nổi tiếng mang tên **Bài toán C10K (C10K Problem)**: Làm thế nào một máy chủ đơn lẻ có thể duy trì và phục vụ đồng thời 10,000 kết nối mạng cùng lúc mà không bị sập nguồn vì cạn kiệt bộ nhớ? Ngày nay, với sự bùng nổ của mạng xã hội, ứng dụng chat thời gian thực và mạng phân tán, thách thức đó đã nâng lên thành **Bài toán C1000K (1 triệu kết nối đồng thời)**.

Mô hình đa luồng truyền thống "1 luồng hệ điều hành = 1 kết nối" (One-Thread-Per-Connection) đã hoàn toàn phá sản trước bài toán này. Để giải quyết triệt để, ngành công nghiệp chuyển dịch sang mô hình **I/O Đa dồn kênh bất đồng bộ (Asynchronous Non-blocking I/O)**. Và trong vũ trụ Rust, vị vua thống trị tuyệt đối lĩnh vực này chính là **Tokio Runtime**.

Trong chương này, chúng ta sẽ mở nắp ca-pô cỗ máy Tokio để khám phá:
- Tại sao luồng hệ điều hành (OS Thread) lại tốn kém và nguyên nhân gây ra sự chậm trễ từ việc hoán đổi ngữ cảnh (Context Switching).
- Cơ chế đa dồn kênh I/O tầng nhân hệ điều hành: `epoll` (trên Linux), `kqueue` (trên macOS), và `IOCP` (trên Windows).
- Cốt lõi của Rust Async: Trait `Future`, Máy trạng thái hữu hạn (Finite State Machine) được sinh tự động, `Poll::Ready` vs `Poll::Pending`, và cơ chế đánh thức `Waker`.
- Kiến trúc điều phối cắp việc (Work-Stealing Scheduler) của Tokio: Làm thế nào hàng chục ngàn Task siêu nhẹ (mỗi Task chỉ tốn đúng kích thước máy trạng thái của nó — thường vài trăm byte đến vài KB) có thể chạy mượt mà trên một số ít nhân CPU thực tế.
- Kỹ thuật lập trình bất đồng bộ thực chiến: Tự tay dựng một Động cơ Mini-Runtime và hiểu thấu đáo cách vận hành của vòng lặp sự kiện (Event Loop).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu rõ sự khác biệt giữa mô hình Đồng bộ chặn (Blocking Sync) và Bất đồng bộ dựa trên sự kiện (Async Event-Driven), hãy quan sát hai quán ăn sau:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA: QUÁN PHỞ TRUYỀN THỐNG VS QUÁN CÀ PHÊ THẺ RUNG       │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. MÔ HÌNH ĐỒNG BỘ CHẶN (BLOCKING SYNC: 1 LUỒNG = 1 KẾT NỐI)]                   │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Khách bước vào bàn ──► 1 Bồi bàn đứng kè kè bên cạnh:                │         │
│ │ - Khách gọi món phở gà ──► Bồi bàn đi xuống bếp.                     │         │
│ │ - Nồi nước sôi mất 10 phút: Bồi bàn ĐỨNG BẤT ĐỘNG CHỜ ĐỢI 10 PHÚT!  │         │
│ │ - Bồi bàn bưng bát phở ra ──► Khách ăn xong ──► Bồi bàn mới rảnh tay!│         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│   ===> 100 khách cần tới 100 bồi bàn đứng đợi đờ đẫn (Lãng phí tài nguyên khủng)!│
│                                                                                  │
│ [2. MÔ HÌNH BẤT ĐỒNG BỘ (ASYNC TOKIO / EPOLL: EVENT-DRIVEN VỚI THẺ RUNG)]        │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ 1. Bạn tới quầy gọi trà sữa ──► Thu ngân trao bạn một THẺ RUNG TỰ ĐỘNG│        │
│ │    (Đây chính là tấm vé hẹn tương lai: Trait Future)!                │         │
│ │ 2. Bạn cầm thẻ về bàn ngồi lướt điện thoại thoải mái.               │         │
│ │ 3. Anh thu ngân LẬP TỨC phục vụ khách hàng tiếp theo (Không hề đợi)! │         │
│ │ 4. Bếp pha xong ──► Thẻ kêu "TÍT TÍT TÍT!" (Cơ chế Waker đánh thức)  │         │
│ │ 5. Bạn thong thả ra quầy nhận cốc trà sữa (Poll::Ready)!             │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│   ===> CHỈ CẦN 1 ANH THU NGÂN PHỤC VỤ CẢ NGÀN KHÁCH MÀ KHÔNG AI PHẢI ĐỢI!        │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Quán phở truyền thống (OS Thread Per Connection)
- Mỗi khi có một kết nối mạng mở ra, hệ điều hành cấp phát một luồng thực thi (OS Thread) riêng biệt.
- Mỗi luồng này được dành sẵn từ `2MB` đến `8MB` không gian địa chỉ ảo cho ngăn xếp (Stack). RAM vật lý chỉ được cấp theo từng trang khi thực sự chạm tới, nên 10,000 luồng không ngốn ngay `20GB`–`80GB` — nhưng mỗi luồng vẫn tốn ít nhất vài chục KB thật (trang stack đã dùng, cấu trúc quản lý của nhân), cộng thêm chi phí lập lịch; tới hàng trăm nghìn luồng thì không gian địa chỉ, giới hạn số luồng của hệ điều hành và chi phí hoán đổi ngữ cảnh trở thành bức tường.
- Tệ hơn nữa, khi một kết nối đang chờ người dùng gõ phím hay chờ dữ liệu từ đĩa cứng (thao tác I/O), luồng đó hoàn toàn bị chặn (`blocked`). CPU phải liên tục hoán đổi qua lại giữa hàng ngàn luồng (Context Switching), tiêu tốn phần lớn năng lượng chỉ để ghi chép sổ sách thay vì xử lý dữ liệu.

### 2. Quán cà phê phát thẻ rung tự động (Async Event Loop & Epoll)
- **Thẻ rung tự động (Trait `Future`)**: Đại diện cho một kết quả chưa hoàn thành ở hiện tại nhưng cam kết sẽ có trong tương lai.
- **Tiếng kêu "Tít tít!" (Cơ chế `Waker`)**: Khi dữ liệu mạng từ card mạng thực sự cập bến (gói tin đã về tới buffer), hệ điều hành (qua `epoll`) phát tín hiệu đánh thức `Waker`.
- **Anh thu ngân siêu tốc (Tokio Event Loop / Executor)**: Chỉ cần vài nhân CPU (thường bằng số nhân phần cứng của máy), Tokio luân phiên kiểm tra và thực thi các Task sẵn sàng chạy, đạt hiệu suất phục vụ hàng triệu kết nối mà mỗi Task chỉ tiêu tốn đúng kích thước máy trạng thái của nó (thường vài trăm byte đến vài KB)!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Cơ chế Đa dồn kênh I/O tầng Nhân: Epoll và Kqueue

Thay vì chương trình phải chủ động đi hỏi từng ổ cắm mạng (Socket Polling làm nóng ran CPU):
- **Cơ chế `epoll` (trên Linux)**: Chương trình đăng ký 100,000 socket vào một "Bảng theo dõi sự kiện" của nhân Linux qua lệnh `epoll_ctl`.
- Sau đó, chương trình chỉ cần gọi duy nhất một lệnh `epoll_wait` và đi ngủ.
- Khi có bất kỳ socket nào nhận được dữ liệu, card mạng gửi tín hiệu ngắt phần cứng (Hardware Interrupt), nhân Linux đánh thức chương trình dậy và trả về đúng danh sách những socket đã sẵn sàng đọc/ghi. Đây là nền tảng giúp máy chủ xử lý hàng triệu kết nối với mức tiêu thụ CPU gần như bằng không khi rảnh rỗi.

### 2. Bản chất của Trait `Future` trong Rust

Trong Rust, lập trình bất đồng bộ tuân theo triết lý **Kéo dữ liệu (Poll-based Model)** thay vì Đẩy dữ liệu (Push-based như JavaScript Promises):

```rust
// Bản rút gọn định nghĩa trong std (std::future::Future, std::task::Poll)
use std::pin::Pin;
use std::task::Context;

pub trait Future {
    type Output;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
}

pub enum Poll<T> {
    Ready(T),   // Tác vụ đã hoàn thành, trả về kết quả
    Pending,    // Dữ liệu chưa sẵn sàng, hãy chờ Waker đánh thức lại!
}
```

- **Tính lười biếng (Futures are Lazy)**: Một `Future` trong Rust sẽ **hoàn toàn không làm gì cả** cho đến khi nó được nạp vào một Executor (như Tokio) và được gọi phương thức `.poll()`.

> **`Future` cũng là một chiếc "hộp" — và `.await` chính là `?` của thế giới bất đồng bộ.**
> Nếu bạn đã học Chương 19, hãy để ý điểm tương đồng này, nó sẽ giúp bạn hiểu `async` nhanh hơn rất nhiều:
>
> | Ngữ cảnh (hộp) | Nghĩa là gì | Lấy giá trị ra bằng |
> |---|---|---|
> | `Option<T>` | có thể rỗng | `?` |
> | `Result<T, E>` | có thể lỗi | `?` |
> | `Future<Output = T>` | **giá trị sẽ có trong tương lai** | `.await` |
>
> Về mặt khái niệm, cả ba đều là **hàm tử** và **đơn nguyên** (có phép nối tiếp phụ thuộc). Nhưng trong std chỉ `Option`/`Result` có sẵn phương thức `map`/`and_then`; trait `Future` của std **không** có `map` hay `and_then` (chúng nằm ở trait mở rộng `FutureExt` của crate `futures`) — với `Future`, phép nối tiếp được viết bằng `.await`. Khác biệt nữa: `Option`/`Result` là giá trị đã có sẵn, còn `Future` thì **lười biếng** giống iterator: không chạy cho tới khi được `poll`.
>
> Chuỗi `async` sau đây chính là một chuỗi `bind` được viết bằng cú pháp thuận mắt:
> ```rust
> // (khai báo giả tối thiểu để đoạn mã biên dịch được)
> struct User; struct Order; struct Invoice; struct SystemError;
> async fn find_user(_id: u64) -> Result<User, SystemError> { Ok(User) }
> async fn find_orders(_u: &User) -> Result<Vec<Order>, SystemError> { Ok(vec![]) }
> fn build_invoice(_o: &[Order]) -> Invoice { Invoice }
> async fn handle(id: u64) -> Result<Invoice, SystemError> {
>     let user = find_user(id).await?;          // bind trong CẢ HAI ngữ cảnh cùng lúc
>     let orders = find_orders(&user).await?;   // (Future và Result lồng nhau)
>     Ok(build_invoice(&orders))
> }
> ```
> Chữ ký `-> Result<Invoice, SystemError>` của một `async fn` thực chất là `Future<Output = Result<Invoice, SystemError>>` — hai chiếc hộp lồng nhau. Đây chính là tình huống mà thế giới Haskell gọi là *chồng đơn nguyên* (monad stack), và `.await?` là cách Rust cho bạn bóc cả hai lớp chỉ bằng ba ký tự.
- **Máy trạng thái không chi phí (Zero-Cost State Machine)**:
  - Khi một tác vụ bất đồng bộ được biên dịch, Rust biến tác vụ đó thành một `enum` máy trạng thái.
  - Mỗi bước tạm dừng tương ứng với một trạng thái của `enum`.
  - Bản thân `async fn` không cấp phát Heap ngầm; toàn bộ kích thước của máy trạng thái được tính toán chính xác ngay khi biên dịch! (Runtime như Tokio cấp phát một lần khi `spawn` để đặt task lên Heap.)

### 3. Kiến trúc Động cơ Điều phối Tokio (Tokio Runtime Architecture)

Runtime Tokio được chia thành hai thành phần cộng sinh hoàn hảo:
1. **Bộ phản ứng (The Reactor)**: Giao tiếp trực tiếp với hệ điều hành thông qua `mio` (`epoll`/`kqueue`), chịu trách nhiệm theo dõi các sự kiện mạng, bộ đếm thời gian (timers), và kích hoạt `Waker` khi sự kiện xảy ra.
2. **Bộ điều hành (The Executor)**:
   - Sử dụng thuật toán **Cắp việc (Work-Stealing Algorithm)**: Mỗi nhân CPU quản lý một hàng đợi tác vụ cục bộ (Local Run Queue).
   - Nếu nhân số 1 xử lý hết việc trong hàng đợi của mình, nó sẽ "liếc sang" hàng đợi của nhân số 2 và cắp bớt một nửa số Task về xử lý, giữ tải trọng giữa các nhân CPU cân bằng.
3. **Đa nhiệm cộng tác (Cooperative Multitasking)**:
   - Mỗi Task chạy cho đến khi gặp điểm tạm dừng thì tự nguyện nhường quyền điều khiển CPU cho Task khác.
   - Cơ chế này kết hợp cùng quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để bảo đảm tài nguyên luôn được giải phóng kịp thời khi task hoàn tất.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn Rust hoàn chỉnh xây dựng một **Động cơ Bất đồng bộ thu nhỏ (Educational Mini-Runtime)**: Tự tay cài đặt máy trạng thái `Future`, cơ chế trả về `Poll::Pending` / `Poll::Ready`, một `Waker` thật (đánh thức luồng executor bằng `Thread::unpark`), và bộ điều phối ngủ (`park`) khi không có việc thay vì quay vòng thăm dò — tất cả không cần thư viện bên ngoài. Để giữ mã ngắn, "reactor" ở đây là mỗi đồng hồ một luồng ngủ nền; reactor thật như của Tokio dùng một luồng với `epoll` và bánh xe hẹn giờ (timer wheel) cho hàng triệu đồng hồ:

```rust
use std::future::Future;
use std::pin::{Pin, pin};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::{Duration, Instant};

/// Một Future hẹn giờ mô phỏng I/O bất đồng bộ.
/// Đóng vai "Reactor" tí hon: lần đầu bị poll mà chưa tới giờ, nó giao cho một luồng
/// hẹn giờ nền nhiệm vụ gọi `Waker` khi tới hạn — đúng hợp đồng của `Future`:
/// **đã trả `Poll::Pending` thì phải sắp xếp để `Waker` được gọi về sau.**
pub struct AsyncTimerFuture {
    target_time: Instant,
    polled_count: usize,
    // Waker mới nhất mà luồng hẹn giờ sẽ gọi (None = chưa khởi động luồng hẹn giờ)
    shared_waker: Option<Arc<Mutex<Waker>>>,
}

impl AsyncTimerFuture {
    pub fn new(duration: Duration) -> Self {
        Self {
            target_time: Instant::now() + duration,
            polled_count: 0,
            shared_waker: None,
        }
    }
}

impl Future for AsyncTimerFuture {
    type Output = String;

    // AsyncTimerFuture là `Unpin` (mọi trường đều Unpin), nên `Pin<&mut Self>`
    // cho phép truy cập trường như `&mut Self` bình thường.
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.polled_count += 1;

        if Instant::now() >= self.target_time {
            // Tác vụ đã hoàn tất! Trả về kết quả
            return Poll::Ready(format!(
                "Tác vụ hoàn thành sau {} lần thăm dò (Poll)!",
                self.polled_count
            ));
        }

        // Chưa tới giờ: ĐĂNG KÝ Waker trước khi trả Pending
        match &self.shared_waker {
            Some(shared) => {
                // Executor có thể đổi Waker giữa các lần poll -> luôn giữ cái mới nhất
                let mut current = shared.lock().unwrap();
                if !current.will_wake(cx.waker()) {
                    *current = cx.waker().clone();
                }
            }
            None => {
                let shared = Arc::new(Mutex::new(cx.waker().clone()));
                let for_timer = Arc::clone(&shared);
                let target = self.target_time;
                thread::spawn(move || {
                    let now = Instant::now();
                    if target > now {
                        thread::sleep(target - now);
                    }
                    for_timer.lock().unwrap().wake_by_ref(); // "Tít tít!" thẻ rung kêu
                });
                self.shared_waker = Some(shared);
            }
        }
        Poll::Pending
    }
}

/// Máy trạng thái tổ hợp gồm 2 bước TUẦN TỰ (Composite State Machine) — đúng thứ
/// mà trình biên dịch sinh ra cho `async { timer1.await; timer2.await; }`.
pub enum CompositeAsyncTask {
    Step1 {
        timer1: AsyncTimerFuture,
        step2_duration: Duration,
    },
    // Đồng hồ bước 2 chỉ được tạo khi bước 1 xong, nên tổng thời gian = 30 + 40 ms
    Step2 {
        timer2: AsyncTimerFuture,
    },
    Done,
}

impl Default for CompositeAsyncTask {
    fn default() -> Self {
        Self::new()
    }
}

impl CompositeAsyncTask {
    pub fn new() -> Self {
        CompositeAsyncTask::Step1 {
            timer1: AsyncTimerFuture::new(Duration::from_millis(30)),
            step2_duration: Duration::from_millis(40),
        }
    }
}

impl Future for CompositeAsyncTask {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match &mut *self {
                CompositeAsyncTask::Step1 {
                    timer1,
                    step2_duration,
                } => {
                    // Thăm dò bước 1. Timer là Unpin nên dùng `Pin::new` an toàn,
                    // không cần `unsafe { Pin::new_unchecked(..) }`.
                    match Pin::new(timer1).poll(cx) {
                        Poll::Ready(msg) => {
                            println!("    [CompositeTask] Bước 1 xong: {}", msg);
                            let timer2 = AsyncTimerFuture::new(*step2_duration);
                            *self = CompositeAsyncTask::Step2 { timer2 };
                            // Tiếp tục vòng lặp sang bước 2
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                CompositeAsyncTask::Step2 { timer2 } => match Pin::new(timer2).poll(cx) {
                    Poll::Ready(msg) => {
                        println!("    [CompositeTask] Bước 2 xong: {}", msg);
                        *self = CompositeAsyncTask::Done;
                        return Poll::Ready("Toàn bộ chuỗi tác vụ đã thành công!".to_string());
                    }
                    Poll::Pending => return Poll::Pending,
                },
                CompositeAsyncTask::Done => panic!("Future bị poll sau khi đã Ready"),
            }
        }
    }
}

/// Waker đánh thức luồng đang chạy executor: `wake()` = `Thread::unpark()`
struct ThreadWaker {
    thread: Thread,
}

impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.thread.unpark();
    }
}

/// Động cơ điều phối thu nhỏ thực thi một Future cho đến khi hoàn tất.
/// Khi Future trả Pending, luồng NGỦ (park) cho tới khi Waker gọi unpark —
/// không quay vòng thăm dò liên tục làm nóng CPU.
pub fn block_on_mini_runtime<F: Future>(future: F) -> F::Output {
    let waker = Waker::from(Arc::new(ThreadWaker {
        thread: thread::current(),
    }));
    let mut context = Context::from_waker(&waker);

    // Ghim cố định Future trên Stack bằng macro `pin!` an toàn (không cần unsafe)
    let mut pinned_future = pin!(future);

    let mut poll_iterations = 0;
    loop {
        poll_iterations += 1;
        match pinned_future.as_mut().poll(&mut context) {
            Poll::Ready(result) => {
                println!(
                    "    [MiniRuntime] Đã nhận Poll::Ready ở vòng lặp #{}",
                    poll_iterations
                );
                return result;
            }
            // Ngủ chờ Waker. `park` có thể thức dậy giả (spurious) — vô hại,
            // vì ta chỉ poll lại và Future sẽ trả Pending lần nữa nếu chưa xong.
            Poll::Pending => thread::park(),
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   ĐỘNG CƠ BẤT ĐỒNG BỘ TOKIO, EVENT LOOP & EPOLL MECHANICS RUST   ");
    println!("==================================================================");

    // 1. Thử nghiệm Custom Future đơn lẻ
    println!("\n[1] Thực thi Custom Future đơn lẻ trên Mini-Runtime:");
    let single_future = AsyncTimerFuture::new(Duration::from_millis(50));
    let outcome = block_on_mini_runtime(single_future);
    println!("    - Kết quả Future: {}", outcome);

    // 2. Thử nghiệm Composite State Machine
    println!("\n[2] Thực thi Composite State Machine gồm 2 giai đoạn I/O tuần tự:");
    let started = Instant::now();
    let final_report = block_on_mini_runtime(CompositeAsyncTask::new());
    println!(
        "    - Kết quả chuỗi nhiệm vụ: {} (mất ~{} ms)",
        final_report,
        started.elapsed().as_millis()
    );

    // 3. Phân tích so sánh tài nguyên (số liệu cỡ độ điển hình)
    println!("\n[3] Phân tích so sánh kiến trúc tài nguyên bộ nhớ:");
    println!(
        "    - Stack dành sẵn cho 1 luồng hệ điều hành : ~2 MB (bộ nhớ ảo, cấp vật lý theo trang)"
    );
    println!(
        "    - Kích thước máy trạng thái của Future này: {} bytes",
        std::mem::size_of::<CompositeAsyncTask>()
    );
    println!("    ==> Một task async chỉ tốn đúng kích thước máy trạng thái của nó (cộng chi phí");
    println!("        quản lý của runtime), nhờ vậy một máy chủ giữ được rất nhiều kết nối.");

    println!("\n==================================================================");
    println!("   XÁC NHẬN: MÔ HÌNH ASYNC RUST HOẠT ĐỘNG ĐÚNG HỢP ĐỒNG WAKER     ");
    println!("==================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingWaker(AtomicUsize);
    impl Wake for CountingWaker {
        fn wake(self: Arc<Self>) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn pending_timer_registers_waker_and_gets_woken() {
        let counter = Arc::new(CountingWaker(AtomicUsize::new(0)));
        let waker = Waker::from(Arc::clone(&counter));
        let mut cx = Context::from_waker(&waker);
        let mut timer = AsyncTimerFuture::new(Duration::from_millis(20));
        assert!(Pin::new(&mut timer).poll(&mut cx).is_pending());
        // Không ai poll lại, nhưng Waker vẫn PHẢI được gọi khi tới hạn
        thread::sleep(Duration::from_millis(200));
        assert!(counter.0.load(Ordering::SeqCst) >= 1);
        assert!(Pin::new(&mut timer).poll(&mut cx).is_ready());
    }

    #[test]
    fn block_on_polls_few_times_instead_of_spinning() {
        let msg = block_on_mini_runtime(AsyncTimerFuture::new(Duration::from_millis(30)));
        // poll lần đầu (Pending) + poll sau khi được đánh thức (Ready);
        // có thể thêm vài lần nếu luồng thức dậy giả, nhưng không phải hàng chục lần.
        let polls: usize = msg.split_whitespace().find_map(|w| w.parse().ok()).unwrap();
        assert!((2..=4).contains(&polls), "polls = {polls}");
    }

    #[test]
    fn composite_steps_run_sequentially() {
        let started = Instant::now();
        block_on_mini_runtime(CompositeAsyncTask::new());
        // 30ms + 40ms tuần tự (bản cũ tạo cả hai đồng hồ cùng lúc -> chỉ ~40ms)
        assert!(started.elapsed() >= Duration::from_millis(70));
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi lập trình bất đồng bộ trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `` `MyType` is not a future `` | Truyền một kiểu dữ liệu không triển khai trait `Future` vào một hàm đòi hỏi Future. | Triển khai trait `Future` cho struct với phương thức `fn poll(...) -> Poll<Self::Output>`. |
| *(không có mã)* | `future cannot be sent between threads safely` (kèm ghi chú `` `Rc<i32>` cannot be sent between threads safely ``) | Giữ một kiểu dữ liệu không an toàn cho luồng (`Rc<T>`, `std::sync::MutexGuard`...) sống qua một điểm `.await` trong future truyền cho `tokio::spawn` (đòi `Send`). | Thay thế bằng kiểu tương đương an toàn đa luồng: dùng `Arc<T>` và `tokio::sync::Mutex<T>`. |
| **E0382** | `` borrow of moved value: `client` `` | Biến bị di chuyển quyền sở hữu (ownership) vào một block bất đồng bộ, sau đó lại được dùng ở bên ngoài. | Nhân bản dữ liệu trước khi di chuyển: `let client_clone = client.clone();`. |
| **E0507** | `` cannot move out of dereference of `Pin<&mut S>` `` | Cố gắng di chuyển một trường dữ liệu ra khỏi một cấu trúc đã bị ghim (`Pin<&mut Self>`). | Sử dụng các phương thức an toàn của `Pin` hoặc truy cập qua tham chiếu mượn (borrow). |

### Ví dụ phân tích lỗi `E0277` khi thiếu triển khai Trait Future:

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

struct NotAFuture;

// Đoạn mã lỗi minh họa E0277:
fn run_broken() {
    // let k = NotAFuture;
    // block_on_mini_runtime(k); // LỖI E0277: `NotAFuture` is not a future
}

// Cách sửa chữa đúng chuẩn: Triển khai trait Future đầy đủ
struct RealFuture;

impl std::future::Future for RealFuture {
    type Output = i32;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(100)
    }
}

fn run_correct() {
    let value = block_on_mini_runtime(RealFuture);
    println!("Future chuẩn mực trả về: {value}");
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Giải pháp cho bài toán C10K/C1M**: Chuyển đổi từ mô hình luồng đồng bộ sang mô hình bất đồng bộ hướng sự kiện dựa trên `epoll`/`kqueue`.
2. **Bản chất của Future trong Rust**: Là máy trạng thái tĩnh lười biếng (Lazy State Machine), không tốn chi phí cấp phát Heap ngầm, chỉ thực thi khi được thăm dò (`poll`).
3. **Cơ chế Waker**: Cho phép Reactor đánh thức Executor một cách chính xác ngay khi dữ liệu sẵn sàng trên card mạng, loại bỏ việc thăm dò liên tục gây lãng phí CPU. Hợp đồng bắt buộc: Future nào trả `Poll::Pending` thì phải đảm bảo `Waker` sẽ được gọi về sau, nếu không task treo vĩnh viễn.
4. **Kiến trúc Tokio Work-Stealing**: Điều phối hàng triệu Task siêu nhẹ trên một nhóm nhỏ luồng công nhân, kết hợp cùng cơ chế quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để đạt thông lượng I/O rất cao.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Xây dựng Bộ đếm nhịp bất đồng bộ - Async Interval)**:  
   Tạo một cấu trúc `AsyncInterval` triển khai trait `Future`, kích hoạt sự kiện sau mỗi khoảng thời gian định kỳ (ví dụ mỗi 100ms phát ra một nhịp đếm), lặp lại đúng 5 lần rồi dừng lại.
2. **Bài tập 2 (Bộ ghép nối hai luồng Future đồng thời - Join Two Futures)**:  
   Viết một hàm nhận vào hai Future độc lập `fut_a` và `fut_b`. Hãy thực thi thăm dò cả hai luồng sao cho khi cả hai đều trả về `Poll::Ready` thì hàm mới trả về kết quả gộp `(OutputA, OutputB)`.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Tại sao không dùng `std::sync::Mutex` trong mã Async?)**:  
   Tại sao các chuyên gia Tokio luôn khuyến cáo tuyệt đối không giữ khóa `std::sync::Mutex` qua các điểm gọi chờ I/O? Nếu một luồng bị dừng trong khi vẫn đang giữ khóa, hiện tượng nghẽn luồng (Thread Starvation / Deadlock) sẽ bùng phát như thế nào trong toàn bộ hệ thống?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Một Future tự cài là một máy trạng thái: mỗi lần bị `poll`, nó hoặc trả `Ready(giá trị)` hoặc `Pending`. Bộ đếm nhịp giữ số nhịp còn lại và mốc thời gian nhịp kế tiếp.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};
use std::time::{Duration, Instant};

/// Future phát ra 5 nhịp, mỗi nhịp cách nhau `period`, rồi kết thúc.
/// Trả về tổng số nhịp đã phát.
pub struct AsyncInterval {
    remaining: u32,
    period: Duration,
    next_tick: Instant,
    emitted: u32,
}

impl AsyncInterval {
    pub fn new(period: Duration, tick_count: u32) -> Self {
        Self { remaining: tick_count, period, next_tick: Instant::now() + period, emitted: 0 }
    }
}

impl Future for AsyncInterval {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        loop {
            if self.remaining == 0 {
                return Poll::Ready(self.emitted); // hết nhịp -> xong
            }
            if Instant::now() >= self.next_tick {
                // Tới giờ một nhịp: cập nhật trạng thái rồi vòng lại kiểm nhịp kế.
                self.emitted += 1;
                self.remaining -= 1;
                let period = self.period;
                self.next_tick = Instant::now() + period;
            } else {
                // Chưa tới giờ: đánh thức lại rồi nhường quyền (Pending).
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }
        }
    }
}

// Bộ chạy tối giản để thử: quay vòng poll cho tới khi Ready.
struct NoopWake;
impl Wake for NoopWake { fn wake(self: Arc<Self>) {} }
fn block_on<F: Future>(f: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut f = std::pin::pin!(f); // ghim an toàn, không cần unsafe
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; }
    }
}

#[test]
fn emits_exactly_5_ticks() {
    // Dùng chu kỳ ngắn để test chạy nhanh; logic không đổi so với 100ms.
    let ticks = block_on(AsyncInterval::new(Duration::from_millis(1), 5));
    assert_eq!(ticks, 5);
}
```

Điểm cốt lõi của `Future` tự cài: nó là **một máy trạng thái bị hỏi đi hỏi lại**. Mỗi lần `poll`, nó nhìn trạng thái hiện tại (còn mấy nhịp, đã tới giờ chưa) và trả lời *"xong rồi"* (`Ready`) hoặc *"chưa, hỏi lại sau"* (`Pending`). Điểm mấu chốt bạn phải làm đúng: khi trả `Pending`, phải **đăng ký đánh thức** qua `cx.waker()` — nếu không, bộ chạy không biết khi nào nên `poll` lại, và future treo vĩnh viễn. (Ở đây ta `wake_by_ref` ngay để bộ chạy quay lại liền; runtime thật như Tokio sẽ đăng ký hẹn giờ và chỉ đánh thức đúng lúc, không quay bận.)
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Ghép hai Future: `poll` cả hai mỗi vòng, giữ lại kết quả của cái nào xong trước. Chỉ trả `Ready((a,b))` khi CẢ HAI đều đã xong.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Ghép hai Future độc lập: chạy song song (thăm dò xen kẽ), trả kết quả gộp
/// (OutputA, OutputB) khi CẢ HAI cùng xong. Đây là phiên bản thu nhỏ của join!.
pub struct JoinTwo<A: Future, B: Future> {
    fut_a: Pin<Box<A>>, out_a: Option<A::Output>,
    fut_b: Pin<Box<B>>, out_b: Option<B::Output>,
}

impl<A: Future, B: Future> JoinTwo<A, B> {
    pub fn new(fut_a: A, fut_b: B) -> Self {
        Self { fut_a: Box::pin(fut_a), out_a: None, fut_b: Box::pin(fut_b), out_b: None }
    }
}

impl<A: Future, B: Future> Future for JoinTwo<A, B>
where
    A::Output: Unpin,
    B::Output: Unpin, // đầu ra Unpin (u32, String... hầu như luôn thế) -> JoinTwo Unpin
{
    type Output = (A::Output, B::Output);
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // JoinTwo là Unpin (future con đã Box::pin, đầu ra Unpin) -> lấy &mut an toàn.
        let this = self.get_mut();
        // Thăm dò A nếu nó CHƯA xong; lưu kết quả lại khi Ready.
        if this.out_a.is_none() {
            if let Poll::Ready(v) = this.fut_a.as_mut().poll(cx) { this.out_a = Some(v); }
        }
        // Thăm dò B tương tự — độc lập với A.
        if this.out_b.is_none() {
            if let Poll::Ready(v) = this.fut_b.as_mut().poll(cx) { this.out_b = Some(v); }
        }
        // Chỉ xong khi CẢ HAI đều có kết quả.
        if this.out_a.is_some() && this.out_b.is_some() {
            Poll::Ready((this.out_a.take().unwrap(), this.out_b.take().unwrap()))
        } else {
            Poll::Pending
        }
    }
}

#[test]
fn join_two_ready_futures() {
    use std::sync::Arc;
    use std::task::{Wake, Waker};
    struct W; impl Wake for W { fn wake(self: Arc<Self>) {} }

    // Hai future tức thì sẵn sàng (async block là Future).
    let j = JoinTwo::new(async { 10u32 }, async { "hai" });
    let waker = Waker::from(Arc::new(W));
    let mut cx = Context::from_waker(&waker);
    let mut j = Box::pin(j);
    match j.as_mut().poll(&mut cx) {
        Poll::Ready((a, b)) => { assert_eq!(a, 10); assert_eq!(b, "hai"); }
        Poll::Pending => panic!("cả hai đều sẵn sàng, phải Ready"),
    }
}
```

Đây là hạt nhân của tổ hợp `join!` mà mọi runtime async cung cấp. Ý tưởng cốt lõi: **thăm dò cả hai future mỗi vòng, ghi nhớ cái nào xong trước, và chỉ hoàn tất khi cả hai cùng xong.** Khác biệt then chốt so với chạy *tuần tự* (`.await` cái này rồi `.await` cái kia): join **xen kẽ** — trong lúc future A đang chờ I/O (`Pending`), ta vẫn thăm dò B, nên hai việc chờ *chồng lấn* thời gian thay vì cộng dồn. Nếu A chờ 2 giây và B chờ 3 giây, join xong sau ~3 giây (max), còn tuần tự mất ~5 giây (tổng).
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Điểm chết người: `std::sync::Mutex` khóa cả *luồng hệ điều hành*. Nếu giữ khóa đó qua một điểm `.await`, luồng bị treo *trong khi vẫn cầm khóa* — và một luồng chạy nhiều tác vụ async.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

Không được giữ `std::sync::Mutex` qua điểm `.await` vì nó có thể gây **deadlock hoặc nghẽn luồng toàn hệ thống** — bắt nguồn từ sự khác biệt căn bản giữa mô hình luồng và mô hình tác vụ async.

**Gốc rễ:** trong runtime async như Tokio, **một luồng hệ điều hành chạy RẤT NHIỀU tác vụ async** bằng cách xen kẽ chúng. Khi một tác vụ chạm `.await` và phải chờ (I/O chưa xong), runtime **cất tác vụ đó đi và cho luồng chạy tác vụ khác**. Đây là toàn bộ điểm mạnh của async: ít luồng phục vụ nhiều việc.

**Điều gì hỏng khi giữ `std::sync::Mutex` qua `.await`:**

`std::sync::Mutex` khóa ở tầng *luồng hệ điều hành* — nó không biết gì về tác vụ async. Xét kịch bản:
```text
Tác vụ 1: khóa mutex M
          .await một thao tác mạng   <- runtime CẤT tác vụ 1 đi (vẫn đang GIỮ M!)
                                         cho luồng chạy tác vụ 2
Tác vụ 2 (cùng luồng): cố khóa M
          -> M đang bị tác vụ 1 giữ -> tác vụ 2 CHẶN CẢ LUỒNG chờ M
          -> nhưng tác vụ 1 chỉ nhả M khi nó chạy tiếp
          -> mà nó chỉ chạy tiếp khi luồng rảnh
          -> mà luồng đang bị tác vụ 2 chặn  ->  DEADLOCK
```
Tác vụ 1 giữ khóa nhưng bị treo chờ I/O; tác vụ 2 trên cùng luồng chặn cả luồng để chờ khóa đó. Với runtime một luồng (`current_thread`) đây là deadlock chắc chắn; với runtime đa luồng, tác vụ 1 có thể được luồng khác "cắp" về chạy tiếp, nhưng mỗi tác vụ chặn như vậy vẫn chiếm trọn một luồng công nhân — vài trường hợp cùng lúc là **cả hệ thống đơ** vì nghẽn luồng (thread starvation).

Tin tốt: với `tokio::spawn`, trình biên dịch thường chặn lỗi này từ sớm — `std::sync::MutexGuard` không phải `Send`, nên future giữ nó qua `.await` không `Send` và bị từ chối với lỗi `future cannot be sent between threads safely`. Nhưng `block_on`, `spawn_local` hay runtime một luồng thì không có hàng rào đó.

Ngay cả khi không deadlock hẳn, giữ khóa qua `.await` cũng **phá tính đồng thời**: khóa lẽ ra chỉ giữ vài micro-giây thì nay bị giữ suốt cả một thao tác mạng dài (hàng chục mili-giây), chặn mọi tác vụ khác cần khóa đó.

**Cách đúng:**
1. **Thu hẹp phạm vi khóa để KHÔNG bắc qua `.await`** — khóa, đọc/ghi thật nhanh, nhả khóa *trước* khi `.await`:
   ```text
   let value = { let guard = m.lock().unwrap(); guard.clone() };  // nhả khóa ở đây
   send_over_network(value).await;                                // await KHÔNG giữ khóa
   ```
2. **Hoặc dùng `tokio::sync::Mutex`** — khóa *async-aware*: khi chờ khóa nó `.await` (nhường luồng) thay vì chặn luồng, và được thiết kế để giữ an toàn qua `.await`. Đổi lại nó chậm hơn `std::sync::Mutex`, nên chỉ dùng khi *thật sự* cần giữ khóa qua điểm chờ.

Quy tắc thực dụng của dân Tokio: **mặc định vẫn dùng `std::sync::Mutex` cho dữ liệu chung, nhưng tuyệt đối nhả nó trước mọi `.await`.** Chỉ khi logic buộc phải giữ khóa xuyên qua thao tác async mới đổi sang `tokio::sync::Mutex`.
</details>
