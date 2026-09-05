#![allow(dead_code, unused_variables, unused_imports)]
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

    println!("\n[2] Mo phong 3 luong customer hang dong thoi rut tien (Concurrent Clients):");
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
