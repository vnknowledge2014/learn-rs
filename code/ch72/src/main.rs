//! Chương 72 — Hợp đồng thông minh bằng Rust: mô hình CosmWasm và mô hình Solana.
//!
//! Hai hệ sinh thái lớn nhất viết hợp đồng bằng Rust, và chúng chọn hai triết lý
//! TRÁI NGƯỢC nhau về nơi cất trạng thái. Hiểu sự khác biệt đó quan trọng hơn
//! thuộc lòng API của bên nào.

use std::collections::BTreeMap;

// ============================================================================
// PHẦN I — MÔ HÌNH COSMWASM: hợp đồng SỞ HỮU kho của chính nó
// ============================================================================

pub type Address = String;
pub type Money = u128;

/// Ba thứ mà mọi hàm hợp đồng CosmWasm đều nhận. Tách bạch rõ ràng:
/// `env` là sự thật của chuỗi, `info` là "ai gọi và gửi kèm bao nhiêu tiền".
#[derive(Debug, Clone)]
pub struct Env {
    pub height: u64,
    pub timestamp: u64,
    pub contract_address: Address,
}

#[derive(Debug, Clone)]
pub struct MessageInfo {
    pub sender: Address,
    pub attached_funds: Money,
}

/// Kho khoá–giá trị riêng của MỖI hợp đồng. Hợp đồng khác không đọc được.
/// Đây chính là điểm khác biệt lớn nhất so với Solana.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Store {
    pub kv: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl Store {
    pub fn set<T: AsRef<[u8]>>(&mut self, key: T, value: &[u8]) {
        self.kv.insert(key.as_ref().to_vec(), value.to_vec());
    }
    pub fn get<T: AsRef<[u8]>>(&self, key: T) -> Option<&Vec<u8>> {
        self.kv.get(key.as_ref())
    }
    pub fn remove<T: AsRef<[u8]>>(&mut self, key: T) {
        self.kv.remove(key.as_ref());
    }

    // Trợ giúp cho số dư: khoá "balance:<địa chỉ>" → u128 dạng big-endian
    pub fn set_balance(&mut self, addr: &str, v: Money) {
        self.set(format!("balance:{addr}"), &v.to_be_bytes());
    }
    pub fn balance(&self, addr: &str) -> Money {
        self.get(format!("balance:{addr}"))
            .map(|b| u128::from_be_bytes(b[..16].try_into().unwrap()))
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContractError {
    InsufficientFunds { needed: Money, available: Money },
    Forbidden { addr: Address },
    BeforeDeadline { remaining: u64 },
    AlreadySettled,
    ZeroAmount,
    Overflow,
}

/// Sự kiện phát ra — cách hợp đồng "kể lại" việc mình đã làm cho thế giới bên ngoài.
#[derive(Debug, Clone, PartialEq)]
pub struct Event {
    pub kind: String,
    pub attributes: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Response {
    pub events: Vec<Event>,
    /// Thông điệp gửi tiếp cho hợp đồng/mô-đun khác. CosmWasm KHÔNG cho gọi
    /// đồng bộ sang hợp đồng khác — bạn trả về thông điệp và để chuỗi thực thi
    /// SAU KHI hàm của bạn kết thúc. Nhờ vậy tấn công tái nhập (reentrancy)
    /// bị chặn ở tầng KIẾN TRÚC, không phải bằng cờ khoá như Solidity.
    pub messages: Vec<String>,
}

impl Response {
    pub fn new() -> Self {
        Response::default()
    }
    pub fn event(mut self, kind: &str, attrs: &[(&str, &str)]) -> Self {
        self.events.push(Event {
            kind: kind.into(),
            attributes: attrs
                .iter()
                .map(|(a, b)| (a.to_string(), b.to_string()))
                .collect(),
        });
        self
    }
    pub fn add_message(mut self, msg: &str) -> Self {
        self.messages.push(msg.into());
        self
    }
}

// ---------------------------------------------------------------------------
// Token kiểu CW20
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TokenMsg {
    Transfer {
        recipient: Address,
        quantity: Money,
    },
    Burn {
        quantity: Money,
    },
    Approve {
        spender: Address,
        quantity: Money,
    },
    TransferFrom {
        owner: Address,
        recipient: Address,
        quantity: Money,
    },
}

pub struct TokenCw20;

impl TokenCw20 {
    pub fn create(store: &mut Store, owner: &str, total_supply: Money) -> Response {
        store.set_balance(owner, total_supply);
        store.set(b"total_supply", &total_supply.to_be_bytes());
        Response::new().event("instantiate", &[("owner", owner)])
    }

    pub fn total_supply(store: &Store) -> Money {
        store
            .get(b"total_supply")
            .map(|b| u128::from_be_bytes(b[..16].try_into().unwrap()))
            .unwrap_or(0)
    }

    fn key_allowance(owner: &str, spender: &str) -> String {
        format!("allowance:{owner}:{spender}")
    }

    pub fn allowance(store: &Store, owner: &str, spender: &str) -> Money {
        store
            .get(Self::key_allowance(owner, spender))
            .map(|b| u128::from_be_bytes(b[..16].try_into().unwrap()))
            .unwrap_or(0)
    }

    pub fn execute(
        store: &mut Store,
        _env: &Env,
        info: &MessageInfo,
        msg: TokenMsg,
    ) -> Result<Response, ContractError> {
        match msg {
            TokenMsg::Transfer {
                recipient,
                quantity,
            } => {
                Self::sub_balance(store, &info.sender, quantity)?;
                Self::add_balance(store, &recipient, quantity)?;
                Ok(Response::new().event(
                    "transfer",
                    &[
                        ("from", &info.sender),
                        ("to", &recipient),
                        ("amount", &quantity.to_string()),
                    ],
                ))
            }
            TokenMsg::Burn { quantity } => {
                Self::sub_balance(store, &info.sender, quantity)?;
                let new = Self::total_supply(store) - quantity;
                store.set(b"total_supply", &new.to_be_bytes());
                Ok(Response::new().event("burn", &[("amount", &quantity.to_string())]))
            }
            TokenMsg::Approve { spender, quantity } => {
                store.set(
                    Self::key_allowance(&info.sender, &spender),
                    &quantity.to_be_bytes(),
                );
                Ok(Response::new().event("approve", &[("spender", &spender)]))
            }
            TokenMsg::TransferFrom {
                owner,
                recipient,
                quantity,
            } => {
                let limit = Self::allowance(store, &owner, &info.sender);
                if limit < quantity {
                    return Err(ContractError::InsufficientFunds {
                        needed: quantity,
                        available: limit,
                    });
                }
                Self::sub_balance(store, &owner, quantity)?;
                Self::add_balance(store, &recipient, quantity)?;
                // Trừ hạn mức SAU KHI chuyển thành công — nếu trừ trước rồi
                // chuyển lỗi, hạn mức bị mất oan. (CosmWasm thật hoàn tác MỌI
                // thay đổi khi `execute` trả lỗi; kho mô phỏng ở đây thì không,
                // nên thứ tự "kiểm hết rồi mới ghi" càng quan trọng.)
                store.set(
                    Self::key_allowance(&owner, &info.sender),
                    &(limit - quantity).to_be_bytes(),
                );
                Ok(Response::new().event("transfer_from", &[("from", &owner), ("to", &recipient)]))
            }
        }
    }

    fn sub_balance(store: &mut Store, addr: &str, v: Money) -> Result<(), ContractError> {
        if v == 0 {
            return Err(ContractError::ZeroAmount);
        }
        let available = store.balance(addr);
        if available < v {
            return Err(ContractError::InsufficientFunds {
                needed: v,
                available,
            });
        }
        store.set_balance(addr, available - v);
        Ok(())
    }
    fn add_balance(store: &mut Store, addr: &str, v: Money) -> Result<(), ContractError> {
        let new = store
            .balance(addr)
            .checked_add(v)
            .ok_or(ContractError::Overflow)?;
        store.set_balance(addr, new);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Ký quỹ (escrow) — máy trạng thái trong hợp đồng
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EscrowState {
    Holding,
    Released,
    Refunded,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Escrow {
    pub buyer: Address,
    pub seller: Address,
    pub arbiter: Address,
    pub amount: Money,
    pub deadline: u64,
    pub state: EscrowState,
}

impl Escrow {
    pub fn create(
        info: &MessageInfo,
        seller: &str,
        arbiter: &str,
        deadline: u64,
    ) -> Result<Escrow, ContractError> {
        if info.attached_funds == 0 {
            return Err(ContractError::ZeroAmount);
        }
        Ok(Escrow {
            buyer: info.sender.clone(),
            seller: seller.into(),
            arbiter: arbiter.into(),
            amount: info.attached_funds,
            deadline,
            state: EscrowState::Holding,
        })
    }

    /// Người mua hoặc trọng tài có quyền giải ngân cho người bán.
    pub fn release(&mut self, info: &MessageInfo) -> Result<Response, ContractError> {
        if self.state != EscrowState::Holding {
            return Err(ContractError::AlreadySettled);
        }
        if info.sender != self.buyer && info.sender != self.arbiter {
            return Err(ContractError::Forbidden {
                addr: info.sender.clone(),
            });
        }
        self.state = EscrowState::Released;
        Ok(Response::new()
            .event("release", &[("to", &self.seller)])
            .add_message(&format!("send {} to {}", self.amount, self.seller)))
    }

    /// Hoàn tiền chỉ được phép SAU hạn chót — hoặc do trọng tài quyết định.
    pub fn refund(&mut self, env: &Env, info: &MessageInfo) -> Result<Response, ContractError> {
        if self.state != EscrowState::Holding {
            return Err(ContractError::AlreadySettled);
        }
        let is_arbiter = info.sender == self.arbiter;
        if !is_arbiter {
            if info.sender != self.buyer {
                return Err(ContractError::Forbidden {
                    addr: info.sender.clone(),
                });
            }
            if env.timestamp < self.deadline {
                return Err(ContractError::BeforeDeadline {
                    remaining: self.deadline - env.timestamp,
                });
            }
        }
        self.state = EscrowState::Refunded;
        Ok(Response::new()
            .event("refund", &[("to", &self.buyer)])
            .add_message(&format!("send {} to {}", self.amount, self.buyer)))
    }
}

// ============================================================================
// PHẦN II — MÔ HÌNH SOLANA: chương trình KHÔNG có trạng thái
// ============================================================================
// Solana lật ngược mọi thứ: chương trình chỉ là mã THUẦN TÚY, mọi dữ liệu nằm
// trong "tài khoản" do người gọi liệt kê SẴN trong giao dịch. Nhờ biết trước
// giao dịch sẽ chạm tài khoản nào, Solana chạy song song các giao dịch không
// đụng nhau — đó là nguồn gốc thông lượng của nó.

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub address: Address,
    /// Chương trình nào SỞ HỮU tài khoản này. Chỉ chủ sở hữu được ghi dữ liệu.
    pub owner: Address,
    pub lamports: u64,
    pub data: Vec<u8>,
    pub is_signer: bool,   // người gọi đã ký cho tài khoản này chưa
    pub is_writable: bool, // giao dịch có khai báo sẽ ghi vào đây không
    pub is_executable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SolanaError {
    MissingSignature(Address),
    WrongOwner {
        account: Address,
        expected: Address,
        actual: Address,
    },
    ReadOnlyAccount(Address),
    WrongPda {
        expected: Address,
        actual: Address,
    },
    InsufficientLamports {
        needed: u64,
        available: u64,
    },
    MissingAccount(usize),
}

/// Địa chỉ dẫn xuất từ chương trình (PDA). Không có khoá riêng — nên KHÔNG AI
/// ký được cho nó, kể cả kẻ tấn công. Chỉ chương trình dẫn xuất ra nó mới
/// "ký" thay được, qua cơ chế `invoke_signed`.
pub fn derive_pda(seeds: &[&[u8]], program_id: &str) -> Address {
    let mut h: u64 = 0xcbf29ce484222325;
    for part in seeds {
        for &b in *part {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= 0xFF; // dấu phân cách giữa các hạt giống
        h = h.wrapping_mul(0x100000001b3);
    }
    for &b in program_id.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("PDA{:016x}", h)
}

/// Bộ kiểm tra bắt buộc trước MỌI thao tác. Bỏ sót một dòng ở đây là nguyên
/// nhân của hầu hết các vụ mất tiền trên Solana.
pub struct CheckAccount;

impl CheckAccount {
    pub fn must_be_signer(account: &Account) -> Result<(), SolanaError> {
        if account.is_signer {
            Ok(())
        } else {
            Err(SolanaError::MissingSignature(account.address.clone()))
        }
    }
    pub fn must_be_owned_by(account: &Account, program_id: &str) -> Result<(), SolanaError> {
        if account.owner == program_id {
            Ok(())
        } else {
            Err(SolanaError::WrongOwner {
                account: account.address.clone(),
                expected: program_id.into(),
                actual: account.owner.clone(),
            })
        }
    }
    pub fn must_be_writable(account: &Account) -> Result<(), SolanaError> {
        if account.is_writable {
            Ok(())
        } else {
            Err(SolanaError::ReadOnlyAccount(account.address.clone()))
        }
    }
    pub fn must_be_valid_pda(
        account: &Account,
        seeds: &[&[u8]],
        program_id: &str,
    ) -> Result<(), SolanaError> {
        let expected = derive_pda(seeds, program_id);
        if account.address == expected {
            Ok(())
        } else {
            Err(SolanaError::WrongPda {
                expected,
                actual: account.address.clone(),
            })
        }
    }
}

/// Chương trình đếm — ví dụ nhỏ nhất thể hiện đủ mô hình tài khoản Solana.
pub struct CounterProgram;
pub const COUNTER_PROGRAM_ID: &str = "Counter1111111111111111111111111";

impl CounterProgram {
    /// Tài khoản đếm là một PDA dẫn xuất từ địa chỉ chủ sở hữu — nên mỗi người
    /// dùng có đúng một bộ đếm, và địa chỉ của nó tính ra được mà không cần tra sổ.
    pub fn counter_address(owner: &str) -> Address {
        derive_pda(&[b"counter", owner.as_bytes()], COUNTER_PROGRAM_ID)
    }

    pub fn increment(account: &mut [Account]) -> Result<u64, SolanaError> {
        // Thứ tự tài khoản là MỘT PHẦN CỦA GIAO DIỆN. Sai thứ tự = sai hợp đồng.
        let owner = account
            .first()
            .ok_or(SolanaError::MissingAccount(0))?
            .clone();
        let counter = account.get_mut(1).ok_or(SolanaError::MissingAccount(1))?;

        CheckAccount::must_be_signer(&owner)?;
        CheckAccount::must_be_writable(counter)?;
        CheckAccount::must_be_owned_by(counter, COUNTER_PROGRAM_ID)?;
        // KIỂM TRA SỐNG CÒN: bộ đếm này có đúng là của người ký không?
        // Thiếu dòng này, ai cũng tăng được bộ đếm của người khác.
        CheckAccount::must_be_valid_pda(
            counter,
            &[b"counter", owner.address.as_bytes()],
            COUNTER_PROGRAM_ID,
        )?;

        let current = u64::from_le_bytes(counter.data[..8].try_into().unwrap());
        let new = current + 1;
        counter.data[..8].copy_from_slice(&new.to_le_bytes());
        Ok(new)
    }

    /// Gọi chéo chương trình (CPI): chuyển lamports qua "chương trình hệ thống".
    pub fn transfer_lamports(
        owner: &mut Account,
        recipient: &mut Account,
        v: u64,
    ) -> Result<(), SolanaError> {
        CheckAccount::must_be_writable(owner)?;
        CheckAccount::must_be_writable(recipient)?;
        if owner.lamports < v {
            return Err(SolanaError::InsufficientLamports {
                needed: v,
                available: owner.lamports,
            });
        }
        owner.lamports -= v;
        recipient.lamports += v;
        Ok(())
    }
}

// ============================================================================
// PHẦN III — SO SÁNH HAI MÔ HÌNH BẰNG CON SỐ
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct ParallelAnalysis {
    pub num_transactions: usize,
    pub batch_count: usize,
    pub batches: Vec<Vec<usize>>,
}

/// Vì Solana bắt khai báo trước tài khoản sẽ ghi, ta xếp lịch được các giao
/// dịch KHÔNG đụng nhau vào cùng một lô chạy song song. CosmWasm/EVM không
/// biết trước nên phải chạy tuần tự tuyệt đối.
///
/// Rút gọn: mỗi giao dịch chỉ liệt kê tài khoản nó GHI. (Bộ lập lịch thật còn
/// cho các giao dịch cùng ĐỌC một tài khoản chạy chung lô.)
pub fn schedule_parallel(transactions: &[Vec<Address>]) -> ParallelAnalysis {
    let mut batches: Vec<Vec<usize>> = Vec::new();
    let mut scheduled = vec![false; transactions.len()];
    let mut remaining = transactions.len();

    while remaining > 0 {
        let mut batch = Vec::new();
        let mut touched: Vec<&Address> = Vec::new();
        for (i, account) in transactions.iter().enumerate() {
            if scheduled[i] {
                continue;
            }
            if account.iter().any(|a| touched.contains(&a)) {
                continue;
            } // xung đột
            batch.push(i);
            touched.extend(account.iter());
            scheduled[i] = true;
            remaining -= 1;
        }
        batches.push(batch);
    }
    ParallelAnalysis {
        num_transactions: transactions.len(),
        batch_count: batches.len(),
        batches,
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   HỢP ĐỒNG THÔNG MINH: COSMWASM vs SOLANA                 ");
    println!("═══════════════════════════════════════════════════════════");

    let env = Env {
        height: 100,
        timestamp: 1000,
        contract_address: "hd1".into(),
    };

    println!("\n1. TOKEN CW20 — hợp đồng sở hữu kho của chính nó");
    let mut store = Store::default();
    TokenCw20::create(&mut store, "An", 1_000_000);
    println!(
        "   Tổng cung {} · số dư An = {}",
        TokenCw20::total_supply(&store),
        store.balance("An")
    );

    let info_an = MessageInfo {
        sender: "An".into(),
        attached_funds: 0,
    };
    let r = TokenCw20::execute(
        &mut store,
        &env,
        &info_an,
        TokenMsg::Transfer {
            recipient: "Binh".into(),
            quantity: 250_000,
        },
    )
    .unwrap();
    println!("   Chuyển 250k cho Bình → sự kiện {:?}", r.events[0].kind);
    println!(
        "   An = {} · Bình = {}",
        store.balance("An"),
        store.balance("Binh")
    );

    let e = TokenCw20::execute(
        &mut store,
        &env,
        &info_an,
        TokenMsg::Transfer {
            recipient: "Cuong".into(),
            quantity: 9_999_999,
        },
    )
    .unwrap_err();
    println!("   Chuyển quá số dư → {:?}", e);

    println!("\n2. UỶ QUYỀN (approve / transferFrom)");
    TokenCw20::execute(
        &mut store,
        &env,
        &info_an,
        TokenMsg::Approve {
            spender: "San".into(),
            quantity: 100_000,
        },
    )
    .unwrap();
    let info_san = MessageInfo {
        sender: "San".into(),
        attached_funds: 0,
    };
    TokenCw20::execute(
        &mut store,
        &env,
        &info_san,
        TokenMsg::TransferFrom {
            owner: "An".into(),
            recipient: "Dung".into(),
            quantity: 60_000,
        },
    )
    .unwrap();
    println!(
        "   Sàn dùng 60k trong hạn mức 100k → hạn mức còn {}",
        TokenCw20::allowance(&store, "An", "San")
    );

    println!("\n3. KÝ QUỸ — máy trạng thái + kiểm soát quyền");
    let info_buyer = MessageInfo {
        sender: "Buyer".into(),
        attached_funds: 500,
    };
    let mut escrow = Escrow::create(&info_buyer, "Seller", "Arbiter", 2000).unwrap();
    let early = Env {
        timestamp: 1500,
        ..env.clone()
    };
    println!(
        "   Người mua đòi hoàn tiền trước hạn → {:?}",
        escrow.clone().refund(&early, &info_buyer).unwrap_err()
    );
    let stranger = MessageInfo {
        sender: "Stranger".into(),
        attached_funds: 0,
    };
    println!(
        "   Người lạ đòi giải ngân            → {:?}",
        escrow.clone().release(&stranger).unwrap_err()
    );
    let r = escrow.release(&info_buyer).unwrap();
    println!(
        "   Người mua giải ngân → {:?} · thông điệp tiếp: {:?}",
        escrow.state, r.messages
    );
    println!(
        "   Giải ngân lần hai                 → {:?}",
        escrow.release(&info_buyer).unwrap_err()
    );

    println!("\n4. MÔ HÌNH SOLANA — PDA và kiểm tra tài khoản");
    let owner = "An11111111111111111111111111111111";
    let pda = CounterProgram::counter_address(owner);
    println!("   PDA bộ đếm của An = {}", pda);
    println!(
        "   Tính lại lần nữa   = {} (tất định)",
        CounterProgram::counter_address(owner)
    );
    println!(
        "   Của người khác     = {}",
        CounterProgram::counter_address("Binh2222222222222222222222222222")
    );

    let mut account = vec![
        Account {
            address: owner.into(),
            owner: "system".into(),
            lamports: 10_000,
            data: vec![],
            is_signer: true,
            is_writable: false,
            is_executable: false,
        },
        Account {
            address: pda.clone(),
            owner: COUNTER_PROGRAM_ID.into(),
            lamports: 1_000,
            data: vec![0u8; 8],
            is_signer: false,
            is_writable: true,
            is_executable: false,
        },
    ];
    for _ in 0..3 {
        CounterProgram::increment(&mut account).unwrap();
    }
    println!(
        "   Tăng 3 lần → bộ đếm = {}",
        u64::from_le_bytes(account[1].data[..8].try_into().unwrap())
    );

    // Kẻ tấn công đưa PDA của người khác vào
    let mut forged = account.clone();
    forged[1].address = CounterProgram::counter_address("Binh2222222222222222222222222222");
    println!(
        "   Dùng bộ đếm của người khác → {:?}",
        CounterProgram::increment(&mut forged).unwrap_err()
    );
    let mut unsigned = account.clone();
    unsigned[0].is_signer = false;
    println!(
        "   Không ký                   → {:?}",
        CounterProgram::increment(&mut unsigned).unwrap_err()
    );

    println!("\n5. VÌ SAO SOLANA CHẠY SONG SONG ĐƯỢC");
    let txs: Vec<Vec<Address>> = vec![
        vec!["A".into(), "B".into()],
        vec!["C".into(), "D".into()], // không đụng txs 0 → song song được
        vec!["B".into(), "E".into()], // đụng "B" → phải chờ
        vec!["F".into(), "G".into()],
        vec!["A".into(), "F".into()], // đụng cả A lẫn F
    ];
    let plan = schedule_parallel(&txs);
    println!(
        "   {} giao dịch → {} lô: {:?}",
        plan.num_transactions, plan.batch_count, plan.batches
    );
    println!("   CosmWasm/EVM sẽ cần {} bước tuần tự.", txs.len());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   COSMWASM: TRẠNG THÁI THUỘC HỢP ĐỒNG                      ");
    println!("   SOLANA  : TRẠNG THÁI THUỘC TÀI KHOẢN, KHAI BÁO TRƯỚC     ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_env() -> Env {
        Env {
            height: 100,
            timestamp: 1000,
            contract_address: "hd".into(),
        }
    }
    fn sender(addr: &str) -> MessageInfo {
        MessageInfo {
            sender: addr.into(),
            attached_funds: 0,
        }
    }
    fn sample_token() -> Store {
        let mut store = Store::default();
        TokenCw20::create(&mut store, "An", 1_000);
        store
    }

    // ---------- Kho ----------
    #[test]
    fn store_read_write_delete_are_correct() {
        let mut store = Store::default();
        store.set(b"a", b"1");
        assert_eq!(store.get(b"a"), Some(&b"1".to_vec()));
        store.remove(b"a");
        assert_eq!(store.get(b"a"), None);
    }

    #[test]
    fn unset_balance_is_zero_not_an_error() {
        let store = Store::default();
        assert_eq!(
            store.balance("nobody"),
            0,
            "mặc định 0 giúp không cần khởi tạo trước"
        );
    }

    // ---------- Token CW20 ----------
    #[test]
    fn transfer_conserves_total_supply() {
        let mut store = sample_token();
        let prev: Money = ["An", "Binh", "Cuong"]
            .iter()
            .map(|a| store.balance(a))
            .sum();
        TokenCw20::execute(
            &mut store,
            &sample_env(),
            &sender("An"),
            TokenMsg::Transfer {
                recipient: "Binh".into(),
                quantity: 300,
            },
        )
        .unwrap();
        let next: Money = ["An", "Binh", "Cuong"]
            .iter()
            .map(|a| store.balance(a))
            .sum();
        assert_eq!(prev, next, "chuyển khoản không được sinh hay huỷ token");
        assert_eq!(store.balance("An"), 700);
        assert_eq!(store.balance("Binh"), 300);
    }

    #[test]
    fn cannot_transfer_beyond_balance() {
        let mut store = sample_token();
        let e = TokenCw20::execute(
            &mut store,
            &sample_env(),
            &sender("An"),
            TokenMsg::Transfer {
                recipient: "Binh".into(),
                quantity: 1_001,
            },
        )
        .unwrap_err();
        assert_eq!(
            e,
            ContractError::InsufficientFunds {
                needed: 1_001,
                available: 1_000
            }
        );
        assert_eq!(
            store.balance("An"),
            1_000,
            "thất bại phải KHÔNG để lại thay đổi nào"
        );
        assert_eq!(store.balance("Binh"), 0);
    }

    #[test]
    fn cannot_transfer_from_empty_account() {
        let mut store = sample_token();
        assert!(
            TokenCw20::execute(
                &mut store,
                &sample_env(),
                &sender("Stranger"),
                TokenMsg::Transfer {
                    recipient: "Stranger2".into(),
                    quantity: 1
                }
            )
            .is_err()
        );
    }

    #[test]
    fn zero_amount_transfer_is_rejected() {
        let mut store = sample_token();
        assert_eq!(
            TokenCw20::execute(
                &mut store,
                &sample_env(),
                &sender("An"),
                TokenMsg::Transfer {
                    recipient: "Binh".into(),
                    quantity: 0
                }
            )
            .unwrap_err(),
            ContractError::ZeroAmount
        );
    }

    #[test]
    fn burning_reduces_both_balance_and_total_supply() {
        let mut store = sample_token();
        TokenCw20::execute(
            &mut store,
            &sample_env(),
            &sender("An"),
            TokenMsg::Burn { quantity: 400 },
        )
        .unwrap();
        assert_eq!(store.balance("An"), 600);
        assert_eq!(
            TokenCw20::total_supply(&store),
            600,
            "đốt phải giảm tổng cung, không chỉ số dư"
        );
    }

    #[test]
    fn allowance_caps_at_its_limit() {
        let mut store = sample_token();
        TokenCw20::execute(
            &mut store,
            &sample_env(),
            &sender("An"),
            TokenMsg::Approve {
                spender: "San".into(),
                quantity: 500,
            },
        )
        .unwrap();
        TokenCw20::execute(
            &mut store,
            &sample_env(),
            &sender("San"),
            TokenMsg::TransferFrom {
                owner: "An".into(),
                recipient: "Binh".into(),
                quantity: 300,
            },
        )
        .unwrap();
        assert_eq!(TokenCw20::allowance(&store, "An", "San"), 200);
        let e = TokenCw20::execute(
            &mut store,
            &sample_env(),
            &sender("San"),
            TokenMsg::TransferFrom {
                owner: "An".into(),
                recipient: "Binh".into(),
                quantity: 300,
            },
        )
        .unwrap_err();
        assert_eq!(
            e,
            ContractError::InsufficientFunds {
                needed: 300,
                available: 200
            },
            "vượt hạn mức phải bị chặn"
        );
    }

    #[test]
    fn no_allowance_means_no_transfer_from() {
        let mut store = sample_token();
        assert!(
            TokenCw20::execute(
                &mut store,
                &sample_env(),
                &sender("Thief"),
                TokenMsg::TransferFrom {
                    owner: "An".into(),
                    recipient: "Thief".into(),
                    quantity: 1
                }
            )
            .is_err()
        );
        assert_eq!(store.balance("An"), 1_000);
    }

    #[test]
    fn failed_transfer_does_not_consume_allowance() {
        let mut store = sample_token();
        TokenCw20::execute(
            &mut store,
            &sample_env(),
            &sender("An"),
            TokenMsg::Approve {
                spender: "San".into(),
                quantity: 5_000,
            },
        )
        .unwrap();
        // hạn mức 5000 nhưng An chỉ có 1000 → chuyển hỏng
        assert!(
            TokenCw20::execute(
                &mut store,
                &sample_env(),
                &sender("San"),
                TokenMsg::TransferFrom {
                    owner: "An".into(),
                    recipient: "B".into(),
                    quantity: 2_000
                }
            )
            .is_err()
        );
        assert_eq!(
            TokenCw20::allowance(&store, "An", "San"),
            5_000,
            "hỏng thì hạn mức phải nguyên vẹn, không mất oan"
        );
    }

    // ---------- Ký quỹ ----------
    #[test]
    fn escrow_rejects_zero_deposit() {
        let i = MessageInfo {
            sender: "M".into(),
            attached_funds: 0,
        };
        assert_eq!(
            Escrow::create(&i, "B", "T", 100).unwrap_err(),
            ContractError::ZeroAmount
        );
    }

    #[test]
    fn buyer_can_release() {
        let i = MessageInfo {
            sender: "M".into(),
            attached_funds: 100,
        };
        let mut escrow = Escrow::create(&i, "B", "T", 2000).unwrap();
        let r = escrow.release(&sender("M")).unwrap();
        assert_eq!(escrow.state, EscrowState::Released);
        assert_eq!(r.messages.len(), 1, "phải phát thông điệp chuyển tiền");
    }

    #[test]
    fn arbiter_can_release() {
        let i = MessageInfo {
            sender: "M".into(),
            attached_funds: 100,
        };
        let mut escrow = Escrow::create(&i, "B", "T", 2000).unwrap();
        assert!(escrow.release(&sender("T")).is_ok());
    }

    #[test]
    fn stranger_can_do_nothing() {
        let i = MessageInfo {
            sender: "M".into(),
            attached_funds: 100,
        };
        let mut escrow = Escrow::create(&i, "B", "T", 2000).unwrap();
        assert_eq!(
            escrow.release(&sender("Stranger")).unwrap_err(),
            ContractError::Forbidden {
                addr: "Stranger".into()
            }
        );
        assert_eq!(
            escrow.state,
            EscrowState::Holding,
            "trạng thái không được đổi"
        );
    }

    #[test]
    fn seller_cannot_release_to_itself() {
        // Lỗi thiết kế kinh điển: quên loại người bán ra khỏi danh sách được phép.
        let i = MessageInfo {
            sender: "M".into(),
            attached_funds: 100,
        };
        let mut escrow = Escrow::create(&i, "B", "T", 2000).unwrap();
        assert!(
            escrow.release(&sender("B")).is_err(),
            "người bán KHÔNG được tự lấy tiền"
        );
    }

    #[test]
    fn refund_blocked_before_deadline_allowed_after() {
        let i = MessageInfo {
            sender: "M".into(),
            attached_funds: 100,
        };
        let mut escrow = Escrow::create(&i, "B", "T", 2000).unwrap();
        let early = Env {
            timestamp: 1500,
            ..sample_env()
        };
        assert_eq!(
            escrow.refund(&early, &sender("M")).unwrap_err(),
            ContractError::BeforeDeadline { remaining: 500 }
        );
        let late = Env {
            timestamp: 2500,
            ..sample_env()
        };
        assert!(escrow.refund(&late, &sender("M")).is_ok());
        assert_eq!(escrow.state, EscrowState::Refunded);
    }

    #[test]
    fn arbiter_can_refund_regardless_of_deadline() {
        let i = MessageInfo {
            sender: "M".into(),
            attached_funds: 100,
        };
        let mut escrow = Escrow::create(&i, "B", "T", 9_999_999).unwrap();
        assert!(escrow.refund(&sample_env(), &sender("T")).is_ok());
    }

    #[test]
    fn cannot_release_twice() {
        // Đây là biến thể "rút hai lần" — lỗi tốn tiền phổ biến nhất.
        let i = MessageInfo {
            sender: "M".into(),
            attached_funds: 100,
        };
        let mut escrow = Escrow::create(&i, "B", "T", 2000).unwrap();
        assert!(escrow.release(&sender("M")).is_ok());
        assert_eq!(
            escrow.release(&sender("M")).unwrap_err(),
            ContractError::AlreadySettled
        );
        assert_eq!(
            escrow.refund(&sample_env(), &sender("T")).unwrap_err(),
            ContractError::AlreadySettled,
            "đã giải ngân thì cũng không hoàn tiền được nữa"
        );
    }

    // ---------- Solana ----------
    #[test]
    fn pda_is_deterministic_and_seed_specific() {
        let a = derive_pda(&[b"counter", b"An"], COUNTER_PROGRAM_ID);
        assert_eq!(
            a,
            derive_pda(&[b"counter", b"An"], COUNTER_PROGRAM_ID),
            "phải tất định"
        );
        assert_ne!(a, derive_pda(&[b"counter", b"Binh"], COUNTER_PROGRAM_ID));
        assert_ne!(a, derive_pda(&[b"vault", b"An"], COUNTER_PROGRAM_ID));
        assert_ne!(a, derive_pda(&[b"counter", b"An"], "OtherProgram"));
    }

    #[test]
    fn pda_distinguishes_seed_boundaries() {
        // Không có dấu phân cách, ["ab","c"] và ["a","bc"] sẽ ra cùng địa chỉ —
        // lỗ hổng thật, cho phép kẻ tấn công tạo PDA trùng của người khác.
        assert_ne!(
            derive_pda(&[b"ab", b"c"], COUNTER_PROGRAM_ID),
            derive_pda(&[b"a", b"bc"], COUNTER_PROGRAM_ID)
        );
    }

    fn account_set(owner: &str) -> Vec<Account> {
        vec![
            Account {
                address: owner.into(),
                owner: "system".into(),
                lamports: 100,
                data: vec![],
                is_signer: true,
                is_writable: false,
                is_executable: false,
            },
            Account {
                address: CounterProgram::counter_address(owner),
                owner: COUNTER_PROGRAM_ID.into(),
                lamports: 100,
                data: vec![0u8; 8],
                is_signer: false,
                is_writable: true,
                is_executable: false,
            },
        ]
    }

    #[test]
    fn increment_succeeds_when_all_checks_pass() {
        let mut account = account_set("An");
        assert_eq!(CounterProgram::increment(&mut account), Ok(1));
        assert_eq!(CounterProgram::increment(&mut account), Ok(2));
        assert_eq!(
            u64::from_le_bytes(account[1].data[..8].try_into().unwrap()),
            2
        );
    }

    #[test]
    fn rejects_missing_signature() {
        let mut account = account_set("An");
        account[0].is_signer = false;
        assert_eq!(
            CounterProgram::increment(&mut account).unwrap_err(),
            SolanaError::MissingSignature("An".into())
        );
    }

    #[test]
    fn rejects_account_not_declared_writable() {
        let mut account = account_set("An");
        account[1].is_writable = false;
        assert!(matches!(
            CounterProgram::increment(&mut account).unwrap_err(),
            SolanaError::ReadOnlyAccount(_)
        ));
    }

    #[test]
    fn rejects_account_owned_by_another_program() {
        let mut account = account_set("An");
        account[1].owner = "FakeProgram".into();
        assert!(matches!(
            CounterProgram::increment(&mut account).unwrap_err(),
            SolanaError::WrongOwner { .. }
        ));
    }

    #[test]
    fn rejects_using_someone_elses_counter() {
        // ĐÂY LÀ BÀI KIỂM THỬ QUAN TRỌNG NHẤT phần Solana. Thiếu kiểm tra PDA,
        // bất kỳ ai cũng tăng/sửa được tài khoản của người khác — miễn là tài
        // khoản đó do đúng chương trình sở hữu.
        let mut account = account_set("An");
        account[1].address = CounterProgram::counter_address("Binh");
        assert!(matches!(
            CounterProgram::increment(&mut account).unwrap_err(),
            SolanaError::WrongPda { .. }
        ));
    }

    #[test]
    fn rejects_missing_account_in_tx() {
        let mut account = account_set("An");
        account.pop();
        assert_eq!(
            CounterProgram::increment(&mut account).unwrap_err(),
            SolanaError::MissingAccount(1)
        );
        let mut empty: Vec<Account> = vec![];
        assert_eq!(
            CounterProgram::increment(&mut empty).unwrap_err(),
            SolanaError::MissingAccount(0)
        );
    }

    #[test]
    fn lamport_transfer_conserves_total() {
        let mut account = account_set("An");
        account[0].is_writable = true;
        let prev_total = account[0].lamports + account[1].lamports;
        let (a, b) = account.split_at_mut(1);
        CounterProgram::transfer_lamports(&mut a[0], &mut b[0], 30).unwrap();
        assert_eq!(account[0].lamports + account[1].lamports, prev_total);
        assert_eq!(account[0].lamports, 70);
    }

    #[test]
    fn lamport_transfer_beyond_balance_is_blocked() {
        let mut account = account_set("An");
        account[0].is_writable = true;
        let (a, b) = account.split_at_mut(1);
        assert_eq!(
            CounterProgram::transfer_lamports(&mut a[0], &mut b[0], 999).unwrap_err(),
            SolanaError::InsufficientLamports {
                needed: 999,
                available: 100
            }
        );
        assert_eq!(account[0].lamports, 100, "thất bại không được đổi số dư");
    }

    // ---------- Song song hoá ----------
    #[test]
    fn disjoint_txs_run_in_the_same_batch() {
        let txs: Vec<Vec<Address>> = vec![
            vec!["A".into(), "B".into()],
            vec!["C".into(), "D".into()],
            vec!["E".into(), "F".into()],
        ];
        let plan = schedule_parallel(&txs);
        assert_eq!(
            plan.batch_count, 1,
            "hoàn toàn rời nhau → chạy hết trong 1 lô"
        );
    }

    #[test]
    fn conflicting_txs_go_to_separate_batches() {
        let txs: Vec<Vec<Address>> = vec![vec!["A".into()], vec!["A".into()], vec!["A".into()]];
        let plan = schedule_parallel(&txs);
        assert_eq!(plan.batch_count, 3, "cùng chạm A → buộc tuần tự hoàn toàn");
    }

    #[test]
    fn every_tx_is_scheduled_exactly_once() {
        let txs: Vec<Vec<Address>> = vec![
            vec!["A".into(), "B".into()],
            vec!["C".into(), "D".into()],
            vec!["B".into(), "E".into()],
            vec!["F".into(), "G".into()],
            vec!["A".into(), "F".into()],
        ];
        let plan = schedule_parallel(&txs);
        let mut all: Vec<usize> = plan.batches.iter().flatten().copied().collect();
        all.sort_unstable();
        assert_eq!(
            all,
            (0..txs.len()).collect::<Vec<_>>(),
            "không bỏ sót, không xếp trùng"
        );
        assert!(
            plan.batch_count < txs.len(),
            "phải tiết kiệm được so với tuần tự"
        );
    }

    #[test]
    fn no_two_txs_in_a_batch_conflict() {
        let txs: Vec<Vec<Address>> = (0..20)
            .map(|i| vec![format!("tk{}", i % 7), format!("tk{}", (i * 3) % 11)])
            .collect();
        let plan = schedule_parallel(&txs);
        for batches in &plan.batches {
            for (x, &i) in batches.iter().enumerate() {
                for &j in &batches[x + 1..] {
                    assert!(
                        txs[i].iter().all(|a| !txs[j].contains(a)),
                        "txs {} và {} cùng lô mà lại đụng tài khoản",
                        i,
                        j
                    );
                }
            }
        }
    }
}
