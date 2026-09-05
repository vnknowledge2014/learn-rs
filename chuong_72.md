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
4. **Nhầm lẫn kiểu tài khoản.** Hai kiểu tài khoản khác nhau cùng kích thước → cần một byte định danh (discriminator) ở đầu dữ liệu. Anchor tự thêm 8 byte cho việc này.

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

pub type DiaChi = String;
pub type SoTien = u128;

/// Ba thứ mà mọi hàm hợp đồng CosmWasm đều nhận. Tách bạch rõ ràng:
/// `env` là sự thật của chuỗi, `info` là "ai gọi và gửi kèm bao nhiêu tiền".
#[derive(Debug, Clone)]
pub struct MoiTruong { pub chieu_cao: u64, pub thoi_diem: u64, pub dia_chi_hop_dong: DiaChi }

#[derive(Debug, Clone)]
pub struct ThongTinGoi { pub nguoi_gui: DiaChi, pub tien_gui_kem: SoTien }

/// Kho khoá–giá trị riêng của MỖI hợp đồng. Hợp đồng khác không đọc được.
/// Đây chính là điểm khác biệt lớn nhất so với Solana.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Kho { pub o: BTreeMap<Vec<u8>, Vec<u8>> }

impl Kho {
    pub fn dat<T: AsRef<[u8]>>(&mut self, khoa: T, gt: &[u8]) {
        self.o.insert(khoa.as_ref().to_vec(), gt.to_vec());
    }
    pub fn lay<T: AsRef<[u8]>>(&self, khoa: T) -> Option<&Vec<u8>> { self.o.get(khoa.as_ref()) }
    pub fn xoa<T: AsRef<[u8]>>(&mut self, khoa: T) { self.o.remove(khoa.as_ref()); }

    // Trợ giúp cho số dư: khoá "so_du:<địa chỉ>" → u128 dạng big-endian
    pub fn dat_so_du(&mut self, ai: &str, v: SoTien) {
        self.dat(format!("so_du:{ai}"), &v.to_be_bytes());
    }
    pub fn so_du(&self, ai: &str) -> SoTien {
        self.lay(format!("so_du:{ai}"))
            .map(|b| u128::from_be_bytes(b[..16].try_into().unwrap()))
            .unwrap_or(0)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoiHopDong {
    KhongDuSoDu { can: SoTien, co: SoTien },
    KhongCoQuyen { ai: DiaChi },
    ChuaDenHan { con_lai: u64 },
    DaHoanTat,
    SoTienBangKhong,
    TranSo,
}

/// Sự kiện phát ra — cách hợp đồng "kể lại" việc mình đã làm cho thế giới bên ngoài.
#[derive(Debug, Clone, PartialEq)]
pub struct SuKien { pub loai: String, pub thuoc_tinh: Vec<(String, String)> }

#[derive(Debug, Clone, PartialEq, Default)]
pub struct PhanHoi {
    pub su_kien: Vec<SuKien>,
    /// Thông điệp gửi tiếp cho hợp đồng/mô-đun khác. CosmWasm KHÔNG cho gọi
    /// đồng bộ sang hợp đồng khác — bạn trả về thông điệp và để chuỗi thực thi
    /// SAU KHI hàm của bạn kết thúc. Nhờ vậy tấn công tái nhập (reentrancy)
    /// bị chặn ở tầng KIẾN TRÚC, không phải bằng cờ khoá như Solidity.
    pub thong_diep_tiep: Vec<String>,
}

impl PhanHoi {
    pub fn moi() -> Self { PhanHoi::default() }
    pub fn su_kien(mut self, loai: &str, tt: &[(&str, &str)]) -> Self {
        self.su_kien.push(SuKien {
            loai: loai.into(),
            thuoc_tinh: tt.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect(),
        });
        self
    }
    pub fn gui_tiep(mut self, td: &str) -> Self { self.thong_diep_tiep.push(td.into()); self }
}

// ---------------------------------------------------------------------------
// Token kiểu CW20
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
pub enum LenhToken {
    ChuyenKhoan { den: DiaChi, so_luong: SoTien },
    Dot { so_luong: SoTien },
    ChoPhep { nguoi_duoc_uy_quyen: DiaChi, so_luong: SoTien },
    ChuyenTuUyQuyen { tu: DiaChi, den: DiaChi, so_luong: SoTien },
}

pub struct TokenCw20;

impl TokenCw20 {
    pub fn khoi_tao(kho: &mut Kho, chu_so_huu: &str, tong_cung: SoTien) -> PhanHoi {
        kho.dat_so_du(chu_so_huu, tong_cung);
        kho.dat(b"tong_cung", &tong_cung.to_be_bytes());
        PhanHoi::moi().su_kien("khoi_tao", &[("chu", chu_so_huu)])
    }

    pub fn tong_cung(kho: &Kho) -> SoTien {
        kho.lay(b"tong_cung").map(|b| u128::from_be_bytes(b[..16].try_into().unwrap())).unwrap_or(0)
    }

    fn khoa_uy_quyen(chu: &str, duoc: &str) -> String { format!("uy_quyen:{chu}:{duoc}") }

    pub fn uy_quyen(kho: &Kho, chu: &str, duoc: &str) -> SoTien {
        kho.lay(Self::khoa_uy_quyen(chu, duoc))
            .map(|b| u128::from_be_bytes(b[..16].try_into().unwrap())).unwrap_or(0)
    }

    pub fn thuc_thi(kho: &mut Kho, _env: &MoiTruong, info: &ThongTinGoi, lenh: LenhToken)
        -> Result<PhanHoi, LoiHopDong>
    {
        match lenh {
            LenhToken::ChuyenKhoan { den, so_luong } => {
                Self::tru(kho, &info.nguoi_gui, so_luong)?;
                Self::cong(kho, &den, so_luong)?;
                Ok(PhanHoi::moi().su_kien("chuyen_khoan",
                    &[("tu", &info.nguoi_gui), ("den", &den), ("so_luong", &so_luong.to_string())]))
            }
            LenhToken::Dot { so_luong } => {
                Self::tru(kho, &info.nguoi_gui, so_luong)?;
                let moi = Self::tong_cung(kho) - so_luong;
                kho.dat(b"tong_cung", &moi.to_be_bytes());
                Ok(PhanHoi::moi().su_kien("dot", &[("so_luong", &so_luong.to_string())]))
            }
            LenhToken::ChoPhep { nguoi_duoc_uy_quyen, so_luong } => {
                kho.dat(Self::khoa_uy_quyen(&info.nguoi_gui, &nguoi_duoc_uy_quyen),
                        &so_luong.to_be_bytes());
                Ok(PhanHoi::moi().su_kien("cho_phep", &[("cho", &nguoi_duoc_uy_quyen)]))
            }
            LenhToken::ChuyenTuUyQuyen { tu, den, so_luong } => {
                let han_muc = Self::uy_quyen(kho, &tu, &info.nguoi_gui);
                if han_muc < so_luong {
                    return Err(LoiHopDong::KhongDuSoDu { can: so_luong, co: han_muc });
                }
                Self::tru(kho, &tu, so_luong)?;
                Self::cong(kho, &den, so_luong)?;
                // Trừ hạn mức SAU KHI chuyển thành công — nếu trừ trước rồi
                // chuyển lỗi, hạn mức bị mất oan.
                kho.dat(Self::khoa_uy_quyen(&tu, &info.nguoi_gui),
                        &(han_muc - so_luong).to_be_bytes());
                Ok(PhanHoi::moi().su_kien("chuyen_uy_quyen", &[("tu", &tu), ("den", &den)]))
            }
        }
    }

    fn tru(kho: &mut Kho, ai: &str, v: SoTien) -> Result<(), LoiHopDong> {
        if v == 0 { return Err(LoiHopDong::SoTienBangKhong); }
        let co = kho.so_du(ai);
        if co < v { return Err(LoiHopDong::KhongDuSoDu { can: v, co }); }
        kho.dat_so_du(ai, co - v);
        Ok(())
    }
    fn cong(kho: &mut Kho, ai: &str, v: SoTien) -> Result<(), LoiHopDong> {
        let moi = kho.so_du(ai).checked_add(v).ok_or(LoiHopDong::TranSo)?;
        kho.dat_so_du(ai, moi);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Ký quỹ (escrow) — máy trạng thái trong hợp đồng
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrangThaiKyQuy { DangGiu, DaGiaiNgan, DaHoanTien }

#[derive(Debug, Clone, PartialEq)]
pub struct KyQuy {
    pub nguoi_mua: DiaChi,
    pub nguoi_ban: DiaChi,
    pub trong_tai: DiaChi,
    pub so_tien: SoTien,
    pub han_chot: u64,
    pub trang_thai: TrangThaiKyQuy,
}

impl KyQuy {
    pub fn khoi_tao(info: &ThongTinGoi, nguoi_ban: &str, trong_tai: &str, han_chot: u64)
        -> Result<KyQuy, LoiHopDong>
    {
        if info.tien_gui_kem == 0 { return Err(LoiHopDong::SoTienBangKhong); }
        Ok(KyQuy {
            nguoi_mua: info.nguoi_gui.clone(),
            nguoi_ban: nguoi_ban.into(),
            trong_tai: trong_tai.into(),
            so_tien: info.tien_gui_kem,
            han_chot,
            trang_thai: TrangThaiKyQuy::DangGiu,
        })
    }

    /// Người mua hoặc trọng tài có quyền giải ngân cho người bán.
    pub fn giai_ngan(&mut self, info: &ThongTinGoi) -> Result<PhanHoi, LoiHopDong> {
        if self.trang_thai != TrangThaiKyQuy::DangGiu { return Err(LoiHopDong::DaHoanTat); }
        if info.nguoi_gui != self.nguoi_mua && info.nguoi_gui != self.trong_tai {
            return Err(LoiHopDong::KhongCoQuyen { ai: info.nguoi_gui.clone() });
        }
        self.trang_thai = TrangThaiKyQuy::DaGiaiNgan;
        Ok(PhanHoi::moi()
            .su_kien("giai_ngan", &[("cho", &self.nguoi_ban)])
            .gui_tiep(&format!("gui {} toi {}", self.so_tien, self.nguoi_ban)))
    }

    /// Hoàn tiền chỉ được phép SAU hạn chót — hoặc do trọng tài quyết định.
    pub fn hoan_tien(&mut self, env: &MoiTruong, info: &ThongTinGoi) -> Result<PhanHoi, LoiHopDong> {
        if self.trang_thai != TrangThaiKyQuy::DangGiu { return Err(LoiHopDong::DaHoanTat); }
        let la_trong_tai = info.nguoi_gui == self.trong_tai;
        if !la_trong_tai {
            if info.nguoi_gui != self.nguoi_mua {
                return Err(LoiHopDong::KhongCoQuyen { ai: info.nguoi_gui.clone() });
            }
            if env.thoi_diem < self.han_chot {
                return Err(LoiHopDong::ChuaDenHan { con_lai: self.han_chot - env.thoi_diem });
            }
        }
        self.trang_thai = TrangThaiKyQuy::DaHoanTien;
        Ok(PhanHoi::moi()
            .su_kien("hoan_tien", &[("cho", &self.nguoi_mua)])
            .gui_tiep(&format!("gui {} toi {}", self.so_tien, self.nguoi_mua)))
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
pub struct TaiKhoan {
    pub dia_chi: DiaChi,
    /// Chương trình nào SỞ HỮU tài khoản này. Chỉ chủ sở hữu được ghi dữ liệu.
    pub chu_so_huu: DiaChi,
    pub lamports: u64,
    pub du_lieu: Vec<u8>,
    pub la_ky: bool,      // người gọi đã ký cho tài khoản này chưa
    pub duoc_ghi: bool,   // giao dịch có khai báo sẽ ghi vào đây không
    pub la_thuc_thi: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LoiSolana {
    ThieuChuKy(DiaChi),
    KhongPhaiChuSoHuu { tai_khoan: DiaChi, mong_doi: DiaChi, thuc_te: DiaChi },
    TaiKhoanChiDoc(DiaChi),
    DiaChiPdaSai { mong_doi: DiaChi, thuc_te: DiaChi },
    KhongDuLamports { can: u64, co: u64 },
    ThieuTaiKhoan(usize),
}

/// Địa chỉ dẫn xuất từ chương trình (PDA). Không có khoá riêng — nên KHÔNG AI
/// ký được cho nó, kể cả kẻ tấn công. Chỉ chương trình dẫn xuất ra nó mới
/// "ký" thay được, qua cơ chế `invoke_signed`.
pub fn dan_xuat_pda(hat_giong: &[&[u8]], ma_chuong_trinh: &str) -> DiaChi {
    let mut h: u64 = 0xcbf29ce484222325;
    for phan in hat_giong {
        for &b in *phan {
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
pub struct KiemTraTaiKhoan;

impl KiemTraTaiKhoan {
    pub fn phai_ky(tk: &TaiKhoan) -> Result<(), LoiSolana> {
        if tk.la_ky { Ok(()) } else { Err(LoiSolana::ThieuChuKy(tk.dia_chi.clone())) }
    }
    pub fn phai_thuoc_so_huu(tk: &TaiKhoan, ct: &str) -> Result<(), LoiSolana> {
        if tk.chu_so_huu == ct { Ok(()) } else {
            Err(LoiSolana::KhongPhaiChuSoHuu {
                tai_khoan: tk.dia_chi.clone(), mong_doi: ct.into(),
                thuc_te: tk.chu_so_huu.clone() })
        }
    }
    pub fn phai_ghi_duoc(tk: &TaiKhoan) -> Result<(), LoiSolana> {
        if tk.duoc_ghi { Ok(()) } else { Err(LoiSolana::TaiKhoanChiDoc(tk.dia_chi.clone())) }
    }
    pub fn phai_dung_pda(tk: &TaiKhoan, hat_giong: &[&[u8]], ct: &str) -> Result<(), LoiSolana> {
        let mong_doi = dan_xuat_pda(hat_giong, ct);
        if tk.dia_chi == mong_doi { Ok(()) } else {
            Err(LoiSolana::DiaChiPdaSai { mong_doi, thuc_te: tk.dia_chi.clone() })
        }
    }
}

/// Chương trình đếm — ví dụ nhỏ nhất thể hiện đủ mô hình tài khoản Solana.
pub struct ChuongTrinhDem;
pub const MA_CHUONG_TRINH: &str = "Dem111111111111111111111111111111";

impl ChuongTrinhDem {
    /// Tài khoản đếm là một PDA dẫn xuất từ địa chỉ chủ sở hữu — nên mỗi người
    /// dùng có đúng một bộ đếm, và địa chỉ của nó tính ra được mà không cần tra sổ.
    pub fn dia_chi_bo_dem(chu: &str) -> DiaChi {
        dan_xuat_pda(&[b"bo_dem", chu.as_bytes()], MA_CHUONG_TRINH)
    }

    pub fn tang(tk: &mut [TaiKhoan]) -> Result<u64, LoiSolana> {
        // Thứ tự tài khoản là MỘT PHẦN CỦA GIAO DIỆN. Sai thứ tự = sai hợp đồng.
        let chu = tk.first().ok_or(LoiSolana::ThieuTaiKhoan(0))?.clone();
        let bo_dem = tk.get_mut(1).ok_or(LoiSolana::ThieuTaiKhoan(1))?;

        KiemTraTaiKhoan::phai_ky(&chu)?;
        KiemTraTaiKhoan::phai_ghi_duoc(bo_dem)?;
        KiemTraTaiKhoan::phai_thuoc_so_huu(bo_dem, MA_CHUONG_TRINH)?;
        // KIỂM TRA SỐNG CÒN: bộ đếm này có đúng là của người ký không?
        // Thiếu dòng này, ai cũng tăng được bộ đếm của người khác.
        KiemTraTaiKhoan::phai_dung_pda(bo_dem, &[b"bo_dem", chu.dia_chi.as_bytes()],
                                       MA_CHUONG_TRINH)?;

        let hien_tai = u64::from_le_bytes(bo_dem.du_lieu[..8].try_into().unwrap());
        let moi = hien_tai + 1;
        bo_dem.du_lieu[..8].copy_from_slice(&moi.to_le_bytes());
        Ok(moi)
    }

    /// Gọi chéo chương trình (CPI): chuyển lamports qua "chương trình hệ thống".
    pub fn chuyen_lamports(tu: &mut TaiKhoan, den: &mut TaiKhoan, v: u64) -> Result<(), LoiSolana> {
        KiemTraTaiKhoan::phai_ghi_duoc(tu)?;
        KiemTraTaiKhoan::phai_ghi_duoc(den)?;
        if tu.lamports < v {
            return Err(LoiSolana::KhongDuLamports { can: v, co: tu.lamports });
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
pub struct PhanTichSongSong {
    pub so_giao_dich: usize,
    pub so_lo_song_song: usize,
    pub lo: Vec<Vec<usize>>,
}

/// Vì Solana bắt khai báo trước tài khoản sẽ ghi, ta xếp lịch được các giao
/// dịch KHÔNG đụng nhau vào cùng một lô chạy song song. CosmWasm/EVM không
/// biết trước nên phải chạy tuần tự tuyệt đối.
pub fn xep_lich_song_song(giao_dich: &[Vec<DiaChi>]) -> PhanTichSongSong {
    let mut lo: Vec<Vec<usize>> = Vec::new();
    let mut da_xep = vec![false; giao_dich.len()];
    let mut con_lai = giao_dich.len();

    while con_lai > 0 {
        let mut lo_nay = Vec::new();
        let mut da_dung: Vec<&DiaChi> = Vec::new();
        for (i, tk) in giao_dich.iter().enumerate() {
            if da_xep[i] { continue; }
            if tk.iter().any(|a| da_dung.contains(&a)) { continue; } // xung đột
            lo_nay.push(i);
            da_dung.extend(tk.iter());
            da_xep[i] = true;
            con_lai -= 1;
        }
        lo.push(lo_nay);
    }
    PhanTichSongSong { so_giao_dich: giao_dich.len(), so_lo_song_song: lo.len(), lo }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   HỢP ĐỒNG THÔNG MINH: COSMWASM vs SOLANA                 ");
    println!("═══════════════════════════════════════════════════════════");

    let env = MoiTruong { chieu_cao: 100, thoi_diem: 1000, dia_chi_hop_dong: "hd1".into() };

    println!("\n1. TOKEN CW20 — hợp đồng sở hữu kho của chính nó");
    let mut kho = Kho::default();
    TokenCw20::khoi_tao(&mut kho, "An", 1_000_000);
    println!("   Tổng cung {} · số dư An = {}", TokenCw20::tong_cung(&kho), kho.so_du("An"));

    let info_an = ThongTinGoi { nguoi_gui: "An".into(), tien_gui_kem: 0 };
    let r = TokenCw20::thuc_thi(&mut kho, &env, &info_an,
        LenhToken::ChuyenKhoan { den: "Binh".into(), so_luong: 250_000 }).unwrap();
    println!("   Chuyển 250k cho Bình → sự kiện {:?}", r.su_kien[0].loai);
    println!("   An = {} · Bình = {}", kho.so_du("An"), kho.so_du("Binh"));

    let e = TokenCw20::thuc_thi(&mut kho, &env, &info_an,
        LenhToken::ChuyenKhoan { den: "Cuong".into(), so_luong: 9_999_999 }).unwrap_err();
    println!("   Chuyển quá số dư → {:?}", e);

    println!("\n2. UỶ QUYỀN (approve / transferFrom)");
    TokenCw20::thuc_thi(&mut kho, &env, &info_an,
        LenhToken::ChoPhep { nguoi_duoc_uy_quyen: "San".into(), so_luong: 100_000 }).unwrap();
    let info_san = ThongTinGoi { nguoi_gui: "San".into(), tien_gui_kem: 0 };
    TokenCw20::thuc_thi(&mut kho, &env, &info_san,
        LenhToken::ChuyenTuUyQuyen { tu: "An".into(), den: "Dung".into(), so_luong: 60_000 }).unwrap();
    println!("   Sàn dùng 60k trong hạn mức 100k → hạn mức còn {}",
             TokenCw20::uy_quyen(&kho, "An", "San"));

    println!("\n3. KÝ QUỸ — máy trạng thái + kiểm soát quyền");
    let info_mua = ThongTinGoi { nguoi_gui: "NguoiMua".into(), tien_gui_kem: 500 };
    let mut kq = KyQuy::khoi_tao(&info_mua, "NguoiBan", "TrongTai", 2000).unwrap();
    let som = MoiTruong { thoi_diem: 1500, ..env.clone() };
    println!("   Người mua đòi hoàn tiền trước hạn → {:?}",
             kq.clone().hoan_tien(&som, &info_mua).unwrap_err());
    let ke_la = ThongTinGoi { nguoi_gui: "NguoiLa".into(), tien_gui_kem: 0 };
    println!("   Người lạ đòi giải ngân            → {:?}",
             kq.clone().giai_ngan(&ke_la).unwrap_err());
    let r = kq.giai_ngan(&info_mua).unwrap();
    println!("   Người mua giải ngân → {:?} · thông điệp tiếp: {:?}",
             kq.trang_thai, r.thong_diep_tiep);
    println!("   Giải ngân lần hai                 → {:?}",
             kq.giai_ngan(&info_mua).unwrap_err());

    println!("\n4. MÔ HÌNH SOLANA — PDA và kiểm tra tài khoản");
    let chu = "An11111111111111111111111111111111";
    let pda = ChuongTrinhDem::dia_chi_bo_dem(chu);
    println!("   PDA bộ đếm của An = {}", pda);
    println!("   Tính lại lần nữa   = {} (tất định)", ChuongTrinhDem::dia_chi_bo_dem(chu));
    println!("   Của người khác     = {}",
             ChuongTrinhDem::dia_chi_bo_dem("Binh2222222222222222222222222222"));

    let mut tk = vec![
        TaiKhoan { dia_chi: chu.into(), chu_so_huu: "he_thong".into(), lamports: 10_000,
                   du_lieu: vec![], la_ky: true, duoc_ghi: false, la_thuc_thi: false },
        TaiKhoan { dia_chi: pda.clone(), chu_so_huu: MA_CHUONG_TRINH.into(), lamports: 1_000,
                   du_lieu: vec![0u8; 8], la_ky: false, duoc_ghi: true, la_thuc_thi: false },
    ];
    for _ in 0..3 { ChuongTrinhDem::tang(&mut tk).unwrap(); }
    println!("   Tăng 3 lần → bộ đếm = {}",
             u64::from_le_bytes(tk[1].du_lieu[..8].try_into().unwrap()));

    // Kẻ tấn công đưa PDA của người khác vào
    let mut xau = tk.clone();
    xau[1].dia_chi = ChuongTrinhDem::dia_chi_bo_dem("Binh2222222222222222222222222222");
    println!("   Dùng bộ đếm của người khác → {:?}",
             ChuongTrinhDem::tang(&mut xau).unwrap_err());
    let mut khong_ky = tk.clone();
    khong_ky[0].la_ky = false;
    println!("   Không ký                   → {:?}",
             ChuongTrinhDem::tang(&mut khong_ky).unwrap_err());

    println!("\n5. VÌ SAO SOLANA CHẠY SONG SONG ĐƯỢC");
    let gd: Vec<Vec<DiaChi>> = vec![
        vec!["A".into(), "B".into()],
        vec!["C".into(), "D".into()],   // không đụng gd 0 → song song được
        vec!["B".into(), "E".into()],   // đụng "B" → phải chờ
        vec!["F".into(), "G".into()],
        vec!["A".into(), "F".into()],   // đụng cả A lẫn F
    ];
    let pt = xep_lich_song_song(&gd);
    println!("   {} giao dịch → {} lô: {:?}", pt.so_giao_dich, pt.so_lo_song_song, pt.lo);
    println!("   CosmWasm/EVM sẽ cần {} bước tuần tự.", gd.len());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   COSMWASM: TRẠNG THÁI THUỘC HỢP ĐỒNG                      ");
    println!("   SOLANA  : TRẠNG THÁI THUỘC TÀI KHOẢN, KHAI BÁO TRƯỚC     ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn env_mau() -> MoiTruong {
        MoiTruong { chieu_cao: 100, thoi_diem: 1000, dia_chi_hop_dong: "hd".into() }
    }
    fn goi(ai: &str) -> ThongTinGoi {
        ThongTinGoi { nguoi_gui: ai.into(), tien_gui_kem: 0 }
    }
    fn token_mau() -> Kho {
        let mut k = Kho::default();
        TokenCw20::khoi_tao(&mut k, "An", 1_000);
        k
    }

    // ---------- Kho ----------
    #[test]
    fn kho_ghi_doc_va_xoa_dung() {
        let mut k = Kho::default();
        k.dat(b"a", b"1");
        assert_eq!(k.lay(b"a"), Some(&b"1".to_vec()));
        k.xoa(b"a");
        assert_eq!(k.lay(b"a"), None);
    }

    #[test]
    fn so_du_chua_ghi_la_khong_chu_khong_phai_loi() {
        let k = Kho::default();
        assert_eq!(k.so_du("chua-ton-tai"), 0, "mặc định 0 giúp không cần khởi tạo trước");
    }

    // ---------- Token CW20 ----------
    #[test]
    fn chuyen_khoan_bao_toan_tong_cung() {
        let mut k = token_mau();
        let truoc: SoTien = ["An", "Binh", "Cuong"].iter().map(|a| k.so_du(a)).sum();
        TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("An"),
            LenhToken::ChuyenKhoan { den: "Binh".into(), so_luong: 300 }).unwrap();
        let sau: SoTien = ["An", "Binh", "Cuong"].iter().map(|a| k.so_du(a)).sum();
        assert_eq!(truoc, sau, "chuyển khoản không được sinh hay huỷ token");
        assert_eq!(k.so_du("An"), 700);
        assert_eq!(k.so_du("Binh"), 300);
    }

    #[test]
    fn khong_chuyen_duoc_qua_so_du() {
        let mut k = token_mau();
        let e = TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("An"),
            LenhToken::ChuyenKhoan { den: "Binh".into(), so_luong: 1_001 }).unwrap_err();
        assert_eq!(e, LoiHopDong::KhongDuSoDu { can: 1_001, co: 1_000 });
        assert_eq!(k.so_du("An"), 1_000, "thất bại phải KHÔNG để lại thay đổi nào");
        assert_eq!(k.so_du("Binh"), 0);
    }

    #[test]
    fn tu_khong_co_gi_thi_khong_chuyen_duoc() {
        let mut k = token_mau();
        assert!(TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("KeLa"),
            LenhToken::ChuyenKhoan { den: "KeLa2".into(), so_luong: 1 }).is_err());
    }

    #[test]
    fn chuyen_so_luong_khong_bi_tu_choi() {
        let mut k = token_mau();
        assert_eq!(TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("An"),
            LenhToken::ChuyenKhoan { den: "Binh".into(), so_luong: 0 }).unwrap_err(),
            LoiHopDong::SoTienBangKhong);
    }

    #[test]
    fn dot_token_lam_giam_ca_so_du_lan_tong_cung() {
        let mut k = token_mau();
        TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("An"),
            LenhToken::Dot { so_luong: 400 }).unwrap();
        assert_eq!(k.so_du("An"), 600);
        assert_eq!(TokenCw20::tong_cung(&k), 600, "đốt phải giảm tổng cung, không chỉ số dư");
    }

    #[test]
    fn uy_quyen_gioi_han_dung_han_muc() {
        let mut k = token_mau();
        TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("An"),
            LenhToken::ChoPhep { nguoi_duoc_uy_quyen: "San".into(), so_luong: 500 }).unwrap();
        TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("San"),
            LenhToken::ChuyenTuUyQuyen { tu: "An".into(), den: "Binh".into(), so_luong: 300 }).unwrap();
        assert_eq!(TokenCw20::uy_quyen(&k, "An", "San"), 200);
        let e = TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("San"),
            LenhToken::ChuyenTuUyQuyen { tu: "An".into(), den: "Binh".into(), so_luong: 300 })
            .unwrap_err();
        assert_eq!(e, LoiHopDong::KhongDuSoDu { can: 300, co: 200 }, "vượt hạn mức phải bị chặn");
    }

    #[test]
    fn khong_uy_quyen_thi_khong_rut_ho_duoc() {
        let mut k = token_mau();
        assert!(TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("KeGian"),
            LenhToken::ChuyenTuUyQuyen { tu: "An".into(), den: "KeGian".into(), so_luong: 1 })
            .is_err());
        assert_eq!(k.so_du("An"), 1_000);
    }

    #[test]
    fn han_muc_khong_bi_tru_khi_chuyen_that_bai() {
        let mut k = token_mau();
        TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("An"),
            LenhToken::ChoPhep { nguoi_duoc_uy_quyen: "San".into(), so_luong: 5_000 }).unwrap();
        // hạn mức 5000 nhưng An chỉ có 1000 → chuyển hỏng
        assert!(TokenCw20::thuc_thi(&mut k, &env_mau(), &goi("San"),
            LenhToken::ChuyenTuUyQuyen { tu: "An".into(), den: "B".into(), so_luong: 2_000 })
            .is_err());
        assert_eq!(TokenCw20::uy_quyen(&k, "An", "San"), 5_000,
                   "hỏng thì hạn mức phải nguyên vẹn, không mất oan");
    }

    // ---------- Ký quỹ ----------
    #[test]
    fn ky_quy_khong_nhan_tien_bang_khong() {
        let i = ThongTinGoi { nguoi_gui: "M".into(), tien_gui_kem: 0 };
        assert_eq!(KyQuy::khoi_tao(&i, "B", "T", 100).unwrap_err(), LoiHopDong::SoTienBangKhong);
    }

    #[test]
    fn nguoi_mua_giai_ngan_duoc() {
        let i = ThongTinGoi { nguoi_gui: "M".into(), tien_gui_kem: 100 };
        let mut kq = KyQuy::khoi_tao(&i, "B", "T", 2000).unwrap();
        let r = kq.giai_ngan(&goi("M")).unwrap();
        assert_eq!(kq.trang_thai, TrangThaiKyQuy::DaGiaiNgan);
        assert_eq!(r.thong_diep_tiep.len(), 1, "phải phát thông điệp chuyển tiền");
    }

    #[test]
    fn trong_tai_cung_giai_ngan_duoc() {
        let i = ThongTinGoi { nguoi_gui: "M".into(), tien_gui_kem: 100 };
        let mut kq = KyQuy::khoi_tao(&i, "B", "T", 2000).unwrap();
        assert!(kq.giai_ngan(&goi("T")).is_ok());
    }

    #[test]
    fn nguoi_la_khong_dong_duoc_gi() {
        let i = ThongTinGoi { nguoi_gui: "M".into(), tien_gui_kem: 100 };
        let mut kq = KyQuy::khoi_tao(&i, "B", "T", 2000).unwrap();
        assert_eq!(kq.giai_ngan(&goi("KeLa")).unwrap_err(),
                   LoiHopDong::KhongCoQuyen { ai: "KeLa".into() });
        assert_eq!(kq.trang_thai, TrangThaiKyQuy::DangGiu, "trạng thái không được đổi");
    }

    #[test]
    fn nguoi_ban_khong_tu_giai_ngan_cho_minh_duoc() {
        // Lỗi thiết kế kinh điển: quên loại người bán ra khỏi danh sách được phép.
        let i = ThongTinGoi { nguoi_gui: "M".into(), tien_gui_kem: 100 };
        let mut kq = KyQuy::khoi_tao(&i, "B", "T", 2000).unwrap();
        assert!(kq.giai_ngan(&goi("B")).is_err(), "người bán KHÔNG được tự lấy tiền");
    }

    #[test]
    fn hoan_tien_bi_chan_truoc_han_va_cho_qua_sau_han() {
        let i = ThongTinGoi { nguoi_gui: "M".into(), tien_gui_kem: 100 };
        let mut kq = KyQuy::khoi_tao(&i, "B", "T", 2000).unwrap();
        let som = MoiTruong { thoi_diem: 1500, ..env_mau() };
        assert_eq!(kq.hoan_tien(&som, &goi("M")).unwrap_err(),
                   LoiHopDong::ChuaDenHan { con_lai: 500 });
        let muon = MoiTruong { thoi_diem: 2500, ..env_mau() };
        assert!(kq.hoan_tien(&muon, &goi("M")).is_ok());
        assert_eq!(kq.trang_thai, TrangThaiKyQuy::DaHoanTien);
    }

    #[test]
    fn trong_tai_hoan_tien_duoc_bat_ke_thoi_han() {
        let i = ThongTinGoi { nguoi_gui: "M".into(), tien_gui_kem: 100 };
        let mut kq = KyQuy::khoi_tao(&i, "B", "T", 9_999_999).unwrap();
        assert!(kq.hoan_tien(&env_mau(), &goi("T")).is_ok());
    }

    #[test]
    fn khong_the_giai_ngan_hai_lan() {
        // Đây là biến thể "rút hai lần" — lỗi tốn tiền phổ biến nhất.
        let i = ThongTinGoi { nguoi_gui: "M".into(), tien_gui_kem: 100 };
        let mut kq = KyQuy::khoi_tao(&i, "B", "T", 2000).unwrap();
        assert!(kq.giai_ngan(&goi("M")).is_ok());
        assert_eq!(kq.giai_ngan(&goi("M")).unwrap_err(), LoiHopDong::DaHoanTat);
        assert_eq!(kq.hoan_tien(&env_mau(), &goi("T")).unwrap_err(), LoiHopDong::DaHoanTat,
                   "đã giải ngân thì cũng không hoàn tiền được nữa");
    }

    // ---------- Solana ----------
    #[test]
    fn pda_tat_dinh_va_khac_nhau_theo_hat_giong() {
        let a = dan_xuat_pda(&[b"bo_dem", b"An"], MA_CHUONG_TRINH);
        assert_eq!(a, dan_xuat_pda(&[b"bo_dem", b"An"], MA_CHUONG_TRINH), "phải tất định");
        assert_ne!(a, dan_xuat_pda(&[b"bo_dem", b"Binh"], MA_CHUONG_TRINH));
        assert_ne!(a, dan_xuat_pda(&[b"kho", b"An"], MA_CHUONG_TRINH));
        assert_ne!(a, dan_xuat_pda(&[b"bo_dem", b"An"], "ChuongTrinhKhac"));
    }

    #[test]
    fn pda_phan_biet_duoc_ranh_gioi_hat_giong() {
        // Không có dấu phân cách, ["ab","c"] và ["a","bc"] sẽ ra cùng địa chỉ —
        // lỗ hổng thật, cho phép kẻ tấn công tạo PDA trùng của người khác.
        assert_ne!(dan_xuat_pda(&[b"ab", b"c"], MA_CHUONG_TRINH),
                   dan_xuat_pda(&[b"a", b"bc"], MA_CHUONG_TRINH));
    }

    fn bo_tai_khoan(chu: &str) -> Vec<TaiKhoan> {
        vec![
            TaiKhoan { dia_chi: chu.into(), chu_so_huu: "he_thong".into(), lamports: 100,
                       du_lieu: vec![], la_ky: true, duoc_ghi: false, la_thuc_thi: false },
            TaiKhoan { dia_chi: ChuongTrinhDem::dia_chi_bo_dem(chu),
                       chu_so_huu: MA_CHUONG_TRINH.into(), lamports: 100,
                       du_lieu: vec![0u8; 8], la_ky: false, duoc_ghi: true, la_thuc_thi: false },
        ]
    }

    #[test]
    fn tang_bo_dem_thanh_cong_khi_moi_kiem_tra_deu_qua() {
        let mut tk = bo_tai_khoan("An");
        assert_eq!(ChuongTrinhDem::tang(&mut tk), Ok(1));
        assert_eq!(ChuongTrinhDem::tang(&mut tk), Ok(2));
        assert_eq!(u64::from_le_bytes(tk[1].du_lieu[..8].try_into().unwrap()), 2);
    }

    #[test]
    fn tu_choi_khi_thieu_chu_ky() {
        let mut tk = bo_tai_khoan("An");
        tk[0].la_ky = false;
        assert_eq!(ChuongTrinhDem::tang(&mut tk).unwrap_err(),
                   LoiSolana::ThieuChuKy("An".into()));
    }

    #[test]
    fn tu_choi_khi_tai_khoan_khong_khai_bao_ghi() {
        let mut tk = bo_tai_khoan("An");
        tk[1].duoc_ghi = false;
        assert!(matches!(ChuongTrinhDem::tang(&mut tk).unwrap_err(),
                         LoiSolana::TaiKhoanChiDoc(_)));
    }

    #[test]
    fn tu_choi_khi_chuong_trinh_khac_so_huu_tai_khoan() {
        let mut tk = bo_tai_khoan("An");
        tk[1].chu_so_huu = "ChuongTrinhGia".into();
        assert!(matches!(ChuongTrinhDem::tang(&mut tk).unwrap_err(),
                         LoiSolana::KhongPhaiChuSoHuu { .. }));
    }

    #[test]
    fn tu_choi_khi_dung_bo_dem_cua_nguoi_khac() {
        // ĐÂY LÀ BÀI KIỂM THỬ QUAN TRỌNG NHẤT phần Solana. Thiếu kiểm tra PDA,
        // bất kỳ ai cũng tăng/sửa được tài khoản của người khác — miễn là tài
        // khoản đó do đúng chương trình sở hữu.
        let mut tk = bo_tai_khoan("An");
        tk[1].dia_chi = ChuongTrinhDem::dia_chi_bo_dem("Binh");
        assert!(matches!(ChuongTrinhDem::tang(&mut tk).unwrap_err(),
                         LoiSolana::DiaChiPdaSai { .. }));
    }

    #[test]
    fn tu_choi_khi_thieu_tai_khoan_trong_giao_dich() {
        let mut tk = bo_tai_khoan("An");
        tk.pop();
        assert_eq!(ChuongTrinhDem::tang(&mut tk).unwrap_err(), LoiSolana::ThieuTaiKhoan(1));
        let mut rong: Vec<TaiKhoan> = vec![];
        assert_eq!(ChuongTrinhDem::tang(&mut rong).unwrap_err(), LoiSolana::ThieuTaiKhoan(0));
    }

    #[test]
    fn chuyen_lamports_bao_toan_tong_so() {
        let mut tk = bo_tai_khoan("An");
        tk[0].duoc_ghi = true;
        let tong_truoc = tk[0].lamports + tk[1].lamports;
        let (a, b) = tk.split_at_mut(1);
        ChuongTrinhDem::chuyen_lamports(&mut a[0], &mut b[0], 30).unwrap();
        assert_eq!(tk[0].lamports + tk[1].lamports, tong_truoc);
        assert_eq!(tk[0].lamports, 70);
    }

    #[test]
    fn chuyen_lamports_qua_so_du_bi_chan() {
        let mut tk = bo_tai_khoan("An");
        tk[0].duoc_ghi = true;
        let (a, b) = tk.split_at_mut(1);
        assert_eq!(ChuongTrinhDem::chuyen_lamports(&mut a[0], &mut b[0], 999).unwrap_err(),
                   LoiSolana::KhongDuLamports { can: 999, co: 100 });
        assert_eq!(tk[0].lamports, 100, "thất bại không được đổi số dư");
    }

    // ---------- Song song hoá ----------
    #[test]
    fn giao_dich_khong_dung_nhau_chay_cung_lo() {
        let gd: Vec<Vec<DiaChi>> = vec![
            vec!["A".into(), "B".into()],
            vec!["C".into(), "D".into()],
            vec!["E".into(), "F".into()],
        ];
        let pt = xep_lich_song_song(&gd);
        assert_eq!(pt.so_lo_song_song, 1, "hoàn toàn rời nhau → chạy hết trong 1 lô");
    }

    #[test]
    fn giao_dich_dung_nhau_phai_tach_lo() {
        let gd: Vec<Vec<DiaChi>> = vec![
            vec!["A".into()], vec!["A".into()], vec!["A".into()],
        ];
        let pt = xep_lich_song_song(&gd);
        assert_eq!(pt.so_lo_song_song, 3, "cùng chạm A → buộc tuần tự hoàn toàn");
    }

    #[test]
    fn moi_giao_dich_duoc_xep_dung_mot_lan() {
        let gd: Vec<Vec<DiaChi>> = vec![
            vec!["A".into(), "B".into()], vec!["C".into(), "D".into()],
            vec!["B".into(), "E".into()], vec!["F".into(), "G".into()],
            vec!["A".into(), "F".into()],
        ];
        let pt = xep_lich_song_song(&gd);
        let mut tat_ca: Vec<usize> = pt.lo.iter().flatten().copied().collect();
        tat_ca.sort_unstable();
        assert_eq!(tat_ca, (0..gd.len()).collect::<Vec<_>>(),
                   "không bỏ sót, không xếp trùng");
        assert!(pt.so_lo_song_song < gd.len(), "phải tiết kiệm được so với tuần tự");
    }

    #[test]
    fn trong_mot_lo_khong_co_hai_giao_dich_nao_dung_nhau() {
        let gd: Vec<Vec<DiaChi>> = (0..20)
            .map(|i| vec![format!("tk{}", i % 7), format!("tk{}", (i * 3) % 11)])
            .collect();
        let pt = xep_lich_song_song(&gd);
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
| `E0502: cannot borrow as mutable` | `self.so_du.get(&a)` còn sống khi `insert` | Đọc ra biến `u128` rồi mới ghi |
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
// `khoa_uy_quyen(chu, duoc)` đã có sẵn trong `TokenCw20` — ta chỉ thêm
// ba thao tác mới lên cùng lược đồ khoá đó.
impl TokenCw20 {
    pub fn tang_han_muc(kho: &mut Kho, chu: &str, duoc: &str, them: SoTien)
        -> Result<PhanHoi, LoiHopDong>
    {
        let k = Self::khoa_uy_quyen(chu, duoc);
        let cu = TokenCw20::uy_quyen(kho, chu, duoc);
        let moi = cu.checked_add(them).ok_or(LoiHopDong::TranSo)?;
        kho.dat(&k, &moi.to_be_bytes());
        Ok(PhanHoi::moi().su_kien("tang_han_muc", &[("chu", chu), ("duoc", duoc)]))
    }

    /// Giảm về 0 nếu trừ quá — an toàn hơn báo lỗi, vì ý định của người dùng
    /// ("bớt quyền") vẫn được thực hiện trọn vẹn.
    pub fn giam_han_muc(kho: &mut Kho, chu: &str, duoc: &str, bot: SoTien) -> PhanHoi {
        let k = Self::khoa_uy_quyen(chu, duoc);
        let moi = TokenCw20::uy_quyen(kho, chu, duoc).saturating_sub(bot);
        kho.dat(&k, &moi.to_be_bytes());
        PhanHoi::moi().su_kien("giam_han_muc", &[("chu", chu), ("duoc", duoc)])
    }

    pub fn chuyen_ho(kho: &mut Kho, nguoi_goi: &str, chu: &str, den: &str, so_luong: SoTien)
        -> Result<PhanHoi, LoiHopDong>
    {
        let han = TokenCw20::uy_quyen(kho, chu, nguoi_goi);
        if han < so_luong { return Err(LoiHopDong::KhongCoQuyen { ai: nguoi_goi.into() }); }

        // Trừ hạn mức TRƯỚC khi chuyển tiền — mẫu "kiểm tra – tác động – tương tác".
        // Thứ tự này đã ngăn được vô số vụ tấn công tái nhập.
        kho.dat(&Self::khoa_uy_quyen(chu, nguoi_goi), &(han - so_luong).to_be_bytes());

        let co = kho.so_du(chu);
        if co < so_luong { return Err(LoiHopDong::KhongDuSoDu { can: so_luong, co }); }
        kho.dat_so_du(chu, co - so_luong);
        let nhan = kho.so_du(den);
        kho.dat_so_du(den, nhan.checked_add(so_luong).ok_or(LoiHopDong::TranSo)?);

        Ok(PhanHoi::moi().su_kien("chuyen_ho", &[("tu", chu), ("den", den)]))
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
pub enum YeuCau { KyTen, GhiDuoc, ThuocChuongTrinh, DungDinhDanh(u8) }

pub struct RangBuoc<'a> {
    pub ten: &'a str,
    pub cac_yeu_cau: &'a [YeuCau],
}

/// Cùng bốn phép kiểm mà `KiemTraTaiKhoan` cung cấp lẻ, nhưng ở dạng KHAI BÁO.
/// Cái gì khai báo được thì không quên được — đó là toàn bộ ý tưởng của Anchor.
pub fn kiem_tra_tat_ca(
    tai_khoan: &[TaiKhoan],
    rang_buoc: &[RangBuoc],
    id_chuong_trinh: &str,
) -> Result<(), LoiSolana> {
    if tai_khoan.len() < rang_buoc.len() {
        return Err(LoiSolana::ThieuTaiKhoan(rang_buoc.len()));
    }
    for (tk, rb) in tai_khoan.iter().zip(rang_buoc) {
        for yc in rb.cac_yeu_cau {
            match yc {
                YeuCau::KyTen => KiemTraTaiKhoan::phai_ky(tk)?,
                YeuCau::GhiDuoc => KiemTraTaiKhoan::phai_ghi_duoc(tk)?,
                YeuCau::ThuocChuongTrinh =>
                    KiemTraTaiKhoan::phai_thuoc_so_huu(tk, id_chuong_trinh)?,
                YeuCau::DungDinhDanh(d) => {
                    // Hai kiểu tài khoản cùng kích thước là lỗ hổng "nhầm lẫn kiểu".
                    // Anchor chống bằng 8 byte định danh ở đầu dữ liệu.
                    if tk.du_lieu.first() != Some(d) {
                        return Err(LoiSolana::KhongPhaiChuSoHuu {
                            tai_khoan: tk.dia_chi.clone(),
                            mong_doi: format!("kieu#{}", d),
                            thuc_te: format!("kieu#{:?}", tk.du_lieu.first()),
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
    #[account(mut)]                  pub nguoi_gui: Signer<'info>,
    #[account(mut, has_one = nguoi_gui)] pub ky_quy: Account<'info, KyQuy>,
    pub he_thong: Program<'info, System>,
}
```

`Signer` buộc `is_signer`, `Account<T>` buộc cả `owner` lẫn định danh 8 byte, `mut` buộc `is_writable`. Bốn phép kiểm dễ quên trở thành bốn thứ **không thể quên**.
</details>
