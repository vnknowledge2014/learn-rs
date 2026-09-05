# Chương 50: Mô hình Actor & Giao tiếp hộp thư đa luồng qua Channel (Actor Model & Thread-Safe Channels mpsc/oneshot)

## Giới thiệu & Mục tiêu học tập

Trong lịch sử lập trình song song và đa luồng, có một nghịch lý cay đắng: Khi các kỹ sư cố gắng tăng tốc hệ thống bằng cách chia sẻ bộ nhớ dùng chung (`Shared Memory`) và bảo vệ nó bằng các ổ khóa như `Mutex` (Mutual Exclusion) hay `RwLock` (Read-Write Lock), họ thường tạo ra một "bãi mìn" lỗi tiềm tàng: **Tranh chấp khóa dữ dội (Lock Contention), Đảo ngược độ ưu tiên (Priority Inversion), và nghiêm trọng nhất là Bế tắc khóa vĩnh viễn (Deadlock)**.

Để thoát khỏi vũng lầy này, ngành khoa học máy tính đã tìm ra một hướng đi thanh lịch: **Triết lý truyền thông điệp (Message Passing Concurrency)**, với châm ngôn bất hủ: *"Đừng giao tiếp bằng cách chia sẻ bộ nhớ; hãy chia sẻ bộ nhớ bằng cách giao tiếp"* (*Do not communicate by sharing memory; instead, share memory by communicating*). Đỉnh cao của triết lý này chính là **Mô hình Actor (Actor Model)** — mô hình đã giúp hãng viễn thông Ericsson vận hành hệ thống tổng đài Erlang với độ sẵn sàng huyền thoại $99.9999999\%$ (chỉ ngừng hoạt động vài phần nghìn giây mỗi năm).

Trong chương này, chúng ta sẽ làm chủ:
- Hạn chế cố hữu của cơ chế khóa chia sẻ bộ nhớ truyền thống và nguồn gốc của bế tắc Deadlock.
- Ba nguyên lý bất biến của Mô hình Actor: Trạng thái đóng kín (Isolated Private State), Hộp thư đến (Mailbox), và Xử lý thông điệp tuần tự (Sequential Message Processing).
- Phân loại các kênh truyền tin (Channels) trong Rust: Kênh nhiều người gửi - một người nhận (`mpsc`), và kênh phản hồi một lần (`oneshot`).
- Kỹ thuật kiến trúc mẫu Yêu cầu - Phản hồi (Request-Response Pattern) giữa các Actor bằng cách đính kèm "Phong bì hồi âm" (`return_envelope`).
- Tự tay lập trình một Actor hoàn chỉnh quản lý tài khoản ngân hàng và kiểm soát giao dịch song song 100% không bao giờ gặp Deadlock.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng đối chiếu hai bức tranh đời thường để thấy rõ tại sao mô hình Actor lại vượt trội hơn hẳn cơ chế khóa Mutex truyền thống:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA: BẠO LOẠN PHÒNG KẾ TOÁN VS BÁC KẾ TOÁN KHE CỬA        │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. CƠ CHẾ CHIA SẺ BỘ NHỚ VỚI KHÓA MUTEX (SHARED MEMORY MUTEX CONTENTION)]       │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ 10 nhân viên cùng xông vào 1 căn phòng nhỏ, tranh nhau giật lấy      │         │
│ │ duy nhất 1 cuốn sổ cái kế toán trên bàn để ghi chép!                 │         │
│ │ - Ai giật được sổ (Khóa Mutex): Được viết 5 giây.                    │         │
│ │ - 9 người còn lại đứng thở dốc chờ đợi (Nghẽn luồng - Contention).   │         │
│ │ - Hai nhân viên giật chéo sổ của nhau: Cả hai ghì chặt không buông,  │         │
│ │   toàn bộ công ty tê liệt vĩnh viễn (DEADLOCK)!                      │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│                                                                                  │
│ [2. MÔ HÌNH ACTOR VỚI HỘP THƯ CHANNEL (ACTOR MODEL & MAILBOXES)]                 │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Bác kế toán trưởng ngồi trong phòng làm việc khóa trái cửa.          │         │
│ │ Ở cửa ra vào có một KHE NHÉT THƯ ĐỘC NHẤT (Kênh mpsc::Receiver)!     │         │
│ │ 1. Bất kỳ ai muốn Nạp tiền hay Rút tiền chỉ việc viết một tờ giấy    │         │
│ │    bỏ vào khe cửa (mpsc::Sender).                                    │         │
│ │ 2. Nếu muốn nhận biên lai, người gửi kẹp sẵn một PHONG BÌ HỒI ÂM     │         │
│ │    (Kênh phản hồi oneshot) vào tờ giấy.                              │         │
│ │ 3. Bác kế toán ngồi nhâm nhi trà, đọc từng bức thư theo thứ tự,      │         │
│ │    cập nhật sổ cái, nhét biên lai vào phong bì trả ra ngoài!         │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│   ===> TUYỆT ĐỐI KHÔNG CÓ TRANH CHẤP, KHÔNG BAO GIỜ BỊ DEADLOCK!                 │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Bạo loạn trong phòng kế toán (Shared Memory Mutex)
- Cuốn sổ cái kế toán tượng trưng cho biến dữ liệu cần bảo vệ.
- Với khóa `Mutex`, bạn bắt các luồng phải xếp hàng tranh cướp quyền truy cập độc quyền. Nếu một luồng giữ khóa A và đợi khóa B, trong khi luồng khác giữ khóa B và đợi khóa A, cả hai sẽ đứng nhìn nhau trừng trừng cho đến khi máy chủ sập nguồn (**Deadlock**).

### 2. Bác kế toán ngồi sau khe nhét thư (The Actor Model)
- **Actor (Bác kế toán)**: Là người duy nhất trên thế giới có quyền nhìn thấy và chạm vào cuốn sổ cái (Private State). Không một ai bên ngoài được phép thò tay vào phòng.
- **Hòm thư (Mailbox / Channel `mpsc`)**: Người bên ngoài chỉ cần ném thông điệp qua khe cửa. Dù 100 nhân viên cùng ném thư tới tấp, các bức thư chỉ tự động xếp hàng ngăn nắp trong hòm thư của bác kế toán.
- **Phong bì hồi âm (Kênh `oneshot`)**: Khi bạn hỏi *"Số dư tài khoản của tôi còn bao nhiêu?"*, bạn không thể đứng chờ bác trả lời ngay. Bạn để lại chiếc phong bì có ghi sẵn địa chỉ bàn làm việc của bạn. Bác kế toán ghi số tiền, bỏ vào phong bì gửi ngược lại cho bạn.
- Nhờ cách ly hoàn toàn, bác kế toán xử lý mọi thứ tuần tự từ trên xuống dưới, không một hạt bụi nào bị xáo trộn, dữ liệu luôn nhất quán 100%!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Khuyết tật Cốt lõi của Cơ chế Khóa Truyền thống (Mutex / RwLock)

Trong các hệ thống phân tán và dịch vụ tải cao:
1. **Tranh chấp khóa (Lock Contention)**: Khi số lượng lõi CPU tăng lên (ví dụ 64 cores), nếu 64 luồng cùng tranh chấp một `Mutex`, thời gian CPU tiêu tốn cho việc chờ đợi và chuyển ngữ cảnh (Context Switching) có thể chiếm tới 80% tổng thời gian tính toán.
2. **Nguy cơ Deadlock**: Xảy ra khi có sự phụ thuộc vòng tròn giữa các khóa.
3. **Mất an toàn ngoại lệ (Lock Poisoning)**: Trong Rust, nếu một luồng đang giữ khóa `Mutex` mà bị `panic!`, ổ khóa đó sẽ bị "nhiễm độc" (`PoisonError`), khiến tất cả các luồng khác sau đó khi gọi `.lock()` đều bị lỗi theo.

### 2. Ba Trụ cột Kiến trúc của Mô hình Actor

Một Actor chuẩn mực bao gồm 3 yếu tố:
1. **Trạng thái riêng tư (Private State)**: Dữ liệu bên trong Actor hoàn toàn được bao bọc kín đáo, không bao giờ để lộ tham chiếu khả biến `&mut` ra bên ngoài.
2. **Hòm thư hàng đợi (Mailbox Queue)**: Thường được hiện thực hóa bằng một kênh truyền tin bất đồng bộ có đệm (Buffered MPSC Channel).
3. **Vòng lặp xử lý sự kiện (Event Processing Loop)**: Actor chạy một vòng lặp liên tục rút từng bức thư ra khỏi hòm và thực thi tương ứng. Vì chỉ có một luồng duy nhất thao tác với trạng thái nội bộ tại một thời điểm, ta hoàn toàn **không cần bất kỳ ổ khóa Mutex nào** bên trong Actor!

### 3. Phân loại Kênh truyền tin (Channels Taxonomy) trong Rust

```
┌──────────────────────────────────────┬──────────────────────────────────────────────────────────┐
│ Loại Kênh Truyền (Channel Type)      │ Đặc điểm kiến trúc & Trường hợp sử dụng                  │
├──────────────────────────────────────┼──────────────────────────────────────────────────────────┤
│ **mpsc** (Multi-Producer, Single-Con)│ Nhiều client gửi thông điệp vào 1 Actor duy nhất.        │
│ **oneshot** (Single-Prod, Single-Con)│ Dùng để phản hồi kết quả 1 lần duy nhất cho client.      │
│ **broadcast** (Multi-P, Multi-C)     │ Phát thanh thông điệp tới tất cả mọi người nghe.         │
│ **watch** (Single-P, Multi-C)        │ Chia sẻ giá trị trạng thái mới nhất cho nhiều bên xem.   │
└──────────────────────────────────────┴──────────────────────────────────────────────────────────┘
```

### 4. Mẫu Thiết kế Request-Response qua Hồi âm Oneshot

Làm thế nào client có thể nhận được kết quả trả về từ Actor khi kênh `mpsc` vốn dĩ là đường truyền một chiều?
- **Giải pháp**: Định nghĩa Enum thông điệp có chứa một trường kênh hồi âm:
```rust
pub enum AccountMessage {
    Deposit { amount: u64 },
    GetBalance { respond_to: oneshot::Sender<u64> }, // Phong bì hồi âm!
}
```
- Khi client gửi `GetBalance`:
  1. Client tạo một cặp kênh `let (resp_tx, resp_rx) = oneshot::channel();`.
  2. Client gửi `AccountMessage::GetBalance { respond_to: resp_tx }` vào hòm thư Actor.
  3. Client chờ nhận kết quả trên đầu nhận `resp_rx`.
  4. Actor xử lý xong, lấy `respond_to.send(self.balance)`. Client lập tức nhận được số dư an toàn!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn hoàn chỉnh của một **Hệ thống Ngân hàng Đa luồng hướng Actor (Thread-Safe Bank Account Actor)** được lập trình bằng Safe Rust chuẩn mực, sử dụng các kênh truyền tin đa luồng và mô hình Request-Response bằng phong bì hồi âm, hoàn toàn không sử dụng khóa chia sẻ bộ nhớ phức tạp:

```rust
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

/// Các loại mệnh lệnh thông điệp có thể gửi tới Actor
#[derive(Debug)]
pub enum AccountMessage {
    /// Nạp tiền: Thao tác 1 chiều (Fire-and-Forget)
    Deposit { amount: u64 },
    /// Rút tiền: Kèm theo kênh hồi âm trả về kết quả thành công hay thất bại
    Withdraw {
        amount: u64,
        respond_to: Sender<Result<u64, &'static str>>,
    },
    /// Vấn tin số dư: Kèm theo kênh hồi âm trả về số dư hiện tại
    GetBalance {
        respond_to: Sender<u64>,
    },
}

/// Thực thể Actor quản lý tài khoản ngân hàng (Sở hữu trạng thái riêng biệt)
pub struct BankAccountActor {
    mailbox_rx: Receiver<AccountMessage>,
    balance: u64, // Trạng thái hoàn toàn riêng tư, không ai ngoài Actor được đụng vào!
}

impl BankAccountActor {
    pub fn new(mailbox_rx: Receiver<AccountMessage>) -> Self {
        Self {
            mailbox_rx,
            balance: 0,
        }
    }

    /// Vòng lặp tiếp nhận và xử lý tuần tự từng thông điệp
    pub fn run(mut self) {
        println!("    [Actor Loop] Bác kế toán bắt đầu mở cửa hòm thư...");

        while let Ok(msg) = self.mailbox_rx.recv() {
            match msg {
                AccountMessage::Deposit { amount } => {
                    self.balance += amount;
                    println!(
                        "    [Actor] Đã nạp thành công {}đ. Số dư hiện tại: {}đ",
                        amount, self.balance
                    );
                }
                AccountMessage::Withdraw { amount, respond_to } => {
                    if self.balance >= amount {
                        self.balance -= amount;
                        println!(
                            "    [Actor] Đã rút thành công {}đ. Số dư còn lại: {}đ",
                            amount, self.balance
                        );
                        let _ = respond_to.send(Ok(self.balance));
                    } else {
                        println!(
                            "    [Actor] Từ chối rút {}đ: Số dư không đủ (Hiện có {}đ)!",
                            amount, self.balance
                        );
                        let _ = respond_to.send(Err("Số dư tài khoản không đủ để thực hiện giao dịch"));
                    }
                }
                AccountMessage::GetBalance { respond_to } => {
                    println!("    [Actor] Vấn tin số dư: Đang gửi kết quả {}đ về phong bì hồi âm...", self.balance);
                    let _ = respond_to.send(self.balance);
                }
            }
        }

        println!("    [Actor Loop] Hòm thư đã đóng. Bác kế toán kết thúc ca làm việc an toàn!");
    }
}

/// Giao diện điều khiển thuận tiện cho Client giao tiếp với Actor (Actor Client Handle)
#[derive(Clone)]
pub struct BankAccountHandle {
    mailbox_tx: Sender<AccountMessage>,
}

impl BankAccountHandle {
    pub fn new(mailbox_tx: Sender<AccountMessage>) -> Self {
        Self { mailbox_tx }
    }

    /// Gửi yêu cầu nạp tiền
    pub fn deposit(&self, amount: u64) {
        let _ = self.mailbox_tx.send(AccountMessage::Deposit { amount });
    }

    /// Gửi yêu cầu rút tiền và chờ nhận kết quả qua phong bì hồi âm
    pub fn withdraw(&self, amount: u64) -> Result<u64, &'static str> {
        let (resp_tx, resp_rx) = channel();
        let msg = AccountMessage::Withdraw {
            amount,
            respond_to: resp_tx,
        };
        let _ = self.mailbox_tx.send(msg);
        resp_rx.recv().unwrap_or(Err("Lỗi nhận phản hồi từ Actor"))
    }

    /// Gửi yêu cầu kiểm tra số dư
    pub fn get_balance(&self) -> u64 {
        let (resp_tx, resp_rx) = channel();
        let msg = AccountMessage::GetBalance {
            respond_to: resp_tx,
        };
        let _ = self.mailbox_tx.send(msg);
        resp_rx.recv().unwrap_or(0)
    }
}

fn main() {
    println!("==================================================================");
    println!("   MO HINH ACTOR & GIAO TIEP KENH DONG THOI AN TOAN TRONG RUST    ");
    println!("==================================================================");

    // 1. Tạo kênh truyền tin chính nối tới hòm thư của Actor
    let (mailbox_tx, mailbox_rx) = channel::<AccountMessage>();

    // 2. Khởi tạo Actor và chạy trên một luồng nền độc lập
    let actor = BankAccountActor::new(mailbox_rx);
    let actor_thread = thread::spawn(move || {
        actor.run();
    });

    // 3. Tạo tay cầm Handle để các client sử dụng
    let handle = BankAccountHandle::new(mailbox_tx);

    println!("\n[1] Thuc hien cac giao dich nap tien ban dau:");
    handle.deposit(100_000);
    handle.deposit(250_000);

    // Kiểm tra số dư qua Request-Response
    let current_bal = handle.get_balance();
    println!("    [Client Main] So du kiem tra duoc: {}d", current_bal);
    assert_eq!(current_bal, 350_000);

    println!("\n[2] Mo phong 3 luong khach hang dong thoi rut tien (Concurrent Clients):");
    let mut client_threads = Vec::new();

    for client_id in 1..=3 {
        let client_handle = handle.clone();
        let t = thread::spawn(move || {
            let withdraw_amount = 150_000;
            println!("    - Khach hang #{} bat dau gui lenh rut {}d...", client_id, withdraw_amount);
            match client_handle.withdraw(withdraw_amount) {
                Ok(remaining) => println!("      + Khach hang #{} rut THANH CONG! So du con: {}d", client_id, remaining),
                Err(err) => println!("      + Khach hang #{} rut THAT BAI: {}", client_id, err),
            }
        });
        client_threads.push(t);
    }

    for t in client_threads {
        let _ = t.join();
    }

    // Kiểm tra số dư cuối cùng
    let final_balance = handle.get_balance();
    println!("\n[3] So du cuoi cung trong so cai Actor: {}d", final_balance);
    assert_eq!(final_balance, 50_000);

    // Tiêu hủy handle để đóng mailbox, luồng Actor sẽ kết thúc êm ái
    drop(handle);
    let _ = actor_thread.join();

    println!("\n==================================================================");
    println!("   XAC NHAN: TOAN BO GIAO DICH DA DONG BO HOAN HAO - ZERO LOCK!  ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi triển khai mô hình Actor và kênh truyền tin trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0382** | `use of moved value: 'mailbox_tx'` | Bạn truyền `mailbox_tx` vào luồng hoặc hàm khiến quyền sở hữu (ownership) bị di chuyển, sau đó lại cố gắng dùng lại nó. | Nhân bản đối tượng người gửi trước khi di chuyển: `let tx_clone = mailbox_tx.clone();`. |
| **E0277** | `the trait 'Send' is not implemented for 'Rc<T>'` | Đặt một kiểu dữ liệu không hỗ trợ đa luồng vào bên trong cấu trúc thông điệp gửi qua kênh `channel`. | Đảm bảo mọi kiểu dữ liệu truyền qua Channel đều phải thỏa mãn ràng buộc `Send`. |
| **E0507** | `cannot move out of a shared reference` | Cố gắng lấy quyền sở hữu của một trường bên trong thông điệp khi chỉ có tham chiếu mượn (borrow). | Triển khai phương thức nhận `self` theo giá trị thay vì tham chiếu `&self` khi chuyển quyền sở hữu thông điệp. |
| **E0599** | `no method named 'recv' found for struct 'Sender'` | Gọi nhầm phương thức `.recv()` trên đầu gửi `Sender` thay vì đầu nhận `Receiver`. | Kiểm tra lại biến: `Sender` chỉ có `.send()`, còn `Receiver` mới có `.recv()`. |

### Ví dụ phân tích lỗi `E0382` khi gửi thông điệp không nhân bản Sender:

```rust
use std::sync::mpsc::channel;

// Đoạn mã lỗi minh họa E0382:
fn vi_du_loi_e0382() {
    let (tx, _rx) = channel::<i32>();
    
    // Gửi giá trị và vô tình di chuyển tx
    // std::thread::spawn(move || { tx.send(10).unwrap(); });
    // std::thread::spawn(move || { tx.send(20).unwrap(); }); // LỖI E0382!
}

// Cách sửa chữa đúng chuẩn: Clone tx cho mỗi luồng
fn vi_du_dung_e0382() {
    let (tx, _rx) = channel::<i32>();
    
    let tx1 = tx.clone();
    std::thread::spawn(move || { tx1.send(10).unwrap(); });
    
    let tx2 = tx.clone();
    std::thread::spawn(move || { tx2.send(20).unwrap(); });
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Triết lý Actor**: Đóng gói trạng thái riêng tư và chỉ giao tiếp qua thông điệp, triệt tiêu hoàn toàn nguy cơ Deadlock và tranh chấp khóa Mutex.
2. **Kênh mpsc và oneshot**: `mpsc` dùng làm hòm thư đến nhiều người gửi, trong khi `oneshot` đóng vai trò là phong bì hồi âm kết quả cho mẫu thiết kế Request-Response.
3. **Xử lý tuần tự (Sequential Execution)**: Mỗi Actor xử lý lần lượt từng thông điệp, biến các bài toán cập nhật đồng thời phức tạp thành logic tuần tự đơn giản.
4. **An toàn Bộ nhớ Đa luồng**: Sự kết hợp giữa quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) bảo đảm thông điệp di chuyển an toàn giữa các luồng mà không bao giờ bị rò rỉ hay hỏng ô nhớ.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung tính năng Chuyển khoản liên Actor - Transfer Between Accounts)**:  
   Tạo hai Actor tài khoản `Account A` và `Account B`. Viết một thông điệp `Transfer { target_account: BankAccountHandle, amount: u64 }` cho phép Actor A tự động rút tiền của mình và nạp sang hòm thư của Actor B một cách an toàn.
2. **Bài tập 2 (Xây dựng Actor Giám sát - Supervisor Actor)**:  
   Theo triết lý Erlang OTP "Let It Crash", hãy lập trình một `SupervisorActor` liên tục theo dõi tiến trình của `BankAccountActor`. Nếu luồng của `BankAccountActor` gặp sự cố (bị panic), Supervisor sẽ lập tức phát hiện và tự động khởi tạo lại một Actor mới thay thế ngay lập tức.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Khi nào nên dùng Mutex thay vì Actor?)**:  
   Mô hình Actor cực kỳ mạnh mẽ, nhưng chi phí sao chép thông điệp qua kênh truyền (Channel Message Allocation) có thể là một điểm trừ. Trong trường hợp nào việc sử dụng một `std::sync::RwLock` đơn giản lại cho hiệu năng đọc tốt hơn mô hình Actor?
