# Chương 75: Dữ liệu thị trường — Giao thức nhị phân, Phát hiện khe & Sổ lệnh

## Giới thiệu & Mục tiêu học tập

Sổ lệnh là **cấu trúc dữ liệu quan trọng nhất trong tài chính**. Mọi giá bạn từng thấy — cổ phiếu, tiền mã hoá, hợp đồng tương lai — đều là kết quả của một sổ lệnh khớp lệnh mua với lệnh bán.

Chương này dựng đường dẫn dữ liệu thị trường đầy đủ:

```
gói UDP → phân tích nhị phân → phát hiện khe → cập nhật sổ lệnh → tín hiệu
```

Ba bài học cốt lõi:

1. **Giao thức nhị phân, không JSON.** ITCH của Nasdaq nhồi một cập nhật vào 36 byte. Cùng nội dung ở JSON tốn khoảng 200 byte và mất hàng microsecond để phân tích.
2. **Multicast UDP mất gói.** Không có TCP để sửa hộ. Bạn phải tự phát hiện khe và tự yêu cầu phát lại — **đúng một lần**, không lặp.
3. **L2 hay L3 là quyết định kiến trúc.** L2 (gộp theo mức giá) đủ cho hầu hết chiến lược. L3 (từng lệnh) cho biết **vị trí xếp hàng** — thứ quyết định lãi lỗ của nhà tạo lập.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  GIAO THỨC NHỊ PHÂN = ĐIỀN VÀO Ô CÓ SẴN, KHÔNG VIẾT VĂN                     │
│                                                                              │
│   JSON (≈200 byte, phân tích ~1 µs):                                        │
│     {"type":"add","order_id":12345,"side":"B","price":10050,"qty":100}      │
│                                                                              │
│   Nhị phân (24 byte, phân tích ~40 ns):                                     │
│     ┌──┬────────┬────────┬──┬────────┬────────┐                            │
│     │01│ 12345  │  ts    │B │ 10050  │  100   │                            │
│     └──┴────────┴────────┴──┴────────┴────────┘                            │
│      loại  u64     u64    u8   u64      u32                                 │
│                                                                              │
│   Không tìm dấu ngoặc, không cấp phát chuỗi. Chỉ đọc theo độ lệch cố định.  │
│                                                                              │
│  PHÁT HIỆN KHE = SỐ THỨ TỰ NHẢY CÓC                                         │
│                                                                              │
│    ...101, 102, 103, ▓▓▓, ▓▓▓, 106, 107...                                 │
│                       └──┬──┘                                               │
│              mất 104,105 → xin phát lại MỘT LẦN                             │
│                                                                              │
│   ⚠ LỖI KINH ĐIỂN: cứ mỗi thông điệp mới lại báo lại cùng một khe.         │
│     107 → "vẫn thiếu 104-105!", 108 → "vẫn thiếu 104-105!"...              │
│     Kết quả: bão yêu cầu phát lại, làm sập chính đường phục hồi.            │
│   → Phải có TRẠNG THÁI "đang chờ khôi phục".                                │
│                                                                              │
│  L2 vs L3                                                                   │
│                                                                              │
│   L2 — gộp theo mức giá        L3 — từng lệnh riêng, có thứ tự              │
│   ┌────────┬─────┐             ┌────────┬───────────────────────┐          │
│   │ 100.50 │ 500 │             │ 100.50 │ #7(200) #9(150) #12(150)│        │
│   │ 100.49 │ 300 │             │ 100.49 │ #3(300)                 │        │
│   └────────┴─────┘             └────────┴───────────────────────┘          │
│                                                                             │
│   Với L3 bạn biết lệnh #9 có 200 đơn vị XẾP TRƯỚC.                         │
│   Phải khớp hết 200 đó thì mới tới lượt bạn.                                │
│   Đó là thông tin quyết định: đứng cuối hàng thì gần như chỉ được khớp     │
│   khi giá sắp đi ngược lại — tức là bị "chọn lọc bất lợi".                 │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Vì sao trường có độ dài cố định

Trong ITCH, mọi trường đều ở vị trí cố định. Bộ phân tích chỉ việc đọc byte tại độ lệch đã biết — không rẽ nhánh, không tìm kiếm, không cấp phát.

Điểm đánh đổi: mở rộng giao thức khó. Thêm trường mới nghĩa là thêm loại thông điệp mới, không phải thêm khoá vào JSON. Các sàn giải quyết bằng cách đánh phiên bản ở mức phiên kết nối.

Về endianness: các giao thức mạng thường dùng big-endian ("thứ tự byte mạng"), còn x86 là little-endian. Nhớ `from_be_bytes` chứ không phải `from_le_bytes` — nhầm ở đây cho ra giá sai lệch hàng triệu lần mà chương trình vẫn chạy vui vẻ.

### 2. Trạng thái phục hồi: lỗi mà chương này sửa

Bản đầu tiên của bộ phát hiện khe trong chương này có một lỗi thật: nó báo lại **cùng một khe** cho mọi thông điệp tiếp theo. Trong sản xuất, lỗi đó tạo ra bão yêu cầu phát lại — và bão đó làm sập chính đường phục hồi mà bạn đang cần.

Cách chữa là thêm trạng thái `khe_dang_cho: Option<(u64, u64)>` và một biến thể kết quả `DangChoKhoiPhuc`. Khi đã yêu cầu phát lại một khe, mọi thông điệp sau đó chỉ báo "đang chờ" chứ không sinh yêu cầu mới.

Đây là ví dụ điển hình của một loại lỗi mà **kiểm thử một thông điệp không bao giờ bắt được** — phải kiểm thử một dòng thông điệp mới lộ.

### 3. Vị trí xếp hàng: nơi lãi lỗ của nhà tạo lập được quyết định

Hầu hết sàn khớp theo **giá – thời gian**: cùng mức giá thì ai đặt trước được khớp trước. Nghĩa là khi bạn đặt lệnh mua ở 100.50 mà đã có 500 đơn vị đứng trước, phải khớp hết 500 đơn vị đó rồi mới tới bạn.

Hệ quả kinh tế rất sắc: nếu bạn đứng cuối hàng, lệnh của bạn thường chỉ được khớp khi có **nhiều** người bán — tức là khi giá đang chuẩn bị đi xuống. Bạn được khớp đúng lúc không nên được khớp. Đó là **chọn lọc bất lợi**, và nó là lý do tốc độ có giá trị: đến sớm nghĩa là đứng đầu hàng.

Có một chi tiết thú vị: hủy rồi đặt lại ở cùng mức giá sẽ **mất toàn bộ vị trí xếp hàng**. Nhưng **giảm** khối lượng của lệnh hiện có thì thường **giữ** được vị trí. Đó là lý do các thuật toán tinh vi giảm khối lượng thay vì hủy-và-đặt-lại.

### 4. Vì sao dùng `BTreeMap` cho sổ lệnh

Sổ lệnh cần: giá tốt nhất (min hoặc max), duyệt theo thứ tự giá, và chèn/xoá nhanh. `BTreeMap` cho cả ba với O(log n), và quan trọng nhất — **thứ tự duyệt là tất định**.

`HashMap` nhanh hơn cho tra cứu điểm, nhưng thứ tự duyệt không xác định. Trong hệ thống giao dịch, thứ tự không xác định nghĩa là **phát lại không tái lập được** — bạn không thể gỡ lỗi một sự cố sản xuất. Chương 76 sẽ cho thấy đúng lỗi này xảy ra như thế nào.

Sổ lệnh sản xuất thực sự thường đi xa hơn: dùng mảng có chỉ số theo giá (vì giá rời rạc theo bước giá), cho O(1) ở mọi thao tác. Nhưng nó tốn bộ nhớ theo dải giá, nên chỉ hợp với thị trường có dải hẹp.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch75`, kiểm thử bằng `cargo test -p ch75`.

```rust
#![allow(dead_code)]
//! Chương 75 — Xử lý luồng dữ liệu thị trường: giao thức nhị phân kiểu ITCH,
//! phát hiện khe số thứ tự, dựng sổ lệnh L2/L3 từ bản tin gia tăng, và kiểm
//! tra chất lượng dữ liệu.
//!
//! Đây là chặng đầu tiên trong ngân sách tick-to-trade của Chương 74. Sai ở
//! đây thì mọi thứ phía sau đều tính trên dữ liệu rác.

use std::collections::{BTreeMap, HashMap};

// ============================================================================
// 1. GIAO THỨC NHỊ PHÂN — vì sao sàn không dùng JSON
// ============================================================================
// Một bản tin JSON tốn ~100 byte và mất hàng micro-giây để phân tích. Cùng
// thông tin đó ở dạng nhị phân cố định tốn 42 byte và đọc xong trong vài chục
// nano-giây — chỉ là vài phép đọc số nguyên từ vị trí đã biết trước.

pub type Gia = i64;      // tick, 1 tick = 0,01 đơn vị tiền
pub type SoLuong = u32;
pub type MaLenh = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chieu { Mua, Ban }

#[derive(Debug, Clone, PartialEq)]
pub enum BanTin {
    /// Thêm lệnh mới vào sổ
    ThemLenh { ma: MaLenh, ma_ck: u32, chieu: Chieu, gia: Gia, so_luong: SoLuong },
    /// Lệnh bị huỷ một phần hoặc toàn bộ
    HuyLenh { ma: MaLenh, so_luong_huy: SoLuong },
    /// Lệnh khớp
    KhopLenh { ma: MaLenh, so_luong: SoLuong, gia: Gia },
    /// Thay thế lệnh: huỷ cũ, tạo mới, MẤT ưu tiên thời gian
    ThayThe { ma_cu: MaLenh, ma_moi: MaLenh, gia: Gia, so_luong: SoLuong },
}

#[derive(Debug, Clone, PartialEq)]
pub struct GoiTinTruong {
    pub so_thu_tu: u64,
    pub thoi_diem_ns: u64,
    pub ban_tin: BanTin,
}

#[derive(Debug, PartialEq)]
pub enum LoiPhanTich {
    QuaNgan { can: usize, co: usize },
    LoaiBanTinLa(u8),
    ChieuLa(u8),
}

/// Phân tích một bản tin nhị phân. Không cấp phát, không sao chép — chỉ đọc
/// số nguyên từ các vị trí cố định. Đây là ý nghĩa của "phân tích zero-copy".
///
/// Bố cục dây (big-endian, như mọi giao thức mạng):
/// ```text
///  0        1        9           17     25      29      30       38
///  +--------+--------+-----------+------+-------+-------+--------+
///  | loại   | stt    | thời điểm | mã   | mã ck | chiều | giá    | số lượng
///  | 1 byte | 8 byte | 8 byte    |8 byte| 4 byte| 1 byte| 8 byte | 4 byte
/// ```
pub fn phan_tich(b: &[u8]) -> Result<GoiTinTruong, LoiPhanTich> {
    if b.len() < 17 { return Err(LoiPhanTich::QuaNgan { can: 17, co: b.len() }); }
    let loai = b[0];
    let so_thu_tu = u64::from_be_bytes(b[1..9].try_into().unwrap());
    let thoi_diem_ns = u64::from_be_bytes(b[9..17].try_into().unwrap());

    let can = match loai { b'A' => 42, b'X' => 29, b'E' => 37, b'R' => 45, _ => 17 };
    if b.len() < can { return Err(LoiPhanTich::QuaNgan { can, co: b.len() }); }

    let doc_u32 = |i: usize| -> u32 { u32::from_be_bytes(b[i..i + 4].try_into().unwrap()) };
    let doc_i64 = |i: usize| -> i64 { i64::from_be_bytes(b[i..i + 8].try_into().unwrap()) };
    let doc_u64 = |i: usize| -> u64 { u64::from_be_bytes(b[i..i + 8].try_into().unwrap()) };

    let ban_tin = match loai {
        b'A' => BanTin::ThemLenh {
            ma: doc_u64(17), ma_ck: doc_u32(25),
            chieu: match b[29] { b'B' => Chieu::Mua, b'S' => Chieu::Ban,
                                 x => return Err(LoiPhanTich::ChieuLa(x)) },
            gia: doc_i64(30), so_luong: doc_u32(38),
        },
        b'X' => BanTin::HuyLenh { ma: doc_u64(17), so_luong_huy: doc_u32(25) },
        b'E' => BanTin::KhopLenh { ma: doc_u64(17), so_luong: doc_u32(25), gia: doc_i64(29) },
        b'R' => BanTin::ThayThe {
            ma_cu: doc_u64(17), ma_moi: doc_u64(25),
            gia: doc_i64(33), so_luong: doc_u32(41),
        },
        x => return Err(LoiPhanTich::LoaiBanTinLa(x)),
    };
    Ok(GoiTinTruong { so_thu_tu, thoi_diem_ns, ban_tin })
}

/// Mã hoá ngược — dùng để sinh dữ liệu kiểm thử và để ghi lại phiên (Chương 76).
pub fn ma_hoa(g: &GoiTinTruong) -> Vec<u8> {
    let mut v = Vec::with_capacity(48);
    let loai = match g.ban_tin {
        BanTin::ThemLenh { .. } => b'A', BanTin::HuyLenh { .. } => b'X',
        BanTin::KhopLenh { .. } => b'E', BanTin::ThayThe { .. } => b'R',
    };
    v.push(loai);
    v.extend_from_slice(&g.so_thu_tu.to_be_bytes());
    v.extend_from_slice(&g.thoi_diem_ns.to_be_bytes());
    match &g.ban_tin {
        BanTin::ThemLenh { ma, ma_ck, chieu, gia, so_luong } => {
            v.extend_from_slice(&ma.to_be_bytes());
            v.extend_from_slice(&ma_ck.to_be_bytes());
            v.push(if *chieu == Chieu::Mua { b'B' } else { b'S' });
            v.extend_from_slice(&gia.to_be_bytes());
            v.extend_from_slice(&so_luong.to_be_bytes());
        }
        BanTin::HuyLenh { ma, so_luong_huy } => {
            v.extend_from_slice(&ma.to_be_bytes());
            v.extend_from_slice(&so_luong_huy.to_be_bytes());
        }
        BanTin::KhopLenh { ma, so_luong, gia } => {
            v.extend_from_slice(&ma.to_be_bytes());
            v.extend_from_slice(&so_luong.to_be_bytes());
            v.extend_from_slice(&gia.to_be_bytes());
        }
        BanTin::ThayThe { ma_cu, ma_moi, gia, so_luong } => {
            v.extend_from_slice(&ma_cu.to_be_bytes());
            v.extend_from_slice(&ma_moi.to_be_bytes());
            v.extend_from_slice(&gia.to_be_bytes());
            v.extend_from_slice(&so_luong.to_be_bytes());
        }
    }
    v
}

// ============================================================================
// 2. PHÁT HIỆN KHE SỐ THỨ TỰ
// ============================================================================
// Dữ liệu thị trường thường đi qua UDP multicast: nhanh, nhưng KHÔNG bảo đảm
// tới nơi và KHÔNG bảo đảm đúng thứ tự. Số thứ tự là thứ duy nhất cho ta biết
// mình có đang nhìn bức tranh đầy đủ hay không.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum KetQuaNhan {
    /// Đúng bản tin kế tiếp — xử lý ngay
    DungThuTu,
    /// Bản tin cũ (bản sao từ luồng dự phòng) — bỏ qua
    TrungLap,
    /// PHÁT HIỆN khe lần đầu: thiếu `so_ban_tin_mat` bản tin.
    /// Đây là lúc DUY NHẤT ta gửi yêu cầu phát lại.
    ThieuBanTin { tu: u64, den: u64, so_ban_tin_mat: u64 },
    /// Đã biết có khe rồi, đang chờ dữ liệu phát lại. Bản tin mới vẫn được
    /// đệm lại nhưng KHÔNG xin phát lại nữa.
    DangChoKhoiPhuc,
}

pub struct BoPhatHienKhe {
    pub ky_vong: u64,
    /// Bản tin tới sớm được giữ lại, xử lý sau khi khe được lấp.
    dem_lai: BTreeMap<u64, GoiTinTruong>,
    /// Khe đang chờ lấp: (bản tin đầu thiếu, bản tin cuối thiếu).
    /// Có giá trị nghĩa là ta đang ở CHẾ ĐỘ KHÔI PHỤC.
    khe_dang_cho: Option<(u64, u64)>,
    pub so_khe: u64,
    pub so_trung_lap: u64,
    pub tong_ban_tin_mat: u64,
}

impl BoPhatHienKhe {
    pub fn moi(bat_dau: u64) -> Self {
        BoPhatHienKhe { ky_vong: bat_dau, dem_lai: BTreeMap::new(), khe_dang_cho: None,
                        so_khe: 0, so_trung_lap: 0, tong_ban_tin_mat: 0 }
    }

    pub fn dang_khoi_phuc(&self) -> bool { self.khe_dang_cho.is_some() }

    pub fn nhan(&mut self, g: GoiTinTruong) -> KetQuaNhan {
        let stt = g.so_thu_tu;
        if stt < self.ky_vong {
            self.so_trung_lap += 1;
            return KetQuaNhan::TrungLap;
        }
        if stt > self.ky_vong {
            self.dem_lai.insert(stt, g); // luôn giữ lại, đừng bao giờ vứt
            // Đã biết có khe rồi thì chỉ đệm tiếp. Nếu báo lại mỗi bản tin,
            // ta sẽ gửi hàng nghìn yêu cầu phát lại cho CÙNG một khe và tự
            // làm sập luồng khôi phục của sàn — lỗi vận hành có thật.
            if let Some((_, den)) = &mut self.khe_dang_cho {
                if stt > *den + 1 { *den = stt - 1; }
                return KetQuaNhan::DangChoKhoiPhuc;
            }
            let (tu, den) = (self.ky_vong, stt - 1);
            self.khe_dang_cho = Some((tu, den));
            self.so_khe += 1;
            self.tong_ban_tin_mat += den - tu + 1;
            return KetQuaNhan::ThieuBanTin { tu, den, so_ban_tin_mat: den - tu + 1 };
        }
        self.ky_vong += 1;
        self.dem_lai.insert(stt, g);
        KetQuaNhan::DungThuTu
    }

    /// Rút các bản tin liền mạch đã sẵn sàng xử lý, theo đúng thứ tự.
    pub fn rut_lien_mach(&mut self) -> Vec<GoiTinTruong> {
        let mut ra = Vec::new();
        let mut mong = match self.dem_lai.keys().next() { Some(&k) => k, None => return ra };
        while let Some(g) = self.dem_lai.remove(&mong) {
            ra.push(g);
            mong += 1;
        }
        ra
    }

    /// Lấp khe bằng dữ liệu phát lại từ luồng khôi phục. Khi mọi bản tin
    /// thiếu đã về đủ, ta rời chế độ khôi phục và chạy bình thường trở lại.
    pub fn lap_khe(&mut self, cac_goi: Vec<GoiTinTruong>) {
        for g in cac_goi {
            let stt = g.so_thu_tu;
            self.dem_lai.insert(stt, g);
        }
        // Đẩy kỳ vọng qua toàn bộ phần đã liền mạch
        while self.dem_lai.contains_key(&self.ky_vong) { self.ky_vong += 1; }
        if let Some((_, den)) = self.khe_dang_cho {
            if self.ky_vong > den { self.khe_dang_cho = None; }
        }
    }

    pub fn so_dang_dem(&self) -> usize { self.dem_lai.len() }
}

// ============================================================================
// 3. SỔ LỆNH L2 — tổng hợp theo MỨC GIÁ
// ============================================================================
// L2 là thứ 95% chiến lược thật sự cần: mỗi mức giá còn bao nhiêu khối lượng.
// Nhẹ hơn L3 rất nhiều, và cập nhật nhanh hơn.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MucGia { pub gia: Gia, pub khoi_luong: u64, pub so_lenh: u32 }

#[derive(Debug, Default)]
pub struct SoLenhL2 {
    /// Bên mua lưu khoá ÂM để `BTreeMap` trả giá cao nhất trước.
    mua: BTreeMap<Gia, (u64, u32)>,
    ban: BTreeMap<Gia, (u64, u32)>,
}

impl SoLenhL2 {
    pub fn moi() -> Self { SoLenhL2::default() }

    pub fn them(&mut self, chieu: Chieu, gia: Gia, kl: SoLuong) {
        let (ban_do, khoa) = match chieu {
            Chieu::Mua => (&mut self.mua, -gia),
            Chieu::Ban => (&mut self.ban, gia),
        };
        let e = ban_do.entry(khoa).or_insert((0, 0));
        e.0 += kl as u64;
        e.1 += 1;
    }

    /// Trả `true` nếu mức giá bị xoá hẳn khỏi sổ.
    pub fn bot(&mut self, chieu: Chieu, gia: Gia, kl: SoLuong, bot_mot_lenh: bool) -> bool {
        let (ban_do, khoa) = match chieu {
            Chieu::Mua => (&mut self.mua, -gia),
            Chieu::Ban => (&mut self.ban, gia),
        };
        if let Some(e) = ban_do.get_mut(&khoa) {
            e.0 = e.0.saturating_sub(kl as u64);
            if bot_mot_lenh { e.1 = e.1.saturating_sub(1); }
            // Mức giá hết khối lượng phải BIẾN MẤT, không được để lại mức rỗng —
            // nếu không, "giá tốt nhất" sẽ trỏ vào chỗ không có gì.
            if e.0 == 0 { ban_do.remove(&khoa); return true; }
        }
        false
    }

    pub fn gia_mua_tot_nhat(&self) -> Option<Gia> { self.mua.keys().next().map(|k| -k) }
    pub fn gia_ban_tot_nhat(&self) -> Option<Gia> { self.ban.keys().next().copied() }
    pub fn chenh_lech(&self) -> Option<Gia> {
        Some(self.gia_ban_tot_nhat()? - self.gia_mua_tot_nhat()?)
    }
    pub fn so_muc(&self, chieu: Chieu) -> usize {
        match chieu { Chieu::Mua => self.mua.len(), Chieu::Ban => self.ban.len() }
    }
    pub fn khoi_luong_tai(&self, chieu: Chieu, gia: Gia) -> u64 {
        let (bd, k) = match chieu {
            Chieu::Mua => (&self.mua, -gia), Chieu::Ban => (&self.ban, gia) };
        bd.get(&k).map_or(0, |e| e.0)
    }

    /// `n` mức giá tốt nhất mỗi bên — đúng thứ mà giao diện và chiến lược cần.
    pub fn dinh_so(&self, n: usize) -> (Vec<MucGia>, Vec<MucGia>) {
        let m = self.mua.iter().take(n)
            .map(|(k, v)| MucGia { gia: -k, khoi_luong: v.0, so_lenh: v.1 }).collect();
        let b = self.ban.iter().take(n)
            .map(|(k, v)| MucGia { gia: *k, khoi_luong: v.0, so_lenh: v.1 }).collect();
        (m, b)
    }

    /// Giá bình quân gia quyền theo khối lượng đối ứng — ước lượng "giá trị
    /// thật" tốt hơn giá giữa, vì nó tính cả độ mất cân bằng cung cầu.
    pub fn gia_can_bang(&self) -> Option<f64> {
        let (m, b) = self.dinh_so(1);
        let (m, b) = (m.first()?, b.first()?);
        let tong = (m.khoi_luong + b.khoi_luong) as f64;
        if tong == 0.0 { return None; }
        // Bên nào NHIỀU khối lượng hơn thì giá cân bằng lệch về phía bên kia
        Some((m.gia as f64 * b.khoi_luong as f64 + b.gia as f64 * m.khoi_luong as f64) / tong)
    }

    // ---- Kiểm tra chất lượng dữ liệu ----

    /// Sổ "khoá" (locked): giá mua = giá bán. Hiếm nhưng hợp lệ ở vài thị trường.
    pub fn bi_khoa(&self) -> bool { self.chenh_lech() == Some(0) }

    /// Sổ "chéo" (crossed): giá mua > giá bán. LUÔN LUÔN là dấu hiệu dữ liệu
    /// hỏng hoặc mất bản tin — phải dừng giao dịch ngay, đừng cố khai thác.
    pub fn bi_cheo(&self) -> bool { self.chenh_lech().is_some_and(|c| c < 0) }

    pub fn lanh_manh(&self) -> bool { !self.bi_cheo() }
}

// ============================================================================
// 4. SỔ LỆNH L3 — theo TỪNG LỆNH
// ============================================================================
// L3 giữ danh tính từng lệnh. Nặng hơn nhiều, nhưng là thứ duy nhất trả lời
// được "lệnh của TÔI đang đứng thứ mấy trong hàng?" — câu hỏi sống còn với
// chiến lược tạo lập thị trường.

#[derive(Debug, Clone, PartialEq)]
pub struct LenhL3 { pub ma: MaLenh, pub chieu: Chieu, pub gia: Gia, pub con_lai: SoLuong }

/// `Chieu` không cài `Ord`, nên dùng bản có thứ tự làm khoá bản đồ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Chieu2 { Mua, Ban }

impl From<Chieu> for Chieu2 {
    fn from(c: Chieu) -> Self { match c { Chieu::Mua => Chieu2::Mua, Chieu::Ban => Chieu2::Ban } }
}

#[derive(Debug, Default)]
pub struct SoLenhL3 {
    pub lenh: HashMap<MaLenh, LenhL3>,
    /// Thứ tự tới của từng mức giá — nền của ưu tiên thời gian.
    hang: BTreeMap<(Chieu2, Gia), Vec<MaLenh>>,
    pub l2: SoLenhL2,
}

impl SoLenhL3 {
    pub fn moi() -> Self { SoLenhL3::default() }

    pub fn ap_dung(&mut self, bt: &BanTin) {
        match bt {
            BanTin::ThemLenh { ma, chieu, gia, so_luong, .. } => {
                self.lenh.insert(*ma,
                    LenhL3 { ma: *ma, chieu: *chieu, gia: *gia, con_lai: *so_luong });
                self.hang.entry(((*chieu).into(), *gia)).or_default().push(*ma);
                self.l2.them(*chieu, *gia, *so_luong);
            }
            BanTin::HuyLenh { ma, so_luong_huy } => {
                if let Some(l) = self.lenh.get_mut(ma) {
                    let thuc_huy = (*so_luong_huy).min(l.con_lai);
                    l.con_lai -= thuc_huy;
                    let (c, g, het) = (l.chieu, l.gia, l.con_lai == 0);
                    self.l2.bot(c, g, thuc_huy, het);
                    if het { self.go_khoi_hang(*ma, c, g); self.lenh.remove(ma); }
                }
            }
            BanTin::KhopLenh { ma, so_luong, .. } => {
                if let Some(l) = self.lenh.get_mut(ma) {
                    let thuc = (*so_luong).min(l.con_lai);
                    l.con_lai -= thuc;
                    let (c, g, het) = (l.chieu, l.gia, l.con_lai == 0);
                    self.l2.bot(c, g, thuc, het);
                    if het { self.go_khoi_hang(*ma, c, g); self.lenh.remove(ma); }
                }
            }
            BanTin::ThayThe { ma_cu, ma_moi, gia, so_luong } => {
                // Thay thế = huỷ hẳn rồi thêm mới. Lệnh MẤT ưu tiên thời gian,
                // xuống cuối hàng — đây là lý do sửa lệnh rất đắt trong HFT.
                if let Some(l) = self.lenh.remove(ma_cu) {
                    self.l2.bot(l.chieu, l.gia, l.con_lai, true);
                    self.go_khoi_hang(*ma_cu, l.chieu, l.gia);
                    self.lenh.insert(*ma_moi,
                        LenhL3 { ma: *ma_moi, chieu: l.chieu, gia: *gia, con_lai: *so_luong });
                    self.hang.entry((l.chieu.into(), *gia)).or_default().push(*ma_moi);
                    self.l2.them(l.chieu, *gia, *so_luong);
                }
            }
        }
    }

    fn go_khoi_hang(&mut self, ma: MaLenh, c: Chieu, g: Gia) {
        if let Some(h) = self.hang.get_mut(&(c.into(), g)) {
            h.retain(|&x| x != ma);
            if h.is_empty() { self.hang.remove(&(c.into(), g)); }
        }
    }

    /// Lệnh này đứng thứ mấy trong hàng ở mức giá của nó? (0 = đầu hàng)
    /// Câu trả lời quyết định xác suất được khớp.
    pub fn vi_tri_trong_hang(&self, ma: MaLenh) -> Option<usize> {
        let l = self.lenh.get(&ma)?;
        self.hang.get(&(l.chieu.into(), l.gia))?.iter().position(|&x| x == ma)
    }

    /// Khối lượng đứng TRƯỚC lệnh này — phải khớp hết chỗ đó thì mới tới lượt ta.
    pub fn khoi_luong_dung_truoc(&self, ma: MaLenh) -> Option<u64> {
        let l = self.lenh.get(&ma)?;
        let h = self.hang.get(&(l.chieu.into(), l.gia))?;
        let vt = h.iter().position(|&x| x == ma)?;
        Some(h[..vt].iter().filter_map(|m| self.lenh.get(m)).map(|x| x.con_lai as u64).sum())
    }

    pub fn so_lenh_dang_mo(&self) -> usize { self.lenh.len() }
}

// ============================================================================
// 5. SINH DỮ LIỆU PHIÊN TẤT ĐỊNH
// ============================================================================

pub fn sinh_phien(so_ban_tin: usize, hat_giong: u64) -> Vec<GoiTinTruong> {
    let mut s = hat_giong;
    let mut ra = Vec::with_capacity(so_ban_tin);
    let mut ma_lenh: u64 = 1;
    let mut dang_mo: Vec<(MaLenh, Chieu, Gia, SoLuong)> = Vec::new();
    let mut t: u64 = 1_000_000_000;

    for stt in 0..so_ban_tin as u64 {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let r = (s >> 33) % 100;
        t += 1_000 + (s >> 20) % 50_000;

        // Giữ sổ có ít nhất vài lệnh trước khi bắt đầu huỷ/khớp
        let bt = if dang_mo.len() < 4 || r < 55 {
            let chieu = if (s >> 40) % 2 == 0 { Chieu::Mua } else { Chieu::Ban };
            // Bên mua đặt dưới 8400, bên bán đặt trên 8400 → sổ không bao giờ chéo
            let lech = ((s >> 44) % 20) as i64;
            let gia = match chieu {
                Chieu::Mua => 8_400 - 1 - lech,
                Chieu::Ban => 8_400 + 1 + lech,
            };
            let sl = 100 + ((s >> 48) % 10) as u32 * 100;
            dang_mo.push((ma_lenh, chieu, gia, sl));
            let bt = BanTin::ThemLenh { ma: ma_lenh, ma_ck: 1, chieu, gia, so_luong: sl };
            ma_lenh += 1;
            bt
        } else {
            let i = ((s >> 52) as usize) % dang_mo.len();
            let (ma, _, gia, sl) = dang_mo[i];
            let phan = (sl / 2).max(1);
            if r < 80 {
                dang_mo.remove(i);
                BanTin::HuyLenh { ma, so_luong_huy: sl }
            } else {
                dang_mo[i].3 -= phan;
                if dang_mo[i].3 == 0 { dang_mo.remove(i); }
                BanTin::KhopLenh { ma, so_luong: phan, gia }
            }
        };
        ra.push(GoiTinTruong { so_thu_tu: stt, thoi_diem_ns: t, ban_tin: bt });
    }
    ra
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LUỒNG DỮ LIỆU THỊ TRƯỜNG: NHỊ PHÂN · KHE · SỔ L2/L3     ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. GIAO THỨC NHỊ PHÂN vs JSON");
    let g = GoiTinTruong {
        so_thu_tu: 12345, thoi_diem_ns: 1_700_000_000_000_000_000,
        ban_tin: BanTin::ThemLenh { ma: 999, ma_ck: 1, chieu: Chieu::Mua,
                                    gia: 8_450, so_luong: 100 },
    };
    let b = ma_hoa(&g);
    let json = r#"{"seq":12345,"ts":1700000000000000000,"type":"add","id":999,"sym":"VNM","side":"B","px":84.50,"qty":100}"#;
    println!("   Nhị phân: {} byte", b.len());
    println!("   JSON    : {} byte → gấp {:.1} lần", json.len(), json.len() as f64 / b.len() as f64);
    println!("   Phân tích ngược ra đúng bản gốc: {}", phan_tich(&b).unwrap() == g);

    println!("\n2. PHÁT HIỆN KHE SỐ THỨ TỰ");
    let mut pd = BoPhatHienKhe::moi(0);
    let phien = sinh_phien(10, 7);
    for (i, gt) in phien.iter().enumerate() {
        if i == 3 || i == 4 { continue; } // giả lập mất 2 gói UDP
        let kq = pd.nhan(gt.clone());
        if kq != KetQuaNhan::DungThuTu { println!("   stt {} → {:?}", gt.so_thu_tu, kq); }
    }
    println!("   Tổng khe: {} · tổng bản tin mất: {} · đang đệm: {}",
             pd.so_khe, pd.tong_ban_tin_mat, pd.so_dang_dem());
    pd.lap_khe(vec![phien[3].clone(), phien[4].clone()]);
    println!("   Đang ở chế độ khôi phục: {}", pd.dang_khoi_phuc());
    println!("   Sau khi phát lại → còn khôi phục: {} · rút liền mạch được {} bản tin",
             pd.dang_khoi_phuc(), pd.rut_lien_mach().len());

    println!("\n3. DỰNG SỔ L2 TỪ 5000 BẢN TIN");
    let mut so = SoLenhL3::moi();
    for g in sinh_phien(5_000, 42) { so.ap_dung(&g.ban_tin); }
    let (mua, ban) = so.l2.dinh_so(5);
    println!("   {} lệnh đang mở · {} mức mua · {} mức bán",
             so.so_lenh_dang_mo(), so.l2.so_muc(Chieu::Mua), so.l2.so_muc(Chieu::Ban));
    println!("   ── 5 MỨC TỐT NHẤT ──");
    for m in ban.iter().rev() {
        println!("        BÁN {:>7.2}  {:>6} ({} lệnh)",
                 m.gia as f64 / 100.0, m.khoi_luong, m.so_lenh);
    }
    println!("        ─────────────  chênh lệch {} tick", so.l2.chenh_lech().unwrap_or(0));
    for m in &mua {
        println!("        MUA {:>7.2}  {:>6} ({} lệnh)",
                 m.gia as f64 / 100.0, m.khoi_luong, m.so_lenh);
    }
    println!("   Giá cân bằng theo khối lượng: {:.2}",
             so.l2.gia_can_bang().unwrap_or(0.0) / 100.0);

    println!("\n4. KIỂM TRA CHẤT LƯỢNG DỮ LIỆU");
    println!("   Sổ lành mạnh: {} · bị khoá: {} · bị chéo: {}",
             so.l2.lanh_manh(), so.l2.bi_khoa(), so.l2.bi_cheo());
    let mut hong = SoLenhL2::moi();
    hong.them(Chieu::Mua, 8_500, 100);
    hong.them(Chieu::Ban, 8_400, 100); // mua CAO hơn bán → vô lý
    println!("   Sổ dựng sai (mua 85.00 > bán 84.00) → bị chéo: {} · lành mạnh: {}",
             hong.bi_cheo(), hong.lanh_manh());
    println!("   → Gặp sổ chéo phải NGỪNG giao dịch, không được coi là cơ hội.");

    println!("\n5. VỊ TRÍ TRONG HÀNG — câu hỏi sống còn của tạo lập thị trường");
    let mut s3 = SoLenhL3::moi();
    for (ma, sl) in [(1u64, 500u32), (2, 300), (3, 200)] {
        s3.ap_dung(&BanTin::ThemLenh { ma, ma_ck: 1, chieu: Chieu::Mua,
                                       gia: 8_400, so_luong: sl });
    }
    for ma in [1u64, 2, 3] {
        println!("   Lệnh #{} → đứng thứ {} · phải chờ {} đơn vị khớp trước",
                 ma, s3.vi_tri_trong_hang(ma).unwrap(),
                 s3.khoi_luong_dung_truoc(ma).unwrap());
    }
    s3.ap_dung(&BanTin::ThayThe { ma_cu: 1, ma_moi: 4, gia: 8_400, so_luong: 500 });
    println!("   Sửa lệnh #1 (thành #4) → giờ đứng thứ {} — MẤT SẠCH ưu tiên thời gian",
             s3.vi_tri_trong_hang(4).unwrap());

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   SAI MỘT BẢN TIN LÀ SAI TOÀN BỘ QUYẾT ĐỊNH SAU ĐÓ         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    // ---------- Giao thức nhị phân ----------
    #[test]
    fn ma_hoa_roi_phan_tich_ra_dung_ban_goc() {
        let cac_bt = vec![
            BanTin::ThemLenh { ma: 1, ma_ck: 7, chieu: Chieu::Mua, gia: 8_450, so_luong: 100 },
            BanTin::ThemLenh { ma: 2, ma_ck: 7, chieu: Chieu::Ban, gia: -50, so_luong: 1 },
            BanTin::HuyLenh { ma: 3, so_luong_huy: 250 },
            BanTin::KhopLenh { ma: 4, so_luong: 75, gia: 8_400 },
            BanTin::ThayThe { ma_cu: 5, ma_moi: 6, gia: 8_390, so_luong: 999 },
        ];
        for bt in cac_bt {
            let g = GoiTinTruong { so_thu_tu: 42, thoi_diem_ns: 1_700_000_000_000_000_000,
                                   ban_tin: bt };
            assert_eq!(phan_tich(&ma_hoa(&g)), Ok(g.clone()), "vòng mã hoá phải khép kín");
        }
    }

    #[test]
    fn phan_tich_tu_choi_goi_qua_ngan() {
        assert_eq!(phan_tich(&[]), Err(LoiPhanTich::QuaNgan { can: 17, co: 0 }));
        assert_eq!(phan_tich(&[b'A'; 10]), Err(LoiPhanTich::QuaNgan { can: 17, co: 10 }));
        // Đủ phần đầu chung nhưng thiếu thân bản tin 'A'
        let mut b = vec![b'A']; b.extend_from_slice(&[0u8; 20]);
        assert!(matches!(phan_tich(&b), Err(LoiPhanTich::QuaNgan { .. })));
    }

    #[test]
    fn phan_tich_tu_choi_loai_ban_tin_la() {
        let mut b = vec![b'Z']; b.extend_from_slice(&[0u8; 60]);
        assert_eq!(phan_tich(&b), Err(LoiPhanTich::LoaiBanTinLa(b'Z')));
    }

    #[test]
    fn phan_tich_tu_choi_ma_chieu_la() {
        let g = GoiTinTruong { so_thu_tu: 1, thoi_diem_ns: 1,
            ban_tin: BanTin::ThemLenh { ma: 1, ma_ck: 1, chieu: Chieu::Mua,
                                        gia: 100, so_luong: 1 } };
        let mut b = ma_hoa(&g);
        b[29] = b'?'; // phá byte chiều
        assert_eq!(phan_tich(&b), Err(LoiPhanTich::ChieuLa(b'?')));
    }

    #[test]
    fn nhi_phan_gon_hon_json_nhieu_lan() {
        let g = GoiTinTruong { so_thu_tu: 12345, thoi_diem_ns: 1_700_000_000_000_000_000,
            ban_tin: BanTin::ThemLenh { ma: 999, ma_ck: 1, chieu: Chieu::Mua,
                                        gia: 8_450, so_luong: 100 } };
        assert_eq!(ma_hoa(&g).len(), 42, "bản tin thêm lệnh dài đúng 42 byte cố định");
        assert!(ma_hoa(&g).len() * 2 < 105, "nhị phân phải gọn hơn JSON ít nhất 2 lần");
    }

    #[test]
    fn dung_thu_tu_byte_lon_truoc() {
        // Giao thức mạng LUÔN dùng big-endian. Nhầm sang little-endian thì
        // số nhỏ vẫn "chạy" nhưng giá trị hoàn toàn sai.
        let g = GoiTinTruong { so_thu_tu: 0x0102030405060708, thoi_diem_ns: 0,
            ban_tin: BanTin::HuyLenh { ma: 1, so_luong_huy: 1 } };
        let b = ma_hoa(&g);
        assert_eq!(&b[1..9], &[1, 2, 3, 4, 5, 6, 7, 8], "byte cao đứng TRƯỚC");
    }

    // ---------- Phát hiện khe ----------
    #[test]
    fn luong_lien_mach_khong_bao_khe() {
        let mut p = BoPhatHienKhe::moi(0);
        for g in sinh_phien(100, 1) {
            assert_eq!(p.nhan(g), KetQuaNhan::DungThuTu);
        }
        assert_eq!(p.so_khe, 0);
        assert_eq!(p.ky_vong, 100);
    }

    #[test]
    fn phat_hien_dung_khe_va_so_ban_tin_mat() {
        let phien = sinh_phien(10, 2);
        let mut p = BoPhatHienKhe::moi(0);
        for (i, g) in phien.iter().enumerate() {
            if (3..=5).contains(&i) { continue; } // mất gói 3,4,5
            let kq = p.nhan(g.clone());
            if i == 6 {
                assert_eq!(kq, KetQuaNhan::ThieuBanTin { tu: 3, den: 5, so_ban_tin_mat: 3 });
            } else if i > 6 {
                assert_eq!(kq, KetQuaNhan::DangChoKhoiPhuc,
                           "các bản tin sau chỉ được đệm, không xin phát lại nữa");
            }
        }
        assert_eq!(p.so_khe, 1);
        assert_eq!(p.tong_ban_tin_mat, 3);
    }

    #[test]
    fn chi_xin_phat_lai_MOT_lan_cho_mot_khe() {
        // Nếu báo khe ở mọi bản tin sau đó, ta sẽ gửi hàng nghìn yêu cầu phát
        // lại cho cùng một khe và tự làm sập luồng khôi phục của sàn.
        let phien = sinh_phien(20, 8);
        let mut p = BoPhatHienKhe::moi(0);
        let mut so_lan_bao_khe = 0;
        for (i, g) in phien.iter().enumerate() {
            if (3..=5).contains(&i) { continue; }
            if matches!(p.nhan(g.clone()), KetQuaNhan::ThieuBanTin { .. }) {
                so_lan_bao_khe += 1;
            }
        }
        assert_eq!(so_lan_bao_khe, 1, "một khe chỉ được xin phát lại đúng một lần");
        assert_eq!(p.so_khe, 1);
        assert_eq!(p.tong_ban_tin_mat, 3);
        assert!(p.dang_khoi_phuc(), "vẫn đang chờ dữ liệu phát lại");
    }

    #[test]
    fn roi_che_do_khoi_phuc_sau_khi_khe_duoc_lap_du() {
        let phien = sinh_phien(20, 8);
        let mut p = BoPhatHienKhe::moi(0);
        for (i, g) in phien.iter().enumerate() {
            if (3..=5).contains(&i) { continue; }
            p.nhan(g.clone());
        }
        assert!(p.dang_khoi_phuc());
        p.lap_khe(vec![phien[3].clone(), phien[4].clone()]);
        assert!(p.dang_khoi_phuc(), "còn thiếu bản tin 5 thì vẫn đang khôi phục");
        p.lap_khe(vec![phien[5].clone()]);
        assert!(!p.dang_khoi_phuc(), "đủ rồi thì phải trở lại bình thường");
        assert_eq!(p.rut_lien_mach().len(), 20);
    }

    #[test]
    fn ban_tin_trung_lap_bi_bo_qua() {
        // Sàn thường phát hai luồng giống hệt (A và B) để chống mất gói.
        // Bản sao đến sau phải bị loại, không được xử lý hai lần.
        let phien = sinh_phien(5, 3);
        let mut p = BoPhatHienKhe::moi(0);
        for g in &phien { p.nhan(g.clone()); }
        for g in &phien {
            assert_eq!(p.nhan(g.clone()), KetQuaNhan::TrungLap);
        }
        assert_eq!(p.so_trung_lap, 5);
        assert_eq!(p.ky_vong, 5, "trùng lặp không được đẩy kỳ vọng đi");
    }

    #[test]
    fn ban_tin_toi_som_duoc_dem_lai_chu_khong_vut_di() {
        let phien = sinh_phien(10, 4);
        let mut p = BoPhatHienKhe::moi(0);
        p.nhan(phien[0].clone());
        p.nhan(phien[5].clone()); // nhảy cóc
        assert_eq!(p.so_dang_dem(), 2, "cả hai đều phải được giữ lại");
        assert_eq!(p.rut_lien_mach().len(), 1, "chỉ rút được phần liền mạch từ đầu");
    }

    #[test]
    fn lap_khe_roi_rut_duoc_toan_bo() {
        let phien = sinh_phien(10, 5);
        let mut p = BoPhatHienKhe::moi(0);
        for (i, g) in phien.iter().enumerate() {
            if i == 3 || i == 4 { continue; }
            p.nhan(g.clone());
        }
        p.lap_khe(vec![phien[3].clone(), phien[4].clone()]);
        let ra = p.rut_lien_mach();
        assert_eq!(ra.len(), 10, "sau khi lấp khe phải rút được đủ 10 bản tin");
        for (i, g) in ra.iter().enumerate() {
            assert_eq!(g.so_thu_tu, i as u64, "và đúng thứ tự");
        }
    }

    // ---------- Sổ L2 ----------
    #[test]
    fn l2_tra_dung_gia_tot_nhat_hai_ben() {
        let mut s = SoLenhL2::moi();
        s.them(Chieu::Mua, 8_390, 100);
        s.them(Chieu::Mua, 8_400, 200); // cao hơn = tốt hơn cho bên mua
        s.them(Chieu::Ban, 8_420, 150);
        s.them(Chieu::Ban, 8_410, 50);  // thấp hơn = tốt hơn cho bên bán
        assert_eq!(s.gia_mua_tot_nhat(), Some(8_400));
        assert_eq!(s.gia_ban_tot_nhat(), Some(8_410));
        assert_eq!(s.chenh_lech(), Some(10));
    }

    #[test]
    fn l2_gop_khoi_luong_va_dem_so_lenh_cung_muc_gia() {
        let mut s = SoLenhL2::moi();
        for _ in 0..3 { s.them(Chieu::Mua, 8_400, 100); }
        let (m, _) = s.dinh_so(1);
        assert_eq!(m[0].khoi_luong, 300);
        assert_eq!(m[0].so_lenh, 3);
    }

    #[test]
    fn muc_gia_het_khoi_luong_phai_bien_mat_khoi_so() {
        // Nếu để lại mức rỗng, `gia_mua_tot_nhat` sẽ trỏ vào chỗ không có gì —
        // và chiến lược sẽ gửi lệnh vào hư không.
        let mut s = SoLenhL2::moi();
        s.them(Chieu::Mua, 8_400, 100);
        s.them(Chieu::Mua, 8_390, 50);
        assert!(s.bot(Chieu::Mua, 8_400, 100, true), "phải báo mức giá đã bị xoá");
        assert_eq!(s.gia_mua_tot_nhat(), Some(8_390), "đỉnh sổ phải tụt xuống mức kế");
        assert_eq!(s.so_muc(Chieu::Mua), 1);
    }

    #[test]
    fn bot_qua_khoi_luong_khong_lam_am_so() {
        let mut s = SoLenhL2::moi();
        s.them(Chieu::Mua, 8_400, 100);
        assert!(s.bot(Chieu::Mua, 8_400, 99_999, true), "trừ quá cũng chỉ về 0");
        assert_eq!(s.khoi_luong_tai(Chieu::Mua, 8_400), 0);
        assert_eq!(s.so_muc(Chieu::Mua), 0);
    }

    #[test]
    fn so_rong_khong_panic_va_khong_bao_cheo() {
        let s = SoLenhL2::moi();
        assert_eq!(s.gia_mua_tot_nhat(), None);
        assert_eq!(s.chenh_lech(), None);
        assert!(!s.bi_cheo() && !s.bi_khoa() && s.lanh_manh());
        assert_eq!(s.gia_can_bang(), None);
    }

    #[test]
    fn phat_hien_so_bi_cheo_va_bi_khoa() {
        let mut cheo = SoLenhL2::moi();
        cheo.them(Chieu::Mua, 8_500, 100);
        cheo.them(Chieu::Ban, 8_400, 100);
        assert!(cheo.bi_cheo(), "mua 85.00 > bán 84.00 là dữ liệu hỏng");
        assert!(!cheo.lanh_manh());

        let mut khoa = SoLenhL2::moi();
        khoa.them(Chieu::Mua, 8_400, 100);
        khoa.them(Chieu::Ban, 8_400, 100);
        assert!(khoa.bi_khoa() && !khoa.bi_cheo(),
                "sổ khoá là hiếm nhưng hợp lệ, khác hẳn sổ chéo");
        assert!(khoa.lanh_manh());
    }

    #[test]
    fn gia_can_bang_lech_ve_phia_it_khoi_luong() {
        // Nhiều người muốn mua hơn bán → áp lực đẩy giá lên → giá cân bằng
        // phải gần giá BÁN hơn.
        let mut s = SoLenhL2::moi();
        s.them(Chieu::Mua, 8_400, 900);
        s.them(Chieu::Ban, 8_410, 100);
        let cb = s.gia_can_bang().unwrap();
        assert!(cb > 8_405.0, "áp lực mua mạnh → giá cân bằng {} phải lệch lên trên", cb);
        assert!(cb < 8_410.0);
    }

    #[test]
    fn dinh_so_tra_dung_thu_tu_uu_tien() {
        let mut s = SoLenhL2::moi();
        for g in [8_380, 8_390, 8_400] { s.them(Chieu::Mua, g, 100); }
        for g in [8_430, 8_420, 8_410] { s.them(Chieu::Ban, g, 100); }
        let (m, b) = s.dinh_so(3);
        assert_eq!(m.iter().map(|x| x.gia).collect::<Vec<_>>(), vec![8_400, 8_390, 8_380],
                   "bên mua: giá cao xuống thấp");
        assert_eq!(b.iter().map(|x| x.gia).collect::<Vec<_>>(), vec![8_410, 8_420, 8_430],
                   "bên bán: giá thấp lên cao");
    }

    // ---------- Sổ L3 ----------
    #[test]
    fn l3_va_l2_luon_nhat_quan_qua_ca_phien_dai() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương: L2 phải luôn là bản tổng hợp
        // đúng của L3. Lệch nhau nghĩa là có bản tin bị xử lý sai.
        let mut s = SoLenhL3::moi();
        for g in sinh_phien(3_000, 99) {
            s.ap_dung(&g.ban_tin);
            assert!(s.l2.lanh_manh(), "sổ không bao giờ được chéo khi dữ liệu sạch");
        }
        // Dựng lại L2 từ L3 rồi so
        let mut kiem = SoLenhL2::moi();
        for l in s.lenh.values() { kiem.them(l.chieu, l.gia, l.con_lai); }
        assert_eq!(kiem.gia_mua_tot_nhat(), s.l2.gia_mua_tot_nhat());
        assert_eq!(kiem.gia_ban_tot_nhat(), s.l2.gia_ban_tot_nhat());
        assert_eq!(kiem.so_muc(Chieu::Mua), s.l2.so_muc(Chieu::Mua));
        assert_eq!(kiem.so_muc(Chieu::Ban), s.l2.so_muc(Chieu::Ban));
        for l in s.lenh.values() {
            assert_eq!(kiem.khoi_luong_tai(l.chieu, l.gia),
                       s.l2.khoi_luong_tai(l.chieu, l.gia));
        }
    }

    #[test]
    fn l3_giu_dung_uu_tien_thoi_gian() {
        let mut s = SoLenhL3::moi();
        for (ma, sl) in [(1u64, 500u32), (2, 300), (3, 200)] {
            s.ap_dung(&BanTin::ThemLenh { ma, ma_ck: 1, chieu: Chieu::Mua,
                                          gia: 8_400, so_luong: sl });
        }
        assert_eq!(s.vi_tri_trong_hang(1), Some(0));
        assert_eq!(s.vi_tri_trong_hang(2), Some(1));
        assert_eq!(s.vi_tri_trong_hang(3), Some(2));
        assert_eq!(s.khoi_luong_dung_truoc(1), Some(0), "đầu hàng thì không chờ ai");
        assert_eq!(s.khoi_luong_dung_truoc(2), Some(500));
        assert_eq!(s.khoi_luong_dung_truoc(3), Some(800));
    }

    #[test]
    fn khop_het_lenh_dau_hang_day_ca_hang_len() {
        let mut s = SoLenhL3::moi();
        for (ma, sl) in [(1u64, 500u32), (2, 300)] {
            s.ap_dung(&BanTin::ThemLenh { ma, ma_ck: 1, chieu: Chieu::Mua,
                                          gia: 8_400, so_luong: sl });
        }
        s.ap_dung(&BanTin::KhopLenh { ma: 1, so_luong: 500, gia: 8_400 });
        assert_eq!(s.vi_tri_trong_hang(2), Some(0), "lệnh #2 lên đầu hàng");
        assert_eq!(s.khoi_luong_dung_truoc(2), Some(0));
        assert_eq!(s.so_lenh_dang_mo(), 1);
    }

    #[test]
    fn khop_mot_phan_giu_nguyen_vi_tri() {
        let mut s = SoLenhL3::moi();
        for (ma, sl) in [(1u64, 500u32), (2, 300)] {
            s.ap_dung(&BanTin::ThemLenh { ma, ma_ck: 1, chieu: Chieu::Mua,
                                          gia: 8_400, so_luong: sl });
        }
        s.ap_dung(&BanTin::KhopLenh { ma: 1, so_luong: 200, gia: 8_400 });
        assert_eq!(s.vi_tri_trong_hang(1), Some(0), "khớp một phần KHÔNG mất chỗ");
        assert_eq!(s.khoi_luong_dung_truoc(2), Some(300), "chỉ còn 300 đứng trước");
        assert_eq!(s.l2.khoi_luong_tai(Chieu::Mua, 8_400), 600);
    }

    #[test]
    fn thay_the_lenh_lam_mat_sach_uu_tien_thoi_gian() {
        // Bài học đắt tiền: sửa giá/khối lượng một lệnh = xuống cuối hàng.
        // Đó là lý do chiến lược tốt cân nhắc rất kỹ trước khi sửa lệnh.
        let mut s = SoLenhL3::moi();
        for (ma, sl) in [(1u64, 500u32), (2, 300), (3, 200)] {
            s.ap_dung(&BanTin::ThemLenh { ma, ma_ck: 1, chieu: Chieu::Mua,
                                          gia: 8_400, so_luong: sl });
        }
        assert_eq!(s.vi_tri_trong_hang(1), Some(0));
        s.ap_dung(&BanTin::ThayThe { ma_cu: 1, ma_moi: 4, gia: 8_400, so_luong: 500 });
        assert_eq!(s.vi_tri_trong_hang(1), None, "mã cũ biến mất");
        assert_eq!(s.vi_tri_trong_hang(4), Some(2), "mã mới xuống CUỐI hàng");
        assert_eq!(s.khoi_luong_dung_truoc(4), Some(500));
    }

    #[test]
    fn huy_lenh_khong_ton_tai_khong_lam_hong_so() {
        let mut s = SoLenhL3::moi();
        s.ap_dung(&BanTin::ThemLenh { ma: 1, ma_ck: 1, chieu: Chieu::Mua,
                                      gia: 8_400, so_luong: 100 });
        s.ap_dung(&BanTin::HuyLenh { ma: 999, so_luong_huy: 50 }); // mã lạ
        assert_eq!(s.so_lenh_dang_mo(), 1);
        assert_eq!(s.l2.khoi_luong_tai(Chieu::Mua, 8_400), 100, "sổ phải nguyên vẹn");
    }

    #[test]
    fn huy_qua_so_luong_con_lai_van_an_toan() {
        let mut s = SoLenhL3::moi();
        s.ap_dung(&BanTin::ThemLenh { ma: 1, ma_ck: 1, chieu: Chieu::Mua,
                                      gia: 8_400, so_luong: 100 });
        s.ap_dung(&BanTin::HuyLenh { ma: 1, so_luong_huy: 99_999 });
        assert_eq!(s.so_lenh_dang_mo(), 0);
        assert_eq!(s.l2.so_muc(Chieu::Mua), 0);
    }

    #[test]
    fn vi_tri_cua_lenh_khong_ton_tai_la_none() {
        let s = SoLenhL3::moi();
        assert_eq!(s.vi_tri_trong_hang(123), None);
        assert_eq!(s.khoi_luong_dung_truoc(123), None);
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn sinh_phien_tat_dinh_va_lien_mach_so_thu_tu() {
        assert_eq!(sinh_phien(50, 9), sinh_phien(50, 9));
        assert_ne!(sinh_phien(50, 9), sinh_phien(50, 10));
        let p = sinh_phien(200, 1);
        for (i, g) in p.iter().enumerate() { assert_eq!(g.so_thu_tu, i as u64); }
    }

    #[test]
    fn thoi_diem_trong_phien_tang_don_dieu() {
        let p = sinh_phien(500, 3);
        for w in p.windows(2) {
            assert!(w[1].thoi_diem_ns > w[0].thoi_diem_ns,
                    "dấu thời gian phải tăng — nền tảng cho phát lại ở Chương 76");
        }
    }

    #[test]
    fn moi_ban_tin_sinh_ra_deu_ma_hoa_phan_tich_duoc() {
        for g in sinh_phien(500, 11) {
            assert_eq!(phan_tich(&ma_hoa(&g)), Ok(g.clone()));
        }
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0308: expected [u8; 8], found &[u8]` | `from_be_bytes` cần mảng cố định | `.try_into().map_err(\|_\| Loi::ThieuByte)?` |
| Giá sai lệch hàng triệu lần | Dùng `from_le_bytes` cho giao thức mạng | Giao thức mạng là big-endian: `from_be_bytes` |
| `E0502: cannot borrow as mutable` | Duyệt `self.cac_muc` rồi muốn `remove` | Thu chỉ số cần xoá vào `Vec` trước, xoá sau |
| Khe bị báo lại vô hạn | Thiếu trạng thái "đang chờ khôi phục" | Thêm `khe_dang_cho: Option<(u64,u64)>` |
| `E0507: cannot move out of BTreeMap` | Lấy `Vec` ra khỏi map | `.remove(&k)` để lấy quyền sở hữu, hoặc mượn |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Giao thức nhị phân trường cố định nhanh hơn JSON khoảng 25 lần** — không tìm kiếm, không cấp phát.
2. **UDP multicast mất gói và bạn phải tự xử.** Yêu cầu phát lại **đúng một lần** — bão yêu cầu còn tệ hơn mất gói.
3. **L3 cho biết vị trí xếp hàng**; vị trí xếp hàng quyết định bạn có bị chọn lọc bất lợi hay không.
4. **`BTreeMap` cho tính tất định**, và tính tất định là điều kiện để phát lại và gỡ lỗi được.
5. **Giảm khối lượng giữ vị trí, huỷ-đặt-lại thì mất.** Một chi tiết nhỏ nhưng ảnh hưởng trực tiếp tới lợi nhuận.

### Bài tập rèn luyện

**Bài 1.** Cài **sổ lệnh gia tăng có kiểm tra bằng ảnh chụp**: dựng sổ từ luồng cập nhật rồi định kỳ đối chiếu với ảnh chụp đầy đủ từ sàn.

<details>
<summary><b>Gợi ý</b></summary>

Sổ lệnh dựng gia tăng sẽ **trôi** theo thời gian — vì gói mất, vì lỗi cài đặt, vì trường hợp biên. Các sàn phát ảnh chụp định kỳ đúng để bạn phát hiện điều đó. Phát hiện lệch thì phải xây lại từ ảnh chụp, không cố "vá".
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, PartialEq)]
pub enum KetQuaDoiChieu {
    Khop,
    Lech { muc_sai: usize, chi_tiet: Vec<String> },
}

impl SoLenhL2 {
    /// Truy cập một bên của sổ dưới dạng bản đồ giá → (khối lượng, số lệnh).
    pub fn cac_muc(&self, chieu: Chieu) -> &BTreeMap<Gia, (u64, u32)> {
        match chieu { Chieu::Mua => &self.mua, Chieu::Ban => &self.ban }
    }

    pub fn doi_chieu(&self, anh: &SoLenhL2) -> KetQuaDoiChieu {
        let mut chi_tiet = Vec::new();
        for (chieu, ta, no) in [("mua", self.cac_muc(Chieu::Mua), anh.cac_muc(Chieu::Mua)),
                                ("ban", self.cac_muc(Chieu::Ban), anh.cac_muc(Chieu::Ban))] {
            for (gia, kl) in ta {
                match no.get(gia) {
                    Some(k) if k == kl => {}
                    Some(k) => chi_tiet.push(
                        format!("{} {}: ta={:?} anh={:?}", chieu, gia, kl, k)),
                    None => chi_tiet.push(
                        format!("{} {}: ta={:?} anh=THIEU", chieu, gia, kl)),
                }
            }
            for gia in no.keys() {
                if !ta.contains_key(gia) {
                    chi_tiet.push(format!("{} {}: ta=THIEU", chieu, gia));
                }
            }
        }
        if chi_tiet.is_empty() { KetQuaDoiChieu::Khop }
        else { KetQuaDoiChieu::Lech { muc_sai: chi_tiet.len(), chi_tiet } }
    }

    /// Khi lệch: XÂY LẠI, không vá. Sổ đã sai thì mọi phép vá đều đoán mò.
    pub fn xay_lai_tu(&mut self, anh: &SoLenhL2) {
        self.mua = anh.mua.clone();
        self.ban = anh.ban.clone();
    }
}
```

Nguyên tắc vận hành: **phát hiện lệch → xây lại → ghi nhật ký → cảnh báo**. Đừng bao giờ cố vá một sổ đã lệch; bạn không biết nó sai từ đâu.
</details>

**Bài 2.** Cài **bộ theo dõi vị trí xếp hàng** cho lệnh của chính mình khi có luồng L3.

<details>
<summary><b>Gợi ý</b></summary>

Vị trí xếp hàng giảm khi lệnh đứng trước bị khớp **hoặc bị huỷ**. Nó không đổi khi có lệnh mới xếp sau bạn. Theo dõi số này cho phép ước lượng xác suất được khớp — và quyết định có nên đặt lại lệnh hay không.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct TheoDoiHang {
    pub ma_lenh_cua_ta: MaLenh,
    pub gia: Gia,
    pub khoi_luong_dung_truoc: u64,
    pub khoi_luong_ban_dau_truoc: u64,
}

impl TheoDoiHang {
    /// Sổ L3 đã cho sẵn `khoi_luong_dung_truoc` — ta chỉ chụp lại giá trị đó
    /// tại thời điểm đặt lệnh để về sau đo được tiến độ.
    pub fn moi(so: &SoLenhL3, ma: MaLenh) -> Option<Self> {
        let l = so.lenh.get(&ma)?;
        let truoc = so.khoi_luong_dung_truoc(ma)?;
        Some(TheoDoiHang {
            ma_lenh_cua_ta: ma,
            gia: l.gia,
            khoi_luong_dung_truoc: truoc,
            khoi_luong_ban_dau_truoc: truoc,
        })
    }

    /// Lệnh đứng trước bị khớp HOẶC bị huỷ → hàng ngắn lại.
    pub fn hang_ngan_lai(&mut self, khoi_luong: u64) {
        self.khoi_luong_dung_truoc =
            self.khoi_luong_dung_truoc.saturating_sub(khoi_luong);
    }

    /// Tỉ lệ đã tiến được, 0.0 → 1.0.
    pub fn tien_do(&self) -> f64 {
        if self.khoi_luong_ban_dau_truoc == 0 { return 1.0; }
        1.0 - self.khoi_luong_dung_truoc as f64
               / self.khoi_luong_ban_dau_truoc as f64
    }

    /// Ước lượng thô xác suất được khớp trước khi giá đi mất.
    pub fn xac_suat_khop(&self, khoi_luong_ky_vong: u64) -> f64 {
        if self.khoi_luong_dung_truoc == 0 { return 1.0; }
        (khoi_luong_ky_vong as f64 / self.khoi_luong_dung_truoc as f64).min(1.0)
    }
}
```

Con số `xac_suat_khop` là đầu vào trực tiếp cho quyết định giao dịch: nếu xác suất quá thấp, tốt hơn là huỷ và đặt ở mức giá tốt hơn — chấp nhận chênh lệch nhỏ hơn để đổi lấy khả năng được khớp.
</details>
