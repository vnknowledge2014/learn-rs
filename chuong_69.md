# Chương 69: Hệ thống giao dịch thuật toán — Sổ Lệnh, Khớp Lệnh & Kiểm Định Chiến Lược (Algorithmic Trading Systems)

## Giới thiệu & Mục tiêu học tập

> ⚠️ **Đây là tài liệu kỹ thuật, không phải lời khuyên đầu tư.** Mọi số liệu trong chương là dữ liệu giả lập tất định, sinh bằng bộ số giả ngẫu nhiên có hạt giống cố định. Mục đích duy nhất là dạy kiến trúc phần mềm.

Các nền tảng giao dịch mã nguồn mở như [OpenAlgo](https://github.com/marketcalls/openalgo) là những **sản phẩm** hoàn chỉnh: hàng chục trình kết nối môi giới, giao diện web, trình dựng chiến lược không cần lập trình. Phần lớn khối lượng công việc của chúng là **tích hợp** — quan trọng, nhưng không dạy được nhiều.

Chương này đi vào phần còn lại, phần **kỹ thuật** — và đó cũng chính là phần mà Rust thắng thuyết phục:

| | Ngôn ngữ có bộ dọn rác | Rust |
|---|---|---|
| Độ trễ **trung bình** | tốt | tốt |
| Độ trễ **phân vị 99,9** | tệ (dọn rác chen ngang) | tốt |
| Có **trần** độ trễ không? | không | **có** |

Trong giao dịch, phân vị 99,9 mới là con số quan trọng. Một cú dừng dọn rác 50 ms xảy ra đúng lúc thị trường biến động là lúc bạn mất tiền — và nó luôn xảy ra đúng lúc đó, vì thị trường biến động chính là lúc hệ thống cấp phát nhiều bộ nhớ nhất.

Chương này cũng là **bài tổng hợp** của nhiều chương trước: typestate của Chương 20 dùng cho vòng đời lệnh, vị nhóm của Chương 18 dùng cho gộp lãi/lỗ, `BTreeMap` của Chương 29 làm sổ lệnh, và nguyên tắc hàm thuần túy của Chương 13 làm nền cho bộ kiểm định.

Mục tiêu học tập:
- Hiểu vì sao **tiền phải là số nguyên**, và tự tay thấy `f64` sai ở đâu.
- Xây **sổ lệnh giới hạn** với ưu tiên **giá–thời gian**, và hiểu quy tắc cải thiện giá.
- Cài **động cơ khớp lệnh** và kiểm chứng bất biến sống còn: khối lượng được bảo toàn.
- Dùng **typestate** để lệnh chưa qua kiểm tra rủi ro **không thể** gửi đi được.
- Nhận ra **vị thế là một vị nhóm**, và vì sao điều đó cho phép gộp song song an toàn.
- Viết **bộ kiểm định chiến lược** không nhìn trộm tương lai, có mô hình phí và trượt giá.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────┐
│   HÌNH TƯỢNG: SỔ LỆNH = BẢNG RAO VẶT Ở CHỢ ĐẦU MỐI                          │
├──────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│   NGƯỜI MUỐN MUA (ghi giá mình SẴN SÀNG TRẢ — càng cao càng dễ mua)         │
│   ┌────────────────────────────────────────────────────────┐                │
│   │  84.00  ×100  (bác An, dán lúc 9:00)  ← TỐT NHẤT      │                │
│   │  84.00  ×200  (chị Bình, dán lúc 9:05)                 │                │
│   │  83.90  ×500  (anh Cường)                              │                │
│   └────────────────────────────────────────────────────────┘                │
│              ↕  CHÊNH LỆCH = 20 xu — chi phí ẩn của mọi giao dịch           │
│   ┌────────────────────────────────────────────────────────┐                │
│   │  84.20  ×150  (cô Dung)               ← TỐT NHẤT      │                │
│   │  84.30  ×300  (chú Em)                                 │                │
│   └────────────────────────────────────────────────────────┘                │
│   NGƯỜI MUỐN BÁN (ghi giá mình CHỊU BÁN — càng thấp càng dễ bán)            │
│                                                                              │
│   HAI QUY TẮC, THEO ĐÚNG THỨ TỰ NÀY:                                        │
│     1️⃣ ƯU TIÊN GIÁ    — ai trả cao hơn được mua trước. Luôn luôn.           │
│     2️⃣ ƯU TIÊN THỜI GIAN — CÙNG giá thì ai dán trước được trước.            │
│                                                                              │
│   → Bác An (9:00) khớp hết TRƯỚC chị Bình (9:05), dù cùng giá 84.00.        │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│   CẢI THIỆN GIÁ = NGƯỜI ĐẾN SAU KHÔNG BAO GIỜ BỊ THIỆT                      │
│                                                                              │
│   Cô Dung dán bán 84.20. Bạn tới, sẵn sàng trả tới 85.00.                   │
│   → Bạn trả 84.20, KHÔNG PHẢI 85.00.                                        │
│   Giá khớp luôn là giá của tờ ĐÃ DÁN SẴN, không phải giá bạn chào.          │
│                                                                              │
├──────────────────────────────────────────────────────────────────────────────┤
│   VÌ SAO TIỀN KHÔNG ĐƯỢC LÀ SỐ THỰC                                          │
│                                                                              │
│   Cộng 0.1 mười lần bằng f64 → 0.99999999999999988898                       │
│   Bằng 1.0? → KHÔNG.                                                        │
│                                                                              │
│   Sai một phần tỉ, nhân với 10 triệu lệnh mỗi ngày = một vụ kiện.           │
│   Ngành tài chính đếm bằng ĐƠN VỊ NHỎ NHẤT: xu, tick, satoshi.              │
│   Số nguyên. Không bao giờ có sai số làm tròn. Không bao giờ tranh cãi.     │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Vì sao `BTreeMap` là cấu trúc đúng cho sổ lệnh

Sổ lệnh cần bốn thao tác, và `BTreeMap` làm tốt cả bốn:

| Thao tác | Tần suất | `BTreeMap` | `HashMap` | `Vec` đã sắp |
|---|---|---|---|---|
| Lấy giá tốt nhất | **rất cao** | O(log n) | O(n) ❌ | O(1) |
| Chèn mức giá mới | cao | O(log n) | O(1) | O(n) ❌ |
| Xóa mức giá rỗng | cao | O(log n) | O(1) | O(n) ❌ |
| Duyệt theo thứ tự giá | cao | O(1)/bước | không được ❌ | O(1)/bước |

`HashMap` thua vì nó **không có thứ tự** — mà thứ tự giá chính là bản chất của sổ lệnh.

Một mẹo nhỏ nhưng quan trọng: bên mua lưu khóa là **giá âm**. `BTreeMap` sắp tăng dần, nên `-8400 < -8390` khiến giá **cao nhất** (8400) nằm đầu — đúng thứ mà `keys().next()` cần trả về.

Ở mỗi mức giá, `VecDeque` giữ ưu tiên thời gian: `push_back` khi vào, `pop_front` khi khớp. Đó chính xác là ngữ nghĩa hàng đợi vào-trước-ra-trước mà quy tắc sàn đòi hỏi.

### 2. Quy tắc cải thiện giá và vì sao nó công bằng

Khi lệnh mua giá 120 gặp lệnh bán đã nằm sẵn giá 100, giá khớp là **100**, không phải 120. Người đến sau được hưởng giá tốt hơn mình đã chào.

Lý do không phải lòng tốt mà là **khuyến khích đúng đắn**: nếu người đến sau phải trả đúng giá mình chào, sẽ không ai dám chào giá cao — ai cũng chào sát giá thị trường, và thanh khoản biến mất. Quy tắc cải thiện giá cho phép bạn nói "tôi trả tới mức này" mà không sợ bị lợi dụng.

### 3. Typestate cho vòng đời lệnh

Hai lỗi đắt tiền nhất trong hệ thống giao dịch:
- Gửi một lệnh **hai lần** (mua gấp đôi số định mua).
- Gửi lệnh **chưa qua** kiểm tra rủi ro.

Typestate loại bỏ cả hai ở tầng biên dịch:

```rust
Lenh<DangSoan>  ──kiem_tra()──►  Lenh<DaKiemTraRuiRo>  ──gui()──►  Lenh<DaGui>
       ▲                                  ▲                            ▲
   vừa tạo ra                      đã qua hạn mức              đã vào sổ lệnh
```

`gui()` chỉ tồn tại trên `Lenh<DaKiemTraRuiRo>` và nhận `self` theo **giá trị**. Nghĩa là: không kiểm tra rủi ro thì không gọi được `gui()`, và gọi rồi thì lệnh cũ bị tiêu thụ, không gửi lại được. Cả hai lỗi trở thành lỗi biên dịch, với chi phí lúc chạy bằng không.

### 4. Vị thế là một vị nhóm — và vì sao điều đó quan trọng

`ViThe { so_luong, tien_mat }` với phép `ghep` cộng từng trường thỏa mãn:
- **Kết hợp**: `(a·b)·c = a·(b·c)`
- **Đơn vị**: `ViThe::RONG` là phần tử trung hòa hai phía

Đây đúng định nghĩa **vị nhóm** ở Chương 18. Hệ quả thực tế không hề trừu tượng:

- Gộp lãi/lỗ của 10 triệu giao dịch có thể chia cho 16 lõi CPU, mỗi lõi gộp một phần, rồi gộp 16 kết quả lại. Luật kết hợp **bảo đảm** kết quả y hệt tính tuần tự — đây là chứng minh toán học, không phải hy vọng.
- Có thể dựng **cây tổng hợp** để tính lãi/lỗ theo tài khoản, theo bàn giao dịch, theo toàn công ty, mà không cần viết lại logic ở mỗi cấp.

Chương này có bài kiểm thử chia 100 giao dịch thành từng khối 7 rồi gộp lại, so với tính tuần tự — kết quả trùng khớp tuyệt đối.

### 5. Ba cách tự lừa mình khi kiểm định chiến lược

Bộ kiểm định là nơi dễ tự dối nhất trong cả ngành. Ba lỗi phổ biến, chương này chặn cả ba:

**a) Nhìn trộm tương lai (look-ahead bias).** Ra quyết định dựa trên nến hôm nay rồi khớp ở giá **đóng cửa** của chính nến đó — nhưng lúc quyết định, giá đóng cửa chưa tồn tại. Chương này khớp ở giá **mở cửa của nến kế tiếp**, và có bài kiểm thử chứng minh nến cuối cùng **không thể** sinh giao dịch nào.

**b) Bỏ qua chi phí.** Không có phí, không có trượt giá, chiến lược nào cũng đẹp. Thực tế: bạn luôn mua đắt hơn và bán rẻ hơn giá lý thuyết. Chương này có bài kiểm thử khẳng định thêm chi phí **luôn** làm kết quả xấu đi — và demo cho thấy cùng một chiến lược đi từ lỗ 22 000 tick xuống lỗ 45 000 tick chỉ vì thêm 2 tick trượt giá và 3 tick phí.

**c) Chỉ nhìn lợi nhuận.** Con số quan trọng hơn là **sụt giảm tối đa** — mức lỗ sâu nhất tính từ đỉnh. Một chiến lược lãi 50% mỗi năm nhưng có lúc sụt 60% là chiến lược mà **không ai** đủ can đảm đi tới cuối.

Một quan sát trung thực từ chính demo của chương: chiến lược giao cắt trung bình động chạy trên dữ liệu **bước ngẫu nhiên** cho kết quả **lỗ**, và lỗ nặng hơn khi tính đủ chi phí. Điều đó đúng như lý thuyết dự đoán — trên dữ liệu không có xu hướng thật, chiến lược theo xu hướng chỉ tạo ra chi phí giao dịch. Bộ kiểm định làm đúng việc của nó: nói cho bạn sự thật khó nghe.

### 6. Có nên xây một OpenAlgo cho Rust không?

Câu trả lời thẳng thắn, để bạn tự cân nhắc:

| Phần của OpenAlgo | Có đáng viết lại bằng Rust? |
|---|---|
| 36 trình kết nối môi giới | ❌ Công việc tích hợp thuần túy. Python phù hợp hơn, và API môi giới đổi liên tục |
| Giao diện web, trình dựng chiến lược | ❌ Hệ sinh thái web front-end không phải thế mạnh so sánh của Rust |
| **Sổ lệnh, động cơ khớp lệnh** | ✅ Đúng thế mạnh: độ trễ có trần, không dọn rác |
| **Bộ kiểm định trên dữ liệu lớn** | ✅ Nhanh hơn Python hàng chục lần, lại song song hóa được nhờ vị nhóm |
| **Cổng rủi ro** | ✅ Typestate biến lỗi vận hành thành lỗi biên dịch |

Kết luận: đừng chép lại cả sản phẩm. Hãy viết **phần lõi hiệu năng** bằng Rust và để phần tích hợp cho ngôn ngữ hợp với nó hơn — đúng mô hình mà các quỹ định lượng thật đang dùng.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chạy bằng `cargo run -p ch69`, kiểm thử bằng `cargo test -p ch69`.

```rust
#![allow(dead_code)]
//! Chương 69 — Hệ thống giao dịch thuật toán: sổ lệnh, động cơ khớp lệnh,
//! quản trị rủi ro bằng kiểu, và bộ kiểm định chiến lược trên dữ liệu quá khứ.
//!
//! Đây là phần LÕI của một nền tảng kiểu OpenAlgo — nhưng viết bằng Rust, nơi
//! không có bộ dọn rác nên độ trễ có TRẦN xác định, chứ không phải trung bình đẹp
//! kèm những cú khựng bất chợt.
//!
//! ⚠️ Đây là tài liệu KỸ THUẬT, không phải lời khuyên đầu tư. Mọi số liệu đều
//! là dữ liệu giả lập tất định.

use std::collections::{BTreeMap, VecDeque};
use std::marker::PhantomData;

// ============================================================================
// 1. TIỀN LÀ SỐ NGUYÊN — sai lầm đắt giá nhất của người mới
// ============================================================================

/// KHÔNG BAO GIỜ dùng `f64` cho tiền. `0.1 + 0.2 != 0.3` trong nhị phân, và
/// sai số một xu nhân với triệu lệnh là một vụ kiện. Ngành tài chính dùng
/// SỐ NGUYÊN đơn vị nhỏ nhất — ở đây là "tick", 1 tick = 0,01 đơn vị tiền.
pub type Gia = i64;      // tính bằng tick
pub type SoLuong = i64;
pub type MaLenh = u64;

pub fn tick_sang_chuoi(t: Gia) -> String {
    format!("{}.{:02}", t / 100, (t % 100).abs())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Chieu { Mua, Ban }

impl Chieu {
    pub fn nguoc_lai(self) -> Chieu {
        match self { Chieu::Mua => Chieu::Ban, Chieu::Ban => Chieu::Mua }
    }
    /// Dấu của vị thế: mua làm vị thế tăng, bán làm giảm.
    pub fn dau(self) -> i64 { match self { Chieu::Mua => 1, Chieu::Ban => -1 } }
}

// ============================================================================
// 2. VÒNG ĐỜI LỆNH BẰNG TYPESTATE — trạng thái nằm trong KIỂU
// ============================================================================
// Áp dụng Chương 20 vào nghiệp vụ thật: gửi hai lần cùng một lệnh, hoặc hủy
// một lệnh đã khớp hết, là những lỗi tốn tiền. Ở đây chúng KHÔNG BIÊN DỊCH ĐƯỢC.

// Ba nhãn trạng thái. Chúng là kiểu RỖNG — không chiếm một byte nào lúc chạy;
// toàn bộ tác dụng của chúng diễn ra trong trình biên dịch.
#[derive(Debug, Clone, Copy)] pub struct DangSoan;
#[derive(Debug, Clone, Copy)] pub struct DaKiemTraRuiRo;
#[derive(Debug, Clone, Copy)] pub struct DaGui;

#[derive(Debug, Clone)]
pub struct Lenh<TrangThai> {
    pub ma: MaLenh,
    pub ma_ck: String,
    pub chieu: Chieu,
    pub gia: Gia,
    pub so_luong: SoLuong,
    pub da_khop: SoLuong,
    _tt: PhantomData<TrangThai>,
}

impl Lenh<DangSoan> {
    pub fn moi(ma: MaLenh, ma_ck: &str, chieu: Chieu, gia: Gia, so_luong: SoLuong) -> Self {
        Lenh { ma, ma_ck: ma_ck.to_string(), chieu, gia, so_luong, da_khop: 0, _tt: PhantomData }
    }
}

impl<TT> Lenh<TT> {
    pub fn con_lai(&self) -> SoLuong { self.so_luong - self.da_khop }
    fn chuyen<Moi>(self) -> Lenh<Moi> {
        Lenh { ma: self.ma, ma_ck: self.ma_ck, chieu: self.chieu, gia: self.gia,
               so_luong: self.so_luong, da_khop: self.da_khop, _tt: PhantomData }
    }
}

// Chỉ lệnh ĐÃ QUA kiểm tra rủi ro mới gửi được vào sổ lệnh.
impl Lenh<DaKiemTraRuiRo> {
    pub fn gui(self) -> Lenh<DaGui> { self.chuyen() }
}

// ============================================================================
// 3. KIỂM TRA RỦI RO — cổng bắt buộc trước khi lệnh ra thị trường
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum LoiRuiRo {
    SoLuongKhongDuong(SoLuong),
    GiaKhongDuong(Gia),
    VuotGiaTriToiDa { gia_tri: i64, tran: i64 },
    VuotViTheToiDa { sau_lenh: i64, tran: i64 },
    MaChungKhoanLa(String),
}

pub struct HanMuc {
    pub gia_tri_lenh_toi_da: i64,
    pub vi_the_toi_da: i64,
    pub danh_sach_cho_phep: Vec<String>,
}

impl HanMuc {
    /// Trả `Result` chứ không panic: từ chối lệnh là chuyện BÌNH THƯỜNG,
    /// không phải lỗi lập trình. Đây là ranh giới "parse, đừng validate".
    pub fn kiem_tra(&self, l: Lenh<DangSoan>, vi_the_hien_tai: i64)
        -> Result<Lenh<DaKiemTraRuiRo>, LoiRuiRo>
    {
        if l.so_luong <= 0 { return Err(LoiRuiRo::SoLuongKhongDuong(l.so_luong)); }
        if l.gia <= 0 { return Err(LoiRuiRo::GiaKhongDuong(l.gia)); }
        if !self.danh_sach_cho_phep.iter().any(|m| *m == l.ma_ck) {
            return Err(LoiRuiRo::MaChungKhoanLa(l.ma_ck.clone()));
        }
        let gia_tri = l.gia * l.so_luong;
        if gia_tri > self.gia_tri_lenh_toi_da {
            return Err(LoiRuiRo::VuotGiaTriToiDa { gia_tri, tran: self.gia_tri_lenh_toi_da });
        }
        let sau_lenh = vi_the_hien_tai + l.chieu.dau() * l.so_luong;
        if sau_lenh.abs() > self.vi_the_toi_da {
            return Err(LoiRuiRo::VuotViTheToiDa { sau_lenh, tran: self.vi_the_toi_da });
        }
        Ok(l.chuyen())
    }
}

// ============================================================================
// 4. SỔ LỆNH & ĐỘNG CƠ KHỚP LỆNH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct KhopLenh {
    pub lenh_chu_dong: MaLenh,
    pub lenh_thu_dong: MaLenh,
    pub gia: Gia,
    pub so_luong: SoLuong,
}

/// Sổ lệnh giới hạn. `BTreeMap` cho phép lấy giá tốt nhất trong O(log n) và
/// duyệt các mức giá theo THỨ TỰ — đúng thứ động cơ khớp lệnh cần.
/// `VecDeque` ở mỗi mức giá giữ ưu tiên THỜI GIAN: ai đặt trước khớp trước.
pub struct SoLenh {
    /// Bên mua: khóa là giá ÂM để `BTreeMap` (vốn tăng dần) trả giá CAO nhất trước.
    ben_mua: BTreeMap<Gia, VecDeque<Lenh<DaGui>>>,
    ben_ban: BTreeMap<Gia, VecDeque<Lenh<DaGui>>>,
}

impl SoLenh {
    pub fn moi() -> Self { SoLenh { ben_mua: BTreeMap::new(), ben_ban: BTreeMap::new() } }

    /// Giá mua cao nhất — cái giá tốt nhất mà người bán có thể nhận ngay.
    pub fn gia_mua_tot_nhat(&self) -> Option<Gia> {
        self.ben_mua.keys().next().map(|k| -k)
    }
    /// Giá bán thấp nhất.
    pub fn gia_ban_tot_nhat(&self) -> Option<Gia> {
        self.ben_ban.keys().next().copied()
    }
    /// Chênh lệch mua-bán: chi phí ẩn của mọi giao dịch.
    pub fn chenh_lech(&self) -> Option<Gia> {
        Some(self.gia_ban_tot_nhat()? - self.gia_mua_tot_nhat()?)
    }
    /// Giá giữa — ước lượng "giá trị thật" tốt hơn giá khớp gần nhất.
    pub fn gia_giua(&self) -> Option<Gia> {
        Some((self.gia_ban_tot_nhat()? + self.gia_mua_tot_nhat()?) / 2)
    }
    pub fn khoi_luong_tai(&self, chieu: Chieu, gia: Gia) -> SoLuong {
        let ban = match chieu { Chieu::Mua => &self.ben_mua, Chieu::Ban => &self.ben_ban };
        let khoa = match chieu { Chieu::Mua => -gia, Chieu::Ban => gia };
        ban.get(&khoa).map_or(0, |q| q.iter().map(|l| l.con_lai()).sum())
    }
    pub fn tong_so_lenh(&self) -> usize {
        self.ben_mua.values().map(|q| q.len()).sum::<usize>()
            + self.ben_ban.values().map(|q| q.len()).sum::<usize>()
    }

    /// Nạp lệnh và khớp ngay phần khớp được; phần dư nằm lại sổ.
    /// Đây là trái tim của sàn: ƯU TIÊN GIÁ trước, rồi ƯU TIÊN THỜI GIAN.
    pub fn nap(&mut self, mut lenh: Lenh<DaGui>) -> Vec<KhopLenh> {
        let mut cac_khop = Vec::new();
        let doi_ung_la_ban = lenh.chieu == Chieu::Mua;

        loop {
            if lenh.con_lai() == 0 { break; }
            // Mức giá đối ứng tốt nhất còn khớp được với giá giới hạn của ta?
            let khoa_tot = {
                let doi_ung = if doi_ung_la_ban { &self.ben_ban } else { &self.ben_mua };
                match doi_ung.keys().next().copied() {
                    Some(k) => {
                        let gia_that = if doi_ung_la_ban { k } else { -k };
                        let khop_duoc = if doi_ung_la_ban { gia_that <= lenh.gia }
                                        else { gia_that >= lenh.gia };
                        if khop_duoc { Some((k, gia_that)) } else { None }
                    }
                    None => None,
                }
            };
            let (khoa, gia_khop) = match khoa_tot { Some(x) => x, None => break };

            let doi_ung = if doi_ung_la_ban { &mut self.ben_ban } else { &mut self.ben_mua };
            let hang = doi_ung.get_mut(&khoa).unwrap();
            while lenh.con_lai() > 0 {
                let doi_tac = match hang.front_mut() { Some(d) => d, None => break };
                let luong = lenh.con_lai().min(doi_tac.con_lai());
                lenh.da_khop += luong;
                doi_tac.da_khop += luong;
                cac_khop.push(KhopLenh {
                    lenh_chu_dong: lenh.ma,
                    lenh_thu_dong: doi_tac.ma,
                    // Giá khớp là giá của lệnh ĐÃ NẰM SẴN trong sổ — người
                    // đến sau được hưởng giá tốt hơn nếu có. Đây là quy tắc
                    // "cải thiện giá" của mọi sàn nghiêm túc.
                    gia: gia_khop,
                    so_luong: luong,
                });
                if doi_tac.con_lai() == 0 { hang.pop_front(); }
            }
            if hang.is_empty() { doi_ung.remove(&khoa); }
        }

        if lenh.con_lai() > 0 {
            let khoa = if lenh.chieu == Chieu::Mua { -lenh.gia } else { lenh.gia };
            let ban = if lenh.chieu == Chieu::Mua { &mut self.ben_mua } else { &mut self.ben_ban };
            ban.entry(khoa).or_default().push_back(lenh);
        }
        cac_khop
    }

    pub fn huy(&mut self, ma: MaLenh) -> bool {
        for ban in [&mut self.ben_mua, &mut self.ben_ban] {
            let mut rong = None;
            for (khoa, hang) in ban.iter_mut() {
                if let Some(i) = hang.iter().position(|l| l.ma == ma) {
                    hang.remove(i);
                    if hang.is_empty() { rong = Some(*khoa); }
                    if let Some(k) = rong { ban.remove(&k); }
                    return true;
                }
            }
        }
        false
    }
}

// ============================================================================
// 5. VỊ THẾ & LÃI/LỖ — một VỊ NHÓM (Chương 18) trá hình
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViThe {
    pub so_luong: i64,
    /// Tiền mặt tính bằng tick. Mua làm tiền giảm, bán làm tiền tăng.
    pub tien_mat: i64,
}

impl ViThe {
    pub const RONG: ViThe = ViThe { so_luong: 0, tien_mat: 0 };

    /// Phép `ghep` này KẾT HỢP và có ĐƠN VỊ `RONG` → đúng định nghĩa vị nhóm.
    /// Nhờ vậy có thể gộp lãi/lỗ song song bằng `rayon` mà kết quả không đổi.
    pub fn ghep(self, k: ViThe) -> ViThe {
        ViThe { so_luong: self.so_luong + k.so_luong, tien_mat: self.tien_mat + k.tien_mat }
    }
    pub fn tu_khop(chieu: Chieu, gia: Gia, so_luong: SoLuong) -> ViThe {
        ViThe {
            so_luong: chieu.dau() * so_luong,
            tien_mat: -chieu.dau() * gia * so_luong,
        }
    }
    /// Giá trị ròng khi định giá lại theo giá thị trường hiện tại.
    pub fn gia_tri_rong(&self, gia_thi_truong: Gia) -> i64 {
        self.tien_mat + self.so_luong * gia_thi_truong
    }
}

// ============================================================================
// 6. BỘ KIỂM ĐỊNH CHIẾN LƯỢC (backtest) — hàm thuần túy trên lịch sử
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Nen { pub thoi_diem: u64, pub mo: Gia, pub cao: Gia, pub thap: Gia, pub dong: Gia }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TinHieu { Mua(SoLuong), Ban(SoLuong), Giu }

/// Chiến lược là một HÀM THUẦN TÚY: cùng lịch sử → cùng tín hiệu, luôn luôn.
/// Nhờ tính chất này mà kết quả kiểm định tái lập được 100%.
pub trait ChienLuoc {
    fn ten(&self) -> &str;
    fn quyet_dinh(&mut self, lich_su: &[Nen], vi_the: &ViThe) -> TinHieu;
}

/// Giao cắt trung bình động: kinh điển, dễ hiểu, và cố tình đơn giản.
pub struct GiaoCatTrungBinh { pub nhanh: usize, pub cham: usize, pub don_vi: SoLuong }

fn trung_binh(nen: &[Nen], n: usize) -> Option<Gia> {
    if nen.len() < n { return None; }
    Some(nen[nen.len() - n..].iter().map(|c| c.dong).sum::<Gia>() / n as Gia)
}

impl ChienLuoc for GiaoCatTrungBinh {
    fn ten(&self) -> &str { "Giao cắt trung bình động" }
    fn quyet_dinh(&mut self, lich_su: &[Nen], vi_the: &ViThe) -> TinHieu {
        let (nhanh, cham) = match (trung_binh(lich_su, self.nhanh), trung_binh(lich_su, self.cham)) {
            (Some(a), Some(b)) => (a, b),
            _ => return TinHieu::Giu, // chưa đủ dữ liệu — KHÔNG đoán mò
        };
        if nhanh > cham && vi_the.so_luong <= 0 { TinHieu::Mua(self.don_vi) }
        else if nhanh < cham && vi_the.so_luong > 0 { TinHieu::Ban(vi_the.so_luong) }
        else { TinHieu::Giu }
    }
}

#[derive(Debug, PartialEq)]
pub struct KetQuaKiemDinh {
    pub vi_the_cuoi: ViThe,
    pub gia_tri_cuoi: i64,
    pub so_giao_dich: usize,
    /// Mức sụt giảm sâu nhất từ đỉnh — con số quan trọng hơn cả lợi nhuận,
    /// vì nó quyết định bạn có chịu nổi để đi hết chiến lược hay không.
    pub sut_giam_toi_da: i64,
    pub duong_von: Vec<i64>,
}

/// Chạy kiểm định. Có mô hình TRƯỢT GIÁ và PHÍ — bỏ hai thứ này là cách
/// nhanh nhất để tự lừa mình bằng một đường vốn đẹp nhưng không có thật.
pub fn chay_kiem_dinh(
    du_lieu: &[Nen],
    chien_luoc: &mut dyn ChienLuoc,
    truot_gia_tick: Gia,
    phi_moi_don_vi: i64,
) -> KetQuaKiemDinh {
    let mut vi_the = ViThe::RONG;
    let mut so_gd = 0;
    let mut duong_von = Vec::with_capacity(du_lieu.len());
    let mut dinh = i64::MIN;
    let mut sut_toi_da = 0;

    for i in 0..du_lieu.len() {
        let lich_su = &du_lieu[..=i];
        // Quyết định dựa trên nến ĐÃ ĐÓNG, khớp ở nến KẾ TIẾP.
        // Bỏ qua chi tiết này = "nhìn trộm tương lai", lỗi kinh điển
        // khiến mọi chiến lược trông như in tiền.
        let tin_hieu = chien_luoc.quyet_dinh(lich_su, &vi_the);
        if let Some(nen_sau) = du_lieu.get(i + 1) {
            let (chieu, luong) = match tin_hieu {
                TinHieu::Mua(q) => (Chieu::Mua, q),
                TinHieu::Ban(q) => (Chieu::Ban, q),
                TinHieu::Giu => { duong_von.push(vi_the.gia_tri_rong(du_lieu[i].dong)); continue; }
            };
            if luong > 0 {
                // Trượt giá: ta luôn mua đắt hơn và bán rẻ hơn giá lý thuyết.
                let gia = nen_sau.mo + chieu.dau() * truot_gia_tick;
                vi_the = vi_the.ghep(ViThe::tu_khop(chieu, gia, luong));
                vi_the.tien_mat -= phi_moi_don_vi * luong;
                so_gd += 1;
            }
        }
        let gt = vi_the.gia_tri_rong(du_lieu[i].dong);
        duong_von.push(gt);
        dinh = dinh.max(gt);
        sut_toi_da = sut_toi_da.max(dinh - gt);
    }

    let gia_cuoi = du_lieu.last().map_or(0, |n| n.dong);
    KetQuaKiemDinh {
        gia_tri_cuoi: vi_the.gia_tri_rong(gia_cuoi),
        vi_the_cuoi: vi_the,
        so_giao_dich: so_gd,
        sut_giam_toi_da: sut_toi_da,
        duong_von,
    }
}

/// Sinh dữ liệu giá tất định (bước ngẫu nhiên có hạt giống cố định).
/// Tất định là điều kiện BẮT BUỘC để kiểm thử hồi quy có ý nghĩa.
pub fn sinh_du_lieu(so_nen: usize, gia_dau: Gia, hat_giong: u64) -> Vec<Nen> {
    let mut s = hat_giong;
    let mut gia = gia_dau;
    (0..so_nen).map(|i| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let buoc = ((s >> 33) % 41) as i64 - 20; // -20..+20 tick
        let mo = gia;
        gia = (gia + buoc).max(1);
        Nen {
            thoi_diem: i as u64,
            mo,
            cao: mo.max(gia) + 5,
            thap: (mo.min(gia) - 5).max(1),
            dong: gia,
        }
    }).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   HỆ THỐNG GIAO DỊCH: SỔ LỆNH · KHỚP LỆNH · KIỂM ĐỊNH     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÌ SAO TIỀN PHẢI LÀ SỐ NGUYÊN");
    let sai: f64 = (0..10).map(|_| 0.1f64).sum();
    println!("   Cộng 0.1 mười lần bằng f64 → {:.20}", sai);
    println!("   Bằng nhau với 1.0?          → {}", sai == 1.0);
    println!("   Bằng số nguyên tick         → {} tick = {}", 100, tick_sang_chuoi(100));

    println!("\n2. CỔNG RỦI RO");
    let hm = HanMuc { gia_tri_lenh_toi_da: 1_000_000, vi_the_toi_da: 500,
                      danh_sach_cho_phep: vec!["VNM".into(), "FPT".into()] };
    for (mo_ta, l) in [
        ("hợp lệ         ", Lenh::moi(1, "VNM", Chieu::Mua, 8_500, 100)),
        ("mã lạ          ", Lenh::moi(2, "XYZ", Chieu::Mua, 8_500, 100)),
        ("quá to         ", Lenh::moi(3, "VNM", Chieu::Mua, 8_500, 1_000)),
        ("số lượng âm    ", Lenh::moi(4, "VNM", Chieu::Mua, 8_500, -5)),
    ] {
        match hm.kiem_tra(l, 0) {
            Ok(_) => println!("   {} → CHO QUA", mo_ta),
            Err(e) => println!("   {} → CHẶN: {:?}", mo_ta, e),
        }
    }

    println!("\n3. SỔ LỆNH & ƯU TIÊN GIÁ–THỜI GIAN");
    let mut so = SoLenh::moi();
    let gui = |ma, chieu, gia, sl| {
        Lenh::<DangSoan>::moi(ma, "VNM", chieu, gia, sl)
            .chuyen::<DaKiemTraRuiRo>().gui()
    };
    for (ma, gia, sl) in [(10u64, 8_400i64, 100i64), (11, 8_400, 200), (12, 8_390, 500)] {
        so.nap(gui(ma, Chieu::Mua, gia, sl));
    }
    for (ma, gia, sl) in [(20u64, 8_420i64, 150i64), (21, 8_430, 300)] {
        so.nap(gui(ma, Chieu::Ban, gia, sl));
    }
    println!("   Mua tốt nhất {} · Bán tốt nhất {} · Chênh lệch {} tick",
             tick_sang_chuoi(so.gia_mua_tot_nhat().unwrap()),
             tick_sang_chuoi(so.gia_ban_tot_nhat().unwrap()),
             so.chenh_lech().unwrap());
    println!("   Khối lượng chờ mua ở {}: {}", tick_sang_chuoi(8_400), so.khoi_luong_tai(Chieu::Mua, 8_400));

    println!("\n4. KHỚP LỆNH — lệnh bán 250 quét qua bên mua");
    let khop = so.nap(gui(30, Chieu::Ban, 8_390, 250));
    for k in &khop {
        println!("   {} đơn vị @ {} (đối tác lệnh #{})",
                 k.so_luong, tick_sang_chuoi(k.gia), k.lenh_thu_dong);
    }
    println!("   → Lệnh #10 (đặt trước) khớp hết TRƯỚC lệnh #11, dù cùng giá.");
    println!("   → Khớp ở giá {} chứ không phải {} — người đến sau được cải thiện giá.",
             tick_sang_chuoi(8_400), tick_sang_chuoi(8_390));

    println!("\n5. VỊ THẾ LÀ MỘT VỊ NHÓM");
    let a = ViThe::tu_khop(Chieu::Mua, 8_400, 100);
    let b = ViThe::tu_khop(Chieu::Ban, 8_500, 60);
    println!("   Mua 100@84.00 rồi bán 60@85.00 → {:?}", a.ghep(b));
    println!("   Kết hợp: (a·b)·c == a·(b·c) → {}",
             a.ghep(b).ghep(ViThe::RONG) == a.ghep(b.ghep(ViThe::RONG)));

    println!("\n6. KIỂM ĐỊNH CHIẾN LƯỢC — 500 nến, có phí và trượt giá");
    let du_lieu = sinh_du_lieu(500, 8_000, 42);
    for (truot, phi) in [(0i64, 0i64), (2, 3)] {
        let mut cl = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
        let kq = chay_kiem_dinh(&du_lieu, &mut cl, truot, phi);
        println!("   trượt {} tick, phí {}/đv → lãi {:>8} tick · {} lệnh · sụt sâu nhất {} tick",
                 truot, phi, kq.gia_tri_cuoi, kq.so_giao_dich, kq.sut_giam_toi_da);
    }
    println!("   → Cùng một chiến lược: bỏ qua phí và trượt giá là tự lừa mình.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   ĐỘ TRỄ CÓ TRẦN XÁC ĐỊNH — LÝ DO NGÀNH NÀY CHỌN RUST      ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn lenh_da_gui(ma: MaLenh, chieu: Chieu, gia: Gia, sl: SoLuong) -> Lenh<DaGui> {
        Lenh::<DangSoan>::moi(ma, "VNM", chieu, gia, sl).chuyen::<DaKiemTraRuiRo>().gui()
    }

    // ---------- Tiền & kiểu ----------
    #[test]
    fn tien_so_nguyen_khong_co_sai_so_tich_luy() {
        let f64_tong: f64 = (0..1000).map(|_| 0.01f64).sum();
        assert_ne!(f64_tong, 10.0, "f64 KHÔNG cộng đúng — đây là lý do không dùng nó cho tiền");
        let tick_tong: i64 = (0..1000).map(|_| 1i64).sum();
        assert_eq!(tick_tong, 1000, "số nguyên thì chính xác tuyệt đối");
    }

    #[test]
    fn hien_thi_tick_dung_ca_so_am() {
        assert_eq!(tick_sang_chuoi(8_450), "84.50");
        assert_eq!(tick_sang_chuoi(5), "0.05");
        assert_eq!(tick_sang_chuoi(-8_450), "-84.50");
    }

    // ---------- Rủi ro ----------
    #[test]
    fn cong_rui_ro_chan_dung_tung_loai_vi_pham() {
        let hm = HanMuc { gia_tri_lenh_toi_da: 1_000_000, vi_the_toi_da: 500,
                          danh_sach_cho_phep: vec!["VNM".into()] };
        // Dùng `unwrap_err()` chứ không `assert_eq!` cả `Result`: `Lenh` không
        // cài `PartialEq` (so sánh hai lệnh theo giá trị là vô nghĩa — mỗi lệnh
        // có danh tính riêng qua `ma`).
        assert!(hm.kiem_tra(Lenh::moi(1, "VNM", Chieu::Mua, 8_500, 100), 0).is_ok());
        assert_eq!(hm.kiem_tra(Lenh::moi(2, "VNM", Chieu::Mua, 8_500, 0), 0).unwrap_err(),
                   LoiRuiRo::SoLuongKhongDuong(0));
        assert_eq!(hm.kiem_tra(Lenh::moi(3, "VNM", Chieu::Mua, 0, 10), 0).unwrap_err(),
                   LoiRuiRo::GiaKhongDuong(0));
        assert_eq!(hm.kiem_tra(Lenh::moi(4, "XYZ", Chieu::Mua, 100, 10), 0).unwrap_err(),
                   LoiRuiRo::MaChungKhoanLa("XYZ".into()));
        assert!(matches!(hm.kiem_tra(Lenh::moi(5, "VNM", Chieu::Mua, 8_500, 1_000), 0).unwrap_err(),
                         LoiRuiRo::VuotGiaTriToiDa { .. }));
    }

    #[test]
    fn han_muc_vi_the_tinh_ca_chieu_ban_khong() {
        let hm = HanMuc { gia_tri_lenh_toi_da: i64::MAX, vi_the_toi_da: 100,
                          danh_sach_cho_phep: vec!["VNM".into()] };
        // bán khống 150 khi đang giữ 0 → vị thế -150, vượt trần 100
        assert_eq!(hm.kiem_tra(Lenh::moi(1, "VNM", Chieu::Ban, 100, 150), 0).unwrap_err(),
                   LoiRuiRo::VuotViTheToiDa { sau_lenh: -150, tran: 100 });
        // nhưng bán 150 khi đang giữ 100 → còn -50, hợp lệ
        assert!(hm.kiem_tra(Lenh::moi(2, "VNM", Chieu::Ban, 100, 150), 100).is_ok());
    }

    // ---------- Sổ lệnh ----------
    #[test]
    fn so_lenh_tra_dung_gia_tot_nhat_hai_ben() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Mua, 100, 10));
        s.nap(lenh_da_gui(2, Chieu::Mua, 105, 10)); // giá cao hơn = tốt hơn cho bên mua
        s.nap(lenh_da_gui(3, Chieu::Ban, 120, 10));
        s.nap(lenh_da_gui(4, Chieu::Ban, 110, 10)); // giá thấp hơn = tốt hơn cho bên bán
        assert_eq!(s.gia_mua_tot_nhat(), Some(105));
        assert_eq!(s.gia_ban_tot_nhat(), Some(110));
        assert_eq!(s.chenh_lech(), Some(5));
        assert_eq!(s.gia_giua(), Some(107));
    }

    #[test]
    fn lenh_khong_giao_nhau_thi_nam_lai_so() {
        let mut s = SoLenh::moi();
        assert!(s.nap(lenh_da_gui(1, Chieu::Mua, 100, 10)).is_empty());
        assert!(s.nap(lenh_da_gui(2, Chieu::Ban, 110, 10)).is_empty());
        assert_eq!(s.tong_so_lenh(), 2);
    }

    #[test]
    fn uu_tien_thoi_gian_o_cung_muc_gia() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Mua, 100, 50));  // đến TRƯỚC
        s.nap(lenh_da_gui(2, Chieu::Mua, 100, 50));  // đến SAU
        let khop = s.nap(lenh_da_gui(3, Chieu::Ban, 100, 60));
        assert_eq!(khop.len(), 2);
        assert_eq!(khop[0].lenh_thu_dong, 1, "lệnh đến trước phải khớp trước");
        assert_eq!(khop[0].so_luong, 50);
        assert_eq!(khop[1].lenh_thu_dong, 2);
        assert_eq!(khop[1].so_luong, 10);
    }

    #[test]
    fn uu_tien_gia_thang_uu_tien_thoi_gian() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Mua, 100, 50));  // đến trước, giá THẤP hơn
        s.nap(lenh_da_gui(2, Chieu::Mua, 105, 50));  // đến sau, giá CAO hơn
        let khop = s.nap(lenh_da_gui(3, Chieu::Ban, 100, 10));
        assert_eq!(khop[0].lenh_thu_dong, 2, "giá tốt hơn thắng, dù đến sau");
        assert_eq!(khop[0].gia, 105);
    }

    #[test]
    fn nguoi_den_sau_duoc_cai_thien_gia() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Ban, 100, 10)); // ai đó chào bán rẻ
        // ta sẵn sàng mua tới 120, nhưng chỉ phải trả 100
        let khop = s.nap(lenh_da_gui(2, Chieu::Mua, 120, 10));
        assert_eq!(khop[0].gia, 100, "khớp ở giá của lệnh nằm sẵn trong sổ");
    }

    #[test]
    fn lenh_lon_quet_qua_nhieu_muc_gia() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Ban, 100, 10));
        s.nap(lenh_da_gui(2, Chieu::Ban, 101, 10));
        s.nap(lenh_da_gui(3, Chieu::Ban, 102, 10));
        let khop = s.nap(lenh_da_gui(4, Chieu::Mua, 102, 25));
        assert_eq!(khop.len(), 3);
        assert_eq!(khop.iter().map(|k| k.gia).collect::<Vec<_>>(), vec![100, 101, 102],
                   "phải ăn từ giá tốt nhất trở đi");
        assert_eq!(khop.iter().map(|k| k.so_luong).sum::<i64>(), 25);
        assert_eq!(s.tong_so_lenh(), 1, "mức 102 còn dư 5 đơn vị");
    }

    #[test]
    fn phan_du_cua_lenh_chu_dong_nam_lai_so() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Ban, 100, 10));
        let khop = s.nap(lenh_da_gui(2, Chieu::Mua, 100, 30));
        assert_eq!(khop.iter().map(|k| k.so_luong).sum::<i64>(), 10);
        assert_eq!(s.gia_mua_tot_nhat(), Some(100), "20 đơn vị còn lại thành lệnh chờ mua");
        assert_eq!(s.khoi_luong_tai(Chieu::Mua, 100), 20);
    }

    #[test]
    fn bao_toan_khoi_luong_qua_moi_lan_khop() {
        // BẤT BIẾN SỐNG CÒN của mọi sàn: không đơn vị nào được sinh ra
        // hay biến mất trong quá trình khớp.
        let mut s = SoLenh::moi();
        let mut da_nap = 0i64;
        let mut da_khop = 0i64;
        for i in 0..60u64 {
            let chieu = if i % 2 == 0 { Chieu::Mua } else { Chieu::Ban };
            let gia = 100 + ((i * 7) % 11) as i64 - 5;
            let sl = 10 + (i % 13) as i64;
            da_nap += sl;
            da_khop += s.nap(lenh_da_gui(i, chieu, gia, sl))
                        .iter().map(|k| k.so_luong).sum::<i64>();
        }
        let con_trong_so: i64 = [Chieu::Mua, Chieu::Ban].iter()
            .flat_map(|&c| (80..=120).map(move |g| (c, g)))
            .map(|(c, g)| s.khoi_luong_tai(c, g)).sum();
        // Mỗi lần khớp tiêu thụ khối lượng từ CẢ HAI phía
        assert_eq!(da_nap - 2 * da_khop, con_trong_so,
                   "khối lượng phải cân bằng tuyệt đối");
    }

    #[test]
    fn huy_lenh_go_dung_lenh_va_don_muc_gia_rong() {
        let mut s = SoLenh::moi();
        s.nap(lenh_da_gui(1, Chieu::Mua, 100, 10));
        s.nap(lenh_da_gui(2, Chieu::Mua, 100, 20));
        assert!(s.huy(1));
        assert_eq!(s.khoi_luong_tai(Chieu::Mua, 100), 20);
        assert!(s.huy(2));
        assert_eq!(s.gia_mua_tot_nhat(), None, "mức giá rỗng phải bị xóa khỏi sổ");
        assert!(!s.huy(999), "hủy lệnh không tồn tại phải trả false");
    }

    #[test]
    fn so_rong_khong_co_gia_va_khong_panic() {
        let s = SoLenh::moi();
        assert_eq!(s.gia_mua_tot_nhat(), None);
        assert_eq!(s.chenh_lech(), None);
        assert_eq!(s.gia_giua(), None);
        assert_eq!(s.tong_so_lenh(), 0);
    }

    // ---------- Vị thế ----------
    #[test]
    fn vi_the_thoa_luat_vi_nhom() {
        let a = ViThe::tu_khop(Chieu::Mua, 100, 10);
        let b = ViThe::tu_khop(Chieu::Ban, 110, 5);
        let c = ViThe::tu_khop(Chieu::Mua, 90, 3);
        assert_eq!(a.ghep(b).ghep(c), a.ghep(b.ghep(c)), "luật kết hợp");
        assert_eq!(a.ghep(ViThe::RONG), a, "luật đơn vị phải");
        assert_eq!(ViThe::RONG.ghep(a), a, "luật đơn vị trái");
    }

    #[test]
    fn gop_vi_the_theo_khoi_cho_cung_ket_qua() {
        // Vì là vị nhóm, chia nhỏ rồi gộp lại (như khi dùng rayon) cho kết quả
        // Y HỆT tính tuần tự. Đây là bảo chứng toán học, không phải may mắn.
        let khop: Vec<ViThe> = (0..100).map(|i| {
            let chieu = if i % 3 == 0 { Chieu::Ban } else { Chieu::Mua };
            ViThe::tu_khop(chieu, 100 + i % 7, 1 + i % 5)
        }).collect();
        let tuan_tu = khop.iter().fold(ViThe::RONG, |a, &b| a.ghep(b));
        let theo_khoi = khop.chunks(7)
            .map(|k| k.iter().fold(ViThe::RONG, |a, &b| a.ghep(b)))
            .fold(ViThe::RONG, |a, b| a.ghep(b));
        assert_eq!(tuan_tu, theo_khoi);
    }

    #[test]
    fn mua_roi_ban_cao_hon_thi_co_lai() {
        let v = ViThe::tu_khop(Chieu::Mua, 8_000, 100)
            .ghep(ViThe::tu_khop(Chieu::Ban, 8_500, 100));
        assert_eq!(v.so_luong, 0, "đã đóng hết vị thế");
        assert_eq!(v.gia_tri_rong(0), 50_000, "(8500-8000) × 100 tick");
    }

    #[test]
    fn vi_the_mo_duoc_dinh_gia_lai_theo_thi_truong() {
        let v = ViThe::tu_khop(Chieu::Mua, 8_000, 100);
        assert_eq!(v.gia_tri_rong(8_000), 0, "vừa mua xong thì hòa vốn");
        assert_eq!(v.gia_tri_rong(8_100), 10_000, "giá lên 100 tick → lãi 10 000");
        assert_eq!(v.gia_tri_rong(7_900), -10_000, "giá xuống thì lỗ đối xứng");
    }

    // ---------- Kiểm định ----------
    #[test]
    fn sinh_du_lieu_tat_dinh_theo_hat_giong() {
        assert_eq!(sinh_du_lieu(50, 8_000, 7), sinh_du_lieu(50, 8_000, 7));
        assert_ne!(sinh_du_lieu(50, 8_000, 7), sinh_du_lieu(50, 8_000, 8));
    }

    #[test]
    fn du_lieu_sinh_ra_luon_hop_le() {
        for nen in sinh_du_lieu(500, 8_000, 99) {
            assert!(nen.cao >= nen.mo && nen.cao >= nen.dong, "đỉnh phải cao nhất");
            assert!(nen.thap <= nen.mo && nen.thap <= nen.dong, "đáy phải thấp nhất");
            assert!(nen.thap > 0, "giá không bao giờ âm");
        }
    }

    #[test]
    fn chien_luoc_giu_im_khi_chua_du_du_lieu() {
        let mut cl = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
        let it_nen = sinh_du_lieu(10, 8_000, 1);
        assert_eq!(cl.quyet_dinh(&it_nen, &ViThe::RONG), TinHieu::Giu,
                   "chưa đủ 20 nến thì KHÔNG được đoán mò");
    }

    #[test]
    fn kiem_dinh_tai_lap_duoc_hoan_toan() {
        let du_lieu = sinh_du_lieu(300, 8_000, 42);
        let chay = || {
            let mut cl = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
            chay_kiem_dinh(&du_lieu, &mut cl, 2, 3)
        };
        assert_eq!(chay(), chay(), "cùng dữ liệu + cùng chiến lược = cùng kết quả, luôn luôn");
    }

    #[test]
    fn phi_va_truot_gia_luon_lam_ket_qua_xau_di() {
        let du_lieu = sinh_du_lieu(400, 8_000, 2024);
        let mut cl1 = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
        let ly_tuong = chay_kiem_dinh(&du_lieu, &mut cl1, 0, 0);
        let mut cl2 = GiaoCatTrungBinh { nhanh: 5, cham: 20, don_vi: 100 };
        let thuc_te = chay_kiem_dinh(&du_lieu, &mut cl2, 2, 3);
        assert_eq!(ly_tuong.so_giao_dich, thuc_te.so_giao_dich, "cùng số lệnh");
        assert!(thuc_te.gia_tri_cuoi < ly_tuong.gia_tri_cuoi,
                "chi phí giao dịch luôn ăn vào lợi nhuận: {} so với {}",
                thuc_te.gia_tri_cuoi, ly_tuong.gia_tri_cuoi);
    }

    #[test]
    fn sut_giam_toi_da_khong_bao_gio_am() {
        for hat in [1u64, 7, 42, 2024, 31337] {
            let du_lieu = sinh_du_lieu(200, 8_000, hat);
            let mut cl = GiaoCatTrungBinh { nhanh: 3, cham: 10, don_vi: 50 };
            let kq = chay_kiem_dinh(&du_lieu, &mut cl, 1, 1);
            assert!(kq.sut_giam_toi_da >= 0, "sụt giảm là khoảng cách, không thể âm");
            assert_eq!(kq.duong_von.len(), du_lieu.len());
        }
    }

    #[test]
    fn khong_giao_dich_thi_khong_lai_khong_lo() {
        struct KhongLamGi;
        impl ChienLuoc for KhongLamGi {
            fn ten(&self) -> &str { "đứng ngoài" }
            fn quyet_dinh(&mut self, _: &[Nen], _: &ViThe) -> TinHieu { TinHieu::Giu }
        }
        let du_lieu = sinh_du_lieu(200, 8_000, 5);
        let kq = chay_kiem_dinh(&du_lieu, &mut KhongLamGi, 5, 10);
        assert_eq!(kq.so_giao_dich, 0);
        assert_eq!(kq.gia_tri_cuoi, 0, "không vào lệnh thì không thể mất tiền");
        assert_eq!(kq.sut_giam_toi_da, 0);
    }

    #[test]
    fn chien_luoc_khong_duoc_nhin_trom_tuong_lai() {
        // Nếu bộ kiểm định khớp ở giá ĐÓNG của chính cây nến ra tín hiệu,
        // ta đã dùng thông tin chưa tồn tại. Ở đây khớp ở giá MỞ của nến kế
        // tiếp, nên nến CUỐI CÙNG không thể sinh giao dịch nào.
        let du_lieu = sinh_du_lieu(30, 8_000, 3);
        struct LuonMua;
        impl ChienLuoc for LuonMua {
            fn ten(&self) -> &str { "luôn mua" }
            fn quyet_dinh(&mut self, _: &[Nen], _: &ViThe) -> TinHieu { TinHieu::Mua(1) }
        }
        let kq = chay_kiem_dinh(&du_lieu, &mut LuonMua, 0, 0);
        assert_eq!(kq.so_giao_dich, du_lieu.len() - 1,
                   "nến cuối không có nến kế tiếp để khớp — không được bịa ra giao dịch");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0277: DaKiemTraRuiRo doesn't implement Debug` | `.unwrap_err()` cần kiểu `Ok` cài `Debug` | Thêm `#[derive(Debug)]` cho các nhãn typestate |
| `E0599: no method named 'gui' found for Lenh<DangSoan>` | **Đây là tính năng!** Chưa qua cổng rủi ro | Gọi `han_muc.kiem_tra(lenh, vi_the)?` trước |
| `E0382: use of moved value: 'lenh'` | Gửi cùng một lệnh hai lần | Đúng như thiết kế — mỗi lệnh chỉ gửi được một lần |
| `E0502: cannot borrow self.ben_ban as mutable` | Đọc `keys().next()` rồi `get_mut` cùng lúc | Tách thành hai khối: lấy khóa ra trước trong `{ }` riêng |
| Số dư tài khoản lệch vài xu sau nhiều giao dịch | Dùng `f64` cho tiền | Chuyển hết sang số nguyên tick |
| Chiến lược lãi phi thực tế trong kiểm định | Nhìn trộm tương lai | Khớp ở nến **kế tiếp**, không phải nến ra tín hiệu |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 6 điểm cốt lõi cần ghi nhớ

1. **Tiền là số nguyên.** Không có ngoại lệ, không có "chỉ chỗ này thôi". Đếm bằng đơn vị nhỏ nhất.
2. **Ưu tiên giá trước, ưu tiên thời gian sau.** Hai quy tắc này định nghĩa mọi sàn giao dịch trên thế giới.
3. **Người đến sau được cải thiện giá.** Giá khớp là giá của lệnh đã nằm sẵn — đó là điều khiến người ta dám đặt lệnh giới hạn.
4. **Typestate biến lỗi vận hành thành lỗi biên dịch.** Lệnh chưa qua rủi ro không gửi được; lệnh đã gửi không gửi lại được.
5. **Vị thế là vị nhóm** — và vì thế gộp song song cho kết quả y hệt gộp tuần tự, có bảo chứng toán học.
6. **Bộ kiểm định trung thực phải làm bạn thất vọng.** Không nhìn trộm tương lai, có phí, có trượt giá, và báo cả sụt giảm tối đa chứ không chỉ lợi nhuận.

### Bài tập rèn luyện tự giải

**Bài 1.** Thêm các **loại lệnh** khác: lệnh thị trường (khớp mọi giá), IOC (khớp được bao nhiêu hay bấy nhiêu, phần dư hủy), và FOK (khớp toàn bộ hoặc không khớp gì).

<details>
<summary><b>Gợi ý</b></summary>

- **Thị trường** = lệnh giới hạn với giá `i64::MAX` (mua) hoặc `1` (bán). Không cần logic mới.
- **IOC** = nạp bình thường, nhưng nếu còn dư thì **không** để lại sổ.
- **FOK** cần *kiểm tra trước*: duyệt sổ tính xem có đủ khối lượng khớp được không, nếu không thì từ chối luôn mà **không** khớp phần nào.

FOK khó nhất vì phải "thử mà không làm". Cách sạch nhất: viết một hàm `khoi_luong_khop_duoc()` chỉ đọc, không sửa sổ.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoaiLenh { GioiHan, ThiTruong, Ioc, Fok }

impl SoLenh {
    /// Tính trước khối lượng khớp được mà KHÔNG sửa sổ — cần cho FOK.
    pub fn khoi_luong_khop_duoc(&self, chieu: Chieu, gia: Gia) -> SoLuong {
        let doi_ung = match chieu { Chieu::Mua => &self.ben_ban, Chieu::Ban => &self.ben_mua };
        doi_ung.iter()
            .take_while(|(khoa, _)| {
                let gia_that = if chieu == Chieu::Mua { **khoa } else { -**khoa };
                if chieu == Chieu::Mua { gia_that <= gia } else { gia_that >= gia }
            })
            .flat_map(|(_, hang)| hang.iter().map(|l| l.con_lai()))
            .sum()
    }

    pub fn nap_voi_loai(&mut self, mut lenh: Lenh<DaGui>, loai: LoaiLenh) -> Vec<KhopLenh> {
        // Lệnh thị trường = lệnh giới hạn với giá cực đoan
        if loai == LoaiLenh::ThiTruong {
            lenh.gia = if lenh.chieu == Chieu::Mua { i64::MAX } else { 1 };
        }
        // FOK: kiểm tra TRƯỚC, không khớp một phần nào
        if loai == LoaiLenh::Fok
            && self.khoi_luong_khop_duoc(lenh.chieu, lenh.gia) < lenh.so_luong {
            return Vec::new();
        }

        let ma = lenh.ma;
        let khop = self.nap(lenh);

        // IOC và thị trường: phần dư KHÔNG được nằm lại sổ
        if matches!(loai, LoaiLenh::Ioc | LoaiLenh::ThiTruong | LoaiLenh::Fok) {
            self.huy(ma);
        }
        khop
    }
}
```

Chú ý `take_while` chứ không phải `filter`: vì `BTreeMap` đã sắp thứ tự, gặp mức giá đầu tiên **không** khớp được nghĩa là mọi mức sau cũng không khớp được. Dừng sớm thay vì duyệt hết sổ — chi tiết nhỏ nhưng ở tốc độ hàng triệu lệnh mỗi giây thì nó quyết định.
</details>

**Bài 2.** Cài chiến lược **hồi quy về trung bình** (mua khi giá thấp hơn trung bình một độ lệch chuẩn, bán khi cao hơn) và so sánh với giao cắt trung bình động trên cùng dữ liệu.

<details>
<summary><b>Gợi ý</b></summary>

Tính trung bình và độ lệch chuẩn trên cửa sổ `n` nến gần nhất. Mua khi `gia < trung_bình - k·độ_lệch`, bán khi `gia > trung_bình + k·độ_lệch`. Đây chính là dải Bollinger.

Vì đang dùng số nguyên tick, hãy tính phương sai bằng số nguyên rồi lấy căn bằng `(x as f64).sqrt() as i64` — hoặc dùng độ lệch tuyệt đối trung bình (MAD) để tránh hẳn dấu phẩy động, như Chương 58 đã bàn về thống kê bền vững.

Dự đoán trước khi chạy: trên dữ liệu **bước ngẫu nhiên**, hồi quy về trung bình thường trông tốt hơn theo xu hướng — nhưng cả hai đều thua chi phí về dài hạn. Đó là bài học chứ không phải thất bại.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct HoiQuyTrungBinh { pub cua_so: usize, pub he_so: i64, pub don_vi: SoLuong }

impl ChienLuoc for HoiQuyTrungBinh {
    fn ten(&self) -> &str { "Hồi quy về trung bình" }

    fn quyet_dinh(&mut self, lich_su: &[Nen], vi_the: &ViThe) -> TinHieu {
        if lich_su.len() < self.cua_so { return TinHieu::Giu; }
        let cua_so = &lich_su[lich_su.len() - self.cua_so..];
        let n = self.cua_so as i64;
        let tb: Gia = cua_so.iter().map(|c| c.dong).sum::<Gia>() / n;

        // Độ lệch tuyệt đối trung bình — toàn số nguyên, bền với giá trị dị biệt
        let dltb: i64 = cua_so.iter().map(|c| (c.dong - tb).abs()).sum::<i64>() / n;
        let gia = lich_su.last().unwrap().dong;
        let nguong = self.he_so * dltb;

        if gia < tb - nguong && vi_the.so_luong <= 0 {
            TinHieu::Mua(self.don_vi)          // rẻ bất thường → mua
        } else if gia > tb + nguong && vi_the.so_luong > 0 {
            TinHieu::Ban(vi_the.so_luong)      // đắt bất thường → chốt
        } else {
            TinHieu::Giu
        }
    }
}
```

Chạy cả hai chiến lược trên cùng dữ liệu và cùng chi phí, rồi so **ba** con số: lợi nhuận, số giao dịch, và sụt giảm tối đa. Bạn sẽ thấy hồi quy về trung bình giao dịch **nhiều hơn** (nên trả phí nhiều hơn) nhưng sụt giảm **nông hơn**.

Bài học quan trọng nhất: đừng chọn chiến lược chỉ vì nó lãi nhất trên một bộ dữ liệu. Đổi hạt giống sinh dữ liệu và chạy lại — nếu thứ hạng đảo lộn, bạn vừa **khớp quá mức** (overfit) chứ không phát hiện ra quy luật nào cả.
</details>

**Bài 3.** Thêm **nhật ký sự kiện** (event sourcing) vào sổ lệnh: ghi mọi lệnh và mọi lần khớp, rồi chứng minh phát lại nhật ký từ đầu tái tạo **chính xác** trạng thái sổ.

<details>
<summary><b>Gợi ý</b></summary>

Đây là kiến trúc mà mọi sàn giao dịch thật đều dùng, vì ba lý do: kiểm toán pháp lý, khôi phục sau sự cố, và gỡ lỗi ("tại sao lệnh này khớp ở giá đó?").

Điều kiện tiên quyết: động cơ khớp lệnh phải **hoàn toàn tất định** — cùng chuỗi sự kiện đầu vào luôn cho cùng trạng thái đầu ra. Không dùng thời gian hệ thống, không dùng số ngẫu nhiên, không dùng thứ tự duyệt `HashMap`.

Bài kiểm thử là phần đắt giá nhất: nạp ngẫu nhiên 1 000 lệnh vào hai sổ — một sổ nạp trực tiếp, một sổ phát lại từ nhật ký — rồi khẳng định trạng thái hai sổ giống hệt nhau.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum SuKien {
    NhanLenh { ma: MaLenh, chieu: Chieu, gia: Gia, so_luong: SoLuong },
    HuyLenh { ma: MaLenh },
}

pub struct SoLenhCoNhatKy {
    pub so: SoLenh,
    pub nhat_ky: Vec<SuKien>,
}

impl SoLenhCoNhatKy {
    pub fn moi() -> Self { SoLenhCoNhatKy { so: SoLenh::moi(), nhat_ky: Vec::new() } }

    pub fn nap(&mut self, lenh: Lenh<DaGui>) -> Vec<KhopLenh> {
        // GHI NHẬT KÝ TRƯỚC khi thay đổi trạng thái — nếu sập giữa chừng,
        // nhật ký vẫn đủ để dựng lại. Đây là nguyên tắc WAL của Chương 34.
        self.nhat_ky.push(SuKien::NhanLenh {
            ma: lenh.ma, chieu: lenh.chieu, gia: lenh.gia, so_luong: lenh.so_luong,
        });
        self.so.nap(lenh)
    }

    pub fn huy(&mut self, ma: MaLenh) -> bool {
        self.nhat_ky.push(SuKien::HuyLenh { ma });
        self.so.huy(ma)
    }

    /// Dựng lại toàn bộ sổ chỉ từ nhật ký. Không cần ảnh chụp trạng thái nào.
    pub fn phat_lai(nhat_ky: &[SuKien]) -> SoLenh {
        let mut so = SoLenh::moi();
        for sk in nhat_ky {
            match sk {
                SuKien::NhanLenh { ma, chieu, gia, so_luong } => {
                    let l = Lenh::<DangSoan>::moi(*ma, "VNM", *chieu, *gia, *so_luong)
                        .chuyen::<DaKiemTraRuiRo>().gui();
                    so.nap(l);
                }
                SuKien::HuyLenh { ma } => { so.huy(*ma); }
            }
        }
        so
    }
}

// Bài kiểm thử quan trọng nhất:
//   let mut s = SoLenhCoNhatKy::moi();
//   for i in 0..1000 { s.nap(sinh_lenh_tat_dinh(i)); }
//   let dung_lai = SoLenhCoNhatKy::phat_lai(&s.nhat_ky);
//   assert_eq!(dung_lai.gia_mua_tot_nhat(), s.so.gia_mua_tot_nhat());
//   assert_eq!(dung_lai.gia_ban_tot_nhat(), s.so.gia_ban_tot_nhat());
//   assert_eq!(dung_lai.tong_so_lenh(), s.so.tong_so_lenh());
```

Chú ý thứ tự trong `nap`: **ghi nhật ký trước, sửa trạng thái sau**. Đây chính là nguyên tắc ghi-trước (Write-Ahead Logging) của Chương 34, áp dụng nguyên vẹn. Nếu tiến trình sập ngay sau khi ghi nhật ký, phát lại vẫn cho trạng thái đúng. Nếu sập trước khi ghi, sự kiện coi như chưa từng xảy ra — cũng nhất quán.

Đây là chỗ ba chương gặp nhau: WAL từ cơ sở dữ liệu (Ch34), tính tất định từ lập trình hàm (Ch13), và sổ lệnh của chương này.
</details>
