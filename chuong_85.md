# Chương 85: Hệ sinh thái HFT tích hợp — Nối mọi mảnh thành một hệ chạy được

## Giới thiệu & Mục tiêu học tập

Chương 74–78 dựng từng mảnh: đo độ trễ, sổ lệnh, phát lại, cổng rủi ro, AMM. Mỗi mảnh đều chạy và đều có kiểm thử. Nhưng **năm mảnh chạy riêng không phải một hệ thống.**

Chương này nối chúng lại thành một hệ chạy end-to-end, đồng thời trên **hai loại thị trường**:

```
nguồn phiên ──► bộ phát lại (đồng hồ ảo, đẩy tốc độ ×N)
                     │
         ┌───────────┴───────────┐
         ▼                       ▼
 sàn TRUYỀN THỐNG          sàn CHUỖI KHỐI
 (sổ lệnh giá–thời gian)   (bể AMM x·y=k)
         └───────────┬───────────┘
                     ▼
           ảnh chụp thị trường hợp nhất
                     ▼
              chiến lược (nhiều)
                     ▼
                cổng rủi ro
                     ▼
      OMS: gửi lệnh CÓ ĐỘ TRỄ (hàng đợi theo thời điểm đến)
                     ▼
         sàn khớp ──► lãi lỗ, tồn kho, đo lường
```

Điều đáng học nhất ở chương này không phải kiến trúc. Đó là **năm lỗi chỉ lộ ra khi ghép các mảnh lại** — mỗi mảnh riêng lẻ đều đúng, nhưng hệ thống thì sai. Toàn bộ năm lỗi đều được phát hiện bằng cách **chạy**, không phải bằng cách đọc, và mỗi lỗi giờ có một bài kiểm thử canh gác.

Ba bất biến bắt buộc, mỗi bất biến một bài kiểm thử:
1. **Tất định** — chạy hai lần, và chạy ở ba tốc độ khác nhau, cho kết quả trùng khớp từng bit.
2. **Nhân quả** — chiến lược không bao giờ thấy dữ liệu tương lai; lệnh tới sàn sau một khoảng trễ và khớp theo trạng thái sàn **tại thời điểm đến**.
3. **Bất biến rủi ro** — không kịch bản nào vượt hạn mức vị thế, và sổ lệnh không bao giờ chéo.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  NĂM LỖI CHỈ LỘ RA KHI GHÉP                                                 │
│                                                                              │
│  ① ĐOÁN CHIỀU TỪ GIÁ                                                        │
│     Khớp thụ động về, ta không biết nó là mua hay bán → đoán từ giá.        │
│     Đoán sai → vị thế chạy NGƯỢC → mọi hạn mức rủi ro thành vô nghĩa.       │
│     Chữa: KhopLenh MANG THEO chiều. Đừng bao giờ suy ra cái mình biết sẵn.  │
│                                                                              │
│  ② KHỚP TRÀN QUA NHIỀU MỨC GIÁ                                              │
│     Lệnh thị trường 10 đơn vị cắt qua mức 100 → ta gọi hàm khớp toàn sổ     │
│     → nó ăn luôn lệnh của ta ở mức 99, 98, 97... 10 đơn vị "khớp" 200.      │
│     Chữa: khớp ĐÚNG một mức, ĐÚNG khối lượng còn lại.                       │
│                                                                              │
│  ③ BÁO GIÁ KHÔNG BAO GIỜ ĐƯỢC RÚT                                           │
│     MM báo giá mỗi 1 ms, không huỷ báo giá cũ → sau 2 giây có 2000 lệnh     │
│     treo → phơi nhiễm chạm trần → cổng rủi ro chặn 99,3% lệnh mới.          │
│     Hệ thống TỰ BÓP CỔ MÌNH. Chữa: chính sách rút báo giá quá tuổi.         │
│                                                                              │
│  ④ CỔNG RỦI RO KHÔNG THẤY LỆNH ĐANG BAY  ← lỗ hổng đắt nhất                │
│                                                                              │
│     nhịp t:  ý định A ──kiểm──► vị thế 0, treo 0  → CHO QUA                 │
│              ý định B ──kiểm──► vị thế 0, treo 0  → CHO QUA  (!!)           │
│              ý định C ──kiểm──► vị thế 0, treo 0  → CHO QUA  (!!)           │
│              ...cả ba đều thấy CÙNG một trạng thái...                       │
│     nhịp t+50µs: cả ba tới sàn, cả ba khớp → vị thế vượt hạn mức 3 LẦN     │
│                                                                              │
│     Mọi phép kiểm đều đã chạy. Mọi phép kiểm đều trả "OK". Vẫn vỡ.          │
│     Chữa: ĐẶT CHỖ phơi nhiễm ngay lúc phát, không đợi lúc giao.             │
│                                                                              │
│  ⑤ NHÀ TẠO LẬP CẮT QUA SỔ                                                   │
│     Báo giá ở giữa ± 2 tick, nhưng sổ đang hẹp hơn → báo giá CẮT QUA        │
│     → MM trở thành người CHỦ ĐỘNG → TRẢ chênh lệch thay vì THU.             │
│     Đo được: tỉ lệ thụ động 19% thay vì >80%. Chữa: kẹp giá, không cắt.     │
│                                                                              │
│  BẤT ĐỐI XỨNG KHỚP — vì sao "hai chân" vẫn chưa đủ                          │
│                                                                              │
│     chân AMM      : LUÔN khớp đủ (công thức không từ chối ai)               │
│     chân sổ lệnh  : khớp MỘT PHẦN (sổ đã đổi trong 50 µs độ trễ)            │
│                     └── chênh lệch đọng lại thành vị thế ròng               │
│                                                                              │
│     Cách của ngành: chạy chân KHÔNG CHẮC trước, rồi phòng vệ ĐÚNG BẰNG      │
│     khối lượng thực sự khớp được. Vị thế ròng khi đó = 0 chính xác.         │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Đồng hồ ảo phải là nguồn thời gian duy nhất

Toàn bộ hệ thống đọc thời gian từ `DongHoAo`. Không thành phần nào gọi `Instant::now()`. Đó không phải quy ước lịch sự — nó là điều kiện để hai tính chất cùng tồn tại:

- **Đẩy tốc độ không đổi kết quả.** Phát lại ở ×1, ×1000 hay vô hạn đều cho cùng dòng lệnh, vì thời gian **ảo** không đổi, chỉ thời gian **tường** bị nén. Chương này kiểm thử đúng điều đó.
- **Tất định.** Một lời gọi đồng hồ thật lạc lõng là đủ khiến hai lần chạy khác nhau, và khi đó bạn không thể gỡ lỗi bất kỳ sự cố sản xuất nào.

`DongHoAo::tien_toi` **từ chối** lùi thời gian thay vì im lặng sắp xếp lại. Dữ liệu xếp sai thứ tự là lỗi thu thập; giấu nó đi thì bộ phát lại sẽ nói dối một cách thuyết phục.

### 2. Cổng rủi ro phải đặt chỗ, không chỉ kiểm tra

Đây là bài học đắt nhất của chương.

Một cổng rủi ro "đúng" theo nghĩa thông thường sẽ kiểm mọi lệnh trước khi gửi. Nhưng nếu ba lệnh được phát trong **cùng một nhịp**, cả ba đều được kiểm trên **cùng một trạng thái vị thế** — vì trạng thái chỉ thay đổi khi lệnh tới sàn 50 µs sau. Cả ba đều qua. Cả ba đều khớp. Hạn mức bị vượt gấp ba lần, và **không phép kiểm nào đã thất bại**.

Cách chữa là tách phơi nhiễm thành ba tầng và cộng đủ cả ba:

```
phơi_nhiễm = vị_thế_đã_khớp  +  lệnh_ĐANG_TREO  +  lệnh_ĐANG_BAY
```

và **đặt chỗ ngay khi cho qua**, trước khi xét ý định tiếp theo. Trong chương này đó là hai biến `bay_mua`/`bay_ban`, tăng lúc phát và chuyển sang `treo_*` lúc giao.

Sau khi sửa, vị thế cuối phiên dừng **đúng** ở hạn mức 500 thay vì vọt lên 514 — con số dừng đúng ở biên là dấu hiệu cổng đang thực sự ràng buộc, không phải đang may mắn.

### 3. Huỷ lệnh phải đi đường ưu tiên

Trong hệ thống này, lệnh **đặt** chịu độ trễ còn lệnh **huỷ** đi thẳng. Đó không phải sự thiên vị tuỳ tiện: sàn thật xử lý huỷ trên đường ưu tiên, và quan trọng hơn — nếu huỷ cũng phải xếp hàng thì **rủi ro tồn kho không bao giờ giảm được**. Bạn sẽ có một hệ thống chỉ biết tăng phơi nhiễm.

Cùng lý do đó, cổng rủi ro **luôn cho lệnh huỷ đi qua**, kể cả khi công tắc ngắt khẩn cấp đã bật. Một công tắc ngắt chặn cả đường rút chân là một cái bẫy, không phải một biện pháp an toàn.

### 4. Nhà tạo lập không được cắt qua sổ

Một nhà tạo lập kiếm tiền bằng cách **thu** chênh lệch: mua ở giá mua, bán ở giá bán, ăn phần giữa. Nếu báo giá của nó cắt qua bên kia, nó trở thành người **chủ động** và **trả** chênh lệch.

Điều này nghe hiển nhiên, nhưng nó xảy ra âm thầm: chiến lược tính giá quanh vi giá với chênh lệch mục tiêu 4 tick, mà sổ lúc đó chỉ rộng 1 tick. Báo giá cắt qua, và không có gì báo lỗi cả.

Cách phát hiện là một chỉ số vận hành: **tỉ lệ thụ động**. Trước khi sửa, chỉ số này là 19%. Sau khi kẹp giá để không bao giờ cắt, nó lên trên 80% khi chạy nhà tạo lập một mình. Một nhà tạo lập có tỉ lệ thụ động thấp là một nhà tạo lập đang lỗ, dù bảng lãi lỗ có nói gì đi nữa.

### 5. Bất đối xứng khớp và phòng vệ theo khối lượng đã khớp

Chênh lệch giá giữa hai sàn nghe như "giao dịch phi rủi ro". Trong một hệ thống thật nó không phải vậy, vì hai chân không khớp giống nhau:

| | Chân sàn truyền thống | Chân bể AMM |
|---|---|---|
| Có thể bị từ chối | Có (sổ đã đổi) | Không |
| Khớp một phần | Có | Không |
| Xếp hàng | Có | Không |

Đặt cứng cả hai chân cùng lúc vẫn hỏng: chân AMM khớp đủ, chân sổ lệnh khớp một phần, và phần chênh đọng lại thành vị thế ròng. Trong chương này, cách làm đó để lại vị thế 111 đơn vị trên một chiến lược đáng lẽ trung tính.

Cách của ngành — và cách chương này cài — là **chạy chân không chắc trước, rồi phòng vệ đúng bằng khối lượng thực sự khớp được**. Sau khi sửa, vị thế ròng của chiến lược chênh lệch là **đúng 0**, và đó là một bài kiểm thử chứ không phải một lời hứa.

Một chi tiết kèm theo: chân phòng vệ phải được ghi sổ ở **giá thực nhận trên bể**, không phải giá của sàn kia. Ghi sai chỗ này khiến chênh lệch thu được luôn bằng 0 và chiến lược "phi rủi ro" chỉ còn lại chi phí. Muốn phòng vệ đúng khối lượng thì cần công thức **nghịch đảo** của bể: cần bỏ vào bao nhiêu để nhận đúng ngần này?

### 6. Vì sao con số lãi lỗ trong chương này không đáng tin

Phiên dùng ở đây là **tổng hợp**, và mối liên kết giữa hai sàn chỉ được mô phỏng một phần. Bộ sinh phiên có một cơ chế kéo giá bể về sát sàn truyền thống — đại diện cho những nhà chênh lệch khác — nhưng cơ chế đó không hoàn hảo. Khe hở còn lại là quà tặng cho chiến lược chênh lệch.

Trên thị trường thật, hàng trăm hãng cùng săn đúng khe hở đó trong vài trăm nanosecond, và nó đóng lại trước khi bạn kịp thấy.

Không có cơ chế kéo về đó, mô hình còn tệ hơn nhiều: bể trôi tự do xuống 9305 trong khi sổ lệnh đứng ở 10000, và chiến lược chênh lệch **in ra 35 triệu** — một con số hoàn toàn giả mà nhìn qua rất thuyết phục.

Đó là bài học cuối và quan trọng nhất: **một backtest có thể đúng về mặt cơ học mà vẫn sai hoàn toàn về mặt kinh tế**, nếu môi trường mô phỏng thiếu một lực mà thị trường thật có. Thứ đáng tin ở chương này là các **bất biến**, không phải lãi lỗ.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch85`, kiểm thử bằng `cargo test -p ch85` (48 bài kiểm thử).

```rust
//! # Chương 85: Hệ sinh thái HFT tích hợp — Nối mọi mảnh thành một hệ chạy được
//!
//! Chương 74–78 dựng từng mảnh rời: đo độ trễ, sổ lệnh, phát lại, cổng rủi ro, AMM.
//! Chương này **nối chúng lại** thành một hệ thống duy nhất chạy end-to-end:
//!
//! ```text
//!   nguồn phiên ──► bộ phát lại (đồng hồ ảo, đẩy tốc độ ×N)
//!                        │
//!            ┌───────────┴───────────┐
//!            ▼                       ▼
//!    sàn TRUYỀN THỐNG          sàn CHUỖI KHỐI
//!    (sổ lệnh giá–thời gian)   (bể AMM x·y=k)
//!            └───────────┬───────────┘
//!                        ▼
//!              ảnh chụp thị trường hợp nhất
//!                        ▼
//!                 chiến lược (nhiều)
//!                        ▼
//!                   cổng rủi ro
//!                        ▼
//!         OMS: gửi lệnh CÓ ĐỘ TRỄ (hàng đợi theo thời điểm đến)
//!                        ▼
//!            sàn khớp ──► lãi lỗ, tồn kho, đo lường
//! ```
//!
//! Ba tính chất bắt buộc, mỗi tính chất có bài kiểm thử riêng:
//! 1. **Tất định** — chạy hai lần cho kết quả trùng khớp từng bit.
//! 2. **Nhân quả** — chiến lược không bao giờ thấy dữ liệu tương lai; lệnh tới sàn
//!    sau một khoảng trễ, và khớp theo trạng thái sàn **tại thời điểm đến**.
//! 3. **Bất biến rủi ro** — không kịch bản nào vượt hạn mức, kể cả khi có lệnh treo.

use std::collections::{BTreeMap, VecDeque};

// ============================================================================
// 1. KIỂU NỀN
// ============================================================================

pub type Gia = i64; // tick: 1 tick = 0,01 đơn vị tiền
pub type SoLuong = i64;
pub type MaLenh = u64;
pub type Nano = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Chieu {
    Mua,
    Ban,
}

impl Chieu {
    pub fn dau(self) -> i64 {
        match self {
            Chieu::Mua => 1,
            Chieu::Ban => -1,
        }
    }
    pub fn nguoc(self) -> Chieu {
        match self {
            Chieu::Mua => Chieu::Ban,
            Chieu::Ban => Chieu::Mua,
        }
    }
}

/// Định danh nơi giao dịch. Hệ sinh thái này chạy đồng thời hai loại sàn —
/// đó chính là "hai hướng" mà một hệ HFT hiện đại phải phủ.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum San {
    /// Sàn truyền thống: sổ lệnh giới hạn, ưu tiên giá–thời gian.
    TruyenThong,
    /// Sàn chuỗi khối: bể thanh khoản tự động, giá theo công thức.
    ChuoiKhoi,
}

// ============================================================================
// 2. ĐỒNG HỒ ẢO — NGUỒN THỜI GIAN DUY NHẤT
// ============================================================================

/// Mọi thành phần đọc thời gian **từ đây**, không bao giờ từ `Instant::now()`.
/// Một lời gọi đồng hồ thật lạc lõng là đủ phá cả tính tất định lẫn tính nhân quả.
#[derive(Debug, Clone, Copy, Default)]
pub struct DongHoAo {
    hien_tai: Nano,
}

impl DongHoAo {
    pub fn moi(bat_dau: Nano) -> Self {
        DongHoAo { hien_tai: bat_dau }
    }
    pub fn bay_gio(&self) -> Nano {
        self.hien_tai
    }
    /// Thời gian chỉ TIẾN. Lùi lại là dấu hiệu dữ liệu phiên bị xếp sai thứ tự.
    pub fn tien_toi(&mut self, t: Nano) -> bool {
        if t < self.hien_tai {
            return false;
        }
        self.hien_tai = t;
        true
    }
}

/// Hệ số nén thời gian tường. Không ảnh hưởng tới thời gian ẢO, nên kết quả
/// chiến lược **không đổi** dù chạy ở tốc độ nào — miễn là không ai đọc đồng hồ thật.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TocDoPhat {
    ThoiGianThuc,
    Nhanh(u32),
    VoHan,
}

impl TocDoPhat {
    pub fn cho_bao_lau(&self, khoang_ao_ns: Nano) -> Nano {
        match self {
            TocDoPhat::ThoiGianThuc => khoang_ao_ns,
            TocDoPhat::Nhanh(n) => khoang_ao_ns / (*n).max(1) as u64,
            TocDoPhat::VoHan => 0,
        }
    }
}

// ============================================================================
// 3. MÔ HÌNH ĐỘ TRỄ
// ============================================================================

/// Ba khoảng trễ có thật, tách riêng vì chúng tối ưu được độc lập.
#[derive(Debug, Clone, Copy)]
pub struct MoHinhDoTre {
    /// Sàn phát ─► ta nhận.
    pub du_lieu_vao_ns: Nano,
    /// Ta gửi ─► sàn nhận.
    pub lenh_ra_ns: Nano,
    /// Biên độ dao động; độ trễ thật có đuôi dài, không phải hằng số.
    pub dao_dong_ns: Nano,
}

impl MoHinhDoTre {
    pub fn dien_hinh() -> Self {
        MoHinhDoTre { du_lieu_vao_ns: 10_000, lenh_ra_ns: 50_000, dao_dong_ns: 5_000 }
    }
    pub fn khong_tre() -> Self {
        MoHinhDoTre { du_lieu_vao_ns: 0, lenh_ra_ns: 0, dao_dong_ns: 0 }
    }

    /// Dao động TẤT ĐỊNH theo hạt giống — cần nhiễu thật, nhưng phải tái lập được.
    pub fn tre_lenh(&self, hat: u64) -> Nano {
        if self.dao_dong_ns == 0 {
            return self.lenh_ra_ns;
        }
        self.lenh_ra_ns + bam_trong_khoang(hat, self.dao_dong_ns)
    }
}

/// splitmix64 — trộn đều thật sự. Phép chia dư đơn thuần làm giá trị co cụm
/// và khiến mọi phép đo dựa trên phân phối trở nên vô nghĩa.
pub fn bam64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E3779B97F4A7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

pub fn bam_trong_khoang(hat: u64, tran: u64) -> u64 {
    if tran == 0 { 0 } else { bam64(hat) % tran }
}

// ============================================================================
// 4. SỰ KIỆN PHIÊN
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum LoaiSuKien {
    /// Sàn truyền thống: một lệnh giới hạn mới vào sổ.
    ThemLenh { ma: MaLenh, chieu: Chieu, gia: Gia, khoi_luong: SoLuong },
    /// Sàn truyền thống: huỷ một lệnh đang treo.
    HuyLenh { ma: MaLenh },
    /// Sàn truyền thống: một giao dịch đã khớp (thông tin, không đổi sổ).
    DaKhop { gia: Gia, khoi_luong: SoLuong },
    /// Sàn chuỗi khối: ai đó hoán đổi trên bể, làm dự trữ đổi → giá đổi.
    HoanDoiTrenBe { vao_x: bool, khoi_luong: u128 },
}

#[derive(Debug, Clone, PartialEq)]
pub struct SuKienPhien {
    pub thoi_diem: Nano,
    pub san: San,
    pub loai: LoaiSuKien,
}

/// Phiên đã ghi. Bất biến sống còn: `thoi_diem` không giảm.
#[derive(Debug, Clone, Default)]
pub struct PhienDaGhi {
    pub cac_su_kien: Vec<SuKienPhien>,
}

impl PhienDaGhi {
    pub fn moi() -> Self {
        PhienDaGhi::default()
    }

    /// Từ chối sự kiện lùi thời gian thay vì im lặng sắp xếp lại — dữ liệu
    /// xếp sai thứ tự là lỗi thu thập, và giấu nó đi thì phát lại sẽ nói dối.
    pub fn ghi(&mut self, sk: SuKienPhien) -> bool {
        if let Some(cuoi) = self.cac_su_kien.last() {
            if sk.thoi_diem < cuoi.thoi_diem {
                return false;
            }
        }
        self.cac_su_kien.push(sk);
        true
    }

    pub fn so_su_kien(&self) -> usize {
        self.cac_su_kien.len()
    }

    pub fn khoang_thoi_gian_ns(&self) -> Nano {
        match (self.cac_su_kien.first(), self.cac_su_kien.last()) {
            (Some(a), Some(b)) => b.thoi_diem - a.thoi_diem,
            _ => 0,
        }
    }

    pub fn dung_thu_tu(&self) -> bool {
        self.cac_su_kien.windows(2).all(|w| w[0].thoi_diem <= w[1].thoi_diem)
    }
}

// ============================================================================
// 5. SÀN TRUYỀN THỐNG — SỔ LỆNH ƯU TIÊN GIÁ–THỜI GIAN
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MucGia {
    pub gia: Gia,
    pub khoi_luong: SoLuong,
}

#[derive(Debug, Clone, Default)]
pub struct SanTruyenThong {
    /// `BTreeMap` chứ không `HashMap`: thứ tự duyệt phải tất định, nếu không
    /// phát lại sẽ không tái lập được và mọi phép gỡ lỗi đều vô nghĩa.
    mua: BTreeMap<Gia, SoLuong>,
    ban: BTreeMap<Gia, SoLuong>,
    /// Lệnh của THỊ TRƯỜNG (không phải của ta) để xử lý huỷ và khớp.
    lenh_thi_truong: BTreeMap<MaLenh, (Chieu, Gia, SoLuong)>,
    /// Hàng đợi FIFO tại mỗi (chiều, giá) — nền của ưu tiên thời gian.
    hang_thi_truong: BTreeMap<(Chieu, Gia), VecDeque<MaLenh>>,
    /// Lệnh của TA đang treo trên sàn.
    lenh_cua_ta: BTreeMap<MaLenh, LenhCuaTa>,
    /// Khớp thụ động chờ bộ điều phối lấy ra.
    khop_thu_dong_cho: Vec<KhopLenh>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LenhCuaTa {
    pub ma: MaLenh,
    pub chieu: Chieu,
    pub gia: Gia,
    pub con_lai: SoLuong,
    pub thoi_diem_vao: Nano,
    /// Khối lượng đứng trước tại thời điểm vào — nền của ước lượng khớp.
    pub khoi_luong_truoc: SoLuong,
}

impl SanTruyenThong {
    pub fn moi() -> Self {
        SanTruyenThong::default()
    }

    pub fn gia_mua_tot_nhat(&self) -> Option<MucGia> {
        self.mua.iter().next_back().map(|(&g, &k)| MucGia { gia: g, khoi_luong: k })
    }
    pub fn gia_ban_tot_nhat(&self) -> Option<MucGia> {
        self.ban.iter().next().map(|(&g, &k)| MucGia { gia: g, khoi_luong: k })
    }

    pub fn gia_giua(&self) -> Option<f64> {
        match (self.gia_mua_tot_nhat(), self.gia_ban_tot_nhat()) {
            (Some(m), Some(b)) => Some((m.gia + b.gia) as f64 / 2.0),
            _ => None,
        }
    }

    /// Vi giá: trọng số NGƯỢC với khối lượng. Bên đông người kéo giá công bằng
    /// về phía bên mỏng, vì áp lực bên đó chưa được thoả mãn.
    pub fn vi_gia(&self) -> Option<f64> {
        let (m, b) = (self.gia_mua_tot_nhat()?, self.gia_ban_tot_nhat()?);
        let tong = (m.khoi_luong + b.khoi_luong) as f64;
        if tong <= 0.0 {
            return None;
        }
        Some((m.gia as f64 * b.khoi_luong as f64 + b.gia as f64 * m.khoi_luong as f64) / tong)
    }

    pub fn mat_can_bang(&self) -> Option<f64> {
        let (m, b) = (self.gia_mua_tot_nhat()?, self.gia_ban_tot_nhat()?);
        let tong = (m.khoi_luong + b.khoi_luong) as f64;
        if tong <= 0.0 {
            return None;
        }
        Some((m.khoi_luong - b.khoi_luong) as f64 / tong)
    }

    pub fn chenh_lech(&self) -> Option<Gia> {
        Some(self.gia_ban_tot_nhat()?.gia - self.gia_mua_tot_nhat()?.gia)
    }

    /// Tổng khối lượng còn treo cả hai bên — thước đo độ "phình" của sổ.
    pub fn tong_khoi_luong(&self) -> SoLuong {
        self.mua.values().sum::<SoLuong>() + self.ban.values().sum::<SoLuong>()
    }

    pub fn bi_cheo(&self) -> bool {
        match (self.gia_mua_tot_nhat(), self.gia_ban_tot_nhat()) {
            (Some(m), Some(b)) => m.gia >= b.gia,
            _ => false,
        }
    }

    fn ben(&mut self, c: Chieu) -> &mut BTreeMap<Gia, SoLuong> {
        match c {
            Chieu::Mua => &mut self.mua,
            Chieu::Ban => &mut self.ban,
        }
    }

    fn them(&mut self, c: Chieu, g: Gia, k: SoLuong) {
        if k <= 0 {
            return;
        }
        *self.ben(c).entry(g).or_insert(0) += k;
    }

    fn bot(&mut self, c: Chieu, g: Gia, k: SoLuong) {
        let ben = self.ben(c);
        if let Some(v) = ben.get_mut(&g) {
            *v -= k;
            if *v <= 0 {
                ben.remove(&g);
            }
        }
    }

    /// Khối lượng đứng trước một mức giá ở cùng chiều — vị trí xếp hàng.
    pub fn khoi_luong_tai(&self, c: Chieu, g: Gia) -> SoLuong {
        match c {
            Chieu::Mua => self.mua.get(&g).copied().unwrap_or(0),
            Chieu::Ban => self.ban.get(&g).copied().unwrap_or(0),
        }
    }

    /// Tiêu thụ `can` đơn vị của lệnh THỊ TRƯỜNG tại (chiều, giá), theo FIFO.
    /// Trả về số thực sự tiêu được.
    fn tieu_thu_thi_truong(&mut self, c: Chieu, g: Gia, mut can: SoLuong) -> SoLuong {
        let mut da = 0;
        let mut het = Vec::new();
        if let Some(q) = self.hang_thi_truong.get(&(c, g)) {
            for &m in q.iter() {
                if can <= 0 {
                    break;
                }
                let con = match self.lenh_thi_truong.get(&m) {
                    Some(&(_, _, k)) => k,
                    None => continue,
                };
                let lay = can.min(con);
                can -= lay;
                da += lay;
                if let Some(v) = self.lenh_thi_truong.get_mut(&m) {
                    v.2 -= lay;
                    if v.2 <= 0 {
                        het.push(m);
                    }
                }
            }
        }
        for m in &het {
            self.lenh_thi_truong.remove(m);
        }
        if let Some(q) = self.hang_thi_truong.get_mut(&(c, g)) {
            q.retain(|m| !het.contains(m));
            if q.is_empty() {
                self.hang_thi_truong.remove(&(c, g));
            }
        }
        self.bot(c, g, da);
        da
    }

    /// Áp dụng một sự kiện thị trường. Bộ phát lại phải xử lý **mọi** loại —
    /// bỏ sót lệnh huỷ khiến sổ chỉ lớn lên rồi chéo vĩnh viễn.
    ///
    /// Lệnh mới cắt qua bên kia được KHỚP, không được chất lên sổ: một sàn thật
    /// không bao giờ để sổ chéo, và mô hình bỏ qua điều này sẽ cho chiến lược
    /// nhìn thấy những mức giá không tồn tại.
    pub fn ap_dung(&mut self, sk: &LoaiSuKien) {
        match sk {
            LoaiSuKien::ThemLenh { ma, chieu, gia, khoi_luong } => {
                let mut con = *khoi_luong;

                // Giai đoạn 1: khớp phần cắt qua với bên đối ứng.
                loop {
                    if con <= 0 {
                        break;
                    }
                    let doi = match chieu {
                        Chieu::Mua => self.ban.iter().next().map(|(&g, &k)| (g, k)),
                        Chieu::Ban => self.mua.iter().next_back().map(|(&g, &k)| (g, k)),
                    };
                    let (g, k) = match doi {
                        Some(x) => x,
                        None => break,
                    };
                    let cat = match chieu {
                        Chieu::Mua => *gia >= g,
                        Chieu::Ban => *gia <= g,
                    };
                    if !cat {
                        break;
                    }
                    // Lệnh của TA ở mức này cũng được khớp — đúng ưu tiên giá.
                    let cua_ta: Vec<MaLenh> = self
                        .lenh_cua_ta
                        .values()
                        .filter(|l| l.chieu == chieu.nguoc() && l.gia == g)
                        .map(|l| l.ma)
                        .collect();
                    let tt = self.tieu_thu_thi_truong(chieu.nguoc(), g, con);
                    con -= tt;
                    if tt == 0 && !cua_ta.is_empty() {
                        // Chỉ còn lệnh của ta ở mức này. Khớp ĐÚNG mức đó và
                        // ĐÚNG khối lượng còn lại — gọi hàm khớp toàn sổ ở đây
                        // sẽ ăn cả lệnh của ta ở những mức giá khác, và vị thế
                        // sẽ vọt qua hạn mức mà cổng rủi ro không hề biết.
                        let khop = self.khop_cua_ta_tai_muc(chieu.nguoc(), g, con);
                        let da: SoLuong = khop.iter().map(|x| x.khoi_luong).sum();
                        self.khop_thu_dong_cho.extend(khop);
                        con -= da;
                        if da == 0 {
                            break;
                        }
                    } else if tt == 0 {
                        break;
                    }
                    let _ = k;
                }

                // Giai đoạn 2: phần còn lại nằm chờ trên sổ.
                if con > 0 {
                    self.them(*chieu, *gia, con);
                    self.lenh_thi_truong.insert(*ma, (*chieu, *gia, con));
                    self.hang_thi_truong.entry((*chieu, *gia)).or_default().push_back(*ma);
                }
            }
            LoaiSuKien::HuyLenh { ma } => {
                if let Some((c, g, k)) = self.lenh_thi_truong.remove(ma) {
                    self.bot(c, g, k);
                    if let Some(q) = self.hang_thi_truong.get_mut(&(c, g)) {
                        q.retain(|x| x != ma);
                        if q.is_empty() {
                            self.hang_thi_truong.remove(&(c, g));
                        }
                    }
                }
            }
            LoaiSuKien::DaKhop { .. } => {}
            LoaiSuKien::HoanDoiTrenBe { .. } => {}
        }
    }

    /// Khớp lệnh của ta tại ĐÚNG một mức giá, không vượt quá `tran` đơn vị.
    /// Ưu tiên thời gian trong nội bộ mức.
    fn khop_cua_ta_tai_muc(&mut self, chieu: Chieu, gia: Gia, tran: SoLuong) -> Vec<KhopLenh> {
        let mut ra = Vec::new();
        let mut con = tran;
        let mut ung_vien: Vec<LenhCuaTa> = self
            .lenh_cua_ta
            .values()
            .copied()
            .filter(|l| l.chieu == chieu && l.gia == gia)
            .collect();
        ung_vien.sort_by_key(|l| (l.thoi_diem_vao, l.ma));

        let mut xong = Vec::new();
        for l in ung_vien {
            if con <= 0 {
                break;
            }
            let lay = con.min(l.con_lai);
            if let Some(m) = self.lenh_cua_ta.get_mut(&l.ma) {
                m.con_lai -= lay;
                if m.con_lai <= 0 {
                    xong.push(l.ma);
                }
            }
            self.bot(chieu, gia, lay);
            ra.push(KhopLenh { ma: l.ma, chieu, gia, khoi_luong: lay, chu_dong: false });
            con -= lay;
        }
        for m in xong {
            self.lenh_cua_ta.remove(&m);
        }
        ra
    }

    /// Lệnh treo của ta cũ hơn `tuoi_ns` — nhà tạo lập thật làm mới báo giá
    /// liên tục, và báo giá cũ là rủi ro chứ không phải cơ hội.
    pub fn lenh_cua_ta_cu_hon(&self, bay_gio: Nano, tuoi_ns: Nano) -> Vec<MaLenh> {
        self.lenh_cua_ta
            .values()
            .filter(|l| bay_gio.saturating_sub(l.thoi_diem_vao) > tuoi_ns)
            .map(|l| l.ma)
            .collect()
    }

    /// Khớp thụ động phát sinh khi lệnh thị trường cắt qua lệnh treo của ta.
    /// Bộ điều phối lấy ra và ghi nhận vào vị thế.
    pub fn lay_khop_thu_dong(&mut self) -> Vec<KhopLenh> {
        std::mem::take(&mut self.khop_thu_dong_cho)
    }

    /// Đặt lệnh của ta. Nếu giá cắt qua bên kia thì khớp NGAY (lệnh chủ động).
    /// Ngược lại nó nằm chờ, và ta ghi lại khối lượng đứng trước.
    pub fn dat_lenh_cua_ta(&mut self, l: LenhCuaTa) -> Vec<KhopLenh> {
        let mut khop = Vec::new();
        let mut con = l.con_lai;

        // Lệnh chủ động: ăn qua các mức đối ứng theo thứ tự giá tốt nhất trước.
        loop {
            if con <= 0 {
                break;
            }
            let doi_ung = match l.chieu {
                Chieu::Mua => self.ban.iter().next().map(|(&g, &k)| (g, k)),
                Chieu::Ban => self.mua.iter().next_back().map(|(&g, &k)| (g, k)),
            };
            let (g, k) = match doi_ung {
                Some(x) => x,
                None => break,
            };
            let cat_qua = match l.chieu {
                Chieu::Mua => l.gia >= g,
                Chieu::Ban => l.gia <= g,
            };
            if !cat_qua {
                break;
            }
            let lay = con.min(k);
            self.bot(l.chieu.nguoc(), g, lay);
            khop.push(KhopLenh { ma: l.ma, chieu: l.chieu, gia: g, khoi_luong: lay, chu_dong: true });
            con -= lay;
        }

        if con > 0 {
            let truoc = self.khoi_luong_tai(l.chieu, l.gia);
            self.them(l.chieu, l.gia, con);
            self.lenh_cua_ta
                .insert(l.ma, LenhCuaTa { con_lai: con, khoi_luong_truoc: truoc, ..l });
        }
        khop
    }

    pub fn huy_lenh_cua_ta(&mut self, ma: MaLenh) -> bool {
        match self.lenh_cua_ta.remove(&ma) {
            Some(l) => {
                self.bot(l.chieu, l.gia, l.con_lai);
                true
            }
            None => false,
        }
    }

    pub fn lenh_treo_cua_ta(&self) -> Vec<LenhCuaTa> {
        self.lenh_cua_ta.values().copied().collect()
    }

    /// Khi thị trường khớp ở giá `g`, lệnh treo của ta ở giá tốt bằng hoặc hơn
    /// sẽ được khớp — nhưng chỉ sau khi hàng đứng trước đã tiêu hết.
    pub fn xu_ly_khop_thi_truong(&mut self, gia: Gia, mut khoi_luong: SoLuong) -> Vec<KhopLenh> {
        let mut ra = Vec::new();
        let mut ma_xong = Vec::new();

        let mut ung_vien: Vec<LenhCuaTa> = self
            .lenh_cua_ta
            .values()
            .copied()
            .filter(|l| match l.chieu {
                Chieu::Mua => l.gia >= gia,
                Chieu::Ban => l.gia <= gia,
            })
            .collect();
        // Ưu tiên thời gian: ai vào trước được phục vụ trước.
        ung_vien.sort_by_key(|l| (l.thoi_diem_vao, l.ma));

        for l in ung_vien {
            if khoi_luong <= 0 {
                break;
            }
            // Hàng đứng trước ăn phần của nó trước.
            let sau_hang = khoi_luong - l.khoi_luong_truoc;
            if sau_hang <= 0 {
                if let Some(m) = self.lenh_cua_ta.get_mut(&l.ma) {
                    m.khoi_luong_truoc -= khoi_luong;
                }
                break;
            }
            let lay = sau_hang.min(l.con_lai);
            if let Some(m) = self.lenh_cua_ta.get_mut(&l.ma) {
                m.khoi_luong_truoc = 0;
                m.con_lai -= lay;
                if m.con_lai <= 0 {
                    ma_xong.push(l.ma);
                }
            }
            self.bot(l.chieu, l.gia, lay);
            ra.push(KhopLenh { ma: l.ma, chieu: l.chieu, gia: l.gia, khoi_luong: lay, chu_dong: false });
            khoi_luong -= lay + l.khoi_luong_truoc;
        }
        for m in ma_xong {
            self.lenh_cua_ta.remove(&m);
        }
        ra
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KhopLenh {
    pub ma: MaLenh,
    /// Chiều của LỆNH TA — mang theo, không suy ra từ giá. Đoán chiều từ giá
    /// là một lỗi thật đã gặp: đoán sai thì vị thế chạy ngược và mọi hạn mức
    /// rủi ro trở nên vô nghĩa.
    pub chieu: Chieu,
    pub gia: Gia,
    pub khoi_luong: SoLuong,
    /// `true` = ta chủ động ăn giá (trả phí taker), `false` = ta được khớp thụ động.
    pub chu_dong: bool,
}

// ============================================================================
// 6. SÀN CHUỖI KHỐI — BỂ TÍCH KHÔNG ĐỔI
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SanChuoiKhoi {
    pub du_tru_x: u128,
    pub du_tru_y: u128,
    /// Phí theo phần vạn: 30 = 0,30%.
    pub phi_phan_van: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoiHoanDoi {
    DauVaoBangKhong,
    BeRong,
    KhongDatToiThieu { nhan_duoc: u128, yeu_cau: u128 },
}

impl SanChuoiKhoi {
    pub fn moi(x: u128, y: u128, phi_phan_van: u32) -> Self {
        SanChuoiKhoi { du_tru_x: x, du_tru_y: y, phi_phan_van }
    }

    pub fn k(&self) -> u128 {
        self.du_tru_x * self.du_tru_y
    }

    /// Giá niêm yết của X tính theo Y. Đây là giá **cận biên**, chỉ đúng cho
    /// khối lượng vô cùng nhỏ — mọi giao dịch thật đều tệ hơn con số này.
    pub fn gia_x(&self) -> f64 {
        if self.du_tru_x == 0 {
            return 0.0;
        }
        self.du_tru_y as f64 / self.du_tru_x as f64
    }

    pub fn thu_hoan_doi(&self, vao_x: bool, vao: u128) -> Result<u128, LoiHoanDoi> {
        if vao == 0 {
            return Err(LoiHoanDoi::DauVaoBangKhong);
        }
        if self.du_tru_x == 0 || self.du_tru_y == 0 {
            return Err(LoiHoanDoi::BeRong);
        }
        let (dt_vao, dt_ra) =
            if vao_x { (self.du_tru_x, self.du_tru_y) } else { (self.du_tru_y, self.du_tru_x) };
        let sau_phi = vao * (10_000 - self.phi_phan_van as u128);
        // Làm tròn LUÔN có lợi cho bể — đó là chủ ý, không phải cẩu thả.
        Ok((sau_phi * dt_ra) / (dt_vao * 10_000 + sau_phi))
    }

    pub fn hoan_doi(&mut self, vao_x: bool, vao: u128, toi_thieu_ra: u128) -> Result<u128, LoiHoanDoi> {
        let ra = self.thu_hoan_doi(vao_x, vao)?;
        if ra < toi_thieu_ra {
            return Err(LoiHoanDoi::KhongDatToiThieu { nhan_duoc: ra, yeu_cau: toi_thieu_ra });
        }
        if vao_x {
            self.du_tru_x += vao;
            self.du_tru_y -= ra;
        } else {
            self.du_tru_y += vao;
            self.du_tru_x -= ra;
        }
        Ok(ra)
    }

    /// Giá trung bình thực nhận — luôn tệ hơn `gia_x()`. Đây mới là con số
    /// dùng để so với sàn truyền thống khi tìm chênh lệch.
    pub fn gia_thuc_te(&self, vao_x: bool, vao: u128) -> Option<f64> {
        let ra = self.thu_hoan_doi(vao_x, vao).ok()?;
        if ra == 0 {
            return None;
        }
        Some(if vao_x { ra as f64 / vao as f64 } else { vao as f64 / ra as f64 })
    }

    /// Nghịch đảo của `thu_hoan_doi`: cần bỏ vào bao nhiêu để nhận ĐÚNG `ra`?
    /// Cần thiết cho phòng vệ chính xác — không có nó, chân phòng vệ lệch khối
    /// lượng và vị thế ròng không bao giờ về 0.
    pub fn dau_vao_can(&self, vao_x: bool, ra_mong_muon: u128) -> Option<u128> {
        if ra_mong_muon == 0 {
            return None;
        }
        let (dt_vao, dt_ra) =
            if vao_x { (self.du_tru_x, self.du_tru_y) } else { (self.du_tru_y, self.du_tru_x) };
        if ra_mong_muon >= dt_ra {
            return None; // không thể rút hết một phía
        }
        let tu = dt_vao * ra_mong_muon * 10_000;
        let mau = (dt_ra - ra_mong_muon) * (10_000 - self.phi_phan_van as u128);
        Some(tu / mau + 1) // +1: làm tròn LÊN, luôn có lợi cho bể
    }

    pub fn ap_dung(&mut self, sk: &LoaiSuKien) {
        if let LoaiSuKien::HoanDoiTrenBe { vao_x, khoi_luong } = sk {
            let _ = self.hoan_doi(*vao_x, *khoi_luong, 0);
        }
    }
}

// ============================================================================
// 7. ẢNH CHỤP THỊ TRƯỜNG HỢP NHẤT
// ============================================================================

/// Cái mà chiến lược được phép nhìn thấy — và **chỉ** cái này. Không có
/// tham chiếu tới phiên, không có chỉ số sự kiện, nên không thể nhìn trộm tương lai.
#[derive(Debug, Clone, Copy)]
pub struct AnhChupThiTruong {
    pub thoi_diem: Nano,
    pub tt_mua: Option<MucGia>,
    pub tt_ban: Option<MucGia>,
    pub tt_vi_gia: Option<f64>,
    pub tt_mat_can_bang: Option<f64>,
    pub ck_gia: f64,
    pub ck_du_tru_x: u128,
    pub ck_du_tru_y: u128,
}

impl AnhChupThiTruong {
    pub fn gia_giua_truyen_thong(&self) -> Option<f64> {
        match (self.tt_mua, self.tt_ban) {
            (Some(m), Some(b)) => Some((m.gia + b.gia) as f64 / 2.0),
            _ => None,
        }
    }

    /// Chênh lệch giá giữa hai sàn, tính bằng điểm cơ bản. Dương nghĩa là
    /// sàn chuỗi khối đang đắt hơn → mua truyền thống, bán chuỗi khối.
    pub fn chenh_lech_hai_san_bp(&self) -> Option<f64> {
        let tt = self.gia_giua_truyen_thong()?;
        if tt <= 0.0 {
            return None;
        }
        Some((self.ck_gia - tt) / tt * 10_000.0)
    }
}

// ============================================================================
// 8. Ý ĐỊNH GIAO DỊCH & CỔNG RỦI RO
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum YDinh {
    DatLenh { san: San, chieu: Chieu, gia: Gia, khoi_luong: SoLuong },
    HuyLenh { san: San, ma: MaLenh },
    /// Lệnh chính kèm **phòng vệ theo khối lượng đã khớp** trên sàn còn lại.
    ///
    /// Đặt cứng cả hai chân cùng lúc nghe có vẻ đúng nhưng vẫn hỏng: chân AMM
    /// luôn khớp đủ (công thức không bao giờ từ chối), còn chân sổ lệnh chỉ khớp
    /// một phần vì sổ đã đổi trong khoảng độ trễ. Chênh lệch đó đọng lại thành
    /// vị thế ròng — **bất đối xứng khớp**.
    ///
    /// Cách làm của ngành: thực thi chân KHÔNG CHẮC trước, rồi phòng vệ đúng
    /// bằng khối lượng thực sự khớp được.
    DatCoPhongVe { san: San, chieu: Chieu, gia: Gia, khoi_luong: SoLuong, phong_ve_tren: San },
}

impl YDinh {
    pub fn chan_don(san: San, chieu: Chieu, gia: Gia, khoi_luong: SoLuong) -> Self {
        YDinh::DatLenh { san, chieu, gia, khoi_luong }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TuChoi {
    DaNgatKhanCap,
    GiaNgoaiBien,
    KhoiLuongQuaLon,
    GiaTriLenhQuaLon,
    VuotHanMucViThe,
    VuotHanMucLo,
    VuotTanSuat,
}

/// Trạng thái vị thế theo giá vốn trung bình. Trường hợp **đảo chiều**
/// (vượt qua 0) phải xử lý riêng, nếu không giá vốn sai và mọi con số sau đó sai theo.
#[derive(Debug, Clone, Copy, Default)]
pub struct ViThe {
    pub so_luong: SoLuong,
    pub gia_von: f64,
    pub lai_lo_da_chot: f64,
}

impl ViThe {
    pub fn ghi_nhan(&mut self, chieu: Chieu, gia: Gia, khoi_luong: SoLuong) {
        let truoc = self.so_luong;
        let d = chieu.dau() * khoi_luong;

        if truoc == 0 || truoc.signum() == d.signum() {
            // Mở rộng cùng chiều: cập nhật giá vốn trung bình có trọng số.
            let tong = (truoc.abs() + khoi_luong) as f64;
            if tong > 0.0 {
                self.gia_von =
                    (self.gia_von * truoc.abs() as f64 + gia as f64 * khoi_luong as f64) / tong;
            }
            self.so_luong = truoc + d;
        } else {
            let dong = khoi_luong.min(truoc.abs());
            self.lai_lo_da_chot +=
                (gia as f64 - self.gia_von) * dong as f64 * truoc.signum() as f64;
            self.so_luong = truoc + d;
            if self.so_luong.signum() != truoc.signum() && self.so_luong != 0 {
                // Đảo chiều: phần dư mở vị thế mới ở đúng giá này.
                self.gia_von = gia as f64;
            } else if self.so_luong == 0 {
                self.gia_von = 0.0;
            }
        }
    }

    pub fn lai_lo_chua_chot(&self, gia_hien_tai: f64) -> f64 {
        (gia_hien_tai - self.gia_von) * self.so_luong as f64
    }

    pub fn tong_lai_lo(&self, gia_hien_tai: f64) -> f64 {
        self.lai_lo_da_chot + self.lai_lo_chua_chot(gia_hien_tai)
    }
}

#[derive(Debug, Clone)]
pub struct CongRuiRo {
    pub gia_toi_thieu: Gia,
    pub gia_toi_da: Gia,
    pub khoi_luong_toi_da: SoLuong,
    pub gia_tri_lenh_toi_da: i64,
    pub vi_the_toi_da: SoLuong,
    pub lo_toi_da: f64,
    pub lenh_moi_giay_toi_da: u32,
    pub da_ngat: bool,
    // trạng thái
    dau_cua_so: Nano,
    dem_trong_cua_so: u32,
    pub so_lan_tu_choi: BTreeMap<u8, u32>,
}

impl CongRuiRo {
    pub fn dien_hinh() -> Self {
        CongRuiRo {
            gia_toi_thieu: 1,
            gia_toi_da: 10_000_000,
            khoi_luong_toi_da: 1_000,
            gia_tri_lenh_toi_da: 100_000_000,
            vi_the_toi_da: 500,
            lo_toi_da: 100_000.0,
            lenh_moi_giay_toi_da: 10_000,
            da_ngat: false,
            dau_cua_so: 0,
            dem_trong_cua_so: 0,
            so_lan_tu_choi: BTreeMap::new(),
        }
    }

    fn ghi_tu_choi(&mut self, t: TuChoi) -> TuChoi {
        *self.so_lan_tu_choi.entry(t as u8).or_insert(0) += 1;
        t
    }

    /// Phơi nhiễm = vị thế đã khớp **cộng** khối lượng đang treo cùng chiều.
    /// Đếm thiếu phần treo là cách vị thế vượt hạn mức gấp ba lần mà không ai hay.
    pub fn kiem(
        &mut self,
        y: &YDinh,
        vi_the: SoLuong,
        treo_mua: SoLuong,
        treo_ban: SoLuong,
        lai_lo: f64,
        bay_gio: Nano,
    ) -> Result<(), TuChoi> {
        let (chieu, gia, khoi_luong) = match y {
            YDinh::HuyLenh { .. } => return Ok(()), // huỷ luôn luôn được phép
            YDinh::DatLenh { chieu, gia, khoi_luong, .. } => (*chieu, *gia, *khoi_luong),
            // Bộ điều phối tách thành chân đơn trước khi tới đây, vì chỉ nó
            // mới biết đặt chỗ tích luỹ cho cả chân chính lẫn chân phòng vệ.
            YDinh::DatCoPhongVe { .. } => return Ok(()),
        };

        if self.da_ngat {
            return Err(self.ghi_tu_choi(TuChoi::DaNgatKhanCap));
        }
        if gia < self.gia_toi_thieu || gia > self.gia_toi_da {
            return Err(self.ghi_tu_choi(TuChoi::GiaNgoaiBien));
        }
        if khoi_luong <= 0 || khoi_luong > self.khoi_luong_toi_da {
            return Err(self.ghi_tu_choi(TuChoi::KhoiLuongQuaLon));
        }
        if gia.saturating_mul(khoi_luong) > self.gia_tri_lenh_toi_da {
            return Err(self.ghi_tu_choi(TuChoi::GiaTriLenhQuaLon));
        }
        if lai_lo < -self.lo_toi_da {
            return Err(self.ghi_tu_choi(TuChoi::VuotHanMucLo));
        }

        // Kiểm CẢ HAI chiều phơi nhiễm, kể cả chiều mà lệnh này không chạm tới:
        // một lệnh mua vẫn phải bị chặn nếu chiều bán đã vượt hạn mức.
        let (sau_mua, sau_ban) = match chieu {
            Chieu::Mua => (vi_the + treo_mua + khoi_luong, vi_the - treo_ban),
            Chieu::Ban => (vi_the + treo_mua, vi_the - treo_ban - khoi_luong),
        };
        if sau_mua.abs() > self.vi_the_toi_da || sau_ban.abs() > self.vi_the_toi_da {
            return Err(self.ghi_tu_choi(TuChoi::VuotHanMucViThe));
        }

        // Cửa sổ trượt một giây.
        if bay_gio.saturating_sub(self.dau_cua_so) >= 1_000_000_000 {
            self.dau_cua_so = bay_gio;
            self.dem_trong_cua_so = 0;
        }
        if self.dem_trong_cua_so >= self.lenh_moi_giay_toi_da {
            return Err(self.ghi_tu_choi(TuChoi::VuotTanSuat));
        }
        self.dem_trong_cua_so += 1;
        Ok(())
    }
}

// ============================================================================
// 9. CHIẾN LƯỢC
// ============================================================================

pub trait ChienLuoc {
    fn ten(&self) -> &str;
    /// Nhận ảnh chụp + vị thế hiện tại, trả về các ý định. Không có tham số nào
    /// cho phép nhìn về tương lai — đó là ràng buộc kiến trúc, không phải quy ước.
    fn danh_gia(&mut self, anh: &AnhChupThiTruong, vi_the: SoLuong) -> Vec<YDinh>;
}

/// Nhà tạo lập hai chiều có kiểm soát tồn kho: càng lệch vị thế thì càng
/// nghiêng báo giá về phía kéo vị thế về 0.
pub struct TaoLapCoKiemSoat {
    pub chenh_lech_muc_tieu: Gia,
    pub khoi_luong: SoLuong,
    pub han_muc_ton_kho: SoLuong,
    pub he_so_nghieng: f64,
    pub lan_bao_gia_cuoi: Nano,
    pub khoang_bao_gia_ns: Nano,
}

impl TaoLapCoKiemSoat {
    pub fn moi(han_muc_ton_kho: SoLuong) -> Self {
        TaoLapCoKiemSoat {
            chenh_lech_muc_tieu: 4,
            khoi_luong: 20,
            han_muc_ton_kho,
            he_so_nghieng: 0.5,
            lan_bao_gia_cuoi: 0,
            khoang_bao_gia_ns: 1_000_000,
        }
    }
}

impl ChienLuoc for TaoLapCoKiemSoat {
    fn ten(&self) -> &str {
        "tao_lap_co_kiem_soat"
    }

    fn danh_gia(&mut self, anh: &AnhChupThiTruong, vi_the: SoLuong) -> Vec<YDinh> {
        if anh.thoi_diem.saturating_sub(self.lan_bao_gia_cuoi) < self.khoang_bao_gia_ns {
            return Vec::new();
        }
        let giua = match anh.tt_vi_gia.or_else(|| anh.gia_giua_truyen_thong()) {
            Some(g) => g,
            None => return Vec::new(),
        };
        self.lan_bao_gia_cuoi = anh.thoi_diem;

        // Nghiêng báo giá theo tồn kho: dài vị thế thì hạ cả hai giá để dễ bán hơn.
        let ty_le = if self.han_muc_ton_kho > 0 {
            vi_the as f64 / self.han_muc_ton_kho as f64
        } else {
            0.0
        };
        let nghieng = ty_le * self.he_so_nghieng * self.chenh_lech_muc_tieu as f64;
        let nua = self.chenh_lech_muc_tieu as f64 / 2.0;

        let mut gia_mua = (giua - nua - nghieng).round() as Gia;
        let mut gia_ban = (giua + nua - nghieng).round() as Gia;

        // KHÔNG BAO GIỜ cắt qua sổ. Một nhà tạo lập cắt giá sẽ TRẢ chênh lệch
        // thay vì THU nó — nó trở thành người chủ động, và toàn bộ mô hình kinh
        // doanh sụp đổ. Đây là ràng buộc, không phải tối ưu hoá.
        if let Some(b) = anh.tt_ban {
            gia_mua = gia_mua.min(b.gia - 1);
        }
        if let Some(m) = anh.tt_mua {
            gia_ban = gia_ban.max(m.gia + 1);
        }
        if gia_mua <= 0 || gia_ban <= gia_mua {
            return Vec::new();
        }

        let mut ra = Vec::new();
        // Chỉ báo giá bên nào chưa chạm hạn mức — hàng phòng vệ thứ nhất,
        // trước cả cổng rủi ro.
        if vi_the < self.han_muc_ton_kho {
            ra.push(YDinh::DatLenh {
                san: San::TruyenThong,
                chieu: Chieu::Mua,
                gia: gia_mua,
                khoi_luong: self.khoi_luong,
            });
        }
        if vi_the > -self.han_muc_ton_kho {
            ra.push(YDinh::DatLenh {
                san: San::TruyenThong,
                chieu: Chieu::Ban,
                gia: gia_ban,
                khoi_luong: self.khoi_luong,
            });
        }
        ra
    }
}

/// Chênh lệch giá giữa hai sàn — chiến lược **duy nhất** chạm cả hai loại thị
/// trường, và là lý do hệ sinh thái này phải hợp nhất chúng vào một ảnh chụp.
pub struct ChenhLechHaiSan {
    pub nguong_bp: f64,
    pub khoi_luong: SoLuong,
    pub so_co_hoi_thay: u64,
}

impl ChenhLechHaiSan {
    pub fn moi(nguong_bp: f64) -> Self {
        ChenhLechHaiSan { nguong_bp, khoi_luong: 10, so_co_hoi_thay: 0 }
    }
}

impl ChienLuoc for ChenhLechHaiSan {
    fn ten(&self) -> &str {
        "chenh_lech_hai_san"
    }

    fn danh_gia(&mut self, anh: &AnhChupThiTruong, _vi_the: SoLuong) -> Vec<YDinh> {
        let cl = match anh.chenh_lech_hai_san_bp() {
            Some(x) => x,
            None => return Vec::new(),
        };
        if cl.abs() < self.nguong_bp {
            return Vec::new();
        }
        self.so_co_hoi_thay += 1;

        // Chênh lệch giá là giao dịch HAI CHÂN. Chỉ đặt một chân thì đó không
        // phải chênh lệch giá — đó là cược một chiều đội lốt, và nó sẽ tích luỹ
        // vị thế cho tới khi chạm hạn mức rồi ngồi đó chịu lỗ.
        let (chieu_tt, muc) = if cl > 0.0 {
            // Chuỗi khối đắt hơn → mua chân rẻ (truyền thống), bán chân đắt.
            (Chieu::Mua, anh.tt_ban)
        } else {
            (Chieu::Ban, anh.tt_mua)
        };
        let m = match muc {
            Some(m) => m,
            None => return Vec::new(),
        };
        let kl = self.khoi_luong.min(m.khoi_luong);
        if kl <= 0 {
            return Vec::new();
        }
        // Chân truyền thống là chân KHÔNG CHẮC (phải xếp hàng, sổ có thể đã đổi).
        // Chân chuỗi khối là phòng vệ, chỉ chạy đúng bằng phần thực sự khớp.
        vec![YDinh::DatCoPhongVe {
            san: San::TruyenThong,
            chieu: chieu_tt,
            gia: m.gia,
            khoi_luong: kl,
            phong_ve_tren: San::ChuoiKhoi,
        }]
    }
}

// ============================================================================
// 10. ĐO LƯỜNG
// ============================================================================

/// Biểu đồ thùng logarit: giữ được dải từ 1 ns tới hàng phút với sai số tương
/// đối cố định, chỉ tốn vài trăm byte.
#[derive(Debug, Clone, Default)]
pub struct BieuDoTre {
    thung: BTreeMap<u32, u64>,
    pub so_mau: u64,
    pub tong: u64,
    pub lon_nhat: u64,
}

impl BieuDoTre {
    pub fn moi() -> Self {
        BieuDoTre::default()
    }

    pub fn ghi(&mut self, ns: u64) {
        let k = if ns == 0 { 0 } else { 64 - ns.leading_zeros() };
        *self.thung.entry(k).or_insert(0) += 1;
        self.so_mau += 1;
        self.tong += ns;
        self.lon_nhat = self.lon_nhat.max(ns);
    }

    pub fn trung_binh(&self) -> f64 {
        if self.so_mau == 0 {
            0.0
        } else {
            self.tong as f64 / self.so_mau as f64
        }
    }

    /// Phân vị là con số DUY NHẤT đáng nhìn trong HFT. Trung bình chỉ hữu ích
    /// để phát hiện là mình đã đo sai.
    pub fn phan_vi(&self, p: f64) -> u64 {
        if self.so_mau == 0 {
            return 0;
        }
        let muc = (self.so_mau as f64 * p).ceil() as u64;
        let mut luy_ke = 0;
        for (&k, &c) in &self.thung {
            luy_ke += c;
            if luy_ke >= muc {
                return if k == 0 { 0 } else { 1u64 << (k - 1) };
            }
        }
        self.lon_nhat
    }
}

#[derive(Debug, Clone, Default)]
pub struct BoDoLuong {
    pub tre_tin_hieu_toi_lenh: BieuDoTre,
    pub so_y_dinh: u64,
    pub so_lenh_gui: u64,
    pub so_lenh_bi_chan: u64,
    pub so_khop: u64,
    pub khoi_luong_khop: SoLuong,
    pub khoi_luong_chu_dong: SoLuong,
    pub duong_von: Vec<f64>,
}

impl BoDoLuong {
    pub fn moi() -> Self {
        BoDoLuong::default()
    }

    pub fn ty_le_chan(&self) -> f64 {
        if self.so_y_dinh == 0 {
            0.0
        } else {
            self.so_lenh_bi_chan as f64 / self.so_y_dinh as f64
        }
    }

    /// Tỉ lệ thụ động: phần khối lượng ta được khớp mà không phải ăn giá.
    /// Nhà tạo lập sống bằng con số này.
    pub fn ty_le_thu_dong(&self) -> f64 {
        if self.khoi_luong_khop == 0 {
            0.0
        } else {
            (self.khoi_luong_khop - self.khoi_luong_chu_dong) as f64 / self.khoi_luong_khop as f64
        }
    }

    pub fn sut_giam_toi_da(&self) -> f64 {
        let mut dinh = f64::NEG_INFINITY;
        let mut sut: f64 = 0.0;
        for &v in &self.duong_von {
            dinh = dinh.max(v);
            if dinh.is_finite() {
                sut = sut.max(dinh - v);
            }
        }
        sut
    }
}

// ============================================================================
// 11. HỆ SINH THÁI — BỘ ĐIỀU PHỐI
// ============================================================================

/// Lệnh đang bay tới sàn. Nó **chưa tồn tại** với sàn cho tới `den_luc`.
/// Bỏ qua khoảng này là dạng nhìn trộm tương lai tinh vi nhất trong HFT.
#[derive(Debug, Clone, Copy)]
struct LenhDangBay {
    den_luc: Nano,
    phat_luc: Nano,
    y_dinh: YDinh,
    ma: MaLenh,
}

pub struct HeSinhThai {
    pub dong_ho: DongHoAo,
    pub toc_do: TocDoPhat,
    pub do_tre: MoHinhDoTre,
    pub san_tt: SanTruyenThong,
    pub san_ck: SanChuoiKhoi,
    pub cong: CongRuiRo,
    pub vi_the: ViThe,
    pub do_luong: BoDoLuong,
    pub ma_ke_tiep: MaLenh,
    dang_bay: VecDeque<LenhDangBay>,
    treo_mua: SoLuong,
    treo_ban: SoLuong,
    /// Phơi nhiễm của lệnh ĐANG BAY — đã phát nhưng chưa tới sàn.
    /// Không đếm phần này là lỗ hổng kinh điển: nhiều lệnh phát trong cùng một
    /// nhịp đều thấy CÙNG một trạng thái vị thế, đều được cổng cho qua, rồi
    /// cùng khớp — và hạn mức bị vượt dù mọi phép kiểm đều "đã chạy".
    bay_mua: SoLuong,
    bay_ban: SoLuong,
    /// Mã lệnh cần phòng vệ, và phòng vệ trên sàn nào.
    can_phong_ve: BTreeMap<MaLenh, San>,
    /// Số lần phòng vệ đã chạy — chỉ số vận hành, không phải trang trí.
    pub so_lan_phong_ve: u64,
    /// Nhật ký lệnh đã gửi — cơ sở của bài kiểm thử tính tất định.
    pub nhat_ky: Vec<(Nano, San, Chieu, Gia, SoLuong)>,
    /// Tuổi tối đa của một báo giá trước khi bị tự động rút. Không có chính sách
    /// này thì báo giá chất đống, phơi nhiễm treo tăng vô hạn và cổng rủi ro
    /// chặn gần như mọi lệnh mới — hệ thống tự bóp cổ mình.
    pub tuoi_bao_gia_toi_da_ns: Nano,
}

impl HeSinhThai {
    pub fn moi(san_ck: SanChuoiKhoi, do_tre: MoHinhDoTre, toc_do: TocDoPhat) -> Self {
        HeSinhThai {
            dong_ho: DongHoAo::default(),
            toc_do,
            do_tre,
            san_tt: SanTruyenThong::moi(),
            san_ck,
            cong: CongRuiRo::dien_hinh(),
            vi_the: ViThe::default(),
            do_luong: BoDoLuong::moi(),
            ma_ke_tiep: 1_000_000,
            dang_bay: VecDeque::new(),
            treo_mua: 0,
            treo_ban: 0,
            bay_mua: 0,
            bay_ban: 0,
            can_phong_ve: BTreeMap::new(),
            so_lan_phong_ve: 0,
            nhat_ky: Vec::new(),
            tuoi_bao_gia_toi_da_ns: 20_000_000, // 20 ms
        }
    }

    pub fn anh_chup(&self) -> AnhChupThiTruong {
        AnhChupThiTruong {
            thoi_diem: self.dong_ho.bay_gio(),
            tt_mua: self.san_tt.gia_mua_tot_nhat(),
            tt_ban: self.san_tt.gia_ban_tot_nhat(),
            tt_vi_gia: self.san_tt.vi_gia(),
            tt_mat_can_bang: self.san_tt.mat_can_bang(),
            ck_gia: self.san_ck.gia_x(),
            ck_du_tru_x: self.san_ck.du_tru_x,
            ck_du_tru_y: self.san_ck.du_tru_y,
        }
    }

    fn gia_tham_chieu(&self) -> f64 {
        self.san_tt
            .gia_giua()
            .or_else(|| self.san_tt.gia_mua_tot_nhat().map(|m| m.gia as f64))
            .unwrap_or(self.vi_the.gia_von)
    }

    /// Giao các lệnh đã tới hạn. Chúng được khớp theo trạng thái sàn
    /// **tại thời điểm đến**, không phải lúc phát — đó là toàn bộ ý nghĩa của độ trễ.
    fn giao_lenh_den_han(&mut self) {
        let bay_gio = self.dong_ho.bay_gio();
        while self.dang_bay.front().map_or(false, |l| l.den_luc <= bay_gio) {
            let l = self.dang_bay.pop_front().unwrap();
            match l.y_dinh {
                YDinh::HuyLenh { san: San::TruyenThong, ma } => {
                    if let Some(t) = self.san_tt.lenh_treo_cua_ta().iter().find(|x| x.ma == ma) {
                        match t.chieu {
                            Chieu::Mua => self.treo_mua -= t.con_lai,
                            Chieu::Ban => self.treo_ban -= t.con_lai,
                        }
                    }
                    self.san_tt.huy_lenh_cua_ta(ma);
                }
                YDinh::HuyLenh { .. } => {}
                YDinh::DatCoPhongVe { .. } => {}
                YDinh::DatLenh { san: San::TruyenThong, chieu, gia, khoi_luong } => {
                    match chieu {
                        Chieu::Mua => {
                            self.bay_mua = (self.bay_mua - khoi_luong).max(0);
                            self.treo_mua += khoi_luong;
                        }
                        Chieu::Ban => {
                            self.bay_ban = (self.bay_ban - khoi_luong).max(0);
                            self.treo_ban += khoi_luong;
                        }
                    }
                    let khop = self.san_tt.dat_lenh_cua_ta(LenhCuaTa {
                        ma: l.ma,
                        chieu,
                        gia,
                        con_lai: khoi_luong,
                        thoi_diem_vao: l.den_luc,
                        khoi_luong_truoc: 0,
                    });
                    self.do_luong.tre_tin_hieu_toi_lenh.ghi(l.den_luc - l.phat_luc);
                    self.nhat_ky.push((l.den_luc, San::TruyenThong, chieu, gia, khoi_luong));
                    for k in khop {
                        self.ap_dung_khop(k);
                    }
                }
                YDinh::DatLenh { san: San::ChuoiKhoi, chieu, gia, khoi_luong } => {
                    // Sàn AMM khớp tức thì theo công thức — không xếp hàng, nhưng
                    // vẫn phải chịu độ trễ tới lượt được đưa vào khối.
                    match chieu {
                        Chieu::Mua => self.bay_mua = (self.bay_mua - khoi_luong).max(0),
                        Chieu::Ban => self.bay_ban = (self.bay_ban - khoi_luong).max(0),
                    }
                    let vao_x = chieu == Chieu::Ban;
                    if self.san_ck.hoan_doi(vao_x, khoi_luong as u128, 0).is_ok() {
                        self.do_luong.tre_tin_hieu_toi_lenh.ghi(l.den_luc - l.phat_luc);
                        self.nhat_ky.push((l.den_luc, San::ChuoiKhoi, chieu, gia, khoi_luong));
                        self.ap_dung_khop(KhopLenh {
                            ma: l.ma,
                            chieu,
                            gia,
                            khoi_luong,
                            chu_dong: true,
                        });
                    }
                }
            }
        }
    }

    fn ap_dung_khop(&mut self, k: KhopLenh) {
        self.vi_the.ghi_nhan(k.chieu, k.gia, k.khoi_luong);

        // Phòng vệ NGAY, đúng bằng khối lượng vừa khớp. Đây là chỗ bất đối xứng
        // khớp được triệt tiêu: chân chắc chắn chỉ chạy sau khi chân không chắc
        // đã cho biết nó khớp được bao nhiêu.
        if let Some(&san_pv) = self.can_phong_ve.get(&k.ma) {
            if san_pv == San::ChuoiKhoi && k.khoi_luong > 0 {
                let nguoc = k.chieu.nguoc();
                let kl = k.khoi_luong as u128;
                // Giá phải là giá THỰC NHẬN trên bể, không phải giá của sàn kia.
                // Ghi sổ chân phòng vệ ở giá sàn truyền thống khiến chênh lệch
                // thu được luôn bằng 0 — chiến lược "phi rủi ro" chỉ còn chi phí.
                let gia_thuc = match nguoc {
                    // Bán X trên bể: bỏ vào kl X, nhận `ra` Y → giá = ra/kl.
                    Chieu::Ban => self
                        .san_ck
                        .hoan_doi(true, kl, 0)
                        .ok()
                        .map(|ra| ra as f64 / kl as f64),
                    // Mua X trên bể: cần bỏ vào bao nhiêu Y để nhận đúng kl X?
                    Chieu::Mua => self.san_ck.dau_vao_can(false, kl).and_then(|vao_y| {
                        self.san_ck
                            .hoan_doi(false, vao_y, 0)
                            .ok()
                            .map(|_| vao_y as f64 / kl as f64)
                    }),
                };
                if let Some(g) = gia_thuc {
                    let gt = g.round().max(1.0) as Gia;
                    self.vi_the.ghi_nhan(nguoc, gt, k.khoi_luong);
                    self.so_lan_phong_ve += 1;
                    self.nhat_ky.push((
                        self.dong_ho.bay_gio(),
                        San::ChuoiKhoi,
                        nguoc,
                        gt,
                        k.khoi_luong,
                    ));
                }
            }
        }

        self.do_luong.so_khop += 1;
        self.do_luong.khoi_luong_khop += k.khoi_luong;
        if k.chu_dong {
            self.do_luong.khoi_luong_chu_dong += k.khoi_luong;
        }
        match k.chieu {
            Chieu::Mua => self.treo_mua = (self.treo_mua - k.khoi_luong).max(0),
            Chieu::Ban => self.treo_ban = (self.treo_ban - k.khoi_luong).max(0),
        }
    }

    /// Đưa các ý định qua cổng rủi ro rồi xếp vào hàng đợi bay.
    pub fn phat_y_dinh(&mut self, cac_y: Vec<YDinh>) {
        let bay_gio = self.dong_ho.bay_gio();
        let lai_lo = self.vi_the.tong_lai_lo(self.gia_tham_chieu());

        for y in cac_y {
            self.do_luong.so_y_dinh += 1;

            // --- lệnh chính + phòng vệ theo khối lượng đã khớp ---
            if let YDinh::DatCoPhongVe { san, chieu, gia, khoi_luong, phong_ve_tren } = y {
                let don = YDinh::chan_don(san, chieu, gia, khoi_luong);
                if self
                    .cong
                    .kiem(
                        &don,
                        self.vi_the.so_luong,
                        self.treo_mua + self.bay_mua,
                        self.treo_ban + self.bay_ban,
                        lai_lo,
                        bay_gio,
                    )
                    .is_err()
                {
                    self.do_luong.so_lenh_bi_chan += 1;
                    continue;
                }
                let ma = self.ma_ke_tiep;
                self.ma_ke_tiep += 1;
                match chieu {
                    Chieu::Mua => self.bay_mua += khoi_luong,
                    Chieu::Ban => self.bay_ban += khoi_luong,
                }
                // Ghi nhớ: mọi phần khớp của mã này phải được phòng vệ ngay.
                self.can_phong_ve.insert(ma, phong_ve_tren);
                let tre = self.do_tre.tre_lenh(ma ^ bay_gio);
                self.dang_bay.push_back(LenhDangBay {
                    den_luc: bay_gio + tre,
                    phat_luc: bay_gio,
                    y_dinh: don,
                    ma,
                });
                self.do_luong.so_lenh_gui += 1;
                continue;
            }

            // Cộng cả phơi nhiễm đang bay: đây là điểm khác biệt giữa một cổng
            // rủi ro đúng và một cổng chỉ trông có vẻ đúng.
            let ok = self.cong.kiem(
                &y,
                self.vi_the.so_luong,
                self.treo_mua + self.bay_mua,
                self.treo_ban + self.bay_ban,
                lai_lo,
                bay_gio,
            );
            if ok.is_err() {
                self.do_luong.so_lenh_bi_chan += 1;
                continue;
            }
            // ĐẶT CHỖ ngay lập tức, trước khi xét ý định tiếp theo trong cùng nhịp.
            #[allow(clippy::single_match)]
            if let YDinh::DatLenh { chieu, khoi_luong, .. } = y {
                match chieu {
                    Chieu::Mua => self.bay_mua += khoi_luong,
                    Chieu::Ban => self.bay_ban += khoi_luong,
                }
            }
            let ma = self.ma_ke_tiep;
            self.ma_ke_tiep += 1;
            // Hạt giống dẫn xuất từ (mã lệnh, thời điểm) → dao động tất định.
            let tre = self.do_tre.tre_lenh(ma ^ bay_gio);
            self.dang_bay.push_back(LenhDangBay {
                den_luc: bay_gio + tre,
                phat_luc: bay_gio,
                y_dinh: y,
                ma,
            });
            self.do_luong.so_lenh_gui += 1;
        }
        // Hàng đợi phải theo thứ tự thời gian đến; dao động có thể đảo thứ tự phát.
        let mut v: Vec<LenhDangBay> = self.dang_bay.drain(..).collect();
        v.sort_by_key(|l| (l.den_luc, l.ma));
        self.dang_bay = v.into();
    }

    /// Chạy trọn một phiên. Đây là điểm mà mọi mảnh của chương 74–78 gặp nhau.
    pub fn chay(&mut self, phien: &PhienDaGhi, cac_chien_luoc: &mut [Box<dyn ChienLuoc>]) {
        for sk in &phien.cac_su_kien {
            // 1. Thời gian tiến tới thời điểm sự kiện.
            if !self.dong_ho.tien_toi(sk.thoi_diem) {
                continue;
            }
            // 2. Giao mọi lệnh đã tới nơi TRƯỚC sự kiện này.
            self.giao_lenh_den_han();

            // 2b. Rút báo giá đã quá cũ. Huỷ đi thẳng, không qua độ trễ gửi:
            // sàn thật xử lý huỷ trên đường ưu tiên, và quan trọng hơn — nếu
            // huỷ cũng phải xếp hàng thì rủi ro tồn kho không bao giờ giảm được.
            let cu = self
                .san_tt
                .lenh_cua_ta_cu_hon(self.dong_ho.bay_gio(), self.tuoi_bao_gia_toi_da_ns);
            for ma in cu {
                if let Some(t) = self.san_tt.lenh_treo_cua_ta().iter().find(|x| x.ma == ma) {
                    match t.chieu {
                        Chieu::Mua => self.treo_mua = (self.treo_mua - t.con_lai).max(0),
                        Chieu::Ban => self.treo_ban = (self.treo_ban - t.con_lai).max(0),
                    }
                }
                self.san_tt.huy_lenh_cua_ta(ma);
            }

            // 3. Áp dụng sự kiện lên đúng sàn của nó.
            match sk.san {
                San::TruyenThong => {
                    self.san_tt.ap_dung(&sk.loai);
                    // Lệnh thị trường cắt qua lệnh treo của ta → khớp thụ động.
                    for k in self.san_tt.lay_khop_thu_dong() {
                        self.ap_dung_khop(k);
                    }
                    if let LoaiSuKien::DaKhop { gia, khoi_luong } = sk.loai {
                        for k in self.san_tt.xu_ly_khop_thi_truong(gia, khoi_luong) {
                            self.ap_dung_khop(k);
                        }
                    }
                }
                San::ChuoiKhoi => self.san_ck.ap_dung(&sk.loai),
            }

            // 4. Chiến lược nhìn ảnh chụp SAU sự kiện — và chỉ ảnh chụp.
            let anh = self.anh_chup();
            let mut y_dinh = Vec::new();
            for cl in cac_chien_luoc.iter_mut() {
                y_dinh.extend(cl.danh_gia(&anh, self.vi_the.so_luong));
            }
            self.phat_y_dinh(y_dinh);

            self.do_luong.duong_von.push(self.vi_the.tong_lai_lo(self.gia_tham_chieu()));
        }
        // Xả nốt các lệnh còn đang bay.
        self.dong_ho.tien_toi(self.dong_ho.bay_gio() + 1_000_000_000);
        self.giao_lenh_den_han();
    }
}

// ============================================================================
// 12. BỘ SINH PHIÊN TỔNG HỢP
// ============================================================================

/// Sinh một phiên hai sàn tất định. Hai chi tiết quan trọng, cả hai đều là
/// bài học rút ra từ lỗi thật: huỷ **lệnh sống cũ nhất** (không phải mã ngẫu
/// nhiên, vì phần lớn mã ngẫu nhiên đã chết), và **giới hạn số lệnh sống**
/// để sổ không phình ra rồi chéo vĩnh viễn.
pub const BE_KHOI_DAU: (u128, u128, u32) = (2_000_000, 20_000_000_000, 30);

pub fn sinh_phien(so_su_kien: usize, hat_giong: u64, gia_neo: Gia) -> PhienDaGhi {
    let mut p = PhienDaGhi::moi();
    let mut t: Nano = 1_000_000_000;
    let mut ma: MaLenh = 1;
    let mut song: VecDeque<MaLenh> = VecDeque::new();
    let mut gia_hien = gia_neo;
    // Bản sao bể để chọn CHIỀU hoán đổi. Nó đại diện cho **phần còn lại của
    // thị trường**: những nhà chênh lệch khác liên tục kéo giá bể về sát sàn
    // truyền thống. Không có lực này, bể trôi tự do và mọi chiến lược chênh
    // lệch trong mô hình sẽ in ra tiền — một kết quả hoàn toàn giả.
    let mut be_mo_phong = SanChuoiKhoi::moi(BE_KHOI_DAU.0, BE_KHOI_DAU.1, BE_KHOI_DAU.2);

    for i in 0..so_su_kien {
        let r = bam64(hat_giong ^ (i as u64).wrapping_mul(0x1000193));
        t += 1_000 + (r % 200_000);

        // Bước ngẫu nhiên có neo: kéo giá về `gia_neo` để chuỗi không trôi mất.
        let buoc = (bam64(r) % 5) as Gia - 2;
        gia_hien = (gia_hien + buoc).max(gia_neo - 40).min(gia_neo + 40);

        let nhanh = r % 100;
        if nhanh < 8 {
            // Hoán đổi trên bể chuỗi khối. Chiều được chọn để KÉO giá bể về
            // phía giá sàn truyền thống, cộng thêm một phần nhiễu từ người
            // giao dịch thường.
            let lech = be_mo_phong.gia_x() - gia_hien as f64;
            let nhieu = bam64(r ^ 0x5A5A) % 5 == 0; // 20% là nhiễu thuần
            let vao_x = if nhieu { (r >> 8) % 2 == 0 } else { lech > 0.0 };
            let kl = 1 + (bam64(r ^ 0xABC) % 500) as u128;
            let _ = be_mo_phong.hoan_doi(vao_x, kl, 0);
            p.ghi(SuKienPhien {
                thoi_diem: t,
                san: San::ChuoiKhoi,
                loai: LoaiSuKien::HoanDoiTrenBe { vao_x, khoi_luong: kl },
            });
        } else if nhanh < 20 && !song.is_empty() {
            // Giao dịch đã khớp trên sàn truyền thống.
            let kl = 1 + (bam64(r ^ 0xDEF) % 40) as SoLuong;
            p.ghi(SuKienPhien {
                thoi_diem: t,
                san: San::TruyenThong,
                loai: LoaiSuKien::DaKhop { gia: gia_hien, khoi_luong: kl },
            });
        } else if song.len() >= 120 || (nhanh < 55 && song.len() > 20) {
            // Huỷ lệnh SỐNG CŨ NHẤT — mô phỏng đúng hành vi nhà tạo lập thật.
            if let Some(m) = song.pop_front() {
                p.ghi(SuKienPhien {
                    thoi_diem: t,
                    san: San::TruyenThong,
                    loai: LoaiSuKien::HuyLenh { ma: m },
                });
            }
        } else {
            let ben_mua = (r >> 16) % 2 == 0;
            let lech = 1 + (bam64(r ^ 0x777) % 6) as Gia;
            let (chieu, gia) = if ben_mua {
                (Chieu::Mua, gia_hien - lech)
            } else {
                (Chieu::Ban, gia_hien + lech)
            };
            let kl = 10 + (bam64(r ^ 0x999) % 90) as SoLuong;
            p.ghi(SuKienPhien {
                thoi_diem: t,
                san: San::TruyenThong,
                loai: LoaiSuKien::ThemLenh { ma, chieu, gia, khoi_luong: kl },
            });
            song.push_back(ma);
            ma += 1;
        }
    }
    p
}

// ============================================================================
// 13. TRÌNH DIỄN
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 85: HỆ SINH THÁI HFT TÍCH HỢP ===\n");

    let phien = sinh_phien(20_000, 0xC0FFEE, 10_000);
    println!("1. PHIÊN ĐÃ GHI");
    println!("   sự kiện        : {}", phien.so_su_kien());
    println!("   khoảng thời gian: {:.3} giây", phien.khoang_thoi_gian_ns() as f64 / 1e9);
    println!("   đúng thứ tự    : {}", phien.dung_thu_tu());

    println!("\n2. PHÁT LẠI Ở NHIỀU TỐC ĐỘ — kết quả PHẢI trùng nhau");
    println!("   {:<16} {:>10} {:>10} {:>12}", "tốc độ", "lệnh gửi", "khớp", "lãi/lỗ");
    let mut dau_tien = None;
    for toc in [TocDoPhat::VoHan, TocDoPhat::Nhanh(1_000), TocDoPhat::ThoiGianThuc] {
        let mut hst = HeSinhThai::moi(
            SanChuoiKhoi::moi(2_000_000, 20_000_000_000, 30),
            MoHinhDoTre::dien_hinh(),
            toc,
        );
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![
            Box::new(TaoLapCoKiemSoat::moi(200)),
            Box::new(ChenhLechHaiSan::moi(150.0)),
        ];
        hst.chay(&phien, &mut cls);
        let ll = hst.vi_the.tong_lai_lo(hst.gia_tham_chieu());
        let ten = match toc {
            TocDoPhat::VoHan => "vô hạn",
            TocDoPhat::Nhanh(n) => {
                let _ = n;
                "×1000"
            }
            TocDoPhat::ThoiGianThuc => "thời gian thực",
        };
        println!(
            "   {:<16} {:>10} {:>10} {:>12.1}",
            ten, hst.do_luong.so_lenh_gui, hst.do_luong.so_khop, ll
        );
        let dau_van = (hst.do_luong.so_lenh_gui, hst.do_luong.so_khop, hst.nhat_ky.len());
        match dau_tien {
            None => dau_tien = Some(dau_van),
            Some(d) => assert_eq!(d, dau_van, "phát lại KHÔNG tất định giữa các tốc độ"),
        }
    }

    println!("\n3. HỆ SINH THÁI ĐẦY ĐỦ — hai sàn, hai chiến lược");
    let mut hst = HeSinhThai::moi(
        SanChuoiKhoi::moi(2_000_000, 20_000_000_000, 30),
        MoHinhDoTre::dien_hinh(),
        TocDoPhat::VoHan,
    );
    let mut cls: Vec<Box<dyn ChienLuoc>> = vec![
        Box::new(TaoLapCoKiemSoat::moi(200)),
        Box::new(ChenhLechHaiSan::moi(150.0)),
    ];
    hst.chay(&phien, &mut cls);

    let m = &hst.do_luong;
    println!("   ý định sinh ra     : {}", m.so_y_dinh);
    println!("   lệnh gửi đi        : {}", m.so_lenh_gui);
    println!("   bị cổng rủi ro chặn: {} ({:.1}%)", m.so_lenh_bi_chan, m.ty_le_chan() * 100.0);
    println!("   số lần khớp        : {}", m.so_khop);
    println!("   khối lượng khớp    : {}", m.khoi_luong_khop);
    println!("   tỉ lệ thụ động     : {:.1}%", m.ty_le_thu_dong() * 100.0);
    println!("   lần phòng vệ chạy  : {}", hst.so_lan_phong_ve);
    println!("   vị thế cuối        : {}", hst.vi_the.so_luong);
    println!("   lãi/lỗ đã chốt     : {:.1}", hst.vi_the.lai_lo_da_chot);
    println!("   sụt giảm tối đa    : {:.1}", m.sut_giam_toi_da());

    println!("\n4. ĐỘ TRỄ TÍN HIỆU → LỆNH TỚI SÀN (nanosecond)");
    let h = &m.tre_tin_hieu_toi_lenh;
    println!("   mẫu   : {}", h.so_mau);
    println!("   trung bình: {:.0}", h.trung_binh());
    println!("   p50   : {}", h.phan_vi(0.50));
    println!("   p99   : {}", h.phan_vi(0.99));
    println!("   lớn nhất: {}", h.lon_nhat);

    println!("\n5. CỔNG RỦI RO ĐÃ CHẶN GÌ");
    let ten = |k: u8| match k {
        0 => "đã ngắt khẩn cấp",
        1 => "giá ngoài biên",
        2 => "khối lượng quá lớn",
        3 => "giá trị lệnh quá lớn",
        4 => "vượt hạn mức vị thế",
        5 => "vượt hạn mức lỗ",
        _ => "vượt tần suất",
    };
    if hst.cong.so_lan_tu_choi.is_empty() {
        println!("   (không có lệnh nào bị chặn)");
    }
    for (k, v) in &hst.cong.so_lan_tu_choi {
        println!("   {:<24} {}", ten(*k), v);
    }

    println!("\n6. VÌ SAO KHÔNG ĐƯỢC TIN CON SỐ LÃI/LỖ Ở TRÊN");
    println!("   Phiên này là TỔNG HỢP, và mối liên kết giữa hai sàn chỉ được mô");
    println!("   phỏng một phần: bể chuỗi khối được kéo về giá sàn truyền thống,");
    println!("   nhưng không hoàn hảo. Khe hở còn lại là quà tặng cho chiến lược");
    println!("   chênh lệch — thứ không tồn tại trên thị trường thật, nơi hàng trăm");
    println!("   hãng cùng săn đúng khe hở đó trong vài trăm nanosecond.");
    println!("   Thứ ĐÁNG tin ở chương này là các BẤT BIẾN bên dưới, không phải lãi/lỗ.");

    println!("\n7. BẤT BIẾN RỦI RO");
    println!(
        "   |vị thế| ≤ hạn mức {} : {}",
        hst.cong.vi_the_toi_da,
        hst.vi_the.so_luong.abs() <= hst.cong.vi_the_toi_da
    );
    println!("   sổ lệnh không chéo   : {}", !hst.san_tt.bi_cheo());
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn he_moi() -> HeSinhThai {
        HeSinhThai::moi(
            SanChuoiKhoi::moi(2_000_000, 20_000_000_000, 30),
            MoHinhDoTre::dien_hinh(),
            TocDoPhat::VoHan,
        )
    }

    // ---- đồng hồ ảo ----

    #[test]
    fn dong_ho_chi_tien_khong_lui() {
        let mut d = DongHoAo::moi(100);
        assert!(d.tien_toi(200));
        assert_eq!(d.bay_gio(), 200);
        assert!(!d.tien_toi(150), "phải từ chối lùi thời gian");
        assert_eq!(d.bay_gio(), 200);
    }

    #[test]
    fn toc_do_phat_khong_doi_thoi_gian_ao() {
        assert_eq!(TocDoPhat::ThoiGianThuc.cho_bao_lau(1_000_000), 1_000_000);
        assert_eq!(TocDoPhat::Nhanh(1000).cho_bao_lau(1_000_000), 1_000);
        assert_eq!(TocDoPhat::VoHan.cho_bao_lau(1_000_000), 0);
    }

    // ---- phiên ----

    #[test]
    fn phien_tu_choi_su_kien_lui_thoi_gian() {
        let mut p = PhienDaGhi::moi();
        assert!(p.ghi(SuKienPhien {
            thoi_diem: 100,
            san: San::TruyenThong,
            loai: LoaiSuKien::DaKhop { gia: 10, khoi_luong: 1 },
        }));
        assert!(!p.ghi(SuKienPhien {
            thoi_diem: 50,
            san: San::TruyenThong,
            loai: LoaiSuKien::DaKhop { gia: 10, khoi_luong: 1 },
        }));
        assert_eq!(p.so_su_kien(), 1);
    }

    #[test]
    fn phien_sinh_ra_luon_dung_thu_tu() {
        let p = sinh_phien(5_000, 1, 10_000);
        assert!(p.dung_thu_tu());
        assert_eq!(p.so_su_kien(), 5_000);
    }

    #[test]
    fn phien_co_ca_hai_san() {
        let p = sinh_phien(5_000, 7, 10_000);
        let tt = p.cac_su_kien.iter().filter(|s| s.san == San::TruyenThong).count();
        let ck = p.cac_su_kien.iter().filter(|s| s.san == San::ChuoiKhoi).count();
        assert!(tt > 0 && ck > 0, "phiên phải phủ cả hai loại thị trường");
    }

    // ---- sổ lệnh truyền thống ----

    #[test]
    fn so_lenh_giu_gia_tot_nhat() {
        let mut s = SanTruyenThong::moi();
        for (ma, c, g, k) in [
            (1, Chieu::Mua, 99, 10),
            (2, Chieu::Mua, 100, 20),
            (3, Chieu::Ban, 102, 15),
            (4, Chieu::Ban, 101, 5),
        ] {
            s.ap_dung(&LoaiSuKien::ThemLenh { ma, chieu: c, gia: g, khoi_luong: k });
        }
        assert_eq!(s.gia_mua_tot_nhat().unwrap().gia, 100);
        assert_eq!(s.gia_ban_tot_nhat().unwrap().gia, 101);
        assert_eq!(s.chenh_lech(), Some(1));
        assert!(!s.bi_cheo());
    }

    #[test]
    fn huy_lenh_lam_so_co_lai() {
        let mut s = SanTruyenThong::moi();
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 1, chieu: Chieu::Mua, gia: 100, khoi_luong: 50 });
        assert_eq!(s.khoi_luong_tai(Chieu::Mua, 100), 50);
        s.ap_dung(&LoaiSuKien::HuyLenh { ma: 1 });
        assert_eq!(s.khoi_luong_tai(Chieu::Mua, 100), 0);
        assert!(s.gia_mua_tot_nhat().is_none());
    }

    #[test]
    fn bo_qua_lenh_huy_lam_so_phinh_va_chenh_lech_gia() {
        // LỖI THẬT đã gặp: bộ phát lại chỉ xử lý "thêm". Với động cơ khớp đúng,
        // hậu quả không phải là sổ chéo (lệnh cắt qua bị khớp mất) mà là
        // BÁO GIÁ CŨ KHÔNG BAO GIỜ BIẾN MẤT: sổ phình ra và chênh lệch hẹp giả tạo.
        // Chiến lược khi đó thấy thanh khoản không tồn tại.
        let mut co_huy = SanTruyenThong::moi();
        let mut khong_huy = SanTruyenThong::moi();
        let p = sinh_phien(3_000, 42, 10_000);
        for sk in &p.cac_su_kien {
            if sk.san != San::TruyenThong {
                continue;
            }
            co_huy.ap_dung(&sk.loai);
            if !matches!(sk.loai, LoaiSuKien::HuyLenh { .. }) {
                khong_huy.ap_dung(&sk.loai);
            }
        }
        assert!(
            khong_huy.tong_khoi_luong() > co_huy.tong_khoi_luong() * 2,
            "bỏ lệnh huỷ thì sổ phình lên: {} so với {}",
            khong_huy.tong_khoi_luong(),
            co_huy.tong_khoi_luong()
        );
    }

    #[test]
    fn vi_gia_nghieng_ve_ben_mong() {
        let mut s = SanTruyenThong::moi();
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 1, chieu: Chieu::Mua, gia: 100, khoi_luong: 900 });
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 2, chieu: Chieu::Ban, gia: 102, khoi_luong: 100 });
        let giua = s.gia_giua().unwrap();
        let vi = s.vi_gia().unwrap();
        assert!(vi > giua, "bên mua đông → vi giá phải cao hơn giá giữa");
        assert!(vi < 102.0);
    }

    #[test]
    fn mat_can_bang_dung_dau() {
        let mut s = SanTruyenThong::moi();
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 1, chieu: Chieu::Mua, gia: 100, khoi_luong: 900 });
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 2, chieu: Chieu::Ban, gia: 102, khoi_luong: 100 });
        assert!((s.mat_can_bang().unwrap() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn lenh_chu_dong_khop_ngay() {
        let mut s = SanTruyenThong::moi();
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 1, chieu: Chieu::Ban, gia: 100, khoi_luong: 30 });
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 2, chieu: Chieu::Ban, gia: 101, khoi_luong: 30 });
        let khop = s.dat_lenh_cua_ta(LenhCuaTa {
            ma: 9,
            chieu: Chieu::Mua,
            gia: 101,
            con_lai: 50,
            thoi_diem_vao: 0,
            khoi_luong_truoc: 0,
        });
        assert_eq!(khop.len(), 2);
        assert_eq!(khop[0].gia, 100, "phải ăn giá tốt nhất trước");
        assert_eq!(khop.iter().map(|k| k.khoi_luong).sum::<SoLuong>(), 50);
        assert!(khop.iter().all(|k| k.chu_dong));
    }

    #[test]
    fn lenh_thu_dong_cho_trong_hang() {
        let mut s = SanTruyenThong::moi();
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 1, chieu: Chieu::Mua, gia: 100, khoi_luong: 200 });
        let khop = s.dat_lenh_cua_ta(LenhCuaTa {
            ma: 9,
            chieu: Chieu::Mua,
            gia: 100,
            con_lai: 50,
            thoi_diem_vao: 0,
            khoi_luong_truoc: 0,
        });
        assert!(khop.is_empty(), "không cắt qua thì không khớp ngay");
        let treo = s.lenh_treo_cua_ta();
        assert_eq!(treo.len(), 1);
        assert_eq!(treo[0].khoi_luong_truoc, 200, "phải ghi nhận hàng đứng trước");
    }

    #[test]
    fn hang_dung_truoc_duoc_phuc_vu_truoc() {
        let mut s = SanTruyenThong::moi();
        s.ap_dung(&LoaiSuKien::ThemLenh { ma: 1, chieu: Chieu::Mua, gia: 100, khoi_luong: 100 });
        s.dat_lenh_cua_ta(LenhCuaTa {
            ma: 9,
            chieu: Chieu::Mua,
            gia: 100,
            con_lai: 50,
            thoi_diem_vao: 1,
            khoi_luong_truoc: 0,
        });
        // Thị trường khớp 60: 100 đơn vị đứng trước chưa tiêu hết → ta không được gì.
        let k = s.xu_ly_khop_thi_truong(100, 60);
        assert!(k.is_empty(), "hàng đứng trước phải tiêu hết trước khi tới lượt ta");
        // Khớp thêm 120: vượt qua 40 còn lại của hàng → ta được khớp phần dư.
        let k2 = s.xu_ly_khop_thi_truong(100, 120);
        assert!(!k2.is_empty());
        assert!(k2.iter().all(|x| !x.chu_dong));
    }

    // ---- sàn chuỗi khối ----

    #[test]
    fn hoan_doi_giu_tich_khong_giam() {
        let mut b = SanChuoiKhoi::moi(1_000_000, 1_000_000, 30);
        let k0 = b.k();
        b.hoan_doi(true, 10_000, 0).unwrap();
        assert!(b.k() >= k0, "phí làm tích TĂNG, không bao giờ giảm");
    }

    #[test]
    fn mua_cang_nhieu_gia_cang_te() {
        let b = SanChuoiKhoi::moi(1_000_000, 1_000_000, 30);
        let nho = b.gia_thuc_te(true, 1_000).unwrap();
        let lon = b.gia_thuc_te(true, 100_000).unwrap();
        assert!(lon < nho, "khối lượng lớn nhận được ít hơn trên mỗi đơn vị");
    }

    #[test]
    fn be_khong_bao_gio_can_kiet() {
        let mut b = SanChuoiKhoi::moi(1_000, 1_000, 30);
        for _ in 0..50 {
            let _ = b.hoan_doi(true, 10_000, 0);
        }
        assert!(b.du_tru_y > 0, "x·y=k khiến bể không thể bị hút cạn");
    }

    #[test]
    fn so_nhan_toi_thieu_chan_gia_xau() {
        let mut b = SanChuoiKhoi::moi(1_000_000, 1_000_000, 30);
        let du_kien = b.thu_hoan_doi(true, 10_000).unwrap();
        let r = b.hoan_doi(true, 10_000, du_kien + 1);
        assert!(matches!(r, Err(LoiHoanDoi::KhongDatToiThieu { .. })));
        assert_eq!(b.du_tru_x, 1_000_000, "giao dịch bị chặn thì bể không đổi");
    }

    // ---- vị thế & lãi lỗ ----

    #[test]
    fn gia_von_trung_binh_dung() {
        let mut v = ViThe::default();
        v.ghi_nhan(Chieu::Mua, 100, 10);
        v.ghi_nhan(Chieu::Mua, 110, 10);
        assert_eq!(v.so_luong, 20);
        assert!((v.gia_von - 105.0).abs() < 1e-9);
    }

    #[test]
    fn dong_bot_chot_lai_dung() {
        let mut v = ViThe::default();
        v.ghi_nhan(Chieu::Mua, 100, 10);
        v.ghi_nhan(Chieu::Ban, 120, 4);
        assert_eq!(v.so_luong, 6);
        assert!((v.lai_lo_da_chot - 80.0).abs() < 1e-9, "(120−100)×4 = 80");
    }

    #[test]
    fn dao_chieu_dat_lai_gia_von() {
        let mut v = ViThe::default();
        v.ghi_nhan(Chieu::Mua, 100, 10);
        v.ghi_nhan(Chieu::Ban, 120, 15); // đóng 10, mở bán 5
        assert_eq!(v.so_luong, -5);
        assert!((v.lai_lo_da_chot - 200.0).abs() < 1e-9);
        assert!((v.gia_von - 120.0).abs() < 1e-9, "phần dư mở ở giá giao dịch");
    }

    #[test]
    fn dong_het_thi_gia_von_ve_khong() {
        let mut v = ViThe::default();
        v.ghi_nhan(Chieu::Mua, 100, 10);
        v.ghi_nhan(Chieu::Ban, 105, 10);
        assert_eq!(v.so_luong, 0);
        assert_eq!(v.gia_von, 0.0);
        assert!((v.lai_lo_da_chot - 50.0).abs() < 1e-9);
    }

    // ---- cổng rủi ro ----

    #[test]
    fn cong_chan_gia_ngoai_bien() {
        let mut c = CongRuiRo::dien_hinh();
        let y = YDinh::DatLenh {
            san: San::TruyenThong,
            chieu: Chieu::Mua,
            gia: 0,
            khoi_luong: 10,
        };
        assert_eq!(c.kiem(&y, 0, 0, 0, 0.0, 0), Err(TuChoi::GiaNgoaiBien));
    }

    #[test]
    fn cong_dem_ca_lenh_dang_treo() {
        let mut c = CongRuiRo::dien_hinh();
        c.vi_the_toi_da = 100;
        let y = YDinh::DatLenh {
            san: San::TruyenThong,
            chieu: Chieu::Mua,
            gia: 100,
            khoi_luong: 50,
        };
        // Vị thế 0 nhưng đã treo mua 60 → thêm 50 nữa là vượt 100.
        assert_eq!(c.kiem(&y, 0, 60, 0, 0.0, 0), Err(TuChoi::VuotHanMucViThe));
        // Không có lệnh treo thì cùng lệnh đó qua được.
        assert!(c.kiem(&y, 0, 0, 0, 0.0, 0).is_ok());
    }

    #[test]
    fn cong_chan_khi_vuot_lo() {
        let mut c = CongRuiRo::dien_hinh();
        c.lo_toi_da = 1_000.0;
        let y = YDinh::DatLenh {
            san: San::TruyenThong,
            chieu: Chieu::Mua,
            gia: 100,
            khoi_luong: 10,
        };
        assert_eq!(c.kiem(&y, 0, 0, 0, -1_500.0, 0), Err(TuChoi::VuotHanMucLo));
    }

    #[test]
    fn cong_ngat_khan_cap_chan_moi_lenh() {
        let mut c = CongRuiRo::dien_hinh();
        c.da_ngat = true;
        let y = YDinh::DatLenh {
            san: San::TruyenThong,
            chieu: Chieu::Mua,
            gia: 100,
            khoi_luong: 1,
        };
        assert_eq!(c.kiem(&y, 0, 0, 0, 0.0, 0), Err(TuChoi::DaNgatKhanCap));
    }

    #[test]
    fn huy_lenh_luon_duoc_phep() {
        let mut c = CongRuiRo::dien_hinh();
        c.da_ngat = true;
        // Ngắt khẩn cấp phải cho HUỶ qua — nếu không, bạn không rút được chân ra.
        let y = YDinh::HuyLenh { san: San::TruyenThong, ma: 1 };
        assert!(c.kiem(&y, 0, 0, 0, 0.0, 0).is_ok());
    }

    #[test]
    fn cong_gioi_han_tan_suat_theo_cua_so_truot() {
        let mut c = CongRuiRo::dien_hinh();
        c.lenh_moi_giay_toi_da = 3;
        let y = YDinh::DatLenh {
            san: San::TruyenThong,
            chieu: Chieu::Mua,
            gia: 100,
            khoi_luong: 1,
        };
        for _ in 0..3 {
            assert!(c.kiem(&y, 0, 0, 0, 0.0, 0).is_ok());
        }
        assert_eq!(c.kiem(&y, 0, 0, 0, 0.0, 0), Err(TuChoi::VuotTanSuat));
        // Sang giây mới thì cửa sổ mở lại.
        assert!(c.kiem(&y, 0, 0, 0, 0.0, 1_000_000_000).is_ok());
    }

    // ---- độ trễ ----

    #[test]
    fn do_tre_co_dao_dong_nhung_tat_dinh() {
        let m = MoHinhDoTre::dien_hinh();
        let a: Vec<u64> = (0..100).map(|i| m.tre_lenh(i)).collect();
        let b: Vec<u64> = (0..100).map(|i| m.tre_lenh(i)).collect();
        assert_eq!(a, b, "cùng hạt giống phải cho cùng độ trễ");
        assert!(a.iter().any(|&x| x != a[0]), "phải có dao động thật, không phải hằng số");
        assert!(a.iter().all(|&x| x >= m.lenh_ra_ns));
    }

    #[test]
    fn bam_phan_bo_deu_khong_co_cum() {
        // Số học chia dư đơn thuần làm giá trị co cụm và phá mọi phép đo phân phối.
        let mut thung = [0u32; 8];
        for i in 0..8_000u64 {
            thung[(bam64(i) % 8) as usize] += 1;
        }
        assert!(thung.iter().all(|&c| c > 800 && c < 1_200), "phân bố phải đều: {:?}", thung);
    }

    // ---- biểu đồ ----

    #[test]
    fn phan_vi_bat_duoc_duoi_ma_trung_binh_bo_lo() {
        let mut h = BieuDoTre::moi();
        for i in 0..10_000 {
            // 99,9% nhanh, 0,1% chậm 50 µs — đúng hình dạng độ trễ thật.
            h.ghi(if i % 1000 == 0 { 50_000 } else { 300 });
        }
        assert!(h.phan_vi(0.50) <= 512);
        assert!(h.phan_vi(0.99) <= 512, "p99 vẫn nhanh — cái đuôi bị giấu");
        assert_eq!(h.lon_nhat, 50_000);
        assert!(h.lon_nhat as f64 > h.trung_binh() * 100.0, "max lớn hơn trung bình >100×");
    }

    // ---- chiến lược ----

    #[test]
    fn tao_lap_nghieng_bao_gia_theo_ton_kho() {
        let anh = AnhChupThiTruong {
            thoi_diem: 10_000_000,
            tt_mua: Some(MucGia { gia: 100, khoi_luong: 50 }),
            tt_ban: Some(MucGia { gia: 104, khoi_luong: 50 }),
            tt_vi_gia: Some(102.0),
            tt_mat_can_bang: Some(0.0),
            ck_gia: 102.0,
            ck_du_tru_x: 1,
            ck_du_tru_y: 102,
        };
        let lay = |vi_the| {
            let mut m = TaoLapCoKiemSoat::moi(100);
            let y = m.danh_gia(&anh, vi_the);
            y.iter()
                .filter_map(|x| match x {
                    YDinh::DatLenh { chieu: Chieu::Mua, gia, .. } => Some(*gia),
                    _ => None,
                })
                .next()
        };
        let trung_lap = lay(0).unwrap();
        let dai = lay(80).unwrap();
        assert!(dai < trung_lap, "dài vị thế → hạ giá mua để bớt mua thêm");
    }

    #[test]
    fn tao_lap_khong_bao_gio_cat_qua_so() {
        // Sổ hẹp hơn chênh lệch mục tiêu của nhà tạo lập — nếu không có ràng buộc,
        // báo giá sẽ cắt qua và biến nhà tạo lập thành người chủ động.
        let anh = AnhChupThiTruong {
            thoi_diem: 10_000_000,
            tt_mua: Some(MucGia { gia: 101, khoi_luong: 50 }),
            tt_ban: Some(MucGia { gia: 102, khoi_luong: 50 }),
            tt_vi_gia: Some(101.5),
            tt_mat_can_bang: Some(0.0),
            ck_gia: 101.5,
            ck_du_tru_x: 1,
            ck_du_tru_y: 101,
        };
        let mut m = TaoLapCoKiemSoat::moi(100);
        for y in m.danh_gia(&anh, 0) {
            if let YDinh::DatLenh { chieu, gia, .. } = y {
                match chieu {
                    Chieu::Mua => assert!(gia < 102, "giá mua {} cắt qua giá bán tốt nhất", gia),
                    Chieu::Ban => assert!(gia > 101, "giá bán {} cắt qua giá mua tốt nhất", gia),
                }
            }
        }
    }

    #[test]
    fn tao_lap_chu_yeu_khop_thu_dong() {
        // Hệ quả đo được của ràng buộc trên: phần lớn khối lượng phải đến từ
        // khớp THỤ ĐỘNG. Nhà tạo lập chủ yếu chủ động là nhà tạo lập đang lỗ.
        let p = sinh_phien(10_000, 0x4242, 10_000);
        let mut h = he_moi();
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
        h.chay(&p, &mut cls);
        assert!(h.do_luong.khoi_luong_khop > 0);
        assert!(
            h.do_luong.ty_le_thu_dong() > 0.8,
            "tỉ lệ thụ động chỉ {:.1}% — nhà tạo lập đang cắt qua sổ",
            h.do_luong.ty_le_thu_dong() * 100.0
        );
    }

    #[test]
    fn tao_lap_ngung_bao_gia_khi_cham_han_muc() {
        let anh = AnhChupThiTruong {
            thoi_diem: 10_000_000,
            tt_mua: Some(MucGia { gia: 100, khoi_luong: 50 }),
            tt_ban: Some(MucGia { gia: 104, khoi_luong: 50 }),
            tt_vi_gia: Some(102.0),
            tt_mat_can_bang: Some(0.0),
            ck_gia: 102.0,
            ck_du_tru_x: 1,
            ck_du_tru_y: 102,
        };
        let mut m = TaoLapCoKiemSoat::moi(100);
        let y = m.danh_gia(&anh, 100);
        assert!(
            y.iter().all(|x| !matches!(x, YDinh::DatLenh { chieu: Chieu::Mua, .. })),
            "chạm hạn mức dài thì không báo giá mua nữa"
        );
    }

    #[test]
    fn chenh_lech_chi_kich_hoat_khi_vuot_nguong() {
        let mut anh = AnhChupThiTruong {
            thoi_diem: 1,
            tt_mua: Some(MucGia { gia: 10_000, khoi_luong: 100 }),
            tt_ban: Some(MucGia { gia: 10_002, khoi_luong: 100 }),
            tt_vi_gia: Some(10_001.0),
            tt_mat_can_bang: Some(0.0),
            ck_gia: 10_001.0,
            ck_du_tru_x: 1,
            ck_du_tru_y: 10_001,
        };
        let mut c = ChenhLechHaiSan::moi(50.0);
        assert!(c.danh_gia(&anh, 0).is_empty(), "hai sàn ngang giá → không giao dịch");

        anh.ck_gia = 10_001.0 * 1.02; // lệch 200 bp
        let y = c.danh_gia(&anh, 0);
        assert_eq!(y.len(), 1);
        match y[0] {
            YDinh::DatCoPhongVe { san, chieu, phong_ve_tren, .. } => {
                // Chân KHÔNG CHẮC (sổ lệnh) chạy trước; chân chắc chắn (AMM)
                // chỉ phòng vệ đúng phần thực sự khớp.
                assert_eq!(san, San::TruyenThong);
                assert_eq!(chieu, Chieu::Mua, "chuỗi khối đắt hơn → mua chân truyền thống");
                assert_eq!(phong_ve_tren, San::ChuoiKhoi);
            }
            _ => panic!("chênh lệch giá phải là lệnh có phòng vệ, không phải lệnh trần"),
        }
    }

    #[test]
    fn chenh_lech_hai_chan_giu_vi_the_gan_bang_khong() {
        // Hệ quả kiểm chứng được của việc đặt đủ hai chân: chiến lược chênh lệch
        // giá KHÔNG tích luỹ vị thế ròng, khác hẳn bản chỉ đặt một chân.
        let p = sinh_phien(8_000, 0x7777, 10_000);
        let mut h = HeSinhThai::moi(
            SanChuoiKhoi::moi(2_000_000, 20_000_000_000, 30),
            MoHinhDoTre::dien_hinh(),
            TocDoPhat::VoHan,
        );
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(ChenhLechHaiSan::moi(20.0))];
        h.chay(&p, &mut cls);
        assert!(h.do_luong.so_khop > 0, "phải có giao dịch xảy ra");
        assert!(h.so_lan_phong_ve > 0, "phải có phòng vệ chạy trên sàn còn lại");
        assert_eq!(
            h.vi_the.so_luong, 0,
            "phòng vệ theo khối lượng đã khớp phải triệt tiêu vị thế ròng hoàn toàn"
        );
    }

    // ---- hệ sinh thái end-to-end ----

    #[test]
    fn he_sinh_thai_chay_tron_phien() {
        let p = sinh_phien(8_000, 0xABC, 10_000);
        let mut h = he_moi();
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
        h.chay(&p, &mut cls);
        assert!(h.do_luong.so_y_dinh > 0, "chiến lược phải sinh ra ý định");
        assert!(h.do_luong.so_lenh_gui > 0, "phải có lệnh ra khỏi cổng rủi ro");
        assert!(h.do_luong.so_khop > 0, "phải có lệnh được khớp");
    }

    #[test]
    fn phat_lai_tat_dinh_giua_hai_lan_chay() {
        let p = sinh_phien(8_000, 0xBEEF, 10_000);
        let chay = || {
            let mut h = he_moi();
            let mut cls: Vec<Box<dyn ChienLuoc>> = vec![
                Box::new(TaoLapCoKiemSoat::moi(200)),
                Box::new(ChenhLechHaiSan::moi(50.0)),
            ];
            h.chay(&p, &mut cls);
            (h.nhat_ky.clone(), h.vi_the.so_luong, h.vi_the.lai_lo_da_chot.to_bits())
        };
        assert_eq!(chay(), chay(), "hai lần chạy phải trùng khớp từng bit");
    }

    #[test]
    fn toc_do_phat_khong_doi_ket_qua() {
        let p = sinh_phien(6_000, 0xF00D, 10_000);
        let chay = |toc| {
            let mut h = HeSinhThai::moi(
                SanChuoiKhoi::moi(2_000_000, 20_000_000_000, 30),
                MoHinhDoTre::dien_hinh(),
                toc,
            );
            let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
            h.chay(&p, &mut cls);
            h.nhat_ky.clone()
        };
        // Đẩy tốc độ chỉ nén thời gian TƯỜNG. Thời gian ẢO không đổi, nên
        // kết quả chiến lược phải y hệt — miễn là không ai đọc đồng hồ thật.
        assert_eq!(chay(TocDoPhat::VoHan), chay(TocDoPhat::Nhanh(1_000)));
        assert_eq!(chay(TocDoPhat::VoHan), chay(TocDoPhat::ThoiGianThuc));
    }

    #[test]
    fn khong_bao_gio_vuot_han_muc_vi_the() {
        for hat in [1u64, 99, 12345, 0xDEAD] {
            let p = sinh_phien(8_000, hat, 10_000);
            let mut h = he_moi();
            h.cong.vi_the_toi_da = 150;
            let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(120))];
            h.chay(&p, &mut cls);
            assert!(
                h.vi_the.so_luong.abs() <= h.cong.vi_the_toi_da,
                "hạt {}: vị thế {} vượt hạn mức {}",
                hat,
                h.vi_the.so_luong,
                h.cong.vi_the_toi_da
            );
        }
    }

    #[test]
    fn do_tre_khien_lenh_toi_san_muon_hon() {
        let p = sinh_phien(4_000, 5, 10_000);
        let chay = |tre| {
            let mut h = HeSinhThai::moi(
                SanChuoiKhoi::moi(2_000_000, 20_000_000_000, 30),
                tre,
                TocDoPhat::VoHan,
            );
            let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
            h.chay(&p, &mut cls);
            h.nhat_ky.first().map(|x| x.0).unwrap_or(0)
        };
        let khong = chay(MoHinhDoTre::khong_tre());
        let co = chay(MoHinhDoTre::dien_hinh());
        assert!(co > khong, "có độ trễ thì lệnh đầu tiên tới sàn muộn hơn");
    }

    #[test]
    fn bo_qua_do_tre_lam_ket_qua_khac_han() {
        // Bỏ qua độ trễ là dạng nhìn trộm tương lai tinh vi nhất: không ai gọi
        // tên nó như vậy, nhưng nó cho chiến lược khớp ở giá đã không còn tồn tại.
        let p = sinh_phien(6_000, 0x1234, 10_000);
        let chay = |tre| {
            let mut h = HeSinhThai::moi(
                SanChuoiKhoi::moi(2_000_000, 20_000_000_000, 30),
                tre,
                TocDoPhat::VoHan,
            );
            let mut cls: Vec<Box<dyn ChienLuoc>> = vec![
                Box::new(TaoLapCoKiemSoat::moi(200)),
                Box::new(ChenhLechHaiSan::moi(50.0)),
            ];
            h.chay(&p, &mut cls);
            (h.nhat_ky.clone(), h.do_luong.khoi_luong_khop)
        };
        assert_ne!(
            chay(MoHinhDoTre::khong_tre()),
            chay(MoHinhDoTre::dien_hinh()),
            "backtest bỏ qua độ trễ cho dòng lệnh KHÁC HẲN — đó chính là vấn đề"
        );
    }

    #[test]
    fn ngat_khan_cap_dung_moi_lenh_moi() {
        let p = sinh_phien(4_000, 77, 10_000);
        let mut h = he_moi();
        h.cong.da_ngat = true;
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
        h.chay(&p, &mut cls);
        assert_eq!(h.do_luong.so_lenh_gui, 0);
        assert!(h.do_luong.so_lenh_bi_chan > 0);
        assert_eq!(h.vi_the.so_luong, 0);
    }

    #[test]
    fn ca_hai_san_deu_duoc_cap_nhat() {
        let p = sinh_phien(6_000, 0x5EED, 10_000);
        let mut h = he_moi();
        let x0 = h.san_ck.du_tru_x;
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
        h.chay(&p, &mut cls);
        assert_ne!(h.san_ck.du_tru_x, x0, "sự kiện chuỗi khối phải làm bể đổi");
        assert!(h.san_tt.gia_giua().is_some(), "sổ truyền thống phải có hai chiều");
    }

    #[test]
    fn nhat_ky_chi_ghi_lenh_da_qua_cong() {
        let p = sinh_phien(5_000, 0x99, 10_000);
        let mut h = he_moi();
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
        h.chay(&p, &mut cls);
        assert_eq!(h.nhat_ky.len() as u64, h.do_luong.so_lenh_gui);
        assert_eq!(h.do_luong.so_y_dinh, h.do_luong.so_lenh_gui + h.do_luong.so_lenh_bi_chan);
    }

    #[test]
    fn thoi_diem_trong_nhat_ky_khong_giam() {
        let p = sinh_phien(6_000, 0x2468, 10_000);
        let mut h = he_moi();
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
        h.chay(&p, &mut cls);
        assert!(
            h.nhat_ky.windows(2).all(|w| w[0].0 <= w[1].0),
            "lệnh phải tới sàn theo đúng thứ tự thời gian, dù dao động đảo thứ tự phát"
        );
    }

    #[test]
    fn sut_giam_toi_da_khong_am() {
        let p = sinh_phien(5_000, 0x1111, 10_000);
        let mut h = he_moi();
        let mut cls: Vec<Box<dyn ChienLuoc>> = vec![Box::new(TaoLapCoKiemSoat::moi(200))];
        h.chay(&p, &mut cls);
        assert!(h.do_luong.sut_giam_toi_da() >= 0.0);
    }

    #[test]
    fn hai_chien_luoc_sinh_nhieu_y_dinh_hon_mot() {
        let p = sinh_phien(6_000, 0x3333, 10_000);
        let dem = |n: usize| {
            let mut h = he_moi();
            let mut cls: Vec<Box<dyn ChienLuoc>> = if n == 1 {
                vec![Box::new(TaoLapCoKiemSoat::moi(200))]
            } else {
                vec![
                    Box::new(TaoLapCoKiemSoat::moi(200)),
                    Box::new(ChenhLechHaiSan::moi(1.0)),
                ]
            };
            h.chay(&p, &mut cls);
            h.do_luong.so_y_dinh
        };
        assert!(dem(2) > dem(1), "thêm chiến lược thì phải có thêm ý định");
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0502: cannot borrow as mutable` | Duyệt `self.lenh_cua_ta` rồi muốn sửa | Thu mã cần sửa vào `Vec` trước, sửa sau vòng lặp |
| `E0499: two mutable borrows` | `self.san_tt` và `self.vi_the` cùng lúc | Tách thành hàm nhận từng `&mut`, hoặc lấy giá trị ra trước |
| `E0038: trait cannot be made into an object` | `ChienLuoc` có phương thức generic | Giữ trait "object-safe": không generic, không `Self` ở vị trí trả về |
| `E0277: dyn ChienLuoc is not Sized` | Chứa trait object trực tiếp trong `Vec` | `Vec<Box<dyn ChienLuoc>>` |
| Kết quả khác nhau giữa hai lần chạy | `HashMap`, RNG chưa gieo hạt, hoặc `Instant::now()` | `BTreeMap` + hạt giống cố định + đồng hồ ảo |
| Vị thế vượt hạn mức dù cổng "đã kiểm" | Không đếm lệnh đang bay | Đặt chỗ phơi nhiễm lúc **phát**, không lúc giao |

---

## Tóm tắt chương & Bài tập rèn luyện

### 6 điểm cốt lõi

1. **Năm mảnh đúng riêng lẻ vẫn ghép thành một hệ sai.** Toàn bộ năm lỗi của chương này chỉ lộ ra ở mức tích hợp.
2. **Cổng rủi ro phải đặt chỗ, không chỉ kiểm tra.** Nhiều lệnh trong cùng một nhịp đều thấy cùng một trạng thái — mọi phép kiểm đều "OK" mà hạn mức vẫn vỡ.
3. **Đừng suy ra cái bạn đã biết.** Đoán chiều khớp từ giá là một dòng code trông vô hại làm hỏng toàn bộ quản trị rủi ro.
4. **Huỷ lệnh phải đi đường ưu tiên**, kể cả khi đã ngắt khẩn cấp — nếu không, bạn không có đường rút chân.
5. **Chênh lệch giá là phòng vệ theo khối lượng đã khớp**, không phải đặt cứng hai chân. Chân chắc chắn chỉ chạy sau khi chân không chắc đã trả lời.
6. **Một backtest đúng cơ học vẫn có thể sai kinh tế.** Hãy hỏi môi trường mô phỏng của bạn thiếu lực nào mà thị trường thật có.

### Bài tập rèn luyện

**Bài 1.** Thêm **bộ giám sát sức khoẻ** phát hiện hệ thống đang tự bóp cổ mình.

<details>
<summary><b>Gợi ý</b></summary>

Ba triệu chứng đã gặp trong chính chương này, và cả ba đều đo được **trước khi** gây thiệt hại: tỉ lệ bị cổng chặn tăng vọt (phơi nhiễm kẹt), tỉ lệ thụ động sụt (đang cắt qua sổ), và số lệnh treo phình ra (không rút báo giá). Giám sát chúng theo cửa sổ trượt để bắt được xu hướng chứ không chỉ mức tuyệt đối.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
#[derive(Debug, PartialEq)]
pub enum CanhBao {
    CongChanQuaNhieu { ty_le: f64 },
    ChuDongQuaNhieu { ty_le_thu_dong: f64 },
    LenhTreoPhinhTo { so_lenh: usize },
    KhongCoKhopNao,
}

pub struct GiamSatSucKhoe {
    pub nguong_chan: f64,
    pub nguong_thu_dong: f64,
    pub nguong_lenh_treo: usize,
}

impl GiamSatSucKhoe {
    pub fn dien_hinh() -> Self {
        GiamSatSucKhoe { nguong_chan: 0.5, nguong_thu_dong: 0.5, nguong_lenh_treo: 200 }
    }

    pub fn kiem(&self, h: &HeSinhThai) -> Vec<CanhBao> {
        let m = &h.do_luong;
        let mut ra = Vec::new();

        // Triệu chứng ③+④: phơi nhiễm kẹt, cổng chặn gần như mọi thứ.
        if m.so_y_dinh > 100 && m.ty_le_chan() > self.nguong_chan {
            ra.push(CanhBao::CongChanQuaNhieu { ty_le: m.ty_le_chan() });
        }
        // Triệu chứng ⑤: nhà tạo lập đang cắt qua sổ.
        if m.khoi_luong_khop > 0 && m.ty_le_thu_dong() < self.nguong_thu_dong {
            ra.push(CanhBao::ChuDongQuaNhieu { ty_le_thu_dong: m.ty_le_thu_dong() });
        }
        // Triệu chứng ③: báo giá không bao giờ được rút.
        let treo = h.san_tt.lenh_treo_cua_ta().len();
        if treo > self.nguong_lenh_treo {
            ra.push(CanhBao::LenhTreoPhinhTo { so_lenh: treo });
        }
        if m.so_lenh_gui > 100 && m.so_khop == 0 {
            ra.push(CanhBao::KhongCoKhopNao);
        }
        ra
    }
}
```

Điểm quan trọng: cả bốn cảnh báo đều dựa trên **tỉ lệ**, không phải số tuyệt đối. Số tuyệt đối phụ thuộc vào phiên; tỉ lệ thì so sánh được giữa các ngày, và đó mới là thứ dùng để đặt ngưỡng cảnh báo trong sản xuất.
</details>

**Bài 2.** Cài **chiến lược lấy tín hiệu từ sàn chuỗi khối để giao dịch sàn truyền thống** — dùng bể AMM làm chỉ báo dẫn dắt.

<details>
<summary><b>Gợi ý</b></summary>

Trên nhiều tài sản, một sàn "dẫn" và sàn kia "theo" — hiện tượng khám phá giá. Nếu bể AMM phản ứng trước, thì độ lệch giữa giá bể và giá giữa sàn truyền thống là dự báo cho bước tiếp theo của sàn truyền thống.

Điểm cần cẩn thận: đây không còn là chênh lệch giá phi rủi ro nữa mà là **cược có hướng**. Nó cần dừng lỗ và cần giới hạn thời gian giữ vị thế, thứ mà chiến lược chênh lệch có phòng vệ không cần.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct TheoDanBe {
    pub nguong_bp: f64,
    pub khoi_luong: SoLuong,
    pub han_muc: SoLuong,
    /// Thời điểm vào lệnh gần nhất — nền của giới hạn thời gian giữ.
    pub vao_luc: Option<Nano>,
    pub thoi_gian_giu_toi_da_ns: Nano,
}

impl TheoDanBe {
    pub fn moi(nguong_bp: f64, han_muc: SoLuong) -> Self {
        TheoDanBe {
            nguong_bp,
            khoi_luong: 10,
            han_muc,
            vao_luc: None,
            thoi_gian_giu_toi_da_ns: 50_000_000, // 50 ms
        }
    }
}

impl ChienLuoc for TheoDanBe {
    fn ten(&self) -> &str { "theo_dan_be" }

    fn danh_gia(&mut self, anh: &AnhChupThiTruong, vi_the: SoLuong) -> Vec<YDinh> {
        // Hết thời gian giữ → thoát, bất kể lãi hay lỗ. Một cược có hướng
        // không có giới hạn thời gian sẽ biến thành khoản đầu tư dài hạn
        // ngoài ý muốn.
        if let Some(t0) = self.vao_luc {
            if anh.thoi_diem.saturating_sub(t0) > self.thoi_gian_giu_toi_da_ns && vi_the != 0 {
                self.vao_luc = None;
                let (chieu, muc) = if vi_the > 0 {
                    (Chieu::Ban, anh.tt_mua)
                } else {
                    (Chieu::Mua, anh.tt_ban)
                };
                if let Some(m) = muc {
                    return vec![YDinh::DatLenh {
                        san: San::TruyenThong,
                        chieu,
                        gia: m.gia,
                        khoi_luong: vi_the.abs().min(m.khoi_luong),
                    }];
                }
            }
        }

        let cl = match anh.chenh_lech_hai_san_bp() { Some(x) => x, None => return Vec::new() };
        if cl.abs() < self.nguong_bp { return Vec::new(); }

        // Bể đắt hơn → dự báo sàn truyền thống sẽ đi LÊN → mua.
        let (chieu, muc) = if cl > 0.0 { (Chieu::Mua, anh.tt_ban) } else { (Chieu::Ban, anh.tt_mua) };
        if (chieu == Chieu::Mua && vi_the >= self.han_muc)
            || (chieu == Chieu::Ban && vi_the <= -self.han_muc)
        {
            return Vec::new();
        }
        match muc {
            Some(m) => {
                self.vao_luc = Some(anh.thoi_diem);
                vec![YDinh::DatLenh {
                    san: San::TruyenThong,
                    chieu,
                    gia: m.gia,
                    khoi_luong: self.khoi_luong.min(m.khoi_luong),
                }]
            }
            None => Vec::new(),
        }
    }
}
```

Khác biệt then chốt so với `ChenhLechHaiSan`: chiến lược này **không phòng vệ**, nên nó mang rủi ro hướng thật. Bù lại nó phải có hạn mức vị thế riêng và giới hạn thời gian giữ. Đó là đánh đổi cơ bản — bỏ phòng vệ để lấy kỳ vọng lợi nhuận cao hơn, và trả bằng rủi ro phải quản lý bằng tay.
</details>
