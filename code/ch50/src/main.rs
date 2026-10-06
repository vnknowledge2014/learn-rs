use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;

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
    GetBalance { respond_to: Sender<u64> },
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
                        let _ =
                            respond_to.send(Err("Số dư tài khoản không đủ để thực hiện giao dịch"));
                    }
                }
                AccountMessage::GetBalance { respond_to } => {
                    println!(
                        "    [Actor] Vấn tin số dư: Đang gửi kết quả {}đ về phong bì hồi âm...",
                        self.balance
                    );
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

// ----------------------------------------------------------------------------
// MINH HỌA: ACTOR VẪN CÓ THỂ DEADLOCK
// Mỗi actor, để trả lời `Ask`, lại gửi `Ask` sang actor kia rồi CHẶN chờ trả lời.
// X đang chặn chờ Y nên không đọc hòm thư; Y gửi `Ask` cho X rồi chặn chờ X ->
// vòng chờ khép kín, y hệt deadlock khóa A-B, chỉ là "khóa" ở đây là `recv()`.
// ----------------------------------------------------------------------------
pub enum PeerMessage {
    SetPeer(Sender<PeerMessage>),
    Ask { reply: Sender<u32> },
}

fn peer_actor(inbox: Receiver<PeerMessage>) {
    let mut peer: Option<Sender<PeerMessage>> = None;
    while let Ok(msg) = inbox.recv() {
        match msg {
            PeerMessage::SetPeer(p) => peer = Some(p),
            PeerMessage::Ask { reply } => {
                let (tx, rx) = channel();
                if let Some(p) = &peer {
                    let _ = p.send(PeerMessage::Ask { reply: tx });
                }
                // CHẶN chờ actor kia — mầm mống deadlock
                if let Ok(v) = rx.recv() {
                    let _ = reply.send(v + 1);
                }
            }
        }
    }
}

/// Trả về `true` nếu yêu cầu không được trả lời trong `timeout` (tức là đã deadlock).
/// Hai luồng actor bị kẹt vĩnh viễn; trong mã thật, cách chữa là không chặn chờ
/// bên trong actor (gửi tiếp kèm phong bì hồi âm của người hỏi gốc), hoặc dùng
/// timeout cho mọi yêu cầu liên actor.
pub fn circular_request_deadlocks(timeout: Duration) -> bool {
    let (tx_x, rx_x) = channel();
    let (tx_y, rx_y) = channel();
    thread::spawn(move || peer_actor(rx_x));
    thread::spawn(move || peer_actor(rx_y));
    let _ = tx_x.send(PeerMessage::SetPeer(tx_y.clone()));
    let _ = tx_y.send(PeerMessage::SetPeer(tx_x.clone()));

    let (reply_tx, reply_rx) = channel();
    let _ = tx_x.send(PeerMessage::Ask { reply: reply_tx });
    reply_rx.recv_timeout(timeout).is_err()
}

fn main() {
    println!("==================================================================");
    println!("   MÔ HÌNH ACTOR & GIAO TIẾP KÊNH ĐỒNG THỜI AN TOÀN TRONG RUST    ");
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

    println!("\n[1] Thực hiện các giao dịch nạp tiền ban đầu:");
    handle.deposit(100_000);
    handle.deposit(250_000);

    // Kiểm tra số dư qua Request-Response
    let current_bal = handle.get_balance();
    println!("    [Client Main] Số dư kiểm tra được: {}đ", current_bal);
    assert_eq!(current_bal, 350_000);

    println!("\n[2] Mô phỏng 3 luồng khách hàng đồng thời rút tiền (Concurrent Clients):");
    let mut client_threads = Vec::new();

    for client_id in 1..=3 {
        let client_handle = handle.clone();
        let t = thread::spawn(move || {
            let withdraw_amount = 150_000;
            println!(
                "    - Khách hàng #{} bắt đầu gửi lệnh rút {}đ...",
                client_id, withdraw_amount
            );
            match client_handle.withdraw(withdraw_amount) {
                Ok(remaining) => println!(
                    "      + Khách hàng #{} rút THÀNH CÔNG! Số dư còn: {}đ",
                    client_id, remaining
                ),
                Err(err) => println!("      + Khách hàng #{} rút THẤT BẠI: {}", client_id, err),
            }
        });
        client_threads.push(t);
    }

    for t in client_threads {
        let _ = t.join();
    }

    // Kiểm tra số dư cuối cùng
    let final_balance = handle.get_balance();
    println!(
        "\n[3] Số dư cuối cùng trong sổ cái Actor: {}đ",
        final_balance
    );
    assert_eq!(final_balance, 50_000);

    // Tiêu hủy handle để đóng mailbox, luồng Actor sẽ kết thúc êm ái
    drop(handle);
    let _ = actor_thread.join();

    // 4. Actor KHÔNG miễn nhiễm deadlock: yêu cầu-phản hồi đồng bộ vòng tròn
    println!("\n[4] Hai actor hỏi nhau đồng bộ theo vòng tròn (X -> Y -> X):");
    if circular_request_deadlocks(Duration::from_millis(300)) {
        println!("    [!] Không có phản hồi sau 300ms: X chờ Y, Y chờ X -> DEADLOCK!");
    }

    println!("\n==================================================================");
    println!("   XÁC NHẬN: GIAO DỊCH NHẤT QUÁN, KHÔNG CẦN MUTEX TRONG MÃ NGHIỆP VỤ ");
    println!("==================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_account() -> BankAccountHandle {
        let (tx, rx) = channel();
        thread::spawn(move || BankAccountActor::new(rx).run());
        BankAccountHandle::new(tx)
    }

    #[test]
    fn concurrent_withdrawals_never_overdraw() {
        let handle = spawn_account();
        handle.deposit(1_000);
        let threads: Vec<_> = (0..20)
            .map(|_| {
                let h = handle.clone();
                thread::spawn(move || h.withdraw(100).is_ok())
            })
            .collect();
        let ok = threads
            .into_iter()
            .map(|t| t.join().unwrap())
            .filter(|&ok| ok)
            .count();
        assert_eq!(ok, 10);
        assert_eq!(handle.get_balance(), 0);
    }

    #[test]
    fn circular_synchronous_requests_deadlock() {
        assert!(circular_request_deadlocks(Duration::from_millis(200)));
    }
}
