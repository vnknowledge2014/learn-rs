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
