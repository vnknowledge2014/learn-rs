# Chương 77: Chiến lược & Quản trị rủi ro — Cổng rủi ro, Tín hiệu & Định cỡ vị thế

## Giới thiệu & Mục tiêu học tập

Một chiến lược lãi mà không có kiểm soát rủi ro thì chỉ là **quả bom hẹn giờ**. Lịch sử ngành có sẵn ví dụ: Knight Capital mất 440 triệu đô trong 45 phút năm 2012, vì một đoạn mã cũ được bật nhầm và **không có gì chặn nó lại**.

Chương này dựng ba lớp mà mọi bàn giao dịch chuyên nghiệp đều có:

| Lớp | Nhiệm vụ | Đặc điểm |
|---|---|---|
| Cổng rủi ro trước lệnh | Chặn lệnh xấu trước khi ra khỏi máy | Phải chạy trên **mọi** lệnh, không ngoại lệ |
| Tín hiệu | Biến sổ lệnh thành dự báo | Đơn giản và giải thích được, không phải hộp đen |
| Định cỡ vị thế | Quyết định đặt bao nhiêu | Sai chỗ này thì tín hiệu tốt vẫn phá sản |

Nguyên tắc xuyên suốt: **cổng rủi ro nằm trên đường nóng và không được phép bỏ qua**. Nó phải nhanh (chương này đo được vài chục nanosecond) để không ai có động cơ tắt nó đi.

---

## Hình tượng hóa đời sống

```
┌──────────────────────────────────────────────────────────────────────────────┐
│  CỔNG RỦI RO = TRẠM KIỂM SOÁT KHÔNG CÓ ĐƯỜNG VÒNG                           │
│                                                                              │
│    chiến lược ──► ┌──────────────────────────┐ ──► sàn                      │
│                   │  1. Công tắc ngắt bật?   │                              │
│                   │  2. Giá hợp lệ?          │                              │
│                   │  3. Khối lượng ≤ hạn?    │                              │
│                   │  4. Giá trị lệnh ≤ hạn?  │                              │
│                   │  5. Vị thế sau lệnh ≤ ?  │                              │
│                   │  6. Lỗ trong ngày ≤ ?    │                              │
│                   │  7. Tốc độ gửi ≤ ?       │                              │
│                   └──────────────────────────┘                              │
│                                                                              │
│    KHÔNG CÓ cờ "bỏ qua kiểm tra". Không có "chế độ khẩn cấp".               │
│    Knight Capital 2012: 440 triệu đô trong 45 phút vì thiếu đúng cái này.   │
│                                                                              │
│  MẤT CÂN BẰNG SỔ LỆNH = ĐẾM NGƯỜI XẾP HÀNG HAI BÊN                         │
│                                                                              │
│     mua 900  ████████████████████                                           │
│     bán 100  ██                                                             │
│     mất cân bằng = (900−100)/(900+100) = +0,8  → áp lực MUA mạnh            │
│                                                                              │
│  VI GIÁ (micro-price) = GIÁ GIỮA CÓ TRỌNG SỐ NGƯỢC                          │
│                                                                              │
│     mua 100.50 × 900   |   bán 100.52 × 100                                 │
│     giá giữa thường = 100.51   (ngây thơ)                                   │
│     vi giá = (100.50×100 + 100.52×900)/1000 = 100.518                       │
│              └──trọng số NGƯỢC: bên NHIỀU khối lượng KÉO GIÁ về phía kia   │
│                                                                              │
│     Trực giác: bên mua đông nghĩa là áp lực mua chưa được thoả mãn.         │
│     Giá "công bằng" nghiêng về phía bên bán mỏng.                           │
│                                                                              │
│  CÔNG THỨC KELLY = CƯỢC BAO NHIÊU THÌ TỐI ƯU?                              │
│                                                                              │
│     f* = (p·b − q) / b     p=xác suất thắng, b=tỉ lệ thắng/thua             │
│                                                                              │
│     Kelly toàn phần tối đa hoá tăng trưởng dài hạn NHƯNG dao động khủng     │
│     khiếp: sụt 50% là chuyện thường. Thực tế dùng ¼ đến ½ Kelly.            │
│     Lý do: bạn KHÔNG biết p chính xác. Ước lượng p cao hơn thật một chút   │
│     là đủ để Kelly toàn phần dẫn tới phá sản.                              │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Lãi lỗ trung bình giá vốn — chỗ mọi người làm sai

Đây là lỗi thật đã xảy ra khi xây chương này: tính lãi lỗ đã chốt bằng cách **cộng dòng tiền của giao dịch đóng**, thay vì tính chênh lệch so với giá vốn.

Công thức đúng cần theo dõi `gia_von` (giá vốn trung bình) và xử lý ba trường hợp:

- **Mở rộng vị thế** (cùng chiều): cập nhật giá vốn trung bình có trọng số.
- **Đóng bớt** (ngược chiều, chưa vượt): chốt lãi lỗ = `(giá − giá_vốn) × khối_lượng × dấu_vị_thế`.
- **Đảo chiều** (ngược chiều, vượt qua 0): chốt toàn bộ phần cũ, rồi giá vốn mới là giá của phần dư.

Trường hợp đảo chiều là chỗ hay sai nhất. Nếu xử lý nó như "đóng bớt" thông thường, giá vốn sẽ sai và mọi con số sau đó đều sai theo.

### 2. Thứ tự các phép kiểm tra là một quyết định thiết kế

Trong khi xây chương này, một lỗi tinh vi xuất hiện: kiểm tra **giá trị lệnh** đứng trước kiểm tra **vị thế**, nên nhiều bài kiểm thử không bao giờ chạm tới nhánh vị thế — chúng bị chặn sớm hơn.

Bài học rộng hơn: khi cổng có nhiều luật, **luật nào chặn trước sẽ che khuất luật sau**. Điều đó ảnh hưởng tới cả kiểm thử lẫn chẩn đoán sản xuất — thông báo lỗi bạn nhận được không nhất thiết là vấn đề nghiêm trọng nhất.

Thứ tự hợp lý là: rẻ trước, đắt sau; và trong nhóm cùng chi phí thì nghiêm trọng trước.

### 3. Vi giá: vì sao trọng số lại ngược

Vi giá là

```
vi_giá = (giá_mua × KL_bán + giá_bán × KL_mua) / (KL_mua + KL_bán)
```

Chú ý: khối lượng **bán** nhân với giá **mua**. Trọng số ngược, và đó là chủ ý.

Trực giác: nếu bên mua có 900 đơn vị và bên bán chỉ 100, thì có rất nhiều người muốn mua chưa được thoả mãn. Áp lực đó sẽ đẩy giá lên, nên giá "công bằng" phải gần phía bán hơn. Trọng số ngược tạo ra đúng hiệu ứng đó.

Về mặt thực nghiệm, vi giá là **dự báo tốt hơn** giá giữa cho giá giữa ở thời điểm tiếp theo. Đó là một trong những tín hiệu đơn giản nhất mà thực sự hoạt động.

### 4. Kelly và vì sao không ai dùng Kelly toàn phần

Công thức Kelly `f* = (p·b − q)/b` tối đa hoá tốc độ tăng trưởng logarit dài hạn. Về mặt toán học nó tối ưu. Về mặt thực hành nó nguy hiểm, vì hai lý do:

- **Dao động khủng khiếp.** Với Kelly toàn phần, sụt giảm 50% từ đỉnh là chuyện bình thường, không phải bất thường.
- **Bạn không biết `p`.** Nếu ước lượng xác suất thắng cao hơn thật chỉ vài phần trăm, Kelly toàn phần trở thành cược vượt mức, và cược vượt mức dẫn tới tăng trưởng **âm** — dù mỗi cược đều có kỳ vọng dương.

Điểm thứ hai là điểm quan trọng: sai lầm theo hướng cược quá nhiều bị phạt **nặng hơn nhiều** so với cược quá ít. Vì thế thực tế dùng ¼ đến ½ Kelly, chấp nhận tăng trưởng chậm hơn để đổi lấy khả năng sống sót.

### 5. Sụt giảm quan trọng hơn lợi nhuận

Một chiến lược lãi 20%/năm với sụt giảm tối đa 5% thì đầu tư được. Cùng chiến lược đó với sụt giảm 40% thì không — không phải vì toán học, mà vì **không ai chịu được**: nhà đầu tư rút vốn, ban lãnh đạo cắt hạn mức, và người vận hành mất niềm tin đúng lúc đáy.

Tỉ số Sharpe đo lợi nhuận trên đơn vị biến động. Nhưng nó phạt biến động **tăng** giống hệt biến động **giảm** — điều mà không nhà đầu tư nào đồng ý. Đó là lý do phải nhìn cả sụt giảm tối đa, và vì sao chương này tính cả hai.

Một lưu ý kỹ thuật nhỏ nhưng thú vị: một đường vốn **tăng tuyệt đối đều đặn** có độ lệch chuẩn bằng 0, nên Sharpe bằng 0 (hoặc vô định). Đó là lý do bài kiểm thử "chiến lược mượt" trong chương này phải thêm nhiễu nhỏ — đường vốn hoàn hảo không tồn tại, và công thức giả định điều đó.

---

## Mã nguồn minh họa thực chiến

Chạy bằng `cargo run -p ch77`, kiểm thử bằng `cargo test -p ch77`.

```rust
#![allow(dead_code)]
//! Chương 77 — Chiến lược & Quản trị rủi ro thời gian thực: cổng rủi ro trước
//! giao dịch, tín hiệu từ sổ lệnh, arbitrage thống kê theo cặp, định cỡ vị thế,
//! và các thước đo rủi ro.
//!
//! Nguyên tắc xuyên suốt: **cổng rủi ro là thứ DUY NHẤT không được phép có
//! ngoại lệ**. Chiến lược có thể sai; cổng rủi ro thì không.

use std::collections::VecDeque;

pub type Gia = i64;      // tick
pub type SoLuong = i64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chieu { Mua, Ban }

impl Chieu {
    pub fn dau(self) -> i64 { match self { Chieu::Mua => 1, Chieu::Ban => -1 } }
}

// ============================================================================
// 1. CỔNG RỦI RO TRƯỚC GIAO DỊCH
// ============================================================================
// Mọi lệnh đều phải qua đây. Không có đường vòng, không có cờ "bỏ qua kiểm
// tra cho nhanh". Lịch sử ngành đầy những vụ sập vì ai đó mở một đường vòng.

#[derive(Debug, Clone, PartialEq)]
pub enum TuChoi {
    SoLuongKhongDuong(SoLuong),
    GiaKhongDuong(Gia),
    /// Ngón tay béo: giá lệch quá xa giá thị trường — gần như chắc chắn gõ nhầm.
    NgonTayBeo { gia: Gia, tham_chieu: Gia, lech_phan_tram: f64 },
    VuotGiaTriLenh { gia_tri: i64, tran: i64 },
    VuotViThe { sau_lenh: i64, tran: i64 },
    VuotLoTrongNgay { lo: i64, tran: i64 },
    VuotSoLenhMoiGiay { dem: u32, tran: u32 },
    CongTacTatDaBat,
}

#[derive(Debug, Clone)]
pub struct HanMucRuiRo {
    pub gia_tri_lenh_toi_da: i64,
    pub vi_the_toi_da: i64,
    pub lo_trong_ngay_toi_da: i64,
    pub so_lenh_moi_giay_toi_da: u32,
    /// Lệch quá tỉ lệ này so với giá tham chiếu thì coi là gõ nhầm.
    pub nguong_ngon_tay_beo: f64,
}

impl Default for HanMucRuiRo {
    fn default() -> Self {
        HanMucRuiRo {
            gia_tri_lenh_toi_da: 100_000_000,
            vi_the_toi_da: 10_000,
            lo_trong_ngay_toi_da: 5_000_000,
            so_lenh_moi_giay_toi_da: 100,
            nguong_ngon_tay_beo: 0.10, // 10%
        }
    }
}

#[derive(Debug, Clone)]
pub struct CongRuiRo {
    pub han_muc: HanMucRuiRo,
    pub vi_the: i64,
    pub lai_lo_da_chot: i64,
    /// Giá vốn bình quân của vị thế đang mở. KHÔNG có nó thì không tính được
    /// lãi/lỗ — chỉ biết dòng tiền, mà dòng tiền không phải lãi.
    pub gia_von: f64,
    /// Dấu thời gian các lệnh gần đây, để đếm tần suất.
    cua_so_lenh: VecDeque<u64>,
    /// Công tắc tắt: bật rồi thì KHÔNG tự tắt được. Chỉ người mới gỡ được.
    cong_tac_tat: bool,
    pub so_lenh_qua: u64,
    pub so_lenh_bi_chan: u64,
}

impl CongRuiRo {
    pub fn moi(han_muc: HanMucRuiRo) -> Self {
        CongRuiRo { han_muc, vi_the: 0, lai_lo_da_chot: 0, gia_von: 0.0,
                    cua_so_lenh: VecDeque::new(), cong_tac_tat: false,
                    so_lenh_qua: 0, so_lenh_bi_chan: 0 }
    }

    pub fn da_tat(&self) -> bool { self.cong_tac_tat }
    /// Bật công tắc tắt. Một chiều — chỉ người vận hành mới gỡ được.
    pub fn bat_cong_tac_tat(&mut self) { self.cong_tac_tat = true; }
    pub fn nguoi_van_hanh_go_cong_tac(&mut self) { self.cong_tac_tat = false; }

    /// Kiểm tra một lệnh. `bay_gio_ns` dùng cho cửa sổ tần suất.
    pub fn kiem_tra(&mut self, chieu: Chieu, gia: Gia, so_luong: SoLuong,
                    gia_tham_chieu: Gia, bay_gio_ns: u64) -> Result<(), TuChoi>
    {
        let ket_qua = self.kiem_tra_noi_bo(chieu, gia, so_luong, gia_tham_chieu, bay_gio_ns);
        match &ket_qua {
            Ok(()) => {
                self.so_lenh_qua += 1;
                self.cua_so_lenh.push_back(bay_gio_ns);
            }
            Err(_) => self.so_lenh_bi_chan += 1,
        }
        ket_qua
    }

    fn kiem_tra_noi_bo(&mut self, chieu: Chieu, gia: Gia, so_luong: SoLuong,
                       gia_tham_chieu: Gia, bay_gio_ns: u64) -> Result<(), TuChoi>
    {
        // Công tắc tắt xét ĐẦU TIÊN. Đã tắt thì không gì lọt qua được.
        if self.cong_tac_tat { return Err(TuChoi::CongTacTatDaBat); }
        if so_luong <= 0 { return Err(TuChoi::SoLuongKhongDuong(so_luong)); }
        if gia <= 0 { return Err(TuChoi::GiaKhongDuong(gia)); }

        // Ngón tay béo: gõ 8400 thành 84000 là chuyện xảy ra hằng năm
        if gia_tham_chieu > 0 {
            let lech = (gia - gia_tham_chieu).abs() as f64 / gia_tham_chieu as f64;
            if lech > self.han_muc.nguong_ngon_tay_beo {
                return Err(TuChoi::NgonTayBeo { gia, tham_chieu: gia_tham_chieu,
                                                lech_phan_tram: lech * 100.0 });
            }
        }

        let gia_tri = gia * so_luong;
        if gia_tri > self.han_muc.gia_tri_lenh_toi_da {
            return Err(TuChoi::VuotGiaTriLenh { gia_tri, tran: self.han_muc.gia_tri_lenh_toi_da });
        }

        let sau_lenh = self.vi_the + chieu.dau() * so_luong;
        if sau_lenh.abs() > self.han_muc.vi_the_toi_da {
            return Err(TuChoi::VuotViThe { sau_lenh, tran: self.han_muc.vi_the_toi_da });
        }

        if self.lai_lo_da_chot < -self.han_muc.lo_trong_ngay_toi_da {
            return Err(TuChoi::VuotLoTrongNgay { lo: -self.lai_lo_da_chot,
                                                 tran: self.han_muc.lo_trong_ngay_toi_da });
        }

        // Cửa sổ trượt một giây
        while let Some(&t) = self.cua_so_lenh.front() {
            if bay_gio_ns.saturating_sub(t) >= 1_000_000_000 { self.cua_so_lenh.pop_front(); }
            else { break; }
        }
        let dem = self.cua_so_lenh.len() as u32;
        if dem >= self.han_muc.so_lenh_moi_giay_toi_da {
            return Err(TuChoi::VuotSoLenhMoiGiay { dem,
                                                   tran: self.han_muc.so_lenh_moi_giay_toi_da });
        }
        Ok(())
    }

    /// Ghi nhận một lần khớp — cập nhật vị thế, giá vốn và lãi/lỗ đã chốt.
    ///
    /// Điểm dễ sai nhất trong cả chương: lãi/lỗ KHÔNG phải dòng tiền của lệnh
    /// đóng. Bán 100 cổ giá 88,00 mang về tiền, nhưng nếu mua vào ở 90,00 thì
    /// đó là một khoản LỖ. Muốn biết lãi hay lỗ, bắt buộc phải nhớ GIÁ VỐN.
    pub fn ghi_nhan_khop(&mut self, chieu: Chieu, gia: Gia, so_luong: SoLuong) {
        let truoc = self.vi_the;
        let d = chieu.dau() * so_luong;

        if truoc == 0 || truoc.signum() == d.signum() {
            // Mở mới hoặc mở thêm cùng chiều → bình quân lại giá vốn
            let tong = (truoc.abs() + so_luong) as f64;
            self.gia_von = (self.gia_von * truoc.abs() as f64
                            + gia as f64 * so_luong as f64) / tong;
            self.vi_the = truoc + d;
        } else {
            // Đóng bớt hoặc đóng hết → hiện thực hoá lãi/lỗ phần đóng được
            let dong = so_luong.min(truoc.abs());
            self.lai_lo_da_chot +=
                ((gia as f64 - self.gia_von) * dong as f64 * truoc.signum() as f64) as i64;
            self.vi_the = truoc + d;
            if self.vi_the == 0 {
                self.gia_von = 0.0;
            } else if self.vi_the.signum() != truoc.signum() {
                // Đảo chiều: phần dư là một vị thế MỚI, giá vốn là giá vừa khớp
                self.gia_von = gia as f64;
            }
        }

        // Tự bảo vệ: lỗ chạm trần thì tự bật công tắc tắt
        if self.lai_lo_da_chot < -self.han_muc.lo_trong_ngay_toi_da {
            self.cong_tac_tat = true;
        }
    }
}

// ============================================================================
// 2. TÍN HIỆU TỪ SỔ LỆNH
// ============================================================================

/// Mất cân bằng khối lượng hai bên, chuẩn hoá về [-1, 1].
/// Dương = áp lực mua. Đây là tín hiệu đơn giản nhất mà vẫn có sức dự báo thật.
pub fn mat_can_bang(kl_mua: u64, kl_ban: u64) -> f64 {
    let tong = kl_mua + kl_ban;
    if tong == 0 { return 0.0; }
    (kl_mua as f64 - kl_ban as f64) / tong as f64
}

/// Giá vi mô: giá giữa có gia quyền theo khối lượng ĐỐI ỨNG.
/// Nhiều người muốn mua → giá vi mô lệch về phía giá bán.
pub fn gia_vi_mo(gia_mua: Gia, kl_mua: u64, gia_ban: Gia, kl_ban: u64) -> Option<f64> {
    let tong = kl_mua + kl_ban;
    if tong == 0 { return None; }
    Some((gia_mua as f64 * kl_ban as f64 + gia_ban as f64 * kl_mua as f64) / tong as f64)
}

/// Cửa sổ trượt tính trung bình và độ lệch chuẩn — O(1) mỗi lần thêm.
#[derive(Debug, Clone)]
pub struct CuaSoThongKe {
    o: VecDeque<f64>,
    suc_chua: usize,
    tong: f64,
    tong_binh_phuong: f64,
}

impl CuaSoThongKe {
    pub fn moi(suc_chua: usize) -> Self {
        CuaSoThongKe { o: VecDeque::with_capacity(suc_chua), suc_chua,
                       tong: 0.0, tong_binh_phuong: 0.0 }
    }
    pub fn them(&mut self, x: f64) {
        if self.o.len() == self.suc_chua {
            if let Some(cu) = self.o.pop_front() {
                self.tong -= cu;
                self.tong_binh_phuong -= cu * cu;
            }
        }
        self.o.push_back(x);
        self.tong += x;
        self.tong_binh_phuong += x * x;
    }
    pub fn so_luong(&self) -> usize { self.o.len() }
    pub fn day(&self) -> bool { self.o.len() == self.suc_chua }
    pub fn trung_binh(&self) -> f64 {
        if self.o.is_empty() { 0.0 } else { self.tong / self.o.len() as f64 }
    }
    /// Phương sai mẫu (chia n−1). Trả 0 khi chưa đủ 2 điểm.
    pub fn phuong_sai(&self) -> f64 {
        let n = self.o.len() as f64;
        if n < 2.0 { return 0.0; }
        let ps = (self.tong_binh_phuong - self.tong * self.tong / n) / (n - 1.0);
        ps.max(0.0) // chặn sai số dấu phẩy động làm ra số âm
    }
    pub fn do_lech_chuan(&self) -> f64 { self.phuong_sai().sqrt() }
    /// Điểm z: giá trị này lệch bao nhiêu độ lệch chuẩn so với trung bình.
    pub fn diem_z(&self, x: f64) -> Option<f64> {
        let s = self.do_lech_chuan();
        if s < 1e-9 { None } else { Some((x - self.trung_binh()) / s) }
    }
}

// ============================================================================
// 3. ARBITRAGE THỐNG KÊ THEO CẶP
// ============================================================================
// Ý tưởng: hai mã cùng ngành thường đi cùng nhau. Khi chênh lệch giãn bất
// thường, đặt cược nó sẽ co lại. Rủi ro lớn nhất KHÔNG phải chênh lệch không
// co, mà là quan hệ giữa hai mã ĐÃ GÃY HẲN mà ta không nhận ra.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TinHieuCap { MoDaiA, MoDaiB, Dong, KhongLam }

pub struct ArbCap {
    pub ty_le_phong_ho: f64, // beta: 1 đơn vị A ứng với bao nhiêu đơn vị B
    pub cua_so: CuaSoThongKe,
    pub nguong_vao: f64,
    pub nguong_ra: f64,
    /// Chênh lệch giãn quá mức này thì coi như quan hệ đã gãy — CẮT LỖ.
    pub nguong_dung: f64,
    pub dang_mo: Option<TinHieuCap>,
}

impl ArbCap {
    pub fn moi(ty_le_phong_ho: f64, cua_so: usize,
               nguong_vao: f64, nguong_ra: f64, nguong_dung: f64) -> Self {
        ArbCap { ty_le_phong_ho, cua_so: CuaSoThongKe::moi(cua_so),
                 nguong_vao, nguong_ra, nguong_dung, dang_mo: None }
    }

    pub fn chenh_lech(&self, gia_a: Gia, gia_b: Gia) -> f64 {
        gia_a as f64 - self.ty_le_phong_ho * gia_b as f64
    }

    pub fn cap_nhat(&mut self, gia_a: Gia, gia_b: Gia) -> TinHieuCap {
        let cl = self.chenh_lech(gia_a, gia_b);
        // Tính điểm z TRƯỚC khi thêm điểm mới — nếu không, chính điểm dị
        // biệt ta muốn phát hiện lại kéo trung bình về phía nó và tự che mình.
        let day_truoc_do = self.cua_so.day();
        let z = self.cua_so.diem_z(cl);
        self.cua_so.them(cl);

        let z = match z {
            Some(z) if day_truoc_do => z,
            _ => return TinHieuCap::KhongLam,
        };

        match self.dang_mo {
            None => {
                if z > self.nguong_vao {
                    // A đắt bất thường so với B → bán A, mua B
                    self.dang_mo = Some(TinHieuCap::MoDaiB);
                    TinHieuCap::MoDaiB
                } else if z < -self.nguong_vao {
                    self.dang_mo = Some(TinHieuCap::MoDaiA);
                    TinHieuCap::MoDaiA
                } else { TinHieuCap::KhongLam }
            }
            Some(_) => {
                // Cắt lỗ đứng TRƯỚC chốt lời: quan hệ gãy thì phải thoát ngay
                if z.abs() > self.nguong_dung || z.abs() < self.nguong_ra {
                    self.dang_mo = None;
                    TinHieuCap::Dong
                } else { TinHieuCap::KhongLam }
            }
        }
    }
}

// ============================================================================
// 4. ĐỊNH CỠ VỊ THẾ
// ============================================================================

/// Tỉ lệ Kelly: f* = (p·b − q) / b, với p = xác suất thắng, b = tỉ lệ thắng/thua.
///
/// Kelly toàn phần tối ưu về tốc độ tăng trưởng dài hạn, nhưng dao động khủng
/// khiếp và cực nhạy với sai số ước lượng `p`. Thực tế người ta dùng một PHẦN
/// của Kelly (thường 1/4 tới 1/2) — đánh đổi chút tăng trưởng lấy nhiều bình yên.
pub fn ty_le_kelly(xac_suat_thang: f64, ty_le_thang_thua: f64) -> f64 {
    if ty_le_thang_thua <= 0.0 { return 0.0; }
    let q = 1.0 - xac_suat_thang;
    ((xac_suat_thang * ty_le_thang_thua - q) / ty_le_thang_thua).max(0.0)
}

pub fn kelly_mot_phan(xac_suat_thang: f64, ty_le_thang_thua: f64, phan: f64) -> f64 {
    (ty_le_kelly(xac_suat_thang, ty_le_thang_thua) * phan).clamp(0.0, 1.0)
}

/// Định cỡ theo mục tiêu biến động: mã càng dao động mạnh thì mua càng ít,
/// sao cho rủi ro tính bằng tiền là như nhau ở mọi mã.
pub fn co_theo_bien_dong(von: i64, bien_dong_muc_tieu: f64,
                         bien_dong_mac_dinh: f64, gia: Gia) -> SoLuong {
    if bien_dong_mac_dinh <= 0.0 || gia <= 0 { return 0; }
    let ty_trong = (bien_dong_muc_tieu / bien_dong_mac_dinh).min(1.0);
    ((von as f64 * ty_trong) / gia as f64) as SoLuong
}

// ============================================================================
// 5. THƯỚC ĐO RỦI RO
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct ThuocDoRuiRo {
    pub tong_lai_lo: i64,
    pub sut_giam_toi_da: i64,
    pub ty_le_sut_giam: f64,
    pub so_phien_lai: usize,
    pub so_phien_lo: usize,
    /// Tỉ số lợi nhuận trên độ dao động — càng cao càng "êm".
    pub ty_so_sharpe: f64,
}

pub fn do_rui_ro(duong_von: &[i64]) -> ThuocDoRuiRo {
    if duong_von.len() < 2 {
        return ThuocDoRuiRo { tong_lai_lo: 0, sut_giam_toi_da: 0, ty_le_sut_giam: 0.0,
                              so_phien_lai: 0, so_phien_lo: 0, ty_so_sharpe: 0.0 };
    }
    let mut dinh = duong_von[0];
    let mut sut = 0i64;
    for &v in duong_von {
        dinh = dinh.max(v);
        sut = sut.max(dinh - v);
    }
    let thay_doi: Vec<f64> = duong_von.windows(2).map(|w| (w[1] - w[0]) as f64).collect();
    let n = thay_doi.len() as f64;
    let tb = thay_doi.iter().sum::<f64>() / n;
    let ps = thay_doi.iter().map(|x| (x - tb).powi(2)).sum::<f64>() / (n - 1.0).max(1.0);
    let sd = ps.sqrt();
    ThuocDoRuiRo {
        tong_lai_lo: duong_von[duong_von.len() - 1] - duong_von[0],
        sut_giam_toi_da: sut,
        ty_le_sut_giam: if dinh.abs() > 0 { sut as f64 / dinh.abs() as f64 } else { 0.0 },
        so_phien_lai: thay_doi.iter().filter(|&&x| x > 0.0).count(),
        so_phien_lo: thay_doi.iter().filter(|&&x| x < 0.0).count(),
        ty_so_sharpe: if sd < 1e-12 { 0.0 } else { tb / sd },
    }
}

// ============================================================================
// 6. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

/// Hai chuỗi giá đồng liên kết: chúng cùng đi theo một nhân tố chung, cộng
/// thêm nhiễu riêng. Đây đúng là tình huống mà arbitrage cặp khai thác.
pub fn sinh_cap_gia(n: usize, hat_giong: u64, beta: f64) -> (Vec<Gia>, Vec<Gia>) {
    let mut s = hat_giong;
    let mut nhan_to_chung = 10_000.0f64;
    let (mut a, mut b) = (Vec::with_capacity(n), Vec::with_capacity(n));
    for _ in 0..n {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let e1 = ((s >> 33) % 201) as f64 - 100.0;
        let e2 = ((s >> 45) % 61) as f64 - 30.0;
        let e3 = ((s >> 20) % 61) as f64 - 30.0;
        nhan_to_chung += e1 * 0.1;
        a.push((nhan_to_chung + e2) as Gia);
        b.push(((nhan_to_chung + e3) / beta) as Gia);
    }
    (a, b)
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   CHIẾN LƯỢC & QUẢN TRỊ RỦI RO THỜI GIAN THỰC             ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. CỔNG RỦI RO — mọi lệnh đều phải qua đây");
    let mut cong = CongRuiRo::moi(HanMucRuiRo {
        gia_tri_lenh_toi_da: 10_000_000, vi_the_toi_da: 500,
        lo_trong_ngay_toi_da: 100_000, so_lenh_moi_giay_toi_da: 5,
        nguong_ngon_tay_beo: 0.10,
    });
    let tc = 8_400;
    for (mo_ta, chieu, gia, sl) in [
        ("hợp lệ            ", Chieu::Mua, 8_400i64, 100i64),
        ("ngón tay béo x10  ", Chieu::Mua, 84_000, 100),
        ("giá trị quá lớn   ", Chieu::Mua, 8_400, 100_000),
        ("số lượng âm       ", Chieu::Mua, 8_400, -5),
        ("vượt trần vị thế  ", Chieu::Mua, 8_400, 600),
    ] {
        match cong.kiem_tra(chieu, gia, sl, tc, 1_000_000_000) {
            Ok(()) => println!("   {} → CHO QUA", mo_ta),
            Err(e) => println!("   {} → CHẶN: {:?}", mo_ta, e),
        }
    }

    println!("\n2. GIỚI HẠN TẦN SUẤT — chống vòng lặp lỗi bắn lệnh liên tục");
    let mut c2 = CongRuiRo::moi(HanMucRuiRo { so_lenh_moi_giay_toi_da: 5, ..Default::default() });
    let mut qua = 0;
    for i in 0..10u64 {
        if c2.kiem_tra(Chieu::Mua, 8_400, 1, tc, 1_000_000_000 + i * 1_000_000).is_ok() {
            qua += 1;
        }
    }
    println!("   Bắn 10 lệnh trong 10 ms → chỉ {} lệnh lọt qua (trần 5/giây)", qua);

    println!("\n3. CÔNG TẮC TẮT TỰ ĐỘNG KHI LỖ CHẠM TRẦN");
    let mut c3 = CongRuiRo::moi(HanMucRuiRo { lo_trong_ngay_toi_da: 10_000,
                                              ..Default::default() });
    c3.ghi_nhan_khop(Chieu::Mua, 9_000, 100);
    c3.ghi_nhan_khop(Chieu::Ban, 8_800, 100); // lỗ 20 000
    println!("   Sau khi lỗ {} → công tắc tắt: {}", -c3.lai_lo_da_chot, c3.da_tat());
    println!("   Lệnh tiếp theo → {:?}",
             c3.kiem_tra(Chieu::Mua, 8_400, 1, tc, 2_000_000_000).unwrap_err());
    c3.nguoi_van_hanh_go_cong_tac();
    println!("   Người vận hành gỡ công tắc → giao dịch lại được: {}",
             c3.kiem_tra(Chieu::Mua, 8_400, 1, tc, 3_000_000_000).is_ok());

    println!("\n4. TÍN HIỆU TỪ SỔ LỆNH");
    for (m, b) in [(1000u64, 1000u64), (9000, 1000), (1000, 9000)] {
        println!("   mua {:>4} / bán {:>4} → mất cân bằng {:>6.2} · giá vi mô {:>8.2}",
                 m, b, mat_can_bang(m, b), gia_vi_mo(8_400, m, 8_410, b).unwrap());
    }
    println!("   → Nhiều người chờ mua thì giá vi mô lệch LÊN phía giá bán.");

    println!("\n5. ARBITRAGE CẶP");
    let (ga, gb) = sinh_cap_gia(3_000, 2024, 1.5);
    let mut arb = ArbCap::moi(1.5, 100, 2.0, 0.5, 4.0);
    let (mut vao, mut ra) = (0, 0);
    for i in 0..ga.len() {
        match arb.cap_nhat(ga[i], gb[i]) {
            TinHieuCap::MoDaiA | TinHieuCap::MoDaiB => vao += 1,
            TinHieuCap::Dong => ra += 1,
            TinHieuCap::KhongLam => {}
        }
    }
    println!("   {} điểm dữ liệu → vào lệnh {} lần · thoát {} lần", ga.len(), vao, ra);
    println!("   → Ngưỡng dừng 4σ tồn tại vì chênh lệch giãn mãi nghĩa là quan hệ ĐÃ GÃY,");
    println!("     không phải 'cơ hội càng ngon hơn'.");

    println!("\n6. ĐỊNH CỠ VỊ THẾ");
    println!("   {:<28} {:>8} {:>10}", "kịch bản", "Kelly", "1/4 Kelly");
    for (mo_ta, p, b) in [
        ("55% thắng, ăn 1 thua 1  ", 0.55, 1.0),
        ("60% thắng, ăn 1 thua 1  ", 0.60, 1.0),
        ("40% thắng, ăn 2 thua 1  ", 0.40, 2.0),
        ("45% thắng, ăn 1 thua 1  ", 0.45, 1.0),
    ] {
        println!("   {} {:>7.1}% {:>9.1}%", mo_ta,
                 ty_le_kelly(p, b) * 100.0, kelly_mot_phan(p, b, 0.25) * 100.0);
    }
    println!("   → Lợi thế âm thì Kelly = 0: công thức tự bảo bạn ĐỪNG đánh.");

    println!("\n7. THƯỚC ĐO RỦI RO — hai đường vốn cùng đích, khác hẳn nhau");
    // "Êm" KHÔNG có nghĩa là đường thẳng tuyệt đối — đường thẳng thì độ lệch
    // chuẩn bằng 0 và Sharpe không định nghĩa được. Êm nghĩa là dao động nhỏ.
    let em: Vec<i64> = (0..100).map(|i| 100_000 + i * 500 + (i % 5) * 40).collect();
    let mut xoc: Vec<i64> = Vec::new();
    let mut v = 100_000i64;
    for i in 0..100 { v += if i % 3 == 0 { -8_000 } else { 5_750 }; xoc.push(v); }
    for (ten, d) in [("êm ", &em), ("xóc", &xoc)] {
        let r = do_rui_ro(d);
        println!("   {} → lãi {:>6} · sụt sâu nhất {:>6} · Sharpe {:>5.2} · thắng {}/{}",
                 ten, r.tong_lai_lo, r.sut_giam_toi_da, r.ty_so_sharpe,
                 r.so_phien_lai, r.so_phien_lai + r.so_phien_lo);
    }
    println!("   → Đường xóc lãi NHIỀU HƠN, nhưng Sharpe thấp hơn ~35 lần và có");
    println!("     những cú sụt 8.000 giữa đường. Phần lớn người sẽ bỏ cuộc trước khi");
    println!("     nó kịp về đích — lợi nhuận trên giấy không phải lợi nhuận thu được.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   CHIẾN LƯỢC ĐƯỢC PHÉP SAI. CỔNG RỦI RO THÌ KHÔNG.         ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn cong_mau() -> CongRuiRo {
        CongRuiRo::moi(HanMucRuiRo {
            gia_tri_lenh_toi_da: 10_000_000, vi_the_toi_da: 500,
            lo_trong_ngay_toi_da: 100_000, so_lenh_moi_giay_toi_da: 5,
            nguong_ngon_tay_beo: 0.10,
        })
    }

    // ---------- Cổng rủi ro ----------
    #[test]
    fn lenh_hop_le_duoc_cho_qua() {
        let mut c = cong_mau();
        assert_eq!(c.kiem_tra(Chieu::Mua, 8_400, 100, 8_400, 1_000_000_000), Ok(()));
        assert_eq!(c.so_lenh_qua, 1);
        assert_eq!(c.so_lenh_bi_chan, 0);
    }

    #[test]
    fn chan_ngon_tay_beo() {
        // Gõ 8400 thành 84000 — lỗi có thật, xảy ra hằng năm ở mọi thị trường.
        let mut c = cong_mau();
        let e = c.kiem_tra(Chieu::Mua, 84_000, 1, 8_400, 1_000_000_000).unwrap_err();
        assert!(matches!(e, TuChoi::NgonTayBeo { .. }));
        // Lệch nhỏ trong ngưỡng thì vẫn cho qua
        assert!(c.kiem_tra(Chieu::Mua, 8_800, 1, 8_400, 1_000_000_000).is_ok());
    }

    #[test]
    fn khong_co_gia_tham_chieu_thi_bo_qua_kiem_tra_ngon_tay_beo() {
        // Mã mới niêm yết chưa có giá tham chiếu — không được chặn oan.
        let mut c = cong_mau();
        assert!(c.kiem_tra(Chieu::Mua, 9_000, 1, 0, 1_000_000_000).is_ok());
    }

    #[test]
    fn chan_so_luong_va_gia_khong_hop_le() {
        let mut c = cong_mau();
        assert_eq!(c.kiem_tra(Chieu::Mua, 8_400, 0, 8_400, 1).unwrap_err(),
                   TuChoi::SoLuongKhongDuong(0));
        assert_eq!(c.kiem_tra(Chieu::Mua, 8_400, -5, 8_400, 1).unwrap_err(),
                   TuChoi::SoLuongKhongDuong(-5));
        assert_eq!(c.kiem_tra(Chieu::Mua, 0, 10, 0, 1).unwrap_err(),
                   TuChoi::GiaKhongDuong(0));
    }

    #[test]
    fn chan_gia_tri_lenh_qua_lon() {
        let mut c = cong_mau();
        assert!(matches!(c.kiem_tra(Chieu::Mua, 8_400, 100_000, 8_400, 1).unwrap_err(),
                         TuChoi::VuotGiaTriLenh { .. }));
    }

    #[test]
    fn chan_vuot_tran_vi_the_ca_hai_chieu() {
        let mut c = cong_mau();
        assert!(matches!(c.kiem_tra(Chieu::Mua, 8_400, 501, 8_400, 1).unwrap_err(),
                         TuChoi::VuotViThe { sau_lenh: 501, tran: 500 }));
        assert!(matches!(c.kiem_tra(Chieu::Ban, 8_400, 501, 8_400, 1).unwrap_err(),
                         TuChoi::VuotViThe { sau_lenh: -501, tran: 500 }),
                "bán khống cũng phải bị chặn, không chỉ mua");
    }

    #[test]
    fn vi_the_hien_tai_duoc_tinh_vao_han_muc() {
        let mut c = cong_mau();
        c.ghi_nhan_khop(Chieu::Mua, 8_400, 400);
        assert!(c.kiem_tra(Chieu::Mua, 8_400, 100, 8_400, 1).is_ok(), "400+100 = 500, vừa trần");
        assert!(c.kiem_tra(Chieu::Mua, 8_400, 101, 8_400, 1).is_err(), "400+101 vượt trần");
        assert!(c.kiem_tra(Chieu::Ban, 8_400, 400, 8_400, 1).is_ok(), "bán thì giảm vị thế");
    }

    #[test]
    fn gioi_han_tan_suat_chan_dung_so_lenh() {
        let mut c = cong_mau(); // trần 5 lệnh/giây
        let mut qua = 0;
        for i in 0..20u64 {
            if c.kiem_tra(Chieu::Mua, 8_400, 1, 8_400, 1_000_000_000 + i * 1_000_000).is_ok() {
                qua += 1;
            }
        }
        assert_eq!(qua, 5, "đúng 5 lệnh lọt qua trong một giây");
    }

    #[test]
    fn cua_so_tan_suat_truot_theo_thoi_gian() {
        let mut c = cong_mau();
        for i in 0..5u64 {
            assert!(c.kiem_tra(Chieu::Mua, 8_400, 1, 8_400, 1_000_000_000 + i).is_ok());
        }
        assert!(c.kiem_tra(Chieu::Mua, 8_400, 1, 8_400, 1_000_000_100).is_err(), "đã đủ 5");
        // Sang giây sau thì cửa sổ trượt qua, lại cho phép
        assert!(c.kiem_tra(Chieu::Mua, 8_400, 1, 8_400, 2_500_000_000).is_ok());
    }

    #[test]
    fn cong_tac_tat_chan_moi_thu_va_khong_tu_go_duoc() {
        let mut c = cong_mau();
        c.bat_cong_tac_tat();
        // Kể cả lệnh hoàn toàn hợp lệ cũng không lọt
        assert_eq!(c.kiem_tra(Chieu::Mua, 8_400, 1, 8_400, 1).unwrap_err(),
                   TuChoi::CongTacTatDaBat);
        assert!(c.da_tat(), "công tắc KHÔNG được tự tắt sau khi chặn");
        c.nguoi_van_hanh_go_cong_tac();
        assert!(c.kiem_tra(Chieu::Mua, 8_400, 1, 8_400, 1).is_ok());
    }

    #[test]
    fn lo_cham_tran_thi_tu_bat_cong_tac_tat() {
        let mut c = CongRuiRo::moi(HanMucRuiRo { lo_trong_ngay_toi_da: 10_000,
                                                 ..Default::default() });
        assert!(!c.da_tat());
        c.ghi_nhan_khop(Chieu::Mua, 9_000, 100);
        c.ghi_nhan_khop(Chieu::Ban, 8_800, 100); // lỗ 20 000 > trần 10 000
        assert_eq!(c.lai_lo_da_chot, -20_000);
        assert!(c.da_tat(), "vượt trần lỗ phải tự dừng, không chờ người can thiệp");
    }

    #[test]
    fn dong_vi_the_co_lai_thi_khong_bat_cong_tac() {
        let mut c = CongRuiRo::moi(HanMucRuiRo { lo_trong_ngay_toi_da: 10_000,
                                                 ..Default::default() });
        c.ghi_nhan_khop(Chieu::Mua, 8_000, 100);
        c.ghi_nhan_khop(Chieu::Ban, 8_500, 100);
        assert_eq!(c.lai_lo_da_chot, 50_000, "mua 80.00 bán 85.00 → lãi");
        assert!(!c.da_tat());
        assert_eq!(c.vi_the, 0);
    }

    #[test]
    fn gia_von_duoc_binh_quan_khi_mo_them() {
        let mut c = cong_mau();
        c.ghi_nhan_khop(Chieu::Mua, 8_000, 100);
        c.ghi_nhan_khop(Chieu::Mua, 9_000, 100);
        assert!((c.gia_von - 8_500.0).abs() < 1e-9, "bình quân 8000 và 9000 = 8500");
        c.ghi_nhan_khop(Chieu::Ban, 8_500, 200);
        assert_eq!(c.lai_lo_da_chot, 0, "bán đúng giá vốn thì hoà vốn");
        assert_eq!(c.vi_the, 0);
        assert_eq!(c.gia_von, 0.0, "đóng hết thì giá vốn phải về 0");
    }

    #[test]
    fn dao_chieu_vi_the_dat_lai_gia_von() {
        let mut c = cong_mau();
        c.ghi_nhan_khop(Chieu::Mua, 8_000, 100);
        // Bán 300: đóng 100 (lãi) rồi mở mới 200 ở chiều bán
        c.ghi_nhan_khop(Chieu::Ban, 8_500, 300);
        assert_eq!(c.vi_the, -200);
        assert_eq!(c.lai_lo_da_chot, 50_000, "chỉ phần ĐÓNG mới tính lãi");
        assert!((c.gia_von - 8_500.0).abs() < 1e-9, "phần dư là vị thế mới ở giá 8500");
    }

    #[test]
    fn ban_khong_roi_mua_lai_re_hon_thi_co_lai() {
        let mut c = cong_mau();
        c.ghi_nhan_khop(Chieu::Ban, 9_000, 100);
        assert_eq!(c.vi_the, -100);
        c.ghi_nhan_khop(Chieu::Mua, 8_500, 100);
        assert_eq!(c.lai_lo_da_chot, 50_000, "bán khống 90.00 mua lại 85.00 → lãi");
    }

    #[test]
    fn mo_them_cung_chieu_thi_chua_hien_thuc_hoa_lai_lo() {
        let mut c = cong_mau();
        c.ghi_nhan_khop(Chieu::Mua, 8_000, 100);
        c.ghi_nhan_khop(Chieu::Mua, 9_000, 100);
        assert_eq!(c.vi_the, 200);
        assert_eq!(c.lai_lo_da_chot, 0, "chưa đóng gì thì chưa chốt lãi/lỗ");
    }

    #[test]
    fn dem_dung_so_lenh_qua_va_bi_chan() {
        let mut c = cong_mau();
        c.kiem_tra(Chieu::Mua, 8_400, 100, 8_400, 1).ok();
        c.kiem_tra(Chieu::Mua, 84_000, 100, 8_400, 1).ok();
        c.kiem_tra(Chieu::Mua, 8_400, -1, 8_400, 1).ok();
        assert_eq!(c.so_lenh_qua, 1);
        assert_eq!(c.so_lenh_bi_chan, 2);
    }

    // ---------- Tín hiệu ----------
    #[test]
    fn mat_can_bang_nam_trong_khoang_am_mot_den_mot() {
        assert_eq!(mat_can_bang(0, 0), 0.0, "sổ rỗng thì trung tính, không chia cho 0");
        assert_eq!(mat_can_bang(100, 100), 0.0);
        assert_eq!(mat_can_bang(100, 0), 1.0);
        assert_eq!(mat_can_bang(0, 100), -1.0);
        for (m, b) in [(1u64, 999u64), (500, 500), (999, 1), (7, 13)] {
            let x = mat_can_bang(m, b);
            assert!((-1.0..=1.0).contains(&x));
        }
    }

    #[test]
    fn gia_vi_mo_lech_ve_phia_ben_it_khoi_luong() {
        // Nhiều người chờ MUA → áp lực đẩy giá lên → giá vi mô gần giá BÁN.
        let nhieu_mua = gia_vi_mo(8_400, 9_000, 8_410, 1_000).unwrap();
        let nhieu_ban = gia_vi_mo(8_400, 1_000, 8_410, 9_000).unwrap();
        let can_bang = gia_vi_mo(8_400, 1_000, 8_410, 1_000).unwrap();
        assert!(nhieu_mua > can_bang, "áp lực mua đẩy giá vi mô lên");
        assert!(nhieu_ban < can_bang, "áp lực bán kéo xuống");
        assert!((can_bang - 8_405.0).abs() < 1e-9, "cân bằng thì đúng giá giữa");
        assert!(nhieu_mua > 8_400.0 && nhieu_mua < 8_410.0, "luôn nằm trong chênh lệch");
    }

    #[test]
    fn gia_vi_mo_so_rong_tra_none() {
        assert_eq!(gia_vi_mo(8_400, 0, 8_410, 0), None);
    }

    // ---------- Cửa sổ thống kê ----------
    #[test]
    fn cua_so_tinh_dung_trung_binh_va_do_lech() {
        let mut c = CuaSoThongKe::moi(5);
        for x in [2.0, 4.0, 4.0, 4.0, 5.0] { c.them(x); }
        assert!((c.trung_binh() - 3.8).abs() < 1e-9);
        // phương sai mẫu của [2,4,4,4,5] = 1.2
        assert!((c.phuong_sai() - 1.2).abs() < 1e-9);
        assert!(c.day());
    }

    #[test]
    fn cua_so_truot_bo_gia_tri_cu() {
        let mut c = CuaSoThongKe::moi(3);
        for x in [1.0, 2.0, 3.0, 4.0, 5.0] { c.them(x); }
        assert_eq!(c.so_luong(), 3);
        assert!((c.trung_binh() - 4.0).abs() < 1e-9, "chỉ còn [3,4,5]");
    }

    #[test]
    fn phuong_sai_khong_bao_gio_am_du_sai_so_dau_phay_dong() {
        let mut c = CuaSoThongKe::moi(50);
        for _ in 0..50 { c.them(1_000_000.0); } // toàn giá trị giống hệt, cỡ lớn
        assert!(c.phuong_sai() >= 0.0, "phải chặn sai số làm ra số âm");
        assert!(c.phuong_sai() < 1e-3, "dữ liệu không đổi thì phương sai ~0");
        assert_eq!(c.diem_z(1_000_000.0), None, "độ lệch ~0 thì điểm z vô nghĩa");
    }

    #[test]
    fn cua_so_chua_du_hai_diem_thi_phuong_sai_bang_khong() {
        let mut c = CuaSoThongKe::moi(10);
        assert_eq!(c.phuong_sai(), 0.0);
        c.them(5.0);
        assert_eq!(c.phuong_sai(), 0.0, "một điểm thì không có phương sai mẫu");
    }

    #[test]
    fn diem_z_do_dung_do_lech() {
        let mut c = CuaSoThongKe::moi(100);
        for i in 0..100 { c.them((i % 10) as f64); }
        let z = c.diem_z(c.trung_binh()).unwrap();
        assert!(z.abs() < 1e-9, "đúng giá trị trung bình thì z = 0");
        let z2 = c.diem_z(c.trung_binh() + 2.0 * c.do_lech_chuan()).unwrap();
        assert!((z2 - 2.0).abs() < 1e-9);
    }

    // ---------- Arbitrage cặp ----------
    #[test]
    fn arb_khong_ra_tin_hieu_khi_chua_du_du_lieu() {
        let mut a = ArbCap::moi(1.5, 100, 2.0, 0.5, 4.0);
        let (ga, gb) = sinh_cap_gia(50, 1, 1.5);
        for i in 0..50 {
            assert_eq!(a.cap_nhat(ga[i], gb[i]), TinHieuCap::KhongLam,
                       "cửa sổ chưa đầy thì tuyệt đối không được vào lệnh");
        }
    }

    #[test]
    fn arb_vao_lenh_khi_chenh_lech_gian_bat_thuong() {
        let mut a = ArbCap::moi(1.0, 20, 2.0, 0.5, 10.0);
        // 20 điểm ổn định quanh 0 (có dao động nhỏ để độ lệch chuẩn khác 0)
        for i in 0..20 { a.cap_nhat(10_000 + (i % 3), 10_000); }
        // rồi một cú giãn mạnh
        let th = a.cap_nhat(10_100, 10_000);
        assert_eq!(th, TinHieuCap::MoDaiB, "A đắt bất thường → bán A mua B");
        assert_eq!(a.dang_mo, Some(TinHieuCap::MoDaiB));
    }

    #[test]
    fn arb_khong_mo_hai_vi_the_cung_luc() {
        let mut a = ArbCap::moi(1.0, 20, 2.0, 0.5, 100.0);
        for i in 0..20 { a.cap_nhat(10_000 + (i % 3), 10_000); }
        assert_ne!(a.cap_nhat(10_100, 10_000), TinHieuCap::KhongLam);
        for _ in 0..5 {
            let t = a.cap_nhat(10_120, 10_000);
            assert!(matches!(t, TinHieuCap::KhongLam | TinHieuCap::Dong),
                    "đang có vị thế thì không được mở thêm");
        }
    }

    #[test]
    fn arb_cat_lo_khi_chenh_lech_gian_qua_nguong_dung() {
        // Bài học sống còn: chênh lệch giãn mãi nghĩa là quan hệ ĐÃ GÃY,
        // không phải "cơ hội càng tốt hơn". Phải thoát.
        let mut a = ArbCap::moi(1.0, 20, 2.0, 0.5, 3.0);
        for i in 0..20 { a.cap_nhat(10_000 + (i % 3), 10_000); }
        a.cap_nhat(10_050, 10_000); // vào lệnh
        assert!(a.dang_mo.is_some());
        let t = a.cap_nhat(10_500, 10_000); // giãn cực mạnh
        assert_eq!(t, TinHieuCap::Dong, "vượt ngưỡng dừng phải CẮT LỖ");
        assert_eq!(a.dang_mo, None);
    }

    #[test]
    fn chenh_lech_tinh_dung_theo_ty_le_phong_ho() {
        let a = ArbCap::moi(1.5, 10, 2.0, 0.5, 4.0);
        assert!((a.chenh_lech(15_000, 10_000) - 0.0).abs() < 1e-9);
        assert!((a.chenh_lech(15_150, 10_000) - 150.0).abs() < 1e-9);
    }

    // ---------- Định cỡ ----------
    #[test]
    fn kelly_bang_khong_khi_khong_co_loi_the() {
        assert_eq!(ty_le_kelly(0.5, 1.0), 0.0, "tung đồng xu công bằng → đừng đánh");
        assert_eq!(ty_le_kelly(0.4, 1.0), 0.0, "lợi thế âm → tuyệt đối đừng đánh");
        assert_eq!(ty_le_kelly(0.3, 0.5), 0.0);
    }

    #[test]
    fn kelly_tang_theo_loi_the() {
        let mut truoc = 0.0;
        for p in [0.55, 0.60, 0.65, 0.70, 0.80] {
            let f = ty_le_kelly(p, 1.0);
            assert!(f > truoc, "lợi thế lớn hơn phải cho cỡ lớn hơn");
            assert!(f <= 1.0);
            truoc = f;
        }
    }

    #[test]
    fn kelly_dung_gia_tri_kinh_dien() {
        // 60% thắng, ăn 1 thua 1 → Kelly = 2p − 1 = 0.20
        assert!((ty_le_kelly(0.60, 1.0) - 0.20).abs() < 1e-9);
        // 40% thắng, ăn 2 thua 1 → (0.4·2 − 0.6)/2 = 0.10
        assert!((ty_le_kelly(0.40, 2.0) - 0.10).abs() < 1e-9);
    }

    #[test]
    fn kelly_mot_phan_luon_nho_hon_kelly_toan_phan() {
        for p in [0.55, 0.60, 0.75] {
            let toan = ty_le_kelly(p, 1.0);
            let phan = kelly_mot_phan(p, 1.0, 0.25);
            assert!(phan < toan);
            assert!((phan - toan * 0.25).abs() < 1e-9);
        }
    }

    #[test]
    fn kelly_khong_chia_cho_khong() {
        assert_eq!(ty_le_kelly(0.9, 0.0), 0.0);
        assert_eq!(ty_le_kelly(0.9, -1.0), 0.0);
    }

    #[test]
    fn co_theo_bien_dong_giam_khi_bien_dong_tang() {
        let von = 1_000_000i64;
        let a = co_theo_bien_dong(von, 0.10, 0.10, 100);
        let b = co_theo_bien_dong(von, 0.10, 0.40, 100);
        assert!(b < a, "mã dao động mạnh gấp 4 thì mua ít hơn hẳn");
        assert_eq!(a, 10_000, "biến động khớp mục tiêu → dùng toàn bộ vốn");
        assert_eq!(b, 2_500, "gấp 4 lần biến động → 1/4 tỉ trọng");
    }

    #[test]
    fn co_theo_bien_dong_khong_bao_gio_don_bay_qua_von() {
        // Mã êm hơn mục tiêu KHÔNG được dẫn tới mua vượt vốn.
        let c = co_theo_bien_dong(1_000_000, 0.40, 0.05, 100);
        assert_eq!(c, 10_000, "tỉ trọng bị chặn ở 1.0, không dùng đòn bẩy ngầm");
    }

    #[test]
    fn co_theo_bien_dong_an_toan_voi_dau_vao_xau() {
        assert_eq!(co_theo_bien_dong(1_000_000, 0.1, 0.0, 100), 0);
        assert_eq!(co_theo_bien_dong(1_000_000, 0.1, 0.1, 0), 0);
        assert_eq!(co_theo_bien_dong(1_000_000, 0.1, -0.5, 100), 0);
    }

    // ---------- Thước đo rủi ro ----------
    #[test]
    fn duong_von_di_len_deu_thi_khong_sut_giam() {
        let d: Vec<i64> = (0..50).map(|i| 100_000 + i * 100).collect();
        let r = do_rui_ro(&d);
        assert_eq!(r.sut_giam_toi_da, 0);
        assert_eq!(r.so_phien_lo, 0);
        assert_eq!(r.tong_lai_lo, 4_900);
    }

    #[test]
    fn sut_giam_do_dung_khoang_cach_tu_dinh() {
        let d = vec![100, 150, 120, 80, 130];
        let r = do_rui_ro(&d);
        assert_eq!(r.sut_giam_toi_da, 70, "từ đỉnh 150 xuống đáy 80");
    }

    #[test]
    fn sut_giam_khong_bao_gio_am() {
        for hat in [1u64, 7, 42] {
            let mut s = hat;
            let d: Vec<i64> = (0..200).map(|_| {
                s = s.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((s >> 40) % 200_000) as i64
            }).collect();
            assert!(do_rui_ro(&d).sut_giam_toi_da >= 0);
        }
    }

    #[test]
    fn duong_von_em_co_sharpe_cao_hon_duong_xoc() {
        // Cùng đích đến, nhưng đường êm mới là đường người ta đi hết được.
        // Đường "êm" vẫn phải có dao động nhỏ: đường thẳng tuyệt đối cho độ
        // lệch chuẩn 0, và khi đó Sharpe không định nghĩa được (ta trả 0).
        let em: Vec<i64> = (0..100).map(|i| 100_000 + i * 500 + (i % 5) * 40).collect();
        let mut xoc = Vec::new();
        let mut v = 100_000i64;
        for i in 0..100 { v += if i % 3 == 0 { -8_000 } else { 5_750 }; xoc.push(v); }
        let (a, b) = (do_rui_ro(&em), do_rui_ro(&xoc));
        assert!(a.ty_so_sharpe > b.ty_so_sharpe,
                "êm {:.2} phải cao hơn xóc {:.2}", a.ty_so_sharpe, b.ty_so_sharpe);
        assert!(b.sut_giam_toi_da > a.sut_giam_toi_da);
    }

    #[test]
    fn duong_von_qua_ngan_khong_panic() {
        assert_eq!(do_rui_ro(&[]).tong_lai_lo, 0);
        assert_eq!(do_rui_ro(&[100]).sut_giam_toi_da, 0);
        assert_eq!(do_rui_ro(&[100, 100]).ty_so_sharpe, 0.0, "không dao động → Sharpe 0");
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn sinh_cap_gia_tat_dinh() {
        assert_eq!(sinh_cap_gia(100, 5, 1.5), sinh_cap_gia(100, 5, 1.5));
        assert_ne!(sinh_cap_gia(100, 5, 1.5), sinh_cap_gia(100, 6, 1.5));
    }

    #[test]
    fn hai_chuoi_gia_that_su_di_cung_nhau() {
        // Nếu chúng không đồng biến thì cả chương arbitrage cặp là vô nghĩa.
        let (a, b) = sinh_cap_gia(2_000, 2024, 1.5);
        let n = a.len() as f64;
        let (ta, tb) = (a.iter().sum::<i64>() as f64 / n, b.iter().sum::<i64>() as f64 / n);
        let mut tu = 0.0;
        let (mut sa, mut sb) = (0.0, 0.0);
        for i in 0..a.len() {
            let (da, db) = (a[i] as f64 - ta, b[i] as f64 - tb);
            tu += da * db; sa += da * da; sb += db * db;
        }
        let tuong_quan = tu / (sa.sqrt() * sb.sqrt());
        assert!(tuong_quan > 0.8, "tương quan {:.3} phải cao", tuong_quan);
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `attempt to subtract with overflow` | `vi_the - khoi_luong` với `u64` | Vị thế phải là `i64` — nó có thể âm |
| Sharpe bằng 0 với chiến lược "hoàn hảo" | Đường vốn đều tuyệt đối → độ lệch chuẩn 0 | Thêm nhiễu nhỏ; đường vốn hoàn hảo không tồn tại |
| Bài kiểm thử không chạm tới nhánh vị thế | Kiểm giá trị lệnh chặn trước | Nới hạn mức giá trị trong dữ liệu kiểm thử |
| Lãi lỗ sai sau khi đảo chiều | Không xử lý riêng trường hợp vượt qua 0 | Chốt phần cũ, đặt giá vốn mới cho phần dư |
| `E0308: expected f64, found i64` | Trộn tiền nguyên với thống kê thực | Ép kiểu tường minh ở đúng biên |

---

## Tóm tắt chương & Bài tập rèn luyện

### 5 điểm cốt lõi

1. **Cổng rủi ro không được có đường vòng.** Nó phải nhanh để không ai muốn tắt, và bắt buộc để không ai tắt được.
2. **Lãi lỗ phải dựa trên giá vốn trung bình**, và trường hợp đảo chiều phải xử lý riêng.
3. **Vi giá dùng trọng số ngược** và dự báo tốt hơn giá giữa — một trong ít tín hiệu đơn giản mà hiệu quả.
4. **Kelly toàn phần là bẫy.** Bạn không biết `p`, và cược quá nhiều bị phạt nặng hơn cược quá ít rất nhiều.
5. **Sụt giảm quan trọng hơn lợi nhuận**, vì nó quyết định bạn có còn ở lại bàn để thu lợi nhuận hay không.

### Bài tập rèn luyện

**Bài 1.** Cài **kiểm soát rủi ro thích ứng**: tự thu hẹp hạn mức khi hiệu suất xấu đi.

<details>
<summary><b>Gợi ý</b></summary>

Hạn mức tĩnh có một vấn đề: chúng đúng cho điều kiện bình thường, và sai đúng lúc bất thường. Rủi ro thích ứng thu hẹp khi thua và nới ra khi thắng — nhưng phải **nới chậm hơn nhiều** so với tốc độ thu, vì phục hồi cần được chứng minh, còn tổn thất thì không.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct RuiRoThichUng {
    pub han_muc_co_ban: i64,
    pub he_so: f64,           // 0,25 → 1,0
    pub chuoi_thang: u32,
    pub chuoi_thua: u32,
}

impl RuiRoThichUng {
    pub fn ghi_nhan(&mut self, lai_lo: i64) {
        if lai_lo > 0 {
            self.chuoi_thang += 1;
            self.chuoi_thua = 0;
            // Nới CHẬM: cần 5 lần thắng liên tiếp mới tăng 10%
            if self.chuoi_thang >= 5 {
                self.he_so = (self.he_so * 1.1).min(1.0);
                self.chuoi_thang = 0;
            }
        } else if lai_lo < 0 {
            self.chuoi_thua += 1;
            self.chuoi_thang = 0;
            // Thu NHANH: 3 lần thua liên tiếp là cắt 30%
            if self.chuoi_thua >= 3 {
                self.he_so = (self.he_so * 0.7).max(0.25);
                self.chuoi_thua = 0;
            }
        }
    }

    pub fn han_muc_hien_tai(&self) -> i64 {
        (self.han_muc_co_ban as f64 * self.he_so) as i64
    }
}
```

Bất đối xứng là chủ ý: 3 lần thua cắt 30%, nhưng phải 5 lần thắng mới nới 10%. Sàn 0,25 bảo đảm hệ thống không tự tắt hoàn toàn — nếu về 0, bạn không bao giờ có dữ liệu để biết chiến lược đã hồi phục chưa.
</details>

**Bài 2.** Cài **hệ thống nhiều tín hiệu có tổ hợp trọng số** và đo tương quan giữa các tín hiệu.

<details>
<summary><b>Gợi ý</b></summary>

Cộng nhiều tín hiệu chỉ hữu ích nếu chúng **độc lập**. Hai tín hiệu có tương quan 0,95 thực chất là một tín hiệu tính hai lần — và bạn sẽ cược gấp đôi vào cùng một ý tưởng mà tưởng mình đang đa dạng hoá.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct ToHopTinHieu {
    pub ten: Vec<String>,
    pub trong_so: Vec<f64>,
    pub lich_su: Vec<Vec<f64>>,   // lịch sử giá trị từng tín hiệu
}

impl ToHopTinHieu {
    pub fn ket_hop(&mut self, gia_tri: &[f64]) -> f64 {
        for (i, v) in gia_tri.iter().enumerate() {
            if i < self.lich_su.len() { self.lich_su[i].push(*v); }
        }
        let tong_ts: f64 = self.trong_so.iter().map(|w| w.abs()).sum();
        if tong_ts == 0.0 { return 0.0; }
        gia_tri.iter().zip(&self.trong_so).map(|(v, w)| v * w).sum::<f64>() / tong_ts
    }

    pub fn tuong_quan(&self, i: usize, j: usize) -> Option<f64> {
        let (a, b) = (self.lich_su.get(i)?, self.lich_su.get(j)?);
        let n = a.len().min(b.len());
        if n < 2 { return None; }
        let (ma, mb) = (a[..n].iter().sum::<f64>() / n as f64,
                        b[..n].iter().sum::<f64>() / n as f64);
        let (mut cov, mut va, mut vb) = (0.0, 0.0, 0.0);
        for k in 0..n {
            let (da, db) = (a[k] - ma, b[k] - mb);
            cov += da * db; va += da * da; vb += db * db;
        }
        if va == 0.0 || vb == 0.0 { return None; }
        Some(cov / (va * vb).sqrt())
    }

    /// Các cặp tín hiệu quá giống nhau — chúng KHÔNG đa dạng hoá gì cả.
    pub fn cap_du_thua(&self, nguong: f64) -> Vec<(usize, usize, f64)> {
        let mut ra = Vec::new();
        for i in 0..self.lich_su.len() {
            for j in (i + 1)..self.lich_su.len() {
                if let Some(r) = self.tuong_quan(i, j) {
                    if r.abs() > nguong { ra.push((i, j, r)); }
                }
            }
        }
        ra
    }
}
```

`cap_du_thua` là công cụ chẩn đoán quan trọng: nếu hai tín hiệu có tương quan trên 0,8, bạn nên bỏ một hoặc gộp chúng — nếu không, "danh mục tín hiệu" của bạn thực chất chỉ có một ý tưởng được đặt cược nhiều lần.
</details>
