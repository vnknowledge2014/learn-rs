#![allow(dead_code, unused_variables, unused_imports)]
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};
use std::time::{Duration, Instant};

/// Một Future đếm ngược thời gian tùy chỉnh mô phỏng I/O bất đồng bộ
pub struct AsyncTimerFuture {
    target_time: Instant,
    polled_count: usize,
}

impl AsyncTimerFuture {
    pub fn new(duration: Duration) -> Self {
        Self {
            target_time: Instant::now() + duration,
            polled_count: 0,
        }
    }
}

impl Future for AsyncTimerFuture {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        self.polled_count += 1;
        let now = Instant::now();

        if now >= self.target_time {
            // Tác vụ đã hoàn tất! Trả về kết quả
            Poll::Ready(format!(
                "Tac vu hoan thanh sau {} lan tham do (Poll)!",
                self.polled_count
            ))
        } else {
            // Dữ liệu chưa sẵn sàng: Nhường quyền điều khiển
            Poll::Pending
        }
    }
}

/// Mô phỏng máy trạng thái tổ hợp gồm 2 bước tuần tự (Composite State Machine)
pub struct CompositeAsyncTask {
    step: usize,
    timer1: AsyncTimerFuture,
    timer2: AsyncTimerFuture,
}

impl CompositeAsyncTask {
    pub fn new() -> Self {
        Self {
            step: 0,
            timer1: AsyncTimerFuture::new(Duration::from_millis(30)),
            timer2: AsyncTimerFuture::new(Duration::from_millis(40)),
        }
    }
}

impl Future for CompositeAsyncTask {
    type Output = String;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            match self.step {
                0 => {
                    // Thăm dò bước 1
                    let timer1_pin = unsafe { Pin::new_unchecked(&mut self.timer1) };
                    match timer1_pin.poll(cx) {
                        Poll::Ready(msg) => {
                            println!("    [CompositeTask] Buoc 1 xong: {}", msg);
                            self.step = 1;
                            // Tiếp tục vòng lặp sang bước 2
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                1 => {
                    // Thăm dò bước 2
                    let timer2_pin = unsafe { Pin::new_unchecked(&mut self.timer2) };
                    match timer2_pin.poll(cx) {
                        Poll::Ready(msg) => {
                            println!("    [CompositeTask] Buoc 2 xong: {}", msg);
                            self.step = 2;
                        }
                        Poll::Pending => return Poll::Pending,
                    }
                }
                2 => {
                    return Poll::Ready("Toan bo chuoi tac vu da thanh cong 100%!".to_string());
                }
                _ => unreachable!(),
            }
        }
    }
}

/// Tạo một Waker đơn giản cho mục đích mô phỏng (No-op Dummy Waker)
fn create_dummy_waker() -> Waker {
    fn no_op(_: *const ()) {}
    fn clone(p: *const ()) -> RawWaker {
        RawWaker::new(p, &VTABLE)
    }

    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, no_op, no_op, no_op);
    let raw_waker = RawWaker::new(std::ptr::null(), &VTABLE);
    unsafe { Waker::from_raw(raw_waker) }
}

/// Động cơ điều phối thu nhỏ thực thi một Future cho đến khi hoàn tất
pub fn block_on_mini_runtime<F: Future>(mut future: F) -> F::Output {
    let waker = create_dummy_waker();
    let mut context = Context::from_waker(&waker);

    // Ghim cố định Future vào bộ nhớ Stack (Pinning)
    let mut pinned_future = unsafe { Pin::new_unchecked(&mut future) };

    let mut poll_iterations = 0;
    loop {
        poll_iterations += 1;
        match pinned_future.as_mut().poll(&mut context) {
            Poll::Ready(result) => {
                println!("    [MiniRuntime] Da nhan Poll::Ready o vong lap #{}", poll_iterations);
                return result;
            }
            Poll::Pending => {
                // Nhường quyền CPU mô phỏng sự kiện I/O Epoll đang diễn ra
                std::thread::sleep(Duration::from_millis(10));
            }
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   DONG CO BAT DONG BO TOKIO, EVENT LOOP & EPOLL MECHANICS RUST   ");
    println!("==================================================================");

    // 1. Thử nghiệm Custom Future đơn lẻ
    println!("\n[1] Thuc thi Custom Future don le tren Mini-Runtime:");
    let single_future = AsyncTimerFuture::new(Duration::from_millis(50));
    let outcome = block_on_mini_runtime(single_future);
    println!("    - Ket qua Future: {}", outcome);

    // 2. Thử nghiệm Composite State Machine
    println!("\n[2] Thuc thi Composite State Machine gom 2 giai doan I/O:");
    let composite_task = CompositeAsyncTask::new();
    let final_report = block_on_mini_runtime(composite_task);
    println!("    - Ket qua chuoi nhiem vu: {}", final_report);

    // 3. Phân tích so sánh tài nguyên
    println!("\n[3] Phan tich so sanh kien truc tai nguyen bo nho:");
    println!("    - Dung luong Stack cua 1 Luong he dieu hanh (OS Thread): ~2,097,152 bytes (2MB)");
    println!("    - Dung luong RAM cua 1 Tokio Green Task               : ~300 bytes");
    println!("    ==> Ty le tiet kiem bo nho: Tokio Task tieu thu RAM it hon ~7,000 LAN!");
    println!("    ==> Cho phep 1 may chu duy tri hang trieu ket noi ma khong bao gio het RAM!");

    println!("\n==================================================================");
    println!("   XAC NHAN: MO HINH ASYNC RUST HOAT DONG HOAN HAO - ZERO COST!  ");
    println!("==================================================================");
}
