# Chương 81: Mô hình lập trình GPU — SIMT, Gộp truy cập & Bờ ngân hàng (LeetGPU)

## Giới thiệu & Mục tiêu học tập

Chương này lấy cảm hứng từ **leetgpu.com** — nền tảng bài tập lập trình GPU. Nguồn tham khảo mở tương ứng là kho `AlphaGPU/leetgpu-challenges` với 99 bài, chia thành các nhóm: cơ bản (cộng vector, GEMM), rút gọn, quét, sắp xếp, tích chập, và các nhân của học sâu.

Cái bẫy lớn nhất khi chuyển từ CPU sang GPU là tưởng rằng GPU chỉ là "CPU có nhiều lõi". Không phải. GPU là **SIMT** — Single Instruction, Multiple Threads. Cả một nhóm 32 luồng (gọi là **warp**) thực thi **cùng một lệnh** ở cùng một thời điểm.

Hệ quả trực tiếp là hai vấn đề mà CPU không có:

| Vấn đề | Nội dung |
|---|---|
| Phân kỳ warp | 32 luồng rẽ khác hướng → phần cứng chạy **cả hai** nhánh tuần tự |
| Gộp truy cập | 32 luồng đọc rải rác → 32 giao dịch bộ nhớ thay vì 1 |

Hai vấn đề này quyết định hiệu năng nhiều hơn mọi thứ khác cộng lại.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  WARP = TIỂU ĐỘI 32 NGƯỜI DIỄU HÀNH ĐỒNG BỘ                                 │
│                                                                              │
│   Cả 32 người BẮT BUỘC bước cùng một bước, cùng một lúc.                    │
│   Muốn 16 người rẽ trái, 16 người rẽ phải?                                  │
│     → 16 người rẽ trái đi, 16 người ĐỨNG CHỜ                                │
│     → rồi 16 người rẽ phải đi, 16 người kia ĐỨNG CHỜ                        │
│     → tốn GẤP ĐÔI thời gian, hiệu suất còn 50%                              │
│                                                                              │
│   if (threadIdx.x % 2 == 0)  → phân kỳ TỆ NHẤT (50%)                       │
│   if (threadIdx.x < 32)      → KHÔNG phân kỳ (cả warp cùng hướng)           │
│                                                                              │
│  GỘP TRUY CẬP = 32 NGƯỜI LẤY HÀNG TỪ MỘT KỆ                                 │
│                                                                              │
│   GỘP ĐƯỢC — luồng i đọc ô i:                                               │
│     [ 0][ 1][ 2][ 3]...[31]   ← liền mạch                                   │
│     → 1 giao dịch 128 byte phục vụ CẢ 32 luồng                              │
│                                                                              │
│   KHÔNG GỘP — luồng i đọc ô i×32:                                           │
│     [0]...........[32]...........[64]...                                    │
│     → 32 giao dịch riêng biệt → CHẬM GẤP 32 LẦN                            │
│                                                                              │
│   Đây là lỗi phổ biến nhất khi mới học GPU, và cũng đắt nhất.               │
│                                                                              │
│  XUNG ĐỘT NGÂN HÀNG = 32 QUẦY THU NGÂN, AI CŨNG XẾP MỘT QUẦY               │
│                                                                              │
│   Bộ nhớ chia sẻ có 32 "ngân hàng". Ngân hàng = (địa_chỉ / 4) % 32.        │
│                                                                              │
│   float lat[32][32];  lat[threadIdx.x][0]                                   │
│     luồng 0 → ô 0    → ngân hàng 0                                          │
│     luồng 1 → ô 32   → ngân hàng 0   ← XUNG ĐỘT!                           │
│     luồng 2 → ô 64   → ngân hàng 0   ← XUNG ĐỘT!                           │
│     → tuần tự hoá 32 chiều → chậm 32 lần                                   │
│                                                                              │
│   MẸO ĐỆM +1:  float lat[32][33];                                           │
│     luồng 0 → ô 0    → ngân hàng 0                                          │
│     luồng 1 → ô 33   → ngân hàng 1    ← hết xung đột!                      │
│     luồng 2 → ô 66   → ngân hàng 2                                          │
│                                                                              │
│   Một phần tử thừa mỗi hàng. Nhanh gấp 32 lần. Đây là mẹo nổi tiếng nhất   │
│   trong lập trình CUDA.                                                     │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Phân kỳ: chi phí phụ thuộc vào cách bạn viết điều kiện

Điều đáng nói không phải là "tránh `if`", mà là **`if` theo cái gì**.

```
if (mang[i] > 0)          → phân kỳ nếu dữ liệu hỗn tạp
if (threadIdx.x % 2 == 0) → phân kỳ TỐI ĐA, 50% hiệu suất
if (threadIdx.x / 32 == 0)→ KHÔNG phân kỳ, vì cả warp cùng hướng
if (blockIdx.x < 10)      → KHÔNG phân kỳ, điều kiện đồng nhất trong khối
```

Nguyên tắc: nếu điều kiện **đồng nhất trong một warp**, không có phân kỳ nào cả. Vì thế nhiều tối ưu GPU thực chất là **sắp xếp lại dữ liệu** để các luồng trong cùng warp có chung số phận.

Chi phí phân kỳ bằng số nhánh khác nhau trong warp. Hai nhánh → 2×. Bốn nhánh → 4×. Trường hợp tệ nhất là 32 nhánh khác nhau → chậm 32 lần, tức là warp trở thành hoàn toàn tuần tự.

### 2. Gộp truy cập: quy tắc 128 byte

Bộ nhớ toàn cục được phục vụ theo giao dịch **128 byte**. Nếu 32 luồng đọc 32 số `float` liên tiếp và căn chỉnh, đó đúng là 128 byte — **một** giao dịch.

Nếu 32 luồng đọc rải rác, mỗi luồng cần một giao dịch riêng. Băng thông hiệu dụng còn 1/32.

Điều này có hệ quả kiến trúc lớn: **trên GPU, SoA gần như luôn thắng AoS**. Với AoS, luồng i đọc `mang[i].x` — các phần tử `x` cách nhau bằng kích thước struct, nên rải rác. Với SoA, chúng liền nhau, nên gộp được.

Đây là ngược lại với CPU, nơi AoS thường tốt hơn khi bạn dùng nhiều trường của cùng một phần tử.

### 3. Rút gọn song song: cây, không phải vòng lặp

Cộng một triệu số trên GPU không làm bằng vòng lặp. Làm bằng **cây**:

```
Bước 1: 512 luồng, mỗi luồng cộng 2 phần tử → còn 512
Bước 2: 256 luồng                            → còn 256
...
Bước 10: 1 luồng                             → còn 1
```

10 bước thay vì 1024. Nhưng cách viết vòng lặp rất quan trọng:

- **Sai**: `for (s = 1; s < n; s *= 2) if (tid % (2*s) == 0)` — điều kiện `tid % ...` gây phân kỳ tối đa.
- **Đúng**: `for (s = n/2; s > 0; s /= 2) if (tid < s)` — các luồng hoạt động luôn là những luồng đầu, nên warp hoặc toàn bộ hoạt động hoặc toàn bộ nghỉ. Không phân kỳ.

Cùng thuật toán, cùng số phép cộng, khác nhau vài lần về tốc độ chỉ vì cách viết điều kiện.

### 4. GEMM theo lát: câu chuyện cường độ số học

Nhân ma trận ngây thơ trên GPU bị chặn bởi **băng thông bộ nhớ**, không phải sức tính. Với mỗi phép nhân-cộng, bạn phải đọc 2 giá trị — tỉ lệ tính/đọc quá thấp.

Chia lát (tiling) sửa điều đó: nạp một lát `T×T` vào bộ nhớ chia sẻ, rồi mỗi giá trị được dùng `T` lần.

**Cường độ số học** tăng từ khoảng 1 phép/8 byte lên `T/2` phép/8 byte. Với T = 32, đó là gấp 16 lần — đủ để chuyển từ "bị chặn bởi bộ nhớ" sang "bị chặn bởi sức tính", tức là dùng hết khả năng của GPU.

### 5. Mức chiếm dụng: nhiều warp để che giấu độ trễ

GPU giấu độ trễ bộ nhớ bằng cách **chuyển sang warp khác** khi một warp phải chờ. Muốn vậy phải có đủ warp thường trú.

Mức chiếm dụng bị giới hạn bởi ba tài nguyên:
- **Thanh ghi**: mỗi SM có ngân sách thanh ghi cố định. Nhân dùng nhiều thanh ghi → ít warp hơn.
- **Bộ nhớ chia sẻ**: cùng logic.
- **Số warp tối đa** của phần cứng.

Một điểm phản trực giác: **mức chiếm dụng cao không phải lúc nào cũng tốt**. Một nhân dùng nhiều thanh ghi (mức chiếm dụng thấp) nhưng có nhiều ILP có thể nhanh hơn một nhân mức chiếm dụng 100% mà chuỗi phụ thuộc dài. Đây là kết luận nổi tiếng từ nghiên cứu của Volkov về "hiệu năng thấp ở mức chiếm dụng thấp là một huyền thoại".

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch81`, kiểm thử bằng `cargo test -p ch81`.

```rust
#![allow(dead_code)]
//! Chương 81 — Lập trình GPU: mô hình thực thi SIMT, phân kỳ warp, gộp truy
//! cập bộ nhớ, bộ nhớ chia sẻ và xung đột ngân hàng, rút gọn song song, và
//! nhân ma trận theo lát.
//!
//! Theo phân loại bài tập của [LeetGPU](https://leetgpu.com/) — 99 bài chia
//! ba mức, từ cộng vector tới khối transformer. Ở đây ta ĐẾM số giao dịch bộ
//! nhớ và số lần thực thi bị tuần tự hoá bằng mô phỏng tất định, thay vì đo
//! đồng hồ — nhờ vậy kiểm thử được mà không cần GPU.
//!
//! Rust chạm tới GPU qua `wgpu` (đa nền tảng, dùng WGSL — xem Chương 63),
//! `cudarc`/`cust` (ràng buộc CUDA), hoặc `rust-gpu` (biên dịch Rust sang SPIR-V).

// ============================================================================
// 1. MÔ HÌNH THỰC THI SIMT
// ============================================================================
// CPU: vài lõi mạnh, mỗi lõi chạy một luồng khác nhau, có dự đoán nhánh xịn.
// GPU: hàng nghìn lõi yếu, gom thành nhóm 32 luồng gọi là WARP. Cả warp thực
// thi CÙNG MỘT lệnh trên dữ liệu khác nhau — "một lệnh, nhiều luồng" (SIMT).

pub const LUONG_MOI_WARP: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CauHinhPhat {
    pub so_khoi: usize,
    pub luong_moi_khoi: usize,
}

impl CauHinhPhat {
    /// Cách phát chuẩn: đủ luồng để phủ hết `n` phần tử, làm tròn LÊN.
    pub fn cho_n_phan_tu(n: usize, luong_moi_khoi: usize) -> Self {
        let l = luong_moi_khoi.max(1);
        CauHinhPhat { so_khoi: n.div_ceil(l), luong_moi_khoi: l }
    }
    pub fn tong_luong(&self) -> usize { self.so_khoi * self.luong_moi_khoi }

    /// Số warp mỗi khối. Nếu `luong_moi_khoi` không chia hết 32 thì warp cuối
    /// chạy thiếu luồng — phần cứng vẫn tốn nguyên một warp cho nó.
    pub fn warp_moi_khoi(&self) -> usize { self.luong_moi_khoi.div_ceil(LUONG_MOI_WARP) }

    /// Số làn bị lãng phí ở warp cuối của mỗi khối.
    pub fn luong_lang_phi_moi_khoi(&self) -> usize {
        self.warp_moi_khoi() * LUONG_MOI_WARP - self.luong_moi_khoi
    }

    /// Số luồng chạy nhưng không có việc (vì `tong_luong` > n).
    pub fn luong_thua(&self, n: usize) -> usize { self.tong_luong().saturating_sub(n) }
}

// ============================================================================
// 2. PHÂN KỲ WARP — cái bẫy lớn nhất của người mới
// ============================================================================
// Cả warp chạy CÙNG một lệnh. Gặp `if` mà 32 luồng chia hai phe, phần cứng
// buộc phải chạy nhánh `then` (tắt phe kia), rồi chạy nhánh `else` (tắt phe
// này). Hai nhánh chạy TUẦN TỰ — warp mất gấp đôi thời gian.
//
// Điểm mấu chốt: phân kỳ chỉ tính TRONG một warp. Nếu warp 0 toàn đi nhánh A
// và warp 1 toàn đi nhánh B thì KHÔNG có phân kỳ nào cả.

#[derive(Debug, PartialEq)]
pub struct PhanTichPhanKy {
    pub so_warp: usize,
    pub so_warp_phan_ky: usize,
    /// Tổng "lượt thực thi nhánh" — warp không phân kỳ tốn 1, phân kỳ tốn 2.
    pub luot_thuc_thi: usize,
    pub he_so_cham: f64,
}

/// `dieu_kien[i]` là kết quả `if` của luồng thứ `i`.
pub fn phan_tich_phan_ky(dieu_kien: &[bool]) -> PhanTichPhanKy {
    let so_warp = dieu_kien.len().div_ceil(LUONG_MOI_WARP);
    let mut phan_ky = 0;
    let mut luot = 0;
    for w in dieu_kien.chunks(LUONG_MOI_WARP) {
        let co_dung = w.iter().any(|&x| x);
        let co_sai = w.iter().any(|&x| !x);
        if co_dung && co_sai { phan_ky += 1; luot += 2; } else { luot += 1; }
    }
    PhanTichPhanKy {
        so_warp, so_warp_phan_ky: phan_ky, luot_thuc_thi: luot,
        he_so_cham: if so_warp == 0 { 1.0 } else { luot as f64 / so_warp as f64 },
    }
}

/// Cách viết TỆ: rẽ nhánh theo tính chẵn lẻ của chỉ số luồng.
/// Trong mỗi warp có 16 luồng chẵn và 16 luồng lẻ → phân kỳ 100%.
pub fn dieu_kien_theo_chan_le(n: usize) -> Vec<bool> {
    (0..n).map(|i| i % 2 == 0).collect()
}

/// Cách viết TỐT: rẽ nhánh theo chỉ số WARP. Mỗi warp đi trọn một nhánh
/// → không warp nào phân kỳ, dù tỉ lệ hai nhánh vẫn là 50/50.
pub fn dieu_kien_theo_warp(n: usize) -> Vec<bool> {
    (0..n).map(|i| (i / LUONG_MOI_WARP) % 2 == 0).collect()
}

// ============================================================================
// 3. GỘP TRUY CẬP BỘ NHỚ
// ============================================================================
// Bộ nhớ toàn cục của GPU phục vụ theo GIAO DỊCH 128 byte. Nếu 32 luồng trong
// warp đọc 32 số f32 LIỀN NHAU, cả warp gói gọn trong 1 giao dịch. Nếu chúng
// đọc cách quãng, mỗi luồng có thể tốn một giao dịch riêng — chậm gấp 32 lần
// dù đọc cùng số byte có ích.

pub const BYTE_MOI_GIAO_DICH: usize = 128;

#[derive(Debug, PartialEq)]
pub struct PhanTichGop {
    pub so_luong: usize,
    pub so_giao_dich: usize,
    pub byte_co_ich: usize,
    pub byte_da_chuyen: usize,
    /// Tỉ lệ băng thông thực sự dùng được. 1.0 = hoàn hảo.
    pub hieu_suat: f64,
}

/// Đếm số giao dịch bộ nhớ cho một warp truy cập theo `buoc_nhay`.
pub fn phan_tich_gop(so_luong: usize, byte_moi_phan_tu: usize, buoc_nhay: usize)
    -> PhanTichGop
{
    let mut cac_dong = std::collections::HashSet::new();
    for i in 0..so_luong {
        let dia_chi = i * buoc_nhay * byte_moi_phan_tu;
        cac_dong.insert(dia_chi / BYTE_MOI_GIAO_DICH);
    }
    let so_gd = cac_dong.len();
    let co_ich = so_luong * byte_moi_phan_tu;
    let da_chuyen = so_gd * BYTE_MOI_GIAO_DICH;
    PhanTichGop {
        so_luong, so_giao_dich: so_gd,
        byte_co_ich: co_ich, byte_da_chuyen: da_chuyen,
        hieu_suat: if da_chuyen == 0 { 0.0 } else { co_ich as f64 / da_chuyen as f64 },
    }
}

// ============================================================================
// 4. BỘ NHỚ CHIA SẺ & XUNG ĐỘT NGÂN HÀNG
// ============================================================================
// Bộ nhớ chia sẻ nhanh gần bằng thanh ghi, nhưng chia thành 32 NGÂN HÀNG.
// Hai luồng cùng warp chạm hai địa chỉ khác nhau trên CÙNG một ngân hàng thì
// phải xếp hàng. Ngân hàng = chỉ_số % 32 (với phần tử 4 byte).

pub const SO_NGAN_HANG: usize = 32;

#[derive(Debug, PartialEq)]
pub struct PhanTichNganHang {
    /// Số luồng nhiều nhất dồn vào một ngân hàng — đúng bằng số lượt xếp hàng.
    pub muc_xung_dot: usize,
    pub co_xung_dot: bool,
}

pub fn phan_tich_ngan_hang(chi_so_o_nho: &[usize]) -> PhanTichNganHang {
    let mut dem = [0usize; SO_NGAN_HANG];
    for &i in chi_so_o_nho { dem[i % SO_NGAN_HANG] += 1; }
    let muc = dem.iter().copied().max().unwrap_or(0);
    PhanTichNganHang { muc_xung_dot: muc, co_xung_dot: muc > 1 }
}

/// Truy cập lát ma trận theo CỘT với bề rộng 32: mọi luồng rơi vào CÙNG một
/// ngân hàng → xung đột 32 lối, chậm gấp 32 lần.
pub fn truy_cap_cot_lat(be_rong: usize) -> Vec<usize> {
    (0..LUONG_MOI_WARP).map(|i| i * be_rong).collect()
}

/// Thủ thuật kinh điển: ĐỆM lát thêm một cột. Bề rộng 33 làm chỉ số lệch dần
/// nên 32 luồng rơi vào 32 ngân hàng khác nhau. Tốn thêm 1/32 bộ nhớ để đổi
/// lấy tốc độ gấp 32 lần.
pub fn truy_cap_cot_lat_co_dem(be_rong: usize) -> Vec<usize> {
    (0..LUONG_MOI_WARP).map(|i| i * (be_rong + 1)).collect()
}

// ============================================================================
// 5. RÚT GỌN SONG SONG
// ============================================================================
// Cộng n số: CPU tuần tự mất n bước. GPU gộp theo CÂY — mỗi bước một nửa số
// luồng cộng cặp của mình → log₂(n) bước. Đây là bài tập nền của LeetGPU.

#[derive(Debug, PartialEq)]
pub struct KetQuaRutGon {
    pub tong: i64,
    pub so_buoc: usize,
    /// Tổng số phép cộng thực hiện (bằng nhau ở cả hai cách).
    pub so_phep_cong: usize,
    /// Số luồng còn hoạt động ở bước cuối — đo mức lãng phí.
    pub luong_hoat_dong_buoc_cuoi: usize,
}

/// Rút gọn theo cây, mô phỏng đúng cách GPU làm.
pub fn rut_gon_song_song(du_lieu: &[i64]) -> KetQuaRutGon {
    if du_lieu.is_empty() {
        return KetQuaRutGon { tong: 0, so_buoc: 0, so_phep_cong: 0,
                              luong_hoat_dong_buoc_cuoi: 0 };
    }
    let mut tang: Vec<i64> = du_lieu.to_vec();
    let mut so_buoc = 0;
    let mut so_phep_cong = 0;
    let mut cuoi = tang.len();
    while tang.len() > 1 {
        let mut tren = Vec::with_capacity(tang.len().div_ceil(2));
        for cap in tang.chunks(2) {
            if cap.len() == 2 { so_phep_cong += 1; }
            tren.push(cap[0] + cap.get(1).copied().unwrap_or(0));
        }
        cuoi = tang.len() / 2;
        tang = tren;
        so_buoc += 1;
    }
    KetQuaRutGon {
        tong: tang[0], so_buoc, so_phep_cong,
        luong_hoat_dong_buoc_cuoi: cuoi.max(1),
    }
}

pub fn rut_gon_tuan_tu(du_lieu: &[i64]) -> i64 { du_lieu.iter().sum() }

/// Số bước lý thuyết của rút gọn cây.
pub fn so_buoc_rut_gon(n: usize) -> usize {
    if n <= 1 { return 0; }
    (n as f64).log2().ceil() as usize
}

// ============================================================================
// 6. NHÂN MA TRẬN THEO LÁT
// ============================================================================
// Bản ngây thơ: mỗi luồng đọc cả một hàng và một cột từ bộ nhớ toàn cục —
// mỗi phần tử bị đọc lại n lần. Bản theo lát: cả khối cùng nạp một lát vào
// bộ nhớ chia sẻ, rồi mọi luồng dùng chung. Số lần đọc toàn cục giảm `lat` lần.

#[derive(Debug, PartialEq)]
pub struct PhanTichGemm {
    pub n: usize,
    pub doc_toan_cuc: u64,
    pub doc_chia_se: u64,
    pub so_phep_nhan: u64,
    /// Số phép tính trên mỗi byte đọc từ bộ nhớ toàn cục. Càng cao càng tốt —
    /// đây là con số quyết định bài toán bị chặn bởi TÍNH hay bởi BỘ NHỚ.
    pub cuong_do_tinh_toan: f64,
}

pub fn gemm_ngay_tho(n: usize) -> PhanTichGemm {
    let n64 = n as u64;
    // Mỗi phần tử kết quả cần đọc n phần tử của A và n của B, tất cả từ toàn cục
    let doc = 2 * n64 * n64 * n64;
    PhanTichGemm {
        n, doc_toan_cuc: doc, doc_chia_se: 0,
        so_phep_nhan: n64 * n64 * n64,
        cuong_do_tinh_toan: n64.pow(3) as f64 / (doc * 4) as f64, // 4 byte mỗi f32
    }
}

pub fn gemm_theo_lat(n: usize, lat: usize) -> PhanTichGemm {
    let n64 = n as u64;
    let l = lat.max(1) as u64;
    // Mỗi lát được nạp một lần rồi dùng lại `lat` lần bởi cả khối
    let doc_toan_cuc = 2 * n64 * n64 * n64 / l;
    let doc_chia_se = 2 * n64 * n64 * n64;
    PhanTichGemm {
        n, doc_toan_cuc, doc_chia_se,
        so_phep_nhan: n64 * n64 * n64,
        cuong_do_tinh_toan: n64.pow(3) as f64 / (doc_toan_cuc * 4) as f64,
    }
}

// ============================================================================
// 7. MỨC CHIẾM DỤNG
// ============================================================================
// Mỗi bộ xử lý đa luồng có giới hạn: số thanh ghi, dung lượng bộ nhớ chia sẻ,
// số warp đồng thời. Dùng quá nhiều thanh ghi cho mỗi luồng → ít warp cùng
// chạy → không đủ việc để che độ trễ bộ nhớ.

#[derive(Debug, PartialEq)]
pub struct MucChiemDung {
    pub warp_dong_thoi: usize,
    pub warp_toi_da: usize,
    pub ty_le: f64,
    pub bi_chan_boi: &'static str,
}

pub fn tinh_muc_chiem_dung(luong_moi_khoi: usize, thanh_ghi_moi_luong: usize,
                           chia_se_moi_khoi_byte: usize) -> MucChiemDung
{
    const WARP_TOI_DA: usize = 64;
    const THANH_GHI_MOI_SM: usize = 65_536;
    const CHIA_SE_MOI_SM: usize = 65_536;
    const KHOI_TOI_DA: usize = 32;

    let l = luong_moi_khoi.max(1);
    let warp_moi_khoi = l.div_ceil(LUONG_MOI_WARP);

    let khoi_theo_thanh_ghi = if thanh_ghi_moi_luong == 0 { KHOI_TOI_DA }
        else { THANH_GHI_MOI_SM / (l * thanh_ghi_moi_luong).max(1) };
    let khoi_theo_chia_se = if chia_se_moi_khoi_byte == 0 { KHOI_TOI_DA }
        else { CHIA_SE_MOI_SM / chia_se_moi_khoi_byte };
    let khoi_theo_warp = WARP_TOI_DA / warp_moi_khoi.max(1);

    let (so_khoi, chan) = [(khoi_theo_thanh_ghi, "thanh ghi"),
                           (khoi_theo_chia_se, "bộ nhớ chia sẻ"),
                           (khoi_theo_warp, "số warp"),
                           (KHOI_TOI_DA, "số khối")]
        .into_iter().min_by_key(|(v, _)| *v).unwrap();

    let warp_dong_thoi = (so_khoi * warp_moi_khoi).min(WARP_TOI_DA);
    MucChiemDung {
        warp_dong_thoi, warp_toi_da: WARP_TOI_DA,
        ty_le: warp_dong_thoi as f64 / WARP_TOI_DA as f64,
        bi_chan_boi: chan,
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LẬP TRÌNH GPU: SIMT · PHÂN KỲ · GỘP · LÁT · CHIẾM DỤNG  ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. CẤU HÌNH PHÁT");
    for (n, l) in [(1_000_000usize, 256usize), (1_000_000, 128), (1_000, 256), (100, 256)] {
        let c = CauHinhPhat::cho_n_phan_tu(n, l);
        println!("   n={:>9} khối {:>3} luồng → {:>5} khối · {:>9} luồng · thừa {:>5}",
                 n, l, c.so_khoi, c.tong_luong(), c.luong_thua(n));
    }
    let le = CauHinhPhat { so_khoi: 10, luong_moi_khoi: 100 };
    println!("   Khối 100 luồng → {} warp, lãng phí {} làn ở warp cuối",
             le.warp_moi_khoi(), le.luong_lang_phi_moi_khoi());
    println!("   → Luôn chọn số luồng mỗi khối là bội số của {}.", LUONG_MOI_WARP);

    println!("\n2. PHÂN KỲ WARP — cùng tỉ lệ 50/50, khác hẳn tốc độ");
    let n = 1024;
    for (ten, dk) in [("rẽ theo chẵn/lẻ", dieu_kien_theo_chan_le(n)),
                      ("rẽ theo warp   ", dieu_kien_theo_warp(n))] {
        let p = phan_tich_phan_ky(&dk);
        let ti_le_dung = dk.iter().filter(|&&x| x).count() as f64 / n as f64;
        println!("   {} → {:>2}/{} warp phân kỳ · chậm {:.1}x (tỉ lệ nhánh đúng {:.0}%)",
                 ten, p.so_warp_phan_ky, p.so_warp, p.he_so_cham, ti_le_dung * 100.0);
    }
    println!("   → Cùng 50% luồng đi mỗi nhánh. Chỉ khác CÁCH NHÓM chúng.");

    println!("\n3. GỘP TRUY CẬP BỘ NHỚ (một warp đọc f32)");
    println!("   {:>10} {:>14} {:>14} {:>12}",
             "bước nhảy", "giao dịch", "byte chuyển", "hiệu suất");
    for b in [1usize, 2, 4, 8, 32] {
        let p = phan_tich_gop(LUONG_MOI_WARP, 4, b);
        println!("   {:>10} {:>14} {:>14} {:>11.1}%",
                 b, p.so_giao_dich, p.byte_da_chuyen, p.hieu_suat * 100.0);
    }
    println!("   → Bước nhảy 32 tốn {} giao dịch cho cùng {} byte có ích.",
             phan_tich_gop(32, 4, 32).so_giao_dich, 32 * 4);

    println!("\n4. XUNG ĐỘT NGÂN HÀNG BỘ NHỚ CHIA SẺ");
    let a = phan_tich_ngan_hang(&truy_cap_cot_lat(32));
    let b = phan_tich_ngan_hang(&truy_cap_cot_lat_co_dem(32));
    println!("   Lát 32x32, đọc theo cột  → xung đột {} lối", a.muc_xung_dot);
    println!("   Lát 32x33 (đệm 1 cột)    → xung đột {} lối", b.muc_xung_dot);
    println!("   → Thêm 1/32 bộ nhớ, nhanh gấp {} lần. Thủ thuật rẻ nhất trong GPU.",
             a.muc_xung_dot / b.muc_xung_dot.max(1));

    println!("\n5. RÚT GỌN SONG SONG");
    println!("   {:>10} {:>14} {:>16} {:>16}",
             "phần tử", "bước (cây)", "bước (tuần tự)", "phép cộng");
    for n in [16usize, 1024, 1_048_576] {
        let d: Vec<i64> = (1..=n as i64).collect();
        let r = rut_gon_song_song(&d);
        println!("   {:>10} {:>14} {:>16} {:>16}", n, r.so_buoc, n, r.so_phep_cong);
    }
    let d: Vec<i64> = (1..=1000).collect();
    println!("   Cùng kết quả với cách tuần tự: {}",
             rut_gon_song_song(&d).tong == rut_gon_tuan_tu(&d));
    println!("   → Cùng số phép cộng, nhưng 20 bước thay vì một triệu bước.");

    println!("\n6. NHÂN MA TRẬN THEO LÁT (n = 1024)");
    let n = 1024;
    let nt = gemm_ngay_tho(n);
    println!("   {:<18} {:>18} {:>24}", "cách làm", "đọc toàn cục", "cường độ tính toán");
    println!("   {:<18} {:>18} {:>21.2} FLOP/B", "ngây thơ", nt.doc_toan_cuc,
             nt.cuong_do_tinh_toan);
    for lat in [8usize, 16, 32] {
        let g = gemm_theo_lat(n, lat);
        println!("   {:<18} {:>18} {:>21.2} FLOP/B",
                 format!("lát {}x{}", lat, lat), g.doc_toan_cuc, g.cuong_do_tinh_toan);
    }
    println!("   → Cùng {} phép nhân. Lát 32 đọc ít hơn 32 lần từ bộ nhớ toàn cục.",
             nt.so_phep_nhan);

    println!("\n7. MỨC CHIẾM DỤNG");
    println!("   {:>8} {:>10} {:>14} {:>12} {:>18}",
             "luồng", "thanh ghi", "chia sẻ (B)", "chiếm dụng", "bị chặn bởi");
    for (l, tg, cs) in [(256usize, 32usize, 0usize), (256, 64, 0),
                        (256, 128, 0), (256, 32, 16_384), (1024, 32, 0)] {
        let m = tinh_muc_chiem_dung(l, tg, cs);
        println!("   {:>8} {:>10} {:>14} {:>11.0}% {:>18}",
                 l, tg, cs, m.ty_le * 100.0, m.bi_chan_boi);
    }
    println!("   → Dùng nhiều thanh ghi cho mỗi luồng thì ít warp cùng chạy,");
    println!("     và GPU không còn đủ việc để che độ trễ bộ nhớ.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GPU KHÔNG NHANH HƠN — NÓ RỘNG HƠN. PHẢI CHO NÓ ĐỦ VIỆC.  ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    // ---------- Cấu hình phát ----------
    #[test]
    fn cau_hinh_phat_phu_het_phan_tu() {
        for (n, l) in [(1_000_000usize, 256usize), (1_000, 256), (1, 256), (257, 256)] {
            let c = CauHinhPhat::cho_n_phan_tu(n, l);
            assert!(c.tong_luong() >= n, "phải đủ luồng phủ hết {} phần tử", n);
            assert!(c.tong_luong() < n + l, "nhưng không được thừa quá một khối");
        }
    }

    #[test]
    fn phat_khong_phan_tu_nao_thi_khong_can_khoi() {
        let c = CauHinhPhat::cho_n_phan_tu(0, 256);
        assert_eq!(c.so_khoi, 0);
        assert_eq!(c.tong_luong(), 0);
    }

    #[test]
    fn khoi_khong_boi_so_warp_thi_lang_phi_lan() {
        let tron = CauHinhPhat { so_khoi: 1, luong_moi_khoi: 256 };
        assert_eq!(tron.warp_moi_khoi(), 8);
        assert_eq!(tron.luong_lang_phi_moi_khoi(), 0);
        let le = CauHinhPhat { so_khoi: 1, luong_moi_khoi: 100 };
        assert_eq!(le.warp_moi_khoi(), 4, "100 luồng vẫn tốn 4 warp");
        assert_eq!(le.luong_lang_phi_moi_khoi(), 28, "28 làn ngồi chơi");
    }

    #[test]
    fn moi_kich_thuoc_khoi_boi_so_32_deu_khong_lang_phi() {
        for l in [32usize, 64, 128, 256, 512, 1024] {
            let c = CauHinhPhat { so_khoi: 1, luong_moi_khoi: l };
            assert_eq!(c.luong_lang_phi_moi_khoi(), 0, "khối {} luồng", l);
        }
    }

    // ---------- Phân kỳ warp ----------
    #[test]
    fn warp_dong_nhat_thi_khong_phan_ky() {
        let toan_dung = vec![true; 256];
        let p = phan_tich_phan_ky(&toan_dung);
        assert_eq!(p.so_warp_phan_ky, 0);
        assert!((p.he_so_cham - 1.0).abs() < 1e-9);
        let toan_sai = vec![false; 256];
        assert_eq!(phan_tich_phan_ky(&toan_sai).so_warp_phan_ky, 0);
    }

    #[test]
    fn re_nhanh_theo_chan_le_lam_moi_warp_phan_ky() {
        let p = phan_tich_phan_ky(&dieu_kien_theo_chan_le(1024));
        assert_eq!(p.so_warp, 32);
        assert_eq!(p.so_warp_phan_ky, 32, "warp nào cũng có cả luồng chẵn lẫn lẻ");
        assert!((p.he_so_cham - 2.0).abs() < 1e-9, "chậm gấp đôi");
    }

    #[test]
    fn re_nhanh_theo_warp_thi_khong_phan_ky_chut_nao() {
        // Bài học trung tâm: cùng tỉ lệ 50/50, chỉ khác CÁCH NHÓM.
        let dk = dieu_kien_theo_warp(1024);
        let p = phan_tich_phan_ky(&dk);
        assert_eq!(p.so_warp_phan_ky, 0);
        assert!((p.he_so_cham - 1.0).abs() < 1e-9);
        let dung = dk.iter().filter(|&&x| x).count();
        assert_eq!(dung, 512, "vẫn đúng một nửa số luồng đi nhánh đúng");
    }

    #[test]
    fn chi_mot_luong_lac_dieu_cung_lam_ca_warp_phan_ky() {
        // Đây là điều khiến phân kỳ nguy hiểm: một luồng đủ để phạt cả 32.
        let mut dk = vec![true; 32];
        dk[17] = false;
        let p = phan_tich_phan_ky(&dk);
        assert_eq!(p.so_warp_phan_ky, 1);
        assert!((p.he_so_cham - 2.0).abs() < 1e-9,
                "một luồng lạc điệu → cả warp chậm gấp đôi");
    }

    #[test]
    fn danh_sach_rong_khong_panic() {
        let p = phan_tich_phan_ky(&[]);
        assert_eq!(p.so_warp, 0);
        assert_eq!(p.he_so_cham, 1.0);
    }

    // ---------- Gộp truy cập ----------
    #[test]
    fn truy_cap_lien_tuc_gop_thanh_it_giao_dich_nhat() {
        let p = phan_tich_gop(LUONG_MOI_WARP, 4, 1);
        assert_eq!(p.so_giao_dich, 1, "32 luồng x 4 byte = 128 byte = đúng 1 giao dịch");
        assert!((p.hieu_suat - 1.0).abs() < 1e-9, "hiệu suất băng thông hoàn hảo");
    }

    #[test]
    fn buoc_nhay_cang_lon_thi_cang_nhieu_giao_dich() {
        let mut truoc = 0;
        for b in [1usize, 2, 4, 8, 16, 32] {
            let p = phan_tich_gop(LUONG_MOI_WARP, 4, b);
            assert!(p.so_giao_dich >= truoc, "bước {} phải tốn ít nhất bằng bước trước", b);
            truoc = p.so_giao_dich;
        }
        assert_eq!(phan_tich_gop(LUONG_MOI_WARP, 4, 32).so_giao_dich, 32,
                   "bước nhảy 32 → mỗi luồng một giao dịch riêng");
    }

    #[test]
    fn byte_co_ich_khong_doi_du_buoc_nhay_thay_doi() {
        // Cùng lượng dữ liệu CẦN, khác hẳn lượng dữ liệu PHẢI CHUYỂN.
        for b in [1usize, 4, 32] {
            let p = phan_tich_gop(LUONG_MOI_WARP, 4, b);
            assert_eq!(p.byte_co_ich, 128, "luôn cần đúng 128 byte");
        }
        assert!(phan_tich_gop(32, 4, 32).byte_da_chuyen
                > phan_tich_gop(32, 4, 1).byte_da_chuyen * 30);
    }

    #[test]
    fn hieu_suat_luon_trong_khoang_khong_den_mot() {
        for b in [1usize, 2, 3, 7, 16, 64, 128] {
            let p = phan_tich_gop(LUONG_MOI_WARP, 4, b);
            assert!((0.0..=1.0).contains(&p.hieu_suat),
                    "bước {} cho hiệu suất {}", b, p.hieu_suat);
        }
    }

    // ---------- Xung đột ngân hàng ----------
    #[test]
    fn truy_cap_lien_tuc_khong_xung_dot() {
        let chi_so: Vec<usize> = (0..LUONG_MOI_WARP).collect();
        let p = phan_tich_ngan_hang(&chi_so);
        assert_eq!(p.muc_xung_dot, 1);
        assert!(!p.co_xung_dot, "32 luồng vào 32 ngân hàng khác nhau");
    }

    #[test]
    fn doc_cot_lat_32_gay_xung_dot_toan_phan() {
        let p = phan_tich_ngan_hang(&truy_cap_cot_lat(32));
        assert_eq!(p.muc_xung_dot, 32, "mọi luồng rơi vào CÙNG một ngân hàng");
        assert!(p.co_xung_dot);
    }

    #[test]
    fn dem_them_mot_cot_xoa_sach_xung_dot() {
        // Thủ thuật rẻ nhất trong lập trình GPU: tốn thêm 1/32 bộ nhớ,
        // đổi lấy tốc độ gấp 32 lần.
        let p = phan_tich_ngan_hang(&truy_cap_cot_lat_co_dem(32));
        assert_eq!(p.muc_xung_dot, 1);
        assert!(!p.co_xung_dot);
    }

    #[test]
    fn xung_dot_phu_thuoc_uoc_chung_voi_so_ngan_hang() {
        // Bề rộng nguyên tố cùng nhau với 32 thì không xung đột.
        for be_rong in [1usize, 3, 33, 65] {
            let p = phan_tich_ngan_hang(&truy_cap_cot_lat(be_rong));
            assert!(!p.co_xung_dot, "bề rộng {} không nên xung đột", be_rong);
        }
        // Bề rộng chẵn có ước chung với 32 thì xung đột
        for be_rong in [2usize, 4, 8, 16, 32] {
            assert!(phan_tich_ngan_hang(&truy_cap_cot_lat(be_rong)).co_xung_dot,
                    "bề rộng {} phải xung đột", be_rong);
        }
    }

    // ---------- Rút gọn ----------
    #[test]
    fn rut_gon_song_song_cho_cung_ket_qua_voi_tuan_tu() {
        // Bất biến sống còn: song song hoá không được đổi kết quả.
        for n in [0usize, 1, 2, 3, 7, 16, 17, 1000, 4096] {
            let d: Vec<i64> = (1..=n as i64).collect();
            assert_eq!(rut_gon_song_song(&d).tong, rut_gon_tuan_tu(&d), "n={}", n);
        }
    }

    #[test]
    fn so_buoc_la_log_chu_khong_tuyen_tinh() {
        for n in [2usize, 4, 16, 1024, 1_048_576] {
            let d: Vec<i64> = vec![1; n];
            let r = rut_gon_song_song(&d);
            assert_eq!(r.so_buoc, so_buoc_rut_gon(n), "n={}", n);
            assert!(r.so_buoc < 25, "một triệu phần tử chỉ tốn 20 bước");
        }
    }

    #[test]
    fn so_phep_cong_van_la_n_tru_mot() {
        // Song song hoá KHÔNG làm ít việc hơn — nó chỉ làm việc song song.
        for n in [2usize, 8, 100, 1024] {
            let d: Vec<i64> = vec![1; n];
            assert_eq!(rut_gon_song_song(&d).so_phep_cong, n - 1, "n={}", n);
        }
    }

    #[test]
    fn rut_gon_mang_rong_va_mot_phan_tu() {
        assert_eq!(rut_gon_song_song(&[]).tong, 0);
        assert_eq!(rut_gon_song_song(&[]).so_buoc, 0);
        assert_eq!(rut_gon_song_song(&[42]).tong, 42);
        assert_eq!(rut_gon_song_song(&[42]).so_buoc, 0);
    }

    #[test]
    fn rut_gon_dung_voi_so_luong_le() {
        // Số lẻ phần tử là chỗ dễ sai nhất: phần tử cuối không có cặp.
        let d = vec![1i64, 2, 3, 4, 5, 6, 7];
        assert_eq!(rut_gon_song_song(&d).tong, 28);
    }

    // ---------- GEMM theo lát ----------
    #[test]
    fn lat_giam_so_lan_doc_toan_cuc() {
        let n = 1024;
        let nt = gemm_ngay_tho(n);
        let mut truoc = nt.doc_toan_cuc;
        for lat in [8usize, 16, 32] {
            let g = gemm_theo_lat(n, lat);
            assert!(g.doc_toan_cuc < truoc, "lát {} phải đọc ít hơn", lat);
            truoc = g.doc_toan_cuc;
        }
        assert_eq!(gemm_theo_lat(n, 32).doc_toan_cuc, nt.doc_toan_cuc / 32);
    }

    #[test]
    fn lat_khong_lam_doi_so_phep_nhan() {
        // Tối ưu không được đổi khối lượng TÍNH TOÁN, chỉ đổi cách chạm bộ nhớ.
        let n = 512;
        let nt = gemm_ngay_tho(n);
        for lat in [1usize, 8, 16, 32] {
            assert_eq!(gemm_theo_lat(n, lat).so_phep_nhan, nt.so_phep_nhan);
        }
    }

    #[test]
    fn cuong_do_tinh_toan_tang_theo_kich_thuoc_lat() {
        let n = 1024;
        let mut truoc = gemm_ngay_tho(n).cuong_do_tinh_toan;
        for lat in [8usize, 16, 32] {
            let c = gemm_theo_lat(n, lat).cuong_do_tinh_toan;
            assert!(c > truoc, "lát {} phải cho cường độ cao hơn", lat);
            truoc = c;
        }
    }

    #[test]
    fn lat_bang_mot_thi_khong_khac_gi_ngay_tho() {
        let n = 256;
        assert_eq!(gemm_theo_lat(n, 1).doc_toan_cuc, gemm_ngay_tho(n).doc_toan_cuc);
        assert_eq!(gemm_theo_lat(n, 0).doc_toan_cuc, gemm_ngay_tho(n).doc_toan_cuc,
                   "lát 0 phải được chặn thành 1, không chia cho 0");
    }

    // ---------- Mức chiếm dụng ----------
    #[test]
    fn it_thanh_ghi_thi_chiem_dung_cao() {
        let m = tinh_muc_chiem_dung(256, 32, 0);
        assert!(m.ty_le > 0.9, "32 thanh ghi/luồng phải cho chiếm dụng cao, thực tế {:.2}",
                m.ty_le);
    }

    #[test]
    fn nhieu_thanh_ghi_thi_chiem_dung_tut() {
        let it = tinh_muc_chiem_dung(256, 32, 0);
        let nhieu = tinh_muc_chiem_dung(256, 128, 0);
        assert!(nhieu.ty_le < it.ty_le,
                "dùng 128 thanh ghi phải giảm chiếm dụng: {:.2} so với {:.2}",
                nhieu.ty_le, it.ty_le);
        assert_eq!(nhieu.bi_chan_boi, "thanh ghi");
    }

    #[test]
    fn bo_nho_chia_se_cung_co_the_thanh_nut_that() {
        let m = tinh_muc_chiem_dung(256, 32, 32_768); // nửa dung lượng chia sẻ mỗi khối
        assert_eq!(m.bi_chan_boi, "bộ nhớ chia sẻ");
        assert!(m.ty_le < 0.5);
    }

    #[test]
    fn muc_chiem_dung_luon_trong_khoang_hop_le() {
        for l in [32usize, 128, 256, 512, 1024] {
            for tg in [16usize, 32, 64, 128, 255] {
                let m = tinh_muc_chiem_dung(l, tg, 0);
                assert!((0.0..=1.0).contains(&m.ty_le),
                        "luồng {} thanh ghi {} cho tỉ lệ {}", l, tg, m.ty_le);
                assert!(m.warp_dong_thoi <= m.warp_toi_da);
            }
        }
    }

    #[test]
    fn khong_dung_thanh_ghi_hay_chia_se_thi_bi_chan_boi_so_warp() {
        let m = tinh_muc_chiem_dung(1024, 0, 0);
        assert_eq!(m.ty_le, 1.0, "không có ràng buộc nào thì chiếm dụng tối đa");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0308: expected usize, found u32` | Chỉ số luồng thường là `u32` trong CUDA | Ép kiểu ở biên; giữ `usize` cho chỉ số Rust |
| Kết quả rút gọn sai | Thiếu rào chắn đồng bộ giữa các bước | Mỗi bước phải có `__syncthreads()` tương đương |
| Nhân chậm bất thường | Truy cập không gộp | Kiểm bước nhảy: luồng i phải đọc phần tử i |
| Chậm gấp 32 lần khi dùng bộ nhớ chia sẻ | Xung đột ngân hàng | Đệm `[N][N+1]` thay vì `[N][N]` |
| `attempt to divide by zero` | Tính mức chiếm dụng với thanh ghi = 0 | Bảo vệ biên trong hàm ước lượng |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **GPU là SIMT, không phải nhiều CPU nhỏ.** 32 luồng trong warp chạy cùng một lệnh, không có ngoại lệ.
2. **Phân kỳ warp chậm theo số nhánh.** Viết điều kiện sao cho đồng nhất trong warp.
3. **Gộp truy cập là yếu tố hiệu năng số một.** Không gộp là chậm 32 lần, và đó thường là lỗi duy nhất.
4. **Mẹo đệm +1 xoá xung đột ngân hàng** — một phần tử thừa đổi lấy 32 lần tốc độ.
5. **Chia lát tăng cường độ số học** từ "bị chặn bởi bộ nhớ" lên "bị chặn bởi sức tính" — đó là toàn bộ mục tiêu.

### Bài tập rèn luyện

**Bài 1.** Cài **histogram GPU** — bài toán khó vì nhiều luồng cùng cập nhật một ô đếm.

<details>
<summary><b>Gợi ý</b></summary>

Cách ngây thơ dùng `atomicAdd` trên bộ nhớ toàn cục và bị tuần tự hoá nặng khi dữ liệu lệch (nhiều giá trị rơi vào cùng thùng). Cách chuẩn: mỗi khối giữ một histogram **riêng** trong bộ nhớ chia sẻ, rồi gộp lại ở cuối. Xung đột nguyên tử chỉ còn trong phạm vi khối.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct MoPhongHistogram {
    pub so_thung: usize,
    pub so_luong_khoi: usize,
    pub luong_moi_khoi: usize,
}

#[derive(Debug)]
pub struct KetQuaHistogram {
    pub dem: Vec<u32>,
    /// Số lần va chạm nguyên tử — chỉ số hiệu năng chính.
    pub va_cham_nguyen_tu: u64,
}

impl MoPhongHistogram {
    /// Cách ngây thơ: mọi luồng nguyên tử lên bộ nhớ TOÀN CỤC.
    pub fn toan_cuc(&self, du_lieu: &[u32]) -> KetQuaHistogram {
        let mut dem = vec![0u32; self.so_thung];
        let mut va_cham = 0u64;
        for lo in du_lieu.chunks(32) {          // mỗi warp
            let mut trong_warp = std::collections::HashMap::new();
            for &x in lo {
                let t = (x as usize) % self.so_thung;
                *trong_warp.entry(t).or_insert(0u64) += 1;
                dem[t] += 1;
            }
            // Nhiều luồng cùng thùng trong một warp = tuần tự hoá
            for (_, n) in trong_warp { if n > 1 { va_cham += n - 1; } }
        }
        KetQuaHistogram { dem, va_cham_nguyen_tu: va_cham }
    }

    /// Cách chuẩn: histogram riêng cho mỗi khối trong bộ nhớ chia sẻ.
    pub fn theo_khoi(&self, du_lieu: &[u32]) -> KetQuaHistogram {
        let mut tong = vec![0u32; self.so_thung];
        let mut va_cham = 0u64;
        for khoi in du_lieu.chunks(self.luong_moi_khoi) {
            let mut cuc_bo = vec![0u32; self.so_thung];   // bộ nhớ chia sẻ
            for &x in khoi { cuc_bo[(x as usize) % self.so_thung] += 1; }
            // Chỉ so_thung phép nguyên tử toàn cục cho CẢ khối
            for (i, v) in cuc_bo.iter().enumerate() {
                if *v > 0 { tong[i] += v; va_cham += 1; }
            }
        }
        KetQuaHistogram { dem: tong, va_cham_nguyen_tu: va_cham }
    }
}
```

Với 1 triệu phần tử, 256 thùng và khối 256 luồng: cách toàn cục có 1 triệu phép nguyên tử; cách theo khối có khoảng 4096×256 ≈ 1 triệu lần cập nhật **chia sẻ** (rẻ) cộng 4096×256 phép nguyên tử toàn cục. Trên phần cứng thật, chênh lệch thường là 5–10 lần.
</details>

**Bài 2.** Cài **phân tích rút gọn ở cấp warp** dùng lệnh trao đổi (`__shfl_down_sync`) thay cho bộ nhớ chia sẻ.

<details>
<summary><b>Gợi ý</b></summary>

Từ kiến trúc Kepler trở đi, các luồng trong cùng warp trao đổi thanh ghi trực tiếp mà **không qua bộ nhớ** và **không cần rào chắn đồng bộ**. Rút gọn 32 phần tử chỉ mất 5 bước trao đổi, nhanh hơn hẳn đường qua bộ nhớ chia sẻ.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
/// Mô phỏng __shfl_down_sync: luồng `lan` nhận giá trị của luồng `lan + delta`.
pub fn shfl_down(gia_tri_warp: &[f32; 32], delta: usize) -> [f32; 32] {
    let mut ra = *gia_tri_warp;
    for lan in 0..32 {
        ra[lan] = if lan + delta < 32 { gia_tri_warp[lan + delta] } else { gia_tri_warp[lan] };
    }
    ra
}

/// Rút gọn cả warp trong 5 bước. Kết quả nằm ở luồng 0.
/// KHÔNG dùng bộ nhớ chia sẻ, KHÔNG cần __syncthreads().
pub fn rut_gon_warp(mut v: [f32; 32]) -> f32 {
    for delta in [16, 8, 4, 2, 1] {
        let nhan = shfl_down(&v, delta);
        for lan in 0..32 { v[lan] += nhan[lan]; }
    }
    v[0]
}

#[derive(Debug)]
pub struct SoSanhRutGon {
    pub buoc_qua_bo_nho_chia_se: usize,
    pub buoc_trao_doi_warp: usize,
    pub rao_chan_can_thiet: usize,
}

pub fn so_sanh_cach_rut_gon(so_luong: usize) -> SoSanhRutGon {
    let buoc = (so_luong as f64).log2().ceil() as usize;
    SoSanhRutGon {
        buoc_qua_bo_nho_chia_se: buoc,
        // 5 bước cuối (trong phạm vi warp) làm bằng trao đổi thanh ghi
        buoc_trao_doi_warp: buoc.saturating_sub(5),
        // Trao đổi trong warp KHÔNG cần rào chắn — đó là cái lợi chính
        rao_chan_can_thiet: buoc.saturating_sub(5),
    }
}
```

Với khối 256 luồng: cách qua bộ nhớ chia sẻ cần 8 bước và 8 rào chắn. Cách lai — 3 bước chia sẻ rồi 5 bước trao đổi warp — chỉ cần 3 rào chắn. Trên nhân bị chặn bởi rút gọn, chênh lệch thường là 20–30%.
</details>
