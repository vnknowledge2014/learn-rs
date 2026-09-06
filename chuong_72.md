# Chương 72: Hợp đồng thông minh với Rust — CosmWasm & Solana

## Giới thiệu & Mục tiêu học tập

Rust là ngôn ngữ chính của hai hệ sinh thái hợp đồng thông minh lớn: **CosmWasm** (Cosmos, biên dịch sang WebAssembly) và **Solana** (biên dịch sang SBF). Chúng chia sẻ cùng một ngôn ngữ nhưng **mô hình dữ liệu hoàn toàn khác nhau**, và hiểu sai điều đó là nguồn gốc của phần lớn lỗ hổng.

| | CosmWasm | Solana |
|---|---|---|
| Trạng thái ở đâu | Bên trong hợp đồng | Trong **tài khoản** riêng biệt |
| Ai sở hữu trạng thái | Hợp đồng | Chương trình sở hữu tài khoản |
| Song song hoá | Tuần tự | Song song nếu không đụng tài khoản |
| Điểm vào | `instantiate` / `execute` / `query` | Một `process_instruction` |
| Rủi ro đặc thù | Thiếu kiểm quyền trong `execute` | Thiếu kiểm `signer` / `owner` |

Mục tiêu: hiểu cả hai mô hình đủ sâu để **đọc được lỗ hổng**, chứ không chỉ viết được hợp đồng chạy.

> **Lưu ý về mã nguồn.** Crate `ch72` chỉ chứa **lõi thuần tuý** — không phụ thuộc `cosmwasm-std` hay `solana-program`, để `cargo test --workspace` chạy được offline. Các kiểu `Deps`, `Env`, `MessageInfo`, `AccountInfo` được mô phỏng đúng ngữ nghĩa. Mã dùng SDK thật nằm trong phần lý thuyết bên dưới.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  COSMWASM = MỘT CỬA HÀNG CÓ KHO RIÊNG BÊN TRONG                             │
│                                                                              │
│    ┌────────────────────────────┐                                           │
│    │  Hợp đồng CW20             │   Khách gọi execute(Transfer)             │
│    │  ┌──────────────────────┐  │   → hợp đồng tự mở kho của mình           │
│    │  │ kho: số dư mọi người │  │   → sửa hai dòng → xong                   │
│    │  └──────────────────────┘  │                                           │
│    └────────────────────────────┘   Đơn giản, nhưng TUẦN TỰ:                │
│                                     hai giao dịch cùng chạm hợp đồng        │
│                                     phải xếp hàng.                          │
│                                                                              │
│  SOLANA = NGÂN HÀNG CÓ HÀNG NGHÌN KÉT SẮT RỜI                              │
│                                                                              │
│    [két An]  [két Bình]  [két Chi]  [két Dũng]                             │
│       │          │           │          │                                   │
│    Chương trình KHÔNG tự biết két nào tồn tại.                              │
│    Giao dịch phải KHAI BÁO TRƯỚC sẽ mở két nào.                             │
│                                                                              │
│    → An chuyển cho Bình  (chạm két An, Bình)  ┐                             │
│    → Chi chuyển cho Dũng (chạm két Chi, Dũng) ┘ CHẠY SONG SONG              │
│                                                                              │
│    Cái giá: chương trình phải TỰ KIỂM MỌI THỨ.                              │
│    Không kiểm `is_signer` → ai cũng rút được két người khác.                │
│    Không kiểm `owner`     → kẻ gian đưa két giả mạo vào.                    │
│                                                                              │
│  PDA = KÉT KHÔNG CÓ CHÌA, CHỈ CHƯƠNG TRÌNH MỞ ĐƯỢC                         │
│    Địa chỉ suy ra từ hạt giống + id chương trình, và cố ý nằm NGOÀI          │
│    đường cong ed25519 → không tồn tại khoá riêng nào mở được nó.            │
│    Đó là cách hợp đồng "sở hữu" tài sản mà không ai cầm chìa.               │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Ba điểm vào của CosmWasm và ý nghĩa của chúng

```rust
// Với SDK thật:
#[entry_point]
pub fn instantiate(deps: DepsMut, env: Env, info: MessageInfo, msg: InstantiateMsg)
    -> Result<Response, ContractError> { … }   // chạy MỘT lần khi tạo

#[entry_point]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg)
    -> Result<Response, ContractError> { … }   // thay đổi trạng thái, có phí

#[entry_point]
pub fn query(deps: Deps, env: Env, msg: QueryMsg)
    -> StdResult<Binary> { … }                 // CHỈ ĐỌC — chú ý `Deps`, không phải `DepsMut`
```

Điều đáng chú ý về mặt kiểu: `query` nhận `Deps` (bất biến) còn `execute` nhận `DepsMut`. **Hệ thống kiểu của Rust bảo đảm truy vấn không thể sửa trạng thái** — đây là một ví dụ đẹp về việc mã hoá luật bảo mật vào kiểu dữ liệu, đúng tinh thần chương 20 về typestate.

`MessageInfo` mang `sender` — người **thực sự** ký giao dịch. Mọi kiểm tra quyền phải dựa trên `info.sender`, không bao giờ dựa trên tham số do người gọi truyền vào.

### 2. Bốn lỗi kiểm tra chết người của Solana

Solana giao toàn bộ việc kiểm tra cho lập trình viên. Bốn lỗi kinh điển:

1. **Thiếu kiểm `is_signer`.** Không có nó, bất kỳ ai cũng đưa tài khoản của người khác vào giao dịch và rút tiền. Đây là lỗ hổng phổ biến nhất.
2. **Thiếu kiểm `owner`.** Kẻ gian tạo một tài khoản có cùng bố cục dữ liệu nhưng do chương trình của họ sở hữu, rồi đưa vào — chương trình của bạn đọc dữ liệu giả mà tưởng thật.
3. **Thiếu kiểm `is_writable`.** Ghi vào tài khoản chỉ-đọc sẽ thất bại ở tầng runtime, nhưng phát hiện sớm cho thông báo lỗi rõ hơn.
4. **Nhầm lẫn kiểu tài khoản (Account type confusion).** Hai kiểu tài khoản khác nhau cùng kích thước → cần một byte định danh (discriminator) ở đầu dữ liệu. Anchor tự thêm 8 byte cho việc này.

### 3. PDA — địa chỉ không có khoá riêng

Một Program Derived Address được sinh bằng cách băm `(hạt giống, bump, id chương trình)` và **yêu cầu kết quả nằm ngoài đường cong ed25519**. Nếu rơi trúng đường cong, ta giảm `bump` từ 255 xuống và thử lại.

Vì sao lại cần "ngoài đường cong"? Vì mọi điểm **trên** đường cong đều tương ứng với một khoá công khai, tức là tồn tại khoá riêng mở được. Điểm ngoài đường cong thì không — nên chỉ chương trình mới ký thay cho nó được (qua `invoke_signed`).

Trong thực tế luôn dùng **bump chuẩn** (bump lớn nhất hợp lệ) và **lưu nó lại**. Chấp nhận bump tuỳ ý là một lỗ hổng: kẻ tấn công có thể tạo nhiều PDA khác nhau cho cùng một hạt giống.

### 4. Song song hoá: điều làm Solana khác biệt

Vì mọi giao dịch phải khai báo trước danh sách tài khoản nó chạm tới, bộ lập lịch có thể xếp chúng thành các nhóm không xung đột **trước khi chạy**. Hai giao dịch chỉ **đọc** cùng một tài khoản vẫn song song được; chỉ khi có ít nhất một bên **ghi** thì mới phải tuần tự.

Đây chính là mô hình đọc/ghi của `RwLock` mà bạn đã gặp ở chương 26 — nhưng áp dụng ở tầm toàn mạng, và quyết định trước lúc chạy chứ không phải lúc chạy.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch72`, kiểm thử bằng `cargo test -p ch72`.

```rust
#![allow(dead_code)]
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
pub struct NewField { pub height: u64, pub timestamp: u64, pub dia_chi_hop_dong: Address }

#[derive(Debug, Clone)]
pub struct ThongTinGoi { pub sender: Address, pub tien_gui_kem: Money }

/// Kho khoá–giá trị riêng của MỖI hợp đồng. Hợp đồng khác không đọc được.
/// Đây chính là điểm khác biệt lớn nhất so với Solana.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Store { pub o: BTreeMap<Vec<u8>, Vec<u8>> }

impl Store {
    pub fn set<T: AsRef<[u8]>>(&mut self, key: T, gt: &[u8]) {
        self.o.insert(key.as_ref().to_vec(), gt.to_vec());
    }
    pub fn lay<T: AsRef<[u8]>>(&self, key: T) -> Option<&Vec<u8>> { self.o.get(key.as_ref()) }
    pub fn remove<T: AsRef<[u8]>>(&mut self, key: T) { self.o.remove(key.as_ref()); }

    // Trợ giúp cho số dư: khoá "balance:<địa chỉ>" → u128 dạng big-endian
    pub fn set_balance(&mut self, ai: &str, v: Money) {
        self.set(format!("so_du:{ai}"), &v.to_be_bytes());
    }
    pub fn balance(&self, ai: &str) -> Money {
        self.lay(format!("so_du:{ai}"))
            .map(|b| u128::from_be_bytes(b[..16].try_into().unwrap()))
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ContractError {
    KhongDuSoDu { can: Money, co: Money },
    KhongCoQuyen { ai: Address },
    ChuaDenHan { remaining: u64 },
    DaHoanTat,
    SoTienBangKhong,
    TranSo,
}

/// Sự kiện phát ra — cách hợp đồng "kể lại" việc mình đã làm cho thế giới bên ngoài.
#[derive(Debug, Clone, PartialEq)]
pub struct Event { pub kind: String, pub attribute: Vec<(String, String)> }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Response {
    pub event: Vec<Event>,
    /// Thông điệp gửi tiếp cho hợp đồng/mô-đun khác. CosmWasm KHÔNG cho gọi
    /// đồng bộ sang hợp đồng khác — bạn trả về thông điệp và để chuỗi thực thi
    /// SAU KHI hàm của bạn kết thúc. Nhờ vậy tấn công tái nhập (reentrancy)
    /// bị chặn ở tầng KIẾN TRÚC, không phải bằng cờ khoá như Solidity.
    pub thong_message_cont: Vec<String>,
}

impl Response {
    pub fn new() -> Self { Response::default() }
    pub fn event(mut self, kind: &str, tt: &[(&str, &str)]) -> Self {
        self.event.push(Event {
            kind: kind.into(),
            attribute: tt.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        });
        self
    }
    pub fn send_cont(mut self, td: &str) -> Self { self.thong_message_cont.push(td.into()); self }
}

// ---------------------------------------------------------------------------
// Token kiểu CW20
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum TokenMsg {
    ChuyenKhoan { den: Address, quantity: Money },
    Dot { quantity: Money },
    ChoPhep { nguoi_duoc_uy_quyen: Address, quantity: Money },
    ChuyenTuUyQuyen { tu: Address, den: Address, quantity: Money },
}

pub struct TokenCw20;

impl TokenCw20 {
    pub fn block_make(store: &mut Store, owner: &str, total_supply: Money) -> Response {
        store.set_balance(owner, total_supply);
        store.set(b"tong_cung", &total_supply.to_be_bytes());
        Response::new().event("khoi_tao", &[("owner", owner)])
    }

    pub fn total_supply(store: &Store) -> Money {
        store.lay(b"tong_cung").map(|b| u128::from_be_bytes(b[..16].try_into().unwrap())).unwrap_or(0)
    }

    fn key_allowance(owner: &str, can: &str) -> String { format!("uy_quyen:{owner}:{can}") }

    pub fn allowance(store: &Store, owner: &str, can: &str) -> Money {
        store.lay(Self::key_allowance(owner, can))
            .map(|b| u128::from_be_bytes(b[..16].try_into().unwrap())).unwrap_or(0)
    }

    pub fn execute(store: &mut Store, _env: &NewField, info: &ThongTinGoi, order: TokenMsg)
        -> Result<Response, ContractError>
    {
        match order {
            TokenMsg::ChuyenKhoan { den, quantity } => {
                Self::subtract(store, &info.sender, quantity)?;
                Self::gate(store, &den, quantity)?;
                Ok(Response::new().event("chuyen_khoan",
                    &[("tu", &info.sender), ("den", &den), ("so_luong", &quantity.to_string())]))
            }
            TokenMsg::Dot { quantity } => {
                Self::subtract(store, &info.sender, quantity)?;
                let new = Self::total_supply(store) - quantity;
                store.set(b"tong_cung", &new.to_be_bytes());
                Ok(Response::new().event("dot", &[("so_luong", &quantity.to_string())]))
            }
            TokenMsg::ChoPhep { nguoi_duoc_uy_quyen, quantity } => {
                store.set(Self::key_allowance(&info.sender, &nguoi_duoc_uy_quyen),
                        &quantity.to_be_bytes());
                Ok(Response::new().event("cho_phep", &[("cho", &nguoi_duoc_uy_quyen)]))
            }
            TokenMsg::ChuyenTuUyQuyen { tu, den, quantity } => {
                let limit = Self::allowance(store, &tu, &info.sender);
                if limit < quantity {
                    return Err(ContractError::KhongDuSoDu { can: quantity, co: limit });
                }
                Self::subtract(store, &tu, quantity)?;
                Self::gate(store, &den, quantity)?;
                // Trừ hạn mức SAU KHI chuyển thành công — nếu trừ trước rồi
                // chuyển lỗi, hạn mức bị mất oan.
                store.set(Self::key_allowance(&tu, &info.sender),
                        &(limit - quantity).to_be_bytes());
                Ok(Response::new().event("chuyen_uy_quyen", &[("tu", &tu), ("den", &den)]))
            }
        }
    }

    fn subtract(store: &mut Store, ai: &str, v: Money) -> Result<(), ContractError> {
        if v == 0 { return Err(ContractError::SoTienBangKhong); }
        let co = store.balance(ai);
        if co < v { return Err(ContractError::KhongDuSoDu { can: v, co }); }
        store.set_balance(ai, co - v);
        Ok(())
    }
    fn gate(store: &mut Store, ai: &str, v: Money) -> Result<(), ContractError> {
        let new = store.balance(ai).checked_add(v).ok_or(ContractError::TranSo)?;
        store.set_balance(ai, new);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Ký quỹ (escrow) — máy trạng thái trong hợp đồng
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateEscrow { DangGiu, DaGiaiNgan, DaHoanTien }

#[derive(Debug, Clone, PartialEq)]
pub struct Escrow {
    pub buyer: Address,
    pub seller: Address,
    pub in_tai: Address,
    pub so_tien: Money,
    pub deadline: u64,
    pub state: StateEscrow,
}

impl Escrow {
    pub fn block_make(info: &ThongTinGoi, seller: &str, in_tai: &str, deadline: u64)
        -> Result<Escrow, ContractError>
    {
        if info.tien_gui_kem == 0 { return Err(ContractError::SoTienBangKhong); }
        Ok(Escrow {
            buyer: info.sender.clone(),
            seller: seller.into(),
            in_tai: in_tai.into(),
            so_tien: info.tien_gui_kem,
            deadline,
            state: StateEscrow::DangGiu,
        })
    }

    /// Người mua hoặc trọng tài có quyền giải ngân cho người bán.
    pub fn release(&mut self, info: &ThongTinGoi) -> Result<Response, ContractError> {
        if self.state != StateEscrow::DangGiu { return Err(ContractError::DaHoanTat); }
        if info.sender != self.buyer && info.sender != self.in_tai {
            return Err(ContractError::KhongCoQuyen { ai: info.sender.clone() });
        }
        self.state = StateEscrow::DaGiaiNgan;
        Ok(Response::new()
            .event("giai_ngan", &[("cho", &self.seller)])
            .send_cont(&format!("gui {} toi {}", self.so_tien, self.seller)))
    }

    /// Hoàn tiền chỉ được phép SAU hạn chót — hoặc do trọng tài quyết định.
    pub fn refund(&mut self, env: &NewField, info: &ThongTinGoi) -> Result<Response, ContractError> {
        if self.state != StateEscrow::DangGiu { return Err(ContractError::DaHoanTat); }
        let is_in_tai = info.sender == self.in_tai;
        if !is_in_tai {
            if info.sender != self.buyer {
                return Err(ContractError::KhongCoQuyen { ai: info.sender.clone() });
            }
            if env.timestamp < self.deadline {
                return Err(ContractError::ChuaDenHan { remaining: self.deadline - env.timestamp });
            }
        }
        self.state = StateEscrow::DaHoanTien;
        Ok(Response::new()
            .event("hoan_tien", &[("cho", &self.buyer)])
            .send_cont(&format!("gui {} toi {}", self.so_tien, self.buyer)))
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
    pub is_signer: bool,      // người gọi đã ký cho tài khoản này chưa
    pub is_writable: bool,   // giao dịch có khai báo sẽ ghi vào đây không
    pub is_executable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SolanaError {
    ThieuChuKy(Address),
    KhongPhaiChuSoHuu { account: Address, mong_doi: Address, actual: Address },
    TaiKhoanChiDoc(Address),
    DiaChiPdaSai { mong_doi: Address, actual: Address },
    KhongDuLamports { can: u64, co: u64 },
    ThieuTaiKhoan(usize),
}

/// Địa chỉ dẫn xuất từ chương trình (PDA). Không có khoá riêng — nên KHÔNG AI
/// ký được cho nó, kể cả kẻ tấn công. Chỉ chương trình dẫn xuất ra nó mới
/// "ký" thay được, qua cơ chế `invoke_signed`.
pub fn derive_pda(hat_giong: &[&[u8]], ma_chuong_trinh: &str) -> Address {
    let mut h: u64 = 0xcbf29ce484222325;
    for part in hat_giong {
        for &b in *part {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        h ^= 0xFF; // dấu phân cách giữa các hạt giống
        h = h.wrapping_mul(0x100000001b3);
    }
    for &b in ma_chuong_trinh.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("PDA{:016x}", h)
}

/// Bộ kiểm tra bắt buộc trước MỌI thao tác. Bỏ sót một dòng ở đây là nguyên
/// nhân của hầu hết các vụ mất tiền trên Solana.
pub struct CheckAccount;

impl CheckAccount {
    pub fn must_ky(account: &Account) -> Result<(), SolanaError> {
        if account.is_signer { Ok(()) } else { Err(SolanaError::ThieuChuKy(account.address.clone())) }
    }
    pub fn must_owned_own(account: &Account, ct: &str) -> Result<(), SolanaError> {
        if account.owner == ct { Ok(()) } else {
            Err(SolanaError::KhongPhaiChuSoHuu {
                account: account.address.clone(), mong_doi: ct.into(),
                actual: account.owner.clone() })
        }
    }
    pub fn must_record_can(account: &Account) -> Result<(), SolanaError> {
        if account.is_writable { Ok(()) } else { Err(SolanaError::TaiKhoanChiDoc(account.address.clone())) }
    }
    pub fn must_be_valid_pda(account: &Account, hat_giong: &[&[u8]], ct: &str) -> Result<(), SolanaError> {
        let mong_doi = derive_pda(hat_giong, ct);
        if account.address == mong_doi { Ok(()) } else {
            Err(SolanaError::DiaChiPdaSai { mong_doi, actual: account.address.clone() })
        }
    }
}

/// Chương trình đếm — ví dụ nhỏ nhất thể hiện đủ mô hình tài khoản Solana.
pub struct CounterProgram;
pub const MA_CHUONG_TRINH: &str = "Dem111111111111111111111111111111";

impl CounterProgram {
    /// Tài khoản đếm là một PDA dẫn xuất từ địa chỉ chủ sở hữu — nên mỗi người
    /// dùng có đúng một bộ đếm, và địa chỉ của nó tính ra được mà không cần tra sổ.
    pub fn address_buffer(owner: &str) -> Address {
        derive_pda(&[b"bo_dem", owner.as_bytes()], MA_CHUONG_TRINH)
    }

    pub fn tang(account: &mut [Account]) -> Result<u64, SolanaError> {
        // Thứ tự tài khoản là MỘT PHẦN CỦA GIAO DIỆN. Sai thứ tự = sai hợp đồng.
        let owner = account.first().ok_or(SolanaError::ThieuTaiKhoan(0))?.clone();
        let buffer = account.get_mut(1).ok_or(SolanaError::ThieuTaiKhoan(1))?;

        CheckAccount::must_ky(&owner)?;
        CheckAccount::must_record_can(buffer)?;
        CheckAccount::must_owned_own(buffer, MA_CHUONG_TRINH)?;
        // KIỂM TRA SỐNG CÒN: bộ đếm này có đúng là của người ký không?
        // Thiếu dòng này, ai cũng tăng được bộ đếm của người khác.
        CheckAccount::must_be_valid_pda(buffer, &[b"bo_dem", owner.address.as_bytes()],
                                       MA_CHUONG_TRINH)?;

        let current = u64::from_le_bytes(buffer.data[..8].try_into().unwrap());
        let new = current + 1;
        buffer.data[..8].copy_from_slice(&new.to_le_bytes());
        Ok(new)
    }

    /// Gọi chéo chương trình (CPI): chuyển lamports qua "chương trình hệ thống".
    pub fn transfer_lamports(tu: &mut Account, den: &mut Account, v: u64) -> Result<(), SolanaError> {
        CheckAccount::must_record_can(tu)?;
        CheckAccount::must_record_can(den)?;
        if tu.lamports < v {
            return Err(SolanaError::KhongDuLamports { can: v, co: tu.lamports });
        }
        tu.lamports -= v;
        den.lamports += v;
        Ok(())
    }
}

// ============================================================================
// PHẦN III — SO SÁNH HAI MÔ HÌNH BẰNG CON SỐ
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct AnalyzeParallel {
    pub num_trade: usize,
    pub so_lo_song_song: usize,
    pub lo: Vec<Vec<usize>>,
}

/// Vì Solana bắt khai báo trước tài khoản sẽ ghi, ta xếp lịch được các giao
/// dịch KHÔNG đụng nhau vào cùng một lô chạy song song. CosmWasm/EVM không
/// biết trước nên phải chạy tuần tự tuyệt đối.
pub fn arrange_schedule_parallel(trade: &[Vec<Address>]) -> AnalyzeParallel {
    let mut lo: Vec<Vec<usize>> = Vec::new();
    let mut da_arrange = vec![false; trade.len()];
    let mut remaining = trade.len();

    while remaining > 0 {
        let mut lo_nay = Vec::new();
        let mut da_dung: Vec<&Address> = Vec::new();
        for (i, account) in trade.iter().enumerate() {
            if da_arrange[i] { continue; }
            if account.iter().any(|a| da_dung.contains(&a)) { continue; } // xung đột
            lo_nay.push(i);
            da_dung.extend(account.iter());
            da_arrange[i] = true;
            remaining -= 1;
        }
        lo.push(lo_nay);
    }
    AnalyzeParallel { num_trade: trade.len(), so_lo_song_song: lo.len(), lo }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   HỢP ĐỒNG THÔNG MINH: COSMWASM vs SOLANA                 ");
    println!("═══════════════════════════════════════════════════════════");

    let env = NewField { height: 100, timestamp: 1000, dia_chi_hop_dong: "hd1".into() };

    println!("\n1. TOKEN CW20 — hợp đồng sở hữu kho của chính nó");
    let mut store = Store::default();
    TokenCw20::block_make(&mut store, "An", 1_000_000);
    println!("   Tổng cung {} · số dư An = {}", TokenCw20::total_supply(&store), store.balance("An"));

    let info_an = ThongTinGoi { sender: "An".into(), tien_gui_kem: 0 };
    let r = TokenCw20::execute(&mut store, &env, &info_an,
        TokenMsg::ChuyenKhoan { den: "Binh".into(), quantity: 250_000 }).unwrap();
    println!("   Chuyển 250k cho Bình → sự kiện {:?}", r.event[0].kind);
    println!("   An = {} · Bình = {}", store.balance("An"), store.balance("Binh"));

    let e = TokenCw20::execute(&mut store, &env, &info_an,
        TokenMsg::ChuyenKhoan { den: "Cuong".into(), quantity: 9_999_999 }).unwrap_err();
    println!("   Chuyển quá số dư → {:?}", e);

    println!("\n2. UỶ QUYỀN (approve / transferFrom)");
    TokenCw20::execute(&mut store, &env, &info_an,
        TokenMsg::ChoPhep { nguoi_duoc_uy_quyen: "San".into(), quantity: 100_000 }).unwrap();
    let info_san = ThongTinGoi { sender: "San".into(), tien_gui_kem: 0 };
    TokenCw20::execute(&mut store, &env, &info_san,
        TokenMsg::ChuyenTuUyQuyen { tu: "An".into(), den: "Dung".into(), quantity: 60_000 }).unwrap();
    println!("   Sàn dùng 60k trong hạn mức 100k → hạn mức còn {}",
             TokenCw20::allowance(&store, "An", "San"));

    println!("\n3. KÝ QUỸ — máy trạng thái + kiểm soát quyền");
    let info_mua = ThongTinGoi { sender: "NguoiMua".into(), tien_gui_kem: 500 };
    let mut kq = Escrow::block_make(&info_mua, "NguoiBan", "TrongTai", 2000).unwrap();
    let som = NewField { timestamp: 1500, ..env.clone() };
    println!("   Người mua đòi hoàn tiền trước hạn → {:?}",
             kq.clone().refund(&som, &info_mua).unwrap_err());
    let ke_is = ThongTinGoi { sender: "NguoiLa".into(), tien_gui_kem: 0 };
    println!("   Người lạ đòi giải ngân            → {:?}",
             kq.clone().release(&ke_is).unwrap_err());
    let r = kq.release(&info_mua).unwrap();
    println!("   Người mua giải ngân → {:?} · thông điệp tiếp: {:?}",
             kq.state, r.thong_message_cont);
    println!("   Giải ngân lần hai                 → {:?}",
             kq.release(&info_mua).unwrap_err());

    println!("\n4. MÔ HÌNH SOLANA — PDA và kiểm tra tài khoản");
    let owner = "An11111111111111111111111111111111";
    let pda = CounterProgram::address_buffer(owner);
    println!("   PDA bộ đếm của An = {}", pda);
    println!("   Tính lại lần nữa   = {} (tất định)", CounterProgram::address_buffer(owner));
    println!("   Của người khác     = {}",
             CounterProgram::address_buffer("Binh2222222222222222222222222222"));

    let mut account = vec![
        Account { address: owner.into(), owner: "he_thong".into(), lamports: 10_000,
                   data: vec![], is_signer: true, is_writable: false, is_executable: false },
        Account { address: pda.clone(), owner: MA_CHUONG_TRINH.into(), lamports: 1_000,
                   data: vec![0u8; 8], is_signer: false, is_writable: true, is_executable: false },
    ];
    for _ in 0..3 { CounterProgram::tang(&mut account).unwrap(); }
    println!("   Tăng 3 lần → bộ đếm = {}",
             u64::from_le_bytes(account[1].data[..8].try_into().unwrap()));

    // Kẻ tấn công đưa PDA của người khác vào
    let mut xau = account.clone();
    xau[1].address = CounterProgram::address_buffer("Binh2222222222222222222222222222");
    println!("   Dùng bộ đếm của người khác → {:?}",
             CounterProgram::tang(&mut xau).unwrap_err());
    let mut no_ky = account.clone();
    no_ky[0].is_signer = false;
    println!("   Không ký                   → {:?}",
             CounterProgram::tang(&mut no_ky).unwrap_err());

    println!("\n5. VÌ SAO SOLANA CHẠY SONG SONG ĐƯỢC");
    let gd: Vec<Vec<Address>> = vec![
        vec!["A".into(), "B".into()],
        vec!["C".into(), "D".into()],   // không đụng gd 0 → song song được
        vec!["B".into(), "E".into()],   // đụng "B" → phải chờ
        vec!["F".into(), "G".into()],
        vec!["A".into(), "F".into()],   // đụng cả A lẫn F
    ];
    let pt = arrange_schedule_parallel(&gd);
    println!("   {} giao dịch → {} lô: {:?}", pt.num_trade, pt.so_lo_song_song, pt.lo);
    println!("   CosmWasm/EVM sẽ cần {} bước tuần tự.", gd.len());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   COSMWASM: TRẠNG THÁI THUỘC HỢP ĐỒNG                      ");
    println!("   SOLANA  : TRẠNG THÁI THUỘC TÀI KHOẢN, KHAI BÁO TRƯỚC     ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_env() -> NewField {
        NewField { height: 100, timestamp: 1000, dia_chi_hop_dong: "hd".into() }
    }
    fn goi(ai: &str) -> ThongTinGoi {
        ThongTinGoi { sender: ai.into(), tien_gui_kem: 0 }
    }
    fn token_mau() -> Store {
        let mut k = Store::default();
        TokenCw20::block_make(&mut k, "An", 1_000);
        k
    }

    // ---------- Kho ----------
    #[test]
    fn store_record_read_and_remove_use() {
        let mut k = Store::default();
        k.set(b"a", b"1");
        assert_eq!(k.lay(b"a"), Some(&b"1".to_vec()));
        k.remove(b"a");
        assert_eq!(k.lay(b"a"), None);
    }

    #[test]
    fn balance_chua_record_is_no_chu_no_must_error() {
        let k = Store::default();
        assert_eq!(k.balance("chua-ton-tai"), 0, "mặc định 0 giúp không cần khởi tạo trước");
    }

    // ---------- Token CW20 ----------
    #[test]
    fn transfer_khoan_report_toan_total_supply() {
        let mut k = token_mau();
        let prev: Money = ["An", "Binh", "Cuong"].iter().map(|a| k.balance(a)).sum();
        TokenCw20::execute(&mut k, &sample_env(), &goi("An"),
            TokenMsg::ChuyenKhoan { den: "Binh".into(), quantity: 300 }).unwrap();
        let next: Money = ["An", "Binh", "Cuong"].iter().map(|a| k.balance(a)).sum();
        assert_eq!(prev, next, "chuyển khoản không được sinh hay huỷ token");
        assert_eq!(k.balance("An"), 700);
        assert_eq!(k.balance("Binh"), 300);
    }

    #[test]
    fn no_transfer_can_qua_balance() {
        let mut k = token_mau();
        let e = TokenCw20::execute(&mut k, &sample_env(), &goi("An"),
            TokenMsg::ChuyenKhoan { den: "Binh".into(), quantity: 1_001 }).unwrap_err();
        assert_eq!(e, ContractError::KhongDuSoDu { can: 1_001, co: 1_000 });
        assert_eq!(k.balance("An"), 1_000, "thất bại phải KHÔNG để lại thay đổi nào");
        assert_eq!(k.balance("Binh"), 0);
    }

    #[test]
    fn cannot_transfer_from_empty_account() {
        let mut k = token_mau();
        assert!(TokenCw20::execute(&mut k, &sample_env(), &goi("KeLa"),
            TokenMsg::ChuyenKhoan { den: "KeLa2".into(), quantity: 1 }).is_err());
    }

    #[test]
    fn transfer_quantity_no_is_reject() {
        let mut k = token_mau();
        assert_eq!(TokenCw20::execute(&mut k, &sample_env(), &goi("An"),
            TokenMsg::ChuyenKhoan { den: "Binh".into(), quantity: 0 }).unwrap_err(),
            ContractError::SoTienBangKhong);
    }

    #[test]
    fn burning_reduces_both_balance_and_total_supply() {
        let mut k = token_mau();
        TokenCw20::execute(&mut k, &sample_env(), &goi("An"),
            TokenMsg::Dot { quantity: 400 }).unwrap();
        assert_eq!(k.balance("An"), 600);
        assert_eq!(TokenCw20::total_supply(&k), 600, "đốt phải giảm tổng cung, không chỉ số dư");
    }

    #[test]
    fn allowance_limit_use_limit() {
        let mut k = token_mau();
        TokenCw20::execute(&mut k, &sample_env(), &goi("An"),
            TokenMsg::ChoPhep { nguoi_duoc_uy_quyen: "San".into(), quantity: 500 }).unwrap();
        TokenCw20::execute(&mut k, &sample_env(), &goi("San"),
            TokenMsg::ChuyenTuUyQuyen { tu: "An".into(), den: "Binh".into(), quantity: 300 }).unwrap();
        assert_eq!(TokenCw20::allowance(&k, "An", "San"), 200);
        let e = TokenCw20::execute(&mut k, &sample_env(), &goi("San"),
            TokenMsg::ChuyenTuUyQuyen { tu: "An".into(), den: "Binh".into(), quantity: 300 })
            .unwrap_err();
        assert_eq!(e, ContractError::KhongDuSoDu { can: 300, co: 200 }, "vượt hạn mức phải bị chặn");
    }

    #[test]
    fn no_allowance_thi_no_rut_proxy_can() {
        let mut k = token_mau();
        assert!(TokenCw20::execute(&mut k, &sample_env(), &goi("KeGian"),
            TokenMsg::ChuyenTuUyQuyen { tu: "An".into(), den: "KeGian".into(), quantity: 1 })
            .is_err());
        assert_eq!(k.balance("An"), 1_000);
    }

    #[test]
    fn failed_transfer_does_not_consume_allowance() {
        let mut k = token_mau();
        TokenCw20::execute(&mut k, &sample_env(), &goi("An"),
            TokenMsg::ChoPhep { nguoi_duoc_uy_quyen: "San".into(), quantity: 5_000 }).unwrap();
        // hạn mức 5000 nhưng An chỉ có 1000 → chuyển hỏng
        assert!(TokenCw20::execute(&mut k, &sample_env(), &goi("San"),
            TokenMsg::ChuyenTuUyQuyen { tu: "An".into(), den: "B".into(), quantity: 2_000 })
            .is_err());
        assert_eq!(TokenCw20::allowance(&k, "An", "San"), 5_000,
                   "hỏng thì hạn mức phải nguyên vẹn, không mất oan");
    }

    // ---------- Ký quỹ ----------
    #[test]
    fn escrow_no_recv_tien_table_no() {
        let i = ThongTinGoi { sender: "M".into(), tien_gui_kem: 0 };
        assert_eq!(Escrow::block_make(&i, "B", "T", 100).unwrap_err(), ContractError::SoTienBangKhong);
    }

    #[test]
    fn nguoi_buy_release_can() {
        let i = ThongTinGoi { sender: "M".into(), tien_gui_kem: 100 };
        let mut kq = Escrow::block_make(&i, "B", "T", 2000).unwrap();
        let r = kq.release(&goi("M")).unwrap();
        assert_eq!(kq.state, StateEscrow::DaGiaiNgan);
        assert_eq!(r.thong_message_cont.len(), 1, "phải phát thông điệp chuyển tiền");
    }

    #[test]
    fn in_tai_same_release_can() {
        let i = ThongTinGoi { sender: "M".into(), tien_gui_kem: 100 };
        let mut kq = Escrow::block_make(&i, "B", "T", 2000).unwrap();
        assert!(kq.release(&goi("T")).is_ok());
    }

    #[test]
    fn stranger_can_do_nothing() {
        let i = ThongTinGoi { sender: "M".into(), tien_gui_kem: 100 };
        let mut kq = Escrow::block_make(&i, "B", "T", 2000).unwrap();
        assert_eq!(kq.release(&goi("KeLa")).unwrap_err(),
                   ContractError::KhongCoQuyen { ai: "KeLa".into() });
        assert_eq!(kq.state, StateEscrow::DangGiu, "trạng thái không được đổi");
    }

    #[test]
    fn nguoi_sell_no_from_release_wait_minh_can() {
        // Lỗi thiết kế kinh điển: quên loại người bán ra khỏi danh sách được phép.
        let i = ThongTinGoi { sender: "M".into(), tien_gui_kem: 100 };
        let mut kq = Escrow::block_make(&i, "B", "T", 2000).unwrap();
        assert!(kq.release(&goi("B")).is_err(), "người bán KHÔNG được tự lấy tiền");
    }

    #[test]
    fn refund_is_block_prev_han_and_wait_qua_next_han() {
        let i = ThongTinGoi { sender: "M".into(), tien_gui_kem: 100 };
        let mut kq = Escrow::block_make(&i, "B", "T", 2000).unwrap();
        let som = NewField { timestamp: 1500, ..sample_env() };
        assert_eq!(kq.refund(&som, &goi("M")).unwrap_err(),
                   ContractError::ChuaDenHan { remaining: 500 });
        let borrow = NewField { timestamp: 2500, ..sample_env() };
        assert!(kq.refund(&borrow, &goi("M")).is_ok());
        assert_eq!(kq.state, StateEscrow::DaHoanTien);
    }

    #[test]
    fn in_tai_refund_can_enable_ke_time_han() {
        let i = ThongTinGoi { sender: "M".into(), tien_gui_kem: 100 };
        let mut kq = Escrow::block_make(&i, "B", "T", 9_999_999).unwrap();
        assert!(kq.refund(&sample_env(), &goi("T")).is_ok());
    }

    #[test]
    fn no_position_release_two_lan() {
        // Đây là biến thể "rút hai lần" — lỗi tốn tiền phổ biến nhất.
        let i = ThongTinGoi { sender: "M".into(), tien_gui_kem: 100 };
        let mut kq = Escrow::block_make(&i, "B", "T", 2000).unwrap();
        assert!(kq.release(&goi("M")).is_ok());
        assert_eq!(kq.release(&goi("M")).unwrap_err(), ContractError::DaHoanTat);
        assert_eq!(kq.refund(&sample_env(), &goi("T")).unwrap_err(), ContractError::DaHoanTat,
                   "đã giải ngân thì cũng không hoàn tiền được nữa");
    }

    // ---------- Solana ----------
    #[test]
    fn pda_is_deterministic_and_seed_specific() {
        let a = derive_pda(&[b"bo_dem", b"An"], MA_CHUONG_TRINH);
        assert_eq!(a, derive_pda(&[b"bo_dem", b"An"], MA_CHUONG_TRINH), "phải tất định");
        assert_ne!(a, derive_pda(&[b"bo_dem", b"Binh"], MA_CHUONG_TRINH));
        assert_ne!(a, derive_pda(&[b"kho", b"An"], MA_CHUONG_TRINH));
        assert_ne!(a, derive_pda(&[b"bo_dem", b"An"], "ChuongTrinhKhac"));
    }

    #[test]
    fn pda_distinguishes_seed_boundaries() {
        // Không có dấu phân cách, ["ab","c"] và ["a","bc"] sẽ ra cùng địa chỉ —
        // lỗ hổng thật, cho phép kẻ tấn công tạo PDA trùng của người khác.
        assert_ne!(derive_pda(&[b"ab", b"c"], MA_CHUONG_TRINH),
                   derive_pda(&[b"a", b"bc"], MA_CHUONG_TRINH));
    }

    fn unit_account(owner: &str) -> Vec<Account> {
        vec![
            Account { address: owner.into(), owner: "he_thong".into(), lamports: 100,
                       data: vec![], is_signer: true, is_writable: false, is_executable: false },
            Account { address: CounterProgram::address_buffer(owner),
                       owner: MA_CHUONG_TRINH.into(), lamports: 100,
                       data: vec![0u8; 8], is_signer: false, is_writable: true, is_executable: false },
        ]
    }

    #[test]
    fn increment_succeeds_when_all_checks_pass() {
        let mut account = unit_account("An");
        assert_eq!(CounterProgram::tang(&mut account), Ok(1));
        assert_eq!(CounterProgram::tang(&mut account), Ok(2));
        assert_eq!(u64::from_le_bytes(account[1].data[..8].try_into().unwrap()), 2);
    }

    #[test]
    fn reject_when_thieu_period() {
        let mut account = unit_account("An");
        account[0].is_signer = false;
        assert_eq!(CounterProgram::tang(&mut account).unwrap_err(),
                   SolanaError::ThieuChuKy("An".into()));
    }

    #[test]
    fn rejects_account_not_declared_writable() {
        let mut account = unit_account("An");
        account[1].is_writable = false;
        assert!(matches!(CounterProgram::tang(&mut account).unwrap_err(),
                         SolanaError::TaiKhoanChiDoc(_)));
    }

    #[test]
    fn rejects_account_owned_by_another_program() {
        let mut account = unit_account("An");
        account[1].owner = "ChuongTrinhGia".into();
        assert!(matches!(CounterProgram::tang(&mut account).unwrap_err(),
                         SolanaError::KhongPhaiChuSoHuu { .. }));
    }

    #[test]
    fn reject_when_use_buffer_cua_nguoi_other() {
        // ĐÂY LÀ BÀI KIỂM THỬ QUAN TRỌNG NHẤT phần Solana. Thiếu kiểm tra PDA,
        // bất kỳ ai cũng tăng/sửa được tài khoản của người khác — miễn là tài
        // khoản đó do đúng chương trình sở hữu.
        let mut account = unit_account("An");
        account[1].address = CounterProgram::address_buffer("Binh");
        assert!(matches!(CounterProgram::tang(&mut account).unwrap_err(),
                         SolanaError::DiaChiPdaSai { .. }));
    }

    #[test]
    fn reject_when_thieu_account_in_trade() {
        let mut account = unit_account("An");
        account.pop();
        assert_eq!(CounterProgram::tang(&mut account).unwrap_err(), SolanaError::ThieuTaiKhoan(1));
        let mut rong: Vec<Account> = vec![];
        assert_eq!(CounterProgram::tang(&mut rong).unwrap_err(), SolanaError::ThieuTaiKhoan(0));
    }

    #[test]
    fn lamport_transfer_conserves_total() {
        let mut account = unit_account("An");
        account[0].is_writable = true;
        let prev_total = account[0].lamports + account[1].lamports;
        let (a, b) = account.split_at_mut(1);
        CounterProgram::transfer_lamports(&mut a[0], &mut b[0], 30).unwrap();
        assert_eq!(account[0].lamports + account[1].lamports, prev_total);
        assert_eq!(account[0].lamports, 70);
    }

    #[test]
    fn lamport_transfer_beyond_balance_is_blocked() {
        let mut account = unit_account("An");
        account[0].is_writable = true;
        let (a, b) = account.split_at_mut(1);
        assert_eq!(CounterProgram::transfer_lamports(&mut a[0], &mut b[0], 999).unwrap_err(),
                   SolanaError::KhongDuLamports { can: 999, co: 100 });
        assert_eq!(account[0].lamports, 100, "thất bại không được đổi số dư");
    }

    // ---------- Song song hoá ----------
    #[test]
    fn trade_no_use_each_run_same_lo() {
        let gd: Vec<Vec<Address>> = vec![
            vec!["A".into(), "B".into()],
            vec!["C".into(), "D".into()],
            vec!["E".into(), "F".into()],
        ];
        let pt = arrange_schedule_parallel(&gd);
        assert_eq!(pt.so_lo_song_song, 1, "hoàn toàn rời nhau → chạy hết trong 1 lô");
    }

    #[test]
    fn trade_use_each_must_split_lo() {
        let gd: Vec<Vec<Address>> = vec![
            vec!["A".into()], vec!["A".into()], vec!["A".into()],
        ];
        let pt = arrange_schedule_parallel(&gd);
        assert_eq!(pt.so_lo_song_song, 3, "cùng chạm A → buộc tuần tự hoàn toàn");
    }

    #[test]
    fn new_trade_can_arrange_use_one_lan() {
        let gd: Vec<Vec<Address>> = vec![
            vec!["A".into(), "B".into()], vec!["C".into(), "D".into()],
            vec!["B".into(), "E".into()], vec!["F".into(), "G".into()],
            vec!["A".into(), "F".into()],
        ];
        let pt = arrange_schedule_parallel(&gd);
        let mut all: Vec<usize> = pt.lo.iter().flatten().copied().collect();
        all.sort_unstable();
        assert_eq!(all, (0..gd.len()).collect::<Vec<_>>(),
                   "không bỏ sót, không xếp trùng");
        assert!(pt.so_lo_song_song < gd.len(), "phải tiết kiệm được so với tuần tự");
    }

    #[test]
    fn in_one_lo_no_has_two_trade_which_use_each() {
        let gd: Vec<Vec<Address>> = (0..20)
            .map(|i| vec![format!("tk{}", i % 7), format!("tk{}", (i * 3) % 11)])
            .collect();
        let pt = arrange_schedule_parallel(&gd);
        for lo in &pt.lo {
            for (x, &i) in lo.iter().enumerate() {
                for &j in &lo[x + 1..] {
                    assert!(gd[i].iter().all(|a| !gd[j].contains(a)),
                            "gd {} và {} cùng lô mà lại đụng tài khoản", i, j);
                }
            }
        }
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0502: cannot borrow as mutable` | `self.balance.get(&a)` còn sống khi `insert` | Đọc ra biến `u128` rồi mới ghi |
| `E0277: ? couldn't convert` | Trộn `StdError` với lỗi hợp đồng riêng | Cài `From<StdError> for ContractError` (`thiserror` làm sẵn) |
| `E0308: expected Deps, found DepsMut` | Gọi hàm truy vấn từ `execute` | `DepsMut` có `.as_ref()` để hạ xuống `Deps` |
| `attempt to subtract with overflow` | Trừ số dư không kiểm | `checked_sub().ok_or(Loi::KhongDu)?` |
| `AccountBorrowFailed` (lúc chạy Solana) | Mượn `try_borrow_mut_data()` hai lần | Gói mỗi lần mượn trong một khối `{ … }` riêng |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **CosmWasm giấu trạng thái trong hợp đồng; Solana phơi nó ra thành tài khoản.** Mọi khác biệt còn lại đều bắt nguồn từ đây.
2. **`Deps` vs `DepsMut` là bảo mật mã hoá vào kiểu.** Truy vấn không thể sửa trạng thái vì trình biên dịch không cho.
3. **Trên Solana, không kiểm là mất tiền.** `is_signer`, `owner`, `is_writable`, và định danh kiểu — bốn thứ phải kiểm mỗi lần.
4. **PDA nằm ngoài đường cong ed25519** nên không có khoá riêng — đó là cách chương trình giữ tài sản.
5. **Khai báo tài khoản trước cho phép lập lịch song song** — cùng mô hình đọc/ghi của `RwLock`, nhưng ở tầm toàn mạng.

### Bài tập rèn luyện

**Bài 1.** Thêm cơ chế **cho phép chi tiêu (allowance)** kiểu CW20: chủ sở hữu uỷ quyền cho người khác tiêu hộ một hạn mức.

<details>
<summary><b>Gợi ý</b></summary>

Cần một map `(chủ, người được uỷ quyền) → hạn mức`. Chú ý lỗi kinh điển của ERC-20: nếu cho phép đặt lại hạn mức trực tiếp, tồn tại điều kiện tranh đua khi người được uỷ quyền tiêu hết hạn mức cũ ngay trước khi hạn mức mới có hiệu lực. Cách chữa là cung cấp `tang_han_muc` / `giam_han_muc` thay vì `dat_han_muc`.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
// `key_allowance(chu, duoc)` đã có sẵn trong `TokenCw20` — ta chỉ thêm
// ba thao tác mới lên cùng lược đồ khoá đó.
impl TokenCw20 {
    pub fn tang_han_muc(store: &mut Store, owner: &str, can: &str, them: Money)
        -> Result<Response, ContractError>
    {
        let k = Self::key_allowance(owner, can);
        let cu = TokenCw20::allowance(store, owner, can);
        let new = cu.checked_add(them).ok_or(ContractError::TranSo)?;
        store.set(&k, &new.to_be_bytes());
        Ok(Response::new().event("tang_han_muc", &[("owner", owner), ("duoc", can)]))
    }

    /// Giảm về 0 nếu trừ quá — an toàn hơn báo lỗi, vì ý định của người dùng
    /// ("bớt quyền") vẫn được thực hiện trọn vẹn.
    pub fn giam_han_muc(store: &mut Store, owner: &str, can: &str, bot: Money) -> Response {
        let k = Self::key_allowance(owner, can);
        let new = TokenCw20::allowance(store, owner, can).saturating_sub(bot);
        store.set(&k, &new.to_be_bytes());
        Response::new().event("giam_han_muc", &[("owner", owner), ("duoc", can)])
    }

    pub fn chuyen_ho(store: &mut Store, caller: &str, owner: &str, den: &str, quantity: Money)
        -> Result<Response, ContractError>
    {
        let han = TokenCw20::allowance(store, owner, caller);
        if han < quantity { return Err(ContractError::KhongCoQuyen { ai: caller.into() }); }

        // Trừ hạn mức TRƯỚC khi chuyển tiền — mẫu "kiểm tra – tác động – tương tác".
        // Thứ tự này đã ngăn được vô số vụ tấn công tái nhập.
        store.set(&Self::key_allowance(owner, caller), &(han - quantity).to_be_bytes());

        let co = store.balance(owner);
        if co < quantity { return Err(ContractError::KhongDuSoDu { can: quantity, co }); }
        store.set_balance(owner, co - quantity);
        let nhan = store.balance(den);
        store.set_balance(den, nhan.checked_add(quantity).ok_or(ContractError::TranSo)?);

        Ok(Response::new().event("chuyen_ho", &[("tu", owner), ("den", den)]))
    }
}
```

Thứ tự trong `chuyen_ho` rất quan trọng: trừ hạn mức **trước** rồi mới chuyển. Đây là mẫu "kiểm tra – tác động – tương tác" (checks-effects-interactions), thứ đã ngăn được vô số vụ tấn công tái nhập.
</details>

**Bài 2.** Viết một **bộ kiểm tra bảo mật tài khoản Solana** kiểu Anchor: khai báo yêu cầu, kiểm tự động.

<details>
<summary><b>Gợi ý</b></summary>

Ý tưởng của Anchor: thay vì để lập trình viên nhớ viết bốn phép kiểm, hãy cho họ **khai báo** yêu cầu, rồi framework sinh mã kiểm. Cái gì khai báo được thì không quên được.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Request { KyTen, GhiDuoc, ThuocChuongTrinh, DungDinhDanh(u8) }

pub struct RangBuoc<'a> {
    pub name: &'a str,
    pub cac_yeu_cau: &'a [Request],
}

/// Cùng bốn phép kiểm mà `CheckAccount` cung cấp lẻ, nhưng ở dạng KHAI BÁO.
/// Cái gì khai báo được thì không quên được — đó là toàn bộ ý tưởng của Anchor.
pub fn kiem_tra_tat_ca(
    account: &[Account],
    rang_buoc: &[RangBuoc],
    id_chuong_trinh: &str,
) -> Result<(), SolanaError> {
    if account.len() < rang_buoc.len() {
        return Err(SolanaError::ThieuTaiKhoan(rang_buoc.len()));
    }
    for (account, rb) in account.iter().zip(rang_buoc) {
        for yc in rb.cac_yeu_cau {
            match yc {
                Request::KyTen => CheckAccount::must_ky(account)?,
                Request::GhiDuoc => CheckAccount::must_record_can(account)?,
                Request::ThuocChuongTrinh =>
                    CheckAccount::must_owned_own(account, id_chuong_trinh)?,
                Request::DungDinhDanh(d) => {
                    // Hai kiểu tài khoản cùng kích thước là lỗ hổng "nhầm lẫn kiểu".
                    // Anchor chống bằng 8 byte định danh ở đầu dữ liệu.
                    if account.data.first() != Some(d) {
                        return Err(SolanaError::KhongPhaiChuSoHuu {
                            account: account.address.clone(),
                            mong_doi: format!("kieu#{}", d),
                            actual: format!("kieu#{:?}", account.data.first()),
                        });
                    }
                }
            }
        }
    }
    Ok(())
}
```

Với Anchor thật, toàn bộ đoạn trên được sinh ra từ vài dòng khai báo:

```rust
#[derive(Accounts)]
pub struct RutTien<'info> {
    #[account(mut)]                  pub sender: Signer<'info>,
    #[account(mut, has_one = sender)] pub ky_quy: Account<'info, Escrow>,
    pub he_thong: Program<'info, System>,
}
```

`Signer` buộc `is_signer`, `Account<T>` buộc cả `owner` lẫn định danh 8 byte, `mut` buộc `is_writable`. Bốn phép kiểm dễ quên trở thành bốn thứ **không thể quên**.
</details>
