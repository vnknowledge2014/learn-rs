# Chương 49: Động cơ bất đồng bộ Tokio Runtime, Vòng lặp sự kiện & Cơ chế Epoll (Asynchronous Tokio Runtime, Event Loops & Epoll)

## Giới thiệu & Mục tiêu học tập

Trong thập niên 2000, thế giới điện toán đối mặt với một bức tường giới hạn nổi tiếng mang tên **Bài toán C10K (C10K Problem)**: Làm thế nào một máy chủ đơn lẻ có thể duy trì và phục vụ đồng thời 10,000 kết nối mạng cùng lúc mà không bị sập nguồn vì cạn kiệt bộ nhớ? Ngày nay, với sự bùng nổ của mạng xã hội, ứng dụng chat thời gian thực và mạng phân tán, thách thức đó đã nâng lên thành **Bài toán C1000K (1 triệu kết nối đồng thời)**.

Mô hình đa luồng truyền thống "1 luồng hệ điều hành = 1 kết nối" (One-Thread-Per-Connection) đã hoàn toàn phá sản trước bài toán này. Để giải quyết triệt để, ngành công nghiệp chuyển dịch sang mô hình **I/O Đa dồn kênh bất đồng bộ (Asynchronous Non-blocking I/O)**. Và trong vũ trụ Rust, vị vua thống trị tuyệt đối lĩnh vực này chính là **Tokio Runtime**.

Trong chương này, chúng ta sẽ mở nắp ca-pô cỗ máy Tokio để khám phá:
- Tại sao luồng hệ điều hành (OS Thread) lại tốn kém và nguyên nhân gây ra sự chậm trễ từ việc hoán đổi ngữ cảnh (Context Switching).
- Cơ chế đa dồn kênh I/O tầng nhân hệ điều hành: `epoll` (trên Linux), `kqueue` (trên macOS), và `IOCP` (trên Windows).
- Cốt lõi của Rust Async: Trait `Future`, Máy trạng thái hữu hạn (Finite State Machine) được sinh tự động, `Poll::Ready` vs `Poll::Pending`, và cơ chế đánh thức `Waker`.
- Kiến trúc điều phối cắp việc (Work-Stealing Scheduler) của Tokio: Làm thế nào hàng chục ngàn Task siêu nhẹ (Green Threads chỉ tốn vài trăm byte RAM) có thể chạy mượt mà trên một số ít nhân CPU thực tế.
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
- Mỗi luồng này ngốn sẵn từ `2MB` đến `8MB` bộ nhớ ngăn xếp (Stack). Nếu có 10,000 kết nối, riêng bộ nhớ Stack đã "ngốn" sạch `20GB` đến `80GB` RAM!
- Tệ hơn nữa, khi một kết nối đang chờ người dùng gõ phím hay chờ dữ liệu từ đĩa cứng (thao tác I/O), luồng đó hoàn toàn bị chặn (`blocked`). CPU phải liên tục hoán đổi qua lại giữa hàng ngàn luồng (Context Switching), tiêu tốn phần lớn năng lượng chỉ để ghi chép sổ sách thay vì xử lý dữ liệu.

### 2. Quán cà phê phát thẻ rung tự động (Async Event Loop & Epoll)
- **Thẻ rung tự động (Trait `Future`)**: Đại diện cho một kết quả chưa hoàn thành ở hiện tại nhưng cam kết sẽ có trong tương lai.
- **Tiếng kêu "Tít tít!" (Cơ chế `Waker`)**: Khi dữ liệu mạng từ card mạng thực sự cập bến (gói tin đã về tới buffer), hệ điều hành (qua `epoll`) phát tín hiệu đánh thức `Waker`.
- **Anh thu ngân siêu tốc (Tokio Event Loop / Executor)**: Chỉ cần vài nhân CPU (thường bằng số nhân phần cứng của máy), Tokio luân phiên kiểm tra và thực thi các Task sẵn sàng chạy, đạt hiệu suất phục vụ hàng triệu kết nối mà mỗi Task chỉ tiêu tốn vỏn vẹn khoảng `300 bytes` RAM!

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
> Cả ba đều là **hàm tử** (có phép `map`) và đều là **đơn nguyên** (có phép nối tiếp phụ thuộc). Và cả ba đều **lười biếng theo cách riêng**: iterator không chạy cho tới khi có consumer; `Future` không chạy cho tới khi được `poll`.
>
> Chuỗi `async` sau đây chính là một chuỗi `bind` được viết bằng cú pháp thuận mắt:
> ```rust
> async fn handle(id: u64) -> Result<Invoice, SystemError> {
>     let user = tim_nguoi_dung(id).await?;   // bind trong CẢ HAI ngữ cảnh cùng lúc
>     let don = tim_don_hang(&user).await?;   // (Future và Result lồng nhau)
>     Ok(invoice_loop(&don))
> }
> ```
> Chữ ký `-> Result<Invoice, LoiHeThong>` của một `async fn` thực chất là `Future<Output = Result<Invoice, LoiHeThong>>` — hai chiếc hộp lồng nhau. Đây chính là tình huống mà thế giới Haskell gọi là *chồng đơn nguyên* (monad stack), và `.await?` là cách Rust cho bạn bóc cả hai lớp chỉ bằng ba ký tự.
- **Máy trạng thái không chi phí (Zero-Cost State Machine)**:
  - Khi một tác vụ bất đồng bộ được biên dịch, Rust biến tác vụ đó thành một `enum` máy trạng thái.
  - Mỗi bước tạm dừng tương ứng với một trạng thái của `enum`.
  - Không có bộ nhớ Heap nào bị cấp phát ngầm; toàn bộ kích thước của máy trạng thái được tính toán chính xác ngay khi biên dịch!

### 3. Kiến trúc Động cơ Điều phối Tokio (Tokio Runtime Architecture)

Runtime Tokio được chia thành hai thành phần cộng sinh hoàn hảo:
1. **Bộ phản ứng (The Reactor)**: Giao tiếp trực tiếp với hệ điều hành thông qua `mio` (`epoll`/`kqueue`), chịu trách nhiệm theo dõi các sự kiện mạng, bộ đếm thời gian (timers), và kích hoạt `Waker` khi sự kiện xảy ra.
2. **Bộ điều hành (The Executor)**:
   - Sử dụng thuật toán **Cắp việc (Work-Stealing Algorithm)**: Mỗi nhân CPU quản lý một hàng đợi tác vụ cục bộ (Local Run Queue).
   - Nếu nhân số 1 xử lý hết việc trong hàng đợi của mình, nó sẽ "liếc sang" hàng đợi của nhân số 2 và cắp bớt một nửa số Task về xử lý, đảm bảo tất cả các nhân CPU luôn hoạt động với tải trọng cân bằng tuyệt đối.
3. **Đa nhiệm cộng tác (Cooperative Multitasking)**:
   - Mỗi Task chạy cho đến khi gặp điểm tạm dừng thì tự nguyện nhường quyền điều khiển CPU cho Task khác.
   - Cơ chế này kết hợp cùng quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để bảo đảm tài nguyên luôn được giải phóng kịp thời khi task hoàn tất.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn Rust hoàn chỉnh xây dựng một **Động cơ Bất đồng bộ thu nhỏ (Educational Mini-Runtime)**: Tự tay cài đặt máy trạng thái `Future`, cơ chế trả về `Poll::Pending` / `Poll::Ready`, và bộ điều phối thực thi tuần tự các tác vụ mà không cần thư viện bên ngoài:

```rust
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

/// Động cơ điều phối attempt nhỏ thực thi một Future cho đến khi hoàn tất
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
    println!("\n[2] Thuc thi Composite State Machine gom 2 giai segment I/O:");
    let composite_task = CompositeAsyncTask::new();
    let final_report = block_on_mini_runtime(composite_task);
    println!("    - Ket qua chuoi nhiem vu: {}", final_report);

    // 3. Phân tích so sánh tài nguyên
    println!("\n[3] Phan products so sanh kien truc tai nguyen bo nho:");
    println!("    - Dung luong Stack cua 1 Luong he dieu hanh (OS Thread): ~2,097,152 bytes (2MB)");
    println!("    - Dung luong RAM cua 1 Tokio Green Task               : ~300 bytes");
    println!("    ==> Ty le tiet kiem bo nho: Tokio Task tieu attempt RAM it hon ~7,000 LAN!");
    println!("    ==> Cho phep 1 may chu duy tri hang trieu ket noi ma khong bao gio het RAM!");

    println!("\n==================================================================");
    println!("   XAC NHAN: MO HINH ASYNC RUST HOAT DONG HOAN HAO - ZERO COST!  ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi lập trình bất đồng bộ trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `the trait 'Future' is not implemented for 'MyType'` | Truyền một kiểu dữ liệu không triển khai trait `Future` vào một hàm đòi hỏi Future. | Triển khai trait `Future` cho struct với phương thức `fn poll(...) -> Poll<Self::Output>`. |
| **E0277** | `the trait 'Send' is not implemented for 'Rc<T>'` | Lưu trữ một kiểu dữ liệu không an toàn cho luồng (`Rc<T>`, `RefCell<T>`) qua một điểm gọi trong một task bất đồng bộ đa luồng. | Thay thế bằng kiểu tương đương an toàn đa luồng: dùng `Arc<T>` và `tokio::sync::Mutex<T>`. |
| **E0382** | `use of moved value: 'client'` | Biến bị di chuyển quyền sở hữu (ownership) vào một block bất đồng bộ, sau đó lại được dùng ở bên ngoài. | Nhân bản dữ liệu trước khi di chuyển: `let client_clone = client.clone();`. |
| **E0507** | `cannot move out of a mutable pin` | Cố gắng di chuyển một trường dữ liệu ra khỏi một cấu trúc đã bị ghim (`Pin<&mut Self>`). | Sử dụng các phương thức an toàn của `Pin` hoặc truy cập qua tham chiếu mượn (borrow). |

### Ví dụ phân tích lỗi `E0277` khi thiếu triển khai Trait Future:

```rust
use std::pin::Pin;
use std::task::{Context, Poll};

struct NotAFuture;

// Đoạn mã lỗi minh họa E0277:
fn run_broken() {
    // let k = NotAFuture;
    // block_on_mini_runtime(k); // LỖI E0277: NotAFuture không triển khai trait Future!
}

// Cách sửa chữa đúng chuẩn: Triển khai trait Future đầy đủ
struct LaFuture;

impl std::future::Future for LaFuture {
    type Output = i32;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
        Poll::Ready(100)
    }
}

fn run_correct() {
    let f = LaFuture;
    println!("Đã sẵn sàng triển khai Future chuẩn mực!");
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Giải pháp cho bài toán C10K/C1M**: Chuyển đổi từ mô hình luồng đồng bộ sang mô hình bất đồng bộ hướng sự kiện dựa trên `epoll`/`kqueue`.
2. **Bản chất của Future trong Rust**: Là máy trạng thái tĩnh lười biếng (Lazy State Machine), không tốn chi phí cấp phát Heap ngầm, chỉ thực thi khi được thăm dò (`poll`).
3. **Cơ chế Waker**: Cho phép Reactor đánh thức Executor một cách chính xác ngay khi dữ liệu sẵn sàng trên card mạng, loại bỏ hoàn toàn việc thăm dò liên tục gây lãng phí CPU.
4. **Kiến trúc Tokio Work-Stealing**: Điều phối hàng triệu Task siêu nhẹ trên một nhóm nhỏ luồng công nhân, kết hợp cùng cơ chế quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để đạt thông lượng I/O cao nhất thế giới.

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

/// Future phát ra 5 nhịp, mỗi nhịp cách nhau `chu_ky`, rồi kết thúc.
/// Trả về tổng số nhịp đã phát.
pub struct AsyncInterval {
    con_lai: u32,
    chu_ky: Duration,
    nhip_ke: Instant,
    da_phat: u32,
}

impl AsyncInterval {
    pub fn new(chu_ky: Duration, so_nhip: u32) -> Self {
        Self { con_lai: so_nhip, chu_ky, nhip_ke: Instant::now() + chu_ky, da_phat: 0 }
    }
}

impl Future for AsyncInterval {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        loop {
            if self.con_lai == 0 {
                return Poll::Ready(self.da_phat); // hết nhịp -> xong
            }
            if Instant::now() >= self.nhip_ke {
                // Tới giờ một nhịp: cập nhật trạng thái rồi vòng lại kiểm nhịp kế.
                self.da_phat += 1;
                self.con_lai -= 1;
                let ck = self.chu_ky;
                self.nhip_ke = Instant::now() + ck;
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
fn block_on<F: Future>(mut f: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut f = unsafe { Pin::new_unchecked(&mut f) };
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; }
    }
}

#[test]
fn phat_dung_5_nhip() {
    // Dùng chu kỳ ngắn để test chạy nhanh; logic không đổi so với 100ms.
    let ket_qua = block_on(AsyncInterval::new(Duration::from_millis(1), 5));
    assert_eq!(ket_qua, 5);
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
    fut_a: Pin<Box<A>>, kq_a: Option<A::Output>,
    fut_b: Pin<Box<B>>, kq_b: Option<B::Output>,
}

impl<A: Future, B: Future> JoinTwo<A, B> {
    pub fn new(fut_a: A, fut_b: B) -> Self {
        Self { fut_a: Box::pin(fut_a), kq_a: None, fut_b: Box::pin(fut_b), kq_b: None }
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
        if this.kq_a.is_none() {
            if let Poll::Ready(v) = this.fut_a.as_mut().poll(cx) { this.kq_a = Some(v); }
        }
        // Thăm dò B tương tự — độc lập với A.
        if this.kq_b.is_none() {
            if let Poll::Ready(v) = this.fut_b.as_mut().poll(cx) { this.kq_b = Some(v); }
        }
        // Chỉ xong khi CẢ HAI đều có kết quả.
        if this.kq_a.is_some() && this.kq_b.is_some() {
            Poll::Ready((this.kq_a.take().unwrap(), this.kq_b.take().unwrap()))
        } else {
            Poll::Pending
        }
    }
}

#[test]
fn ghep_hai_future_san_sang() {
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
Tác vụ 1 giữ khóa nhưng bị treo chờ I/O; tác vụ 2 trên cùng luồng chặn cả luồng để chờ khóa đó. Luồng không tiến được, và nếu runtime chỉ có vài luồng, **vài deadlock kiểu này làm đơ toàn bộ hệ thống** — nghẽn luồng (thread starvation).

Ngay cả khi không deadlock hẳn, giữ khóa qua `.await` cũng **phá tính đồng thời**: khóa lẽ ra chỉ giữ vài micro-giây thì nay bị giữ suốt cả một thao tác mạng dài (hàng chục mili-giây), chặn mọi tác vụ khác cần khóa đó.

**Cách đúng:**
1. **Thu hẹp phạm vi khóa để KHÔNG bắc qua `.await`** — khóa, đọc/ghi thật nhanh, nhả khóa *trước* khi `.await`:
   ```text
   let gia_tri = { let g = m.lock().unwrap(); g.doc() };  // nhả khóa ở đây
   xu_ly_mang(gia_tri).await;                              // await KHÔNG giữ khóa
   ```
2. **Hoặc dùng `tokio::sync::Mutex`** — khóa *async-aware*: khi chờ khóa nó `.await` (nhường luồng) thay vì chặn luồng, và được thiết kế để giữ an toàn qua `.await`. Đổi lại nó chậm hơn `std::sync::Mutex`, nên chỉ dùng khi *thật sự* cần giữ khóa qua điểm chờ.

Quy tắc thực dụng của dân Tokio: **mặc định vẫn dùng `std::sync::Mutex` cho dữ liệu chung, nhưng tuyệt đối nhả nó trước mọi `.await`.** Chỉ khi logic buộc phải giữ khóa xuyên qua thao tác async mới đổi sang `tokio::sync::Mutex`.
</details>
