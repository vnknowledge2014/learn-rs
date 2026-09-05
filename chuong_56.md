# Chương 56: Kỹ nghệ Ngữ cảnh, Bộ khung, Vòng lặp và Đồ thị cho Tác tử AI (Context, Harness, Loop & Graph Engineering)

## Giới thiệu & Mục tiêu học tập

Chủ đề 8 (Chương 43–47) dạy bạn **Prompt Engineering** — nghệ thuật viết câu lệnh cho mô hình ngôn ngữ. Nhưng nghề này đã dịch chuyển rất nhanh. Năm 2023 người ta tuyển "Prompt Engineer"; đến nay, prompt chỉ còn là **một phần nhỏ** của bài toán. Thứ quyết định một ứng dụng AI chạy được hay không nằm ở bốn tầng kỹ nghệ khác:

| Tầng | Câu hỏi cốt lõi | Hỏng thì sao? |
|---|---|---|
| **Context Engineering** | Nhét *cái gì* vào cửa sổ ngữ cảnh có hạn? | Mô hình bỏ sót thông tin quan trọng, hoặc hóa đơn token bùng nổ |
| **Harness Engineering** | Tác tử được phép *làm gì*? | Tác tử gọi hàm không tồn tại, hoặc xóa nhầm dữ liệu thật |
| **Loop Engineering** | Khi nào thì *dừng*? | Vòng lặp vô hạn — hóa đơn API không đáy |
| **Graph Engineering** | Tri thức *liên kết* với nhau ra sao? | Truy xuất bỏ sót thông tin cách 2–3 bước quan hệ |

Chương này dạy cả bốn tầng bằng Rust, và điểm mấu chốt về mặt kỹ thuật: **toàn bộ mã chạy offline**. Mô hình ngôn ngữ được thay bằng một bản giả tất định — đúng kỹ thuật *test double* ở Chương 55. Nhờ vậy bạn học được kiến trúc mà không cần khóa API, và quan trọng hơn: **hệ thống tác tử của bạn trở nên kiểm thử được**, điều mà phần lớn dự án AI ngoài kia không làm được.

Mục tiêu học tập:
- Xem **ngữ cảnh là tài nguyên có hạn** và biết cách phân bổ nó như bài toán xếp ba lô.
- Hiểu hiện tượng **"Lost in the Middle"** và cách sắp xếp ngữ cảnh để chống lại nó.
- Thiết kế **bộ khung (harness)**: định nghĩa không gian hành động của tác tử bằng `trait`, biến "tác tử được phép làm gì" thành một hợp đồng kiểu.
- Viết **vòng lặp tác tử có phanh**: ba điều kiện dừng bắt buộc (hoàn thành, hết ngân sách, phát hiện lặp).
- Xây **đồ thị tri thức** và truy xuất lan tỏa nhiều bước (nền tảng của GraphRAG).
- Biết vì sao mọi thành phần trên đều phải **kiểm thử được**, và cách đạt điều đó bằng test double.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│      HÌNH TƯỢNG: THUÊ MỘT TRỢ LÝ GIỎI NHƯNG MẤT TRÍ NHỚ SAU MỖI CUỘC HỌP         │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  Bạn thuê một chuyên gia cực giỏi. Nhưng anh ta có ba đặc điểm kỳ lạ:            │
│                                                                                  │
│  1. CHIẾC CẶP CÓ HẠN (Context Engineering)                                       │
│     Anh ta chỉ mang được 1 chiếc cặp vào phòng họp. Bạn có 400 trang tài liệu    │
│     nhưng cặp chỉ nhét vừa 100 trang. → Chọn 100 trang NÀO?                      │
│     Và: anh ta đọc kỹ trang đầu, trang cuối, còn phần giữa thì lướt.             │
│                                                                                  │
│  2. THẺ RA VÀO CÓ GIỚI HẠN (Harness Engineering)                                 │
│     Anh ta chỉ mở được những cánh cửa bạn cấp thẻ: phòng kho, phòng kế toán.     │
│     KHÔNG có thẻ phòng máy chủ. → Bạn quyết định anh ta LÀM ĐƯỢC GÌ.             │
│                                                                                  │
│  3. KHÔNG BIẾT KHI NÀO NÊN NGHỈ (Loop Engineering)                               │
│     Nếu không ai bảo dừng, anh ta sẽ đi tra cứu mãi — mỗi lần tra tốn tiền.      │
│     → Phải đặt: "tối đa 5 lần tra" và "nếu tra lại đúng thứ vừa tra, dừng ngay". │
│                                                                                  │
│  4. TẤM BẢN ĐỒ QUAN HỆ (Graph Engineering)                                       │
│     Hỏi "đơn hàng này ở kho nào?" — hồ sơ đơn hàng KHÔNG ghi kho.                │
│     Phải đi: Đơn hàng → Vận đơn → Kho. Tìm kiếm từ khóa phẳng sẽ bó tay.         │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Context Engineering — ngữ cảnh là tài nguyên, không phải thùng rác

Sai lầm phổ biến nhất khi xây ứng dụng AI: *"cứ nhét hết mọi thứ vào cho chắc"*. Ba lý do khiến cách này hỏng:

1. **Cửa sổ có hạn.** Vượt giới hạn thì phần đầu bị cắt — thường lại chính là chỉ dẫn hệ thống quan trọng nhất.
2. **Chi phí tuyến tính theo token.** Nhét thừa 10× ngữ cảnh nghĩa là hóa đơn gấp 10 lần, và độ trễ cũng tăng.
3. **Nhiễu làm giảm chất lượng.** Tài liệu không liên quan *làm loãng* tín hiệu; mô hình dễ bám vào chi tiết sai.

Vậy bài toán thực sự là: **chọn tập con ngữ cảnh có giá trị cao nhất trong một ngân sách token cố định**. Đây chính xác là **bài toán xếp ba lô (knapsack)**. Lời giải thực dụng dùng chiến lược tham lam theo *mật độ giá trị*:

```
điểm ưu tiên = độ liên quan / số token
```

Một tài liệu 2000 token với điểm liên quan 0.95 có thể **thua** một tài liệu 90 token điểm 0.5 — vì cái nhỏ cho nhiều giá trị hơn trên mỗi token bỏ ra. Mã trong chương này cài đúng chiến lược đó, cộng thêm cơ chế **ghim cứng** cho những mẩu không bao giờ được loại (quy tắc an toàn, định danh phiên).

### 2. "Lost in the Middle" và cách sắp xếp chống lại nó

Nghiên cứu về mô hình ngôn ngữ cho thấy một hiện tượng nhất quán: **mô hình ghi nhớ tốt phần đầu và phần cuối của ngữ cảnh, kém nhất ở khoảng giữa** — giống hệt trí nhớ con người khi đọc một danh sách dài.

Hệ quả thực hành rất cụ thể: sau khi đã chọn được các mẩu ngữ cảnh, **thứ tự sắp xếp vẫn còn quan trọng**. Chiến lược đơn giản mà hiệu quả: xếp giảm dần theo độ liên quan rồi **rải xen kẽ ra hai đầu**, để những mẩu quan trọng nhất nằm ở đầu và cuối, mẩu ít quan trọng bị đẩy vào giữa.

### 3. Harness Engineering — không gian hành động là một hợp đồng kiểu

Một tác tử không "biết làm mọi thứ". Nó chỉ làm được đúng những gì bạn **cấp công cụ**. Tập công cụ đó gọi là **bộ khung (harness)**, và trong Rust nó được biểu diễn tự nhiên bằng `trait`:

```rust
pub trait CongCu {
    fn ten(&self) -> &str;
    fn mo_ta(&self) -> &str;              // phần này nạp vào ngữ cảnh cho mô hình đọc
    fn chay(&self, tham_so: &str) -> KetQuaCongCu;
}
```

Ba nguyên tắc thiết kế bộ khung:

1. **Mô tả công cụ chính là giao diện người dùng của tác tử.** Mô hình chỉ biết công cụ qua phần `mo_ta`. Viết mô tả mơ hồ thì tác tử gọi sai — đây là "lỗi giao diện", không phải "lỗi mô hình".
2. **Danh sách trắng, không phải danh sách đen.** Tác tử chỉ gọi được thứ đã đăng ký; mọi thứ khác trả lỗi. Trong mã dưới đây, `khung.goi("xoa_o_cung", "/")` **luôn** thất bại vì công cụ đó chưa từng được đăng ký.
3. **Trả lỗi có nội dung, đừng panic.** `KetQuaCongCu::Loi("\"x\" không phải số")` cho tác tử cơ hội **tự sửa** ở lượt sau. Một `panic!` thì giết cả tiến trình.

> Đây chính là kiến trúc "lõi thuần túy — vỏ mệnh lệnh" ở Chương 20, áp dụng vào AI: bộ khung là **vỏ** kiểm soát mọi tác dụng phụ, còn logic quyết định là **lõi**.

### 4. Loop Engineering — vòng lặp tác tử phải có phanh

Một tác tử hoạt động theo vòng: *quan sát → quyết định → hành động → quan sát...* Nếu vòng này không có điều kiện dừng, bạn có một chương trình gọi API vô hạn. **Ba cái phanh bắt buộc:**

| Phanh | Cơ chế | Chặn được gì |
|---|---|---|
| **Hoàn thành** | Tác tử trả về `TraLoi(...)` | Trường hợp bình thường |
| **Hết ngân sách** | Đếm số lượt, dừng ở `N` | Tác tử lan man mãi không kết luận |
| **Phát hiện lặp** | Băm `(tên công cụ, tham số)`, thấy trùng thì dừng | Tác tử **kẹt**: gọi đi gọi lại y hệt |

Cái phanh thứ ba quan trọng hơn người ta tưởng. Một tác tử kẹt thường **không** vượt ngân sách ngay — nó chỉ đốt tiền chậm rãi trong khi chẳng tiến triển gì. Bài test `vong_lap_phat_hien_tac_tu_bi_ket` dưới đây chứng minh: với ngân sách 50 lượt, tác tử kẹt bị chặn ngay ở bước thứ 2.

### 5. Graph Engineering — khi tìm kiếm phẳng không đủ

Cách truy xuất phổ biến (RAG cơ bản) là: nhúng tài liệu thành vector, tìm k tài liệu *giống nhất* với câu hỏi. Cách này hỏng ở một lớp câu hỏi cụ thể: **câu hỏi cần đi qua nhiều bước quan hệ**.

> *"Đơn hàng ORD-88 xuất từ kho nào?"*

Hồ sơ đơn hàng **không chứa chữ "kho"**. Đường đi thật là: `Đơn hàng → Vận đơn → Kho`. Tìm kiếm theo độ tương tự sẽ không bao giờ tìm ra, vì không có tài liệu nào vừa nói về đơn hàng vừa nói về kho.

**Đồ thị tri thức** giải bài này: mô hình hóa thực thể thành đỉnh, quan hệ thành cạnh có nhãn, rồi **truy xuất lan tỏa** (BFS theo độ sâu) từ điểm xuất phát. Đây là ý tưởng cốt lõi của **GraphRAG**. Hai chi tiết kỹ thuật bắt buộc:
- **Giới hạn độ sâu**: đi 3 bước trên đồ thị dày có thể kéo về nửa cơ sở tri thức.
- **Tập đã thăm**: đồ thị thật luôn có chu trình; thiếu `HashSet` là lặp vô hạn.

Bạn đã có sẵn toàn bộ công cụ cho phần này từ **Chương 30** (đồ thị, BFS, danh sách kề dùng chỉ số).

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch56
cargo test -p ch56
```

```rust
//! Chương 56 — Kỹ nghệ Ngữ cảnh & Tác tử: Context, Harness, Loop, Graph Engineering.
//! Toàn bộ chạy offline: mô hình ngôn ngữ được thay bằng một bản giả tất định,
//! đúng tinh thần "test double" ở Chương 55 — nhờ vậy mọi thứ kiểm thử được.

use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// PHẦN 1: NGÂN SÁCH NGỮ CẢNH — CONTEXT ENGINEERING
// ============================================================================

/// Một mẩu ngữ cảnh có thể nạp vào cửa sổ của mô hình.
#[derive(Debug, Clone, PartialEq)]
pub struct MauNguCanh {
    pub nhan: String,
    pub noi_dung: String,
    pub token: usize,
    /// Điểm liên quan tới truy vấn hiện tại (0.0 – 1.0).
    pub lien_quan: f64,
    /// Ghim cứng: luôn nạp bất kể ngân sách (ví dụ: quy tắc an toàn).
    pub ghim: bool,
}

/// Kết quả sau khi cắt gọt theo ngân sách.
#[derive(Debug, PartialEq)]
pub struct GoiNguCanh {
    pub cac_mau: Vec<MauNguCanh>,
    pub tong_token: usize,
    pub so_mau_bi_loai: usize,
}

/// CONTEXT ENGINEERING: chọn tập con ngữ cảnh tốt nhất trong ngân sách token.
/// Đây là bài toán xếp ba lô (knapsack) đơn giản hóa: ưu tiên điểm liên quan
/// trên mỗi token, và luôn giữ các mẩu bị ghim.
pub fn dong_goi_ngu_canh(mut mau: Vec<MauNguCanh>, ngan_sach: usize) -> GoiNguCanh {
    let tong_ban_dau = mau.len();

    // 1. Tách phần ghim cứng — luôn được nạp trước
    let (ghim, mut tuy_chon): (Vec<_>, Vec<_>) = mau.drain(..).partition(|m| m.ghim);
    let mut da_dung: usize = ghim.iter().map(|m| m.token).sum();
    let mut chon: Vec<MauNguCanh> = ghim;

    // 2. Xếp phần còn lại theo MẬT ĐỘ giá trị (liên quan / token) giảm dần
    tuy_chon.sort_by(|a, b| {
        let ma = a.lien_quan / a.token.max(1) as f64;
        let mb = b.lien_quan / b.token.max(1) as f64;
        mb.partial_cmp(&ma).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.nhan.cmp(&b.nhan)) // phá hòa tất định
    });

    // 3. Nhồi vào cho tới khi hết ngân sách
    for m in tuy_chon {
        if da_dung + m.token <= ngan_sach {
            da_dung += m.token;
            chon.push(m);
        }
    }

    // 4. "Lost in the middle": đặt mẩu quan trọng nhất ở ĐẦU và CUỐI
    chon = sap_xep_chong_lang_quen(chon);

    GoiNguCanh {
        tong_token: da_dung,
        so_mau_bi_loai: tong_ban_dau - chon.len(),
        cac_mau: chon,
    }
}

/// Chống hiện tượng "Lost in the Middle": mô hình nhớ tốt phần đầu và phần cuối,
/// hay quên phần giữa. Vậy hãy đẩy thứ quan trọng nhất ra hai đầu.
pub fn sap_xep_chong_lang_quen(mut mau: Vec<MauNguCanh>) -> Vec<MauNguCanh> {
    mau.sort_by(|a, b| {
        b.lien_quan.partial_cmp(&a.lien_quan).unwrap_or(std::cmp::Ordering::Equal)
            .then(a.nhan.cmp(&b.nhan))
    });
    let mut dau: Vec<MauNguCanh> = Vec::new();
    let mut cuoi: Vec<MauNguCanh> = Vec::new();
    for (i, m) in mau.into_iter().enumerate() {
        if i % 2 == 0 { dau.push(m) } else { cuoi.push(m) }
    }
    cuoi.reverse();
    dau.extend(cuoi);
    dau
}

// ============================================================================
// PHẦN 2: HARNESS ENGINEERING — ĐỊNH NGHĨA KHÔNG GIAN HÀNH ĐỘNG CỦA TÁC TỬ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum KetQuaCongCu {
    Xong(String),
    Loi(String),
}

/// Một CÔNG CỤ mà tác tử được phép gọi. Đây chính là "harness":
/// bạn định nghĩa tác tử ĐƯỢC LÀM GÌ, và mọi thứ khác đều bị cấm.
pub trait CongCu {
    fn ten(&self) -> &str;
    fn mo_ta(&self) -> &str;
    fn chay(&self, tham_so: &str) -> KetQuaCongCu;
}

pub struct CongCuTinhToan;
impl CongCu for CongCuTinhToan {
    fn ten(&self) -> &str { "tinh_tong" }
    fn mo_ta(&self) -> &str { "Cộng các số cách nhau bởi dấu phẩy. Ví dụ: \"3,4,5\"" }
    fn chay(&self, tham_so: &str) -> KetQuaCongCu {
        let mut tong: i64 = 0;
        for phan in tham_so.split(',') {
            match phan.trim().parse::<i64>() {
                Ok(n) => tong += n,
                Err(_) => return KetQuaCongCu::Loi(format!("{:?} không phải số", phan.trim())),
            }
        }
        KetQuaCongCu::Xong(tong.to_string())
    }
}

pub struct CongCuTraCuu {
    pub kho: HashMap<String, String>,
}
impl CongCu for CongCuTraCuu {
    fn ten(&self) -> &str { "tra_cuu" }
    fn mo_ta(&self) -> &str { "Tra cứu định nghĩa một thuật ngữ trong kho tri thức." }
    fn chay(&self, tham_so: &str) -> KetQuaCongCu {
        match self.kho.get(tham_so.trim()) {
            Some(v) => KetQuaCongCu::Xong(v.clone()),
            None => KetQuaCongCu::Loi(format!("Không tìm thấy {:?}", tham_so.trim())),
        }
    }
}

/// Bộ khung (harness) giữ danh mục công cụ và ÁP ĐẶT GIỚI HẠN.
pub struct BoKhung {
    cong_cu: Vec<Box<dyn CongCu>>,
    pub so_lan_goi_toi_da: usize,
}

impl BoKhung {
    pub fn moi(so_lan_goi_toi_da: usize) -> Self {
        BoKhung { cong_cu: Vec::new(), so_lan_goi_toi_da }
    }
    pub fn dang_ky(mut self, cc: Box<dyn CongCu>) -> Self {
        self.cong_cu.push(cc);
        self
    }
    /// Bản mô tả công cụ để nhét vào ngữ cảnh — đây là "giao diện" tác tử nhìn thấy.
    pub fn mo_ta_cong_cu(&self) -> String {
        self.cong_cu.iter()
            .map(|c| format!("- {}: {}", c.ten(), c.mo_ta()))
            .collect::<Vec<_>>().join("\n")
    }
    pub fn goi(&self, ten: &str, tham_so: &str) -> KetQuaCongCu {
        match self.cong_cu.iter().find(|c| c.ten() == ten) {
            Some(c) => c.chay(tham_so),
            None => KetQuaCongCu::Loi(format!("Công cụ {:?} không tồn tại trong bộ khung", ten)),
        }
    }
}

// ============================================================================
// PHẦN 3: LOOP ENGINEERING — VÒNG LẶP TÁC TỬ CÓ ĐIỀU KIỆN DỪNG
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum HanhDong {
    GoiCongCu { ten: String, tham_so: String },
    TraLoi(String),
}

/// Bộ não của tác tử. Trong thực tế đây là lời gọi tới mô hình ngôn ngữ;
/// ở đây ta dùng một bản GIẢ TẤT ĐỊNH để chương trình kiểm thử được.
pub trait BoNao {
    fn quyet_dinh(&self, nhiem_vu: &str, lich_su: &[String]) -> HanhDong;
}

#[derive(Debug, PartialEq)]
pub enum LyDoDung {
    HoanThanh,
    HetLuotGoi,
    LapVoHan,
}

#[derive(Debug, PartialEq)]
pub struct KetQuaVongLap {
    pub tra_loi: Option<String>,
    pub so_buoc: usize,
    pub ly_do_dung: LyDoDung,
    pub nhat_ky: Vec<String>,
}

/// LOOP ENGINEERING: vòng lặp tác tử với BA điều kiện dừng bắt buộc.
/// Một vòng lặp thiếu điều kiện dừng là một hóa đơn API không giới hạn.
pub fn chay_vong_lap(nhiem_vu: &str, nao: &dyn BoNao, khung: &BoKhung) -> KetQuaVongLap {
    let mut lich_su: Vec<String> = Vec::new();
    let mut da_thay: HashSet<String> = HashSet::new();

    for buoc in 1..=khung.so_lan_goi_toi_da {
        match nao.quyet_dinh(nhiem_vu, &lich_su) {
            HanhDong::TraLoi(t) => {
                lich_su.push(format!("[{}] TRẢ LỜI: {}", buoc, t));
                return KetQuaVongLap {
                    tra_loi: Some(t), so_buoc: buoc,
                    ly_do_dung: LyDoDung::HoanThanh, nhat_ky: lich_su,
                };
            }
            HanhDong::GoiCongCu { ten, tham_so } => {
                // DỪNG #3: phát hiện lặp vô hạn (gọi y hệt lần trước)
                let dau_van_tay = format!("{}::{}", ten, tham_so);
                if !da_thay.insert(dau_van_tay.clone()) {
                    lich_su.push(format!("[{}] PHÁT HIỆN LẶP: {}", buoc, dau_van_tay));
                    return KetQuaVongLap {
                        tra_loi: None, so_buoc: buoc,
                        ly_do_dung: LyDoDung::LapVoHan, nhat_ky: lich_su,
                    };
                }
                let kq = khung.goi(&ten, &tham_so);
                lich_su.push(match kq {
                    KetQuaCongCu::Xong(v) => format!("[{}] {}({}) -> {}", buoc, ten, tham_so, v),
                    KetQuaCongCu::Loi(e) => format!("[{}] {}({}) -> LỖI: {}", buoc, ten, tham_so, e),
                });
            }
        }
    }
    // DỪNG #2: hết ngân sách lượt gọi
    KetQuaVongLap {
        tra_loi: None, so_buoc: khung.so_lan_goi_toi_da,
        ly_do_dung: LyDoDung::HetLuotGoi, nhat_ky: lich_su,
    }
}

// ============================================================================
// PHẦN 4: GRAPH ENGINEERING — ĐỒ THỊ TRI THỨC & TRUY XUẤT NHIỀU BƯỚC
// ============================================================================

/// Đồ thị tri thức: các thực thể nối với nhau bằng quan hệ có nhãn.
/// Đây là nền của GraphRAG — truy xuất theo QUAN HỆ, không chỉ theo từ khóa.
pub struct DoThiTriThuc {
    canh: HashMap<String, Vec<(String, String)>>, // đỉnh -> [(nhãn quan hệ, đỉnh đích)]
    mo_ta: HashMap<String, String>,
}

impl DoThiTriThuc {
    pub fn moi() -> Self {
        DoThiTriThuc { canh: HashMap::new(), mo_ta: HashMap::new() }
    }
    pub fn them_thuc_the(&mut self, ten: &str, mo_ta: &str) {
        self.mo_ta.insert(ten.to_string(), mo_ta.to_string());
        self.canh.entry(ten.to_string()).or_default();
    }
    pub fn them_quan_he(&mut self, tu: &str, nhan: &str, den: &str) {
        self.canh.entry(tu.to_string()).or_default()
            .push((nhan.to_string(), den.to_string()));
    }

    /// Truy xuất nhiều bước: từ một điểm xuất phát, đi tối đa `do_sau` bước
    /// để gom ngữ cảnh liên quan. Đây là điểm khác biệt so với tìm kiếm phẳng.
    pub fn truy_xuat_lan_toa(&self, bat_dau: &str, do_sau: usize) -> Vec<String> {
        let mut ket_qua = Vec::new();
        let mut da_tham: HashSet<String> = HashSet::new();
        let mut hang_doi: VecDeque<(String, usize)> = VecDeque::new();

        hang_doi.push_back((bat_dau.to_string(), 0));
        da_tham.insert(bat_dau.to_string());

        while let Some((dinh, sau)) = hang_doi.pop_front() {
            if let Some(m) = self.mo_ta.get(&dinh) {
                ket_qua.push(format!("{}: {}", dinh, m));
            }
            if sau >= do_sau { continue; }
            if let Some(lang_gieng) = self.canh.get(&dinh) {
                let mut sx = lang_gieng.clone();
                sx.sort(); // tất định
                for (nhan, den) in sx {
                    if da_tham.insert(den.clone()) {
                        ket_qua.push(format!("  ({} --{}--> {})", dinh, nhan, den));
                        hang_doi.push_back((den, sau + 1));
                    }
                }
            }
        }
        ket_qua
    }
}

// ============================================================================
// PHẦN 5: BỘ NÃO GIẢ TẤT ĐỊNH (test double cho mô hình ngôn ngữ)
// ============================================================================

/// Bộ não giả: quyết định dựa trên luật cố định, nên chương trình TẤT ĐỊNH
/// và kiểm thử được — không cần khóa API, không cần mạng.
pub struct BoNaoGia {
    pub kich_ban: Vec<HanhDong>,
}
impl BoNao for BoNaoGia {
    fn quyet_dinh(&self, _nhiem_vu: &str, lich_su: &[String]) -> HanhDong {
        self.kich_ban
            .get(lich_su.len())
            .cloned()
            .unwrap_or_else(|| HanhDong::TraLoi("Hết kịch bản".to_string()))
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ NGỮ CẢNH · BỘ KHUNG · VÒNG LẶP · ĐỒ THỊ TRI THỨC    ");
    println!("═══════════════════════════════════════════════════════════════");

    // ---- 1. CONTEXT ENGINEERING ----
    println!("\n1. KỸ NGHỆ NGỮ CẢNH — nhồi 4000 token vào cửa sổ 1000 token");
    let mau = vec![
        MauNguCanh { nhan: "quy_tac_an_toan".into(), noi_dung: "Không tiết lộ khóa bí mật".into(), token: 50, lien_quan: 0.3, ghim: true },
        MauNguCanh { nhan: "tai_lieu_A".into(), noi_dung: "...".into(), token: 800, lien_quan: 0.9, ghim: false },
        MauNguCanh { nhan: "tai_lieu_B".into(), noi_dung: "...".into(), token: 200, lien_quan: 0.85, ghim: false },
        MauNguCanh { nhan: "tai_lieu_C".into(), noi_dung: "...".into(), token: 2000, lien_quan: 0.95, ghim: false },
        MauNguCanh { nhan: "lich_su_chat_cu".into(), noi_dung: "...".into(), token: 900, lien_quan: 0.1, ghim: false },
    ];
    let goi = dong_goi_ngu_canh(mau, 1000);
    println!("   Dùng {} / 1000 token, loại bỏ {} mẩu", goi.tong_token, goi.so_mau_bi_loai);
    for m in &goi.cac_mau {
        println!("     [{:>4} tok · lq {:.2}{}] {}", m.token, m.lien_quan,
                 if m.ghim { " · GHIM" } else { "" }, m.nhan);
    }
    println!("   → tai_lieu_C (2000 tok) bị loại dù liên quan cao nhất: KHÔNG VỪA ngân sách.");
    println!("   → Thứ tự đã đảo để mẩu quan trọng nằm ở ĐẦU và CUỐI (chống Lost-in-the-Middle).");

    // ---- 2 & 3. HARNESS + LOOP ----
    println!("\n2-3. BỘ KHUNG & VÒNG LẶP TÁC TỬ");
    let mut kho = HashMap::new();
    kho.insert("Rust".to_string(), "Ngôn ngữ hệ thống an toàn bộ nhớ".to_string());
    let khung = BoKhung::moi(5)
        .dang_ky(Box::new(CongCuTinhToan))
        .dang_ky(Box::new(CongCuTraCuu { kho }));
    println!("   Công cụ tác tử được phép dùng:\n{}", khung.mo_ta_cong_cu());

    let nao = BoNaoGia { kich_ban: vec![
        HanhDong::GoiCongCu { ten: "tra_cuu".into(), tham_so: "Rust".into() },
        HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "10,20,12".into() },
        HanhDong::TraLoi("Rust là ngôn ngữ hệ thống; tổng là 42.".into()),
    ]};
    let kq = chay_vong_lap("Tra cứu Rust rồi cộng 10+20+12", &nao, &khung);
    for d in &kq.nhat_ky { println!("   {}", d); }
    println!("   Dừng vì: {:?} sau {} bước", kq.ly_do_dung, kq.so_buoc);

    // Vòng lặp hỏng: tác tử lặp mãi một lời gọi
    let nao_ket = BoNaoGia { kich_ban: vec![
        HanhDong::GoiCongCu { ten: "tra_cuu".into(), tham_so: "X".into() },
        HanhDong::GoiCongCu { ten: "tra_cuu".into(), tham_so: "X".into() },
    ]};
    let kq2 = chay_vong_lap("nhiệm vụ hỏng", &nao_ket, &khung);
    println!("   [Tác tử kẹt] dừng vì: {:?} sau {} bước", kq2.ly_do_dung, kq2.so_buoc);

    // ---- 4. GRAPH ENGINEERING ----
    println!("\n4. ĐỒ THỊ TRI THỨC — truy xuất lan tỏa 2 bước");
    let mut g = DoThiTriThuc::moi();
    g.them_thuc_the("DonHang", "Đơn hàng của khách");
    g.them_thuc_the("KhachHang", "Người mua");
    g.them_thuc_the("ThanhToan", "Giao dịch trừ tiền");
    g.them_thuc_the("VanDon", "Phiếu giao hàng");
    g.them_thuc_the("Kho", "Kho hàng vật lý");
    g.them_quan_he("DonHang", "thuoc_ve", "KhachHang");
    g.them_quan_he("DonHang", "duoc_tra_boi", "ThanhToan");
    g.them_quan_he("DonHang", "sinh_ra", "VanDon");
    g.them_quan_he("VanDon", "xuat_tu", "Kho");
    for dong in g.truy_xuat_lan_toa("DonHang", 2) {
        println!("   {}", dong);
    }
    println!("   → Tìm kiếm từ khóa thường sẽ BỎ SÓT \"Kho\" vì nó cách 2 bước.");

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("  NGỮ CẢNH LÀ TÀI NGUYÊN · VÒNG LẶP PHẢI CÓ PHANH · CÔNG CỤ LÀ HỢP ĐỒNG ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn mau(nhan: &str, token: usize, lq: f64, ghim: bool) -> MauNguCanh {
        MauNguCanh { nhan: nhan.into(), noi_dung: "x".into(), token, lien_quan: lq, ghim }
    }

    #[test]
    fn ngu_canh_khong_bao_gio_vuot_ngan_sach() {
        let ds = vec![mau("a", 400, 0.9, false), mau("b", 400, 0.8, false), mau("c", 400, 0.7, false)];
        let g = dong_goi_ngu_canh(ds, 1000);
        assert!(g.tong_token <= 1000, "vượt ngân sách: {}", g.tong_token);
        assert_eq!(g.cac_mau.len(), 2);
    }

    #[test]
    fn mau_ghim_luon_duoc_giu() {
        let ds = vec![
            mau("quy_tac", 100, 0.01, true),  // liên quan cực thấp nhưng GHIM
            mau("to", 900, 0.99, false),
        ];
        let g = dong_goi_ngu_canh(ds, 1000);
        assert!(g.cac_mau.iter().any(|m| m.nhan == "quy_tac"), "mẩu ghim bị loại!");
    }

    #[test]
    fn uu_tien_mat_do_gia_tri_chu_khong_phai_diem_tho() {
        // "nho" có điểm thấp hơn nhưng mật độ (lq/token) cao hơn nhiều
        let ds = vec![mau("to", 900, 0.9, false), mau("nho", 90, 0.5, false)];
        let g = dong_goi_ngu_canh(ds, 500);
        assert_eq!(g.cac_mau.len(), 1);
        assert_eq!(g.cac_mau[0].nhan, "nho");
    }

    #[test]
    fn chong_lang_quen_dat_quan_trong_o_hai_dau() {
        let ds = vec![mau("a", 1, 0.9, false), mau("b", 1, 0.5, false), mau("c", 1, 0.8, false)];
        let sx = sap_xep_chong_lang_quen(ds);
        // xếp giảm dần: a(.9) c(.8) b(.5) -> chẵn ra đầu, lẻ ra cuối (đảo): a, b, c
        assert_eq!(sx.first().unwrap().nhan, "a");
        assert_eq!(sx.last().unwrap().nhan, "c");
    }

    #[test]
    fn cong_cu_tra_loi_dung_va_bao_loi_ro_rang() {
        let cc = CongCuTinhToan;
        assert_eq!(cc.chay("1,2,3"), KetQuaCongCu::Xong("6".into()));
        assert!(matches!(cc.chay("1,x"), KetQuaCongCu::Loi(_)));
    }

    #[test]
    fn bo_khung_tu_choi_cong_cu_ngoai_danh_muc() {
        let khung = BoKhung::moi(3).dang_ky(Box::new(CongCuTinhToan));
        // Tác tử KHÔNG THỂ gọi thứ không được đăng ký — đây là ranh giới an toàn.
        assert!(matches!(khung.goi("xoa_o_cung", "/"), KetQuaCongCu::Loi(_)));
    }

    #[test]
    fn vong_lap_dung_khi_hoan_thanh() {
        let khung = BoKhung::moi(5).dang_ky(Box::new(CongCuTinhToan));
        let nao = BoNaoGia { kich_ban: vec![
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "40,2".into() },
            HanhDong::TraLoi("42".into()),
        ]};
        let kq = chay_vong_lap("nv", &nao, &khung);
        assert_eq!(kq.ly_do_dung, LyDoDung::HoanThanh);
        assert_eq!(kq.tra_loi, Some("42".to_string()));
        assert_eq!(kq.so_buoc, 2);
    }

    #[test]
    fn vong_lap_dung_khi_het_luot_goi() {
        let khung = BoKhung::moi(3).dang_ky(Box::new(CongCuTinhToan));
        // Bộ não không bao giờ trả lời, chỉ gọi công cụ với tham số KHÁC nhau
        let nao = BoNaoGia { kich_ban: vec![
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "1".into() },
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "2".into() },
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "3".into() },
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "4".into() },
        ]};
        let kq = chay_vong_lap("nv", &nao, &khung);
        assert_eq!(kq.ly_do_dung, LyDoDung::HetLuotGoi);
        assert_eq!(kq.so_buoc, 3, "phải dừng đúng ở ngân sách 3 lượt");
    }

    #[test]
    fn vong_lap_phat_hien_tac_tu_bi_ket() {
        let khung = BoKhung::moi(50).dang_ky(Box::new(CongCuTinhToan));
        let nao = BoNaoGia { kich_ban: vec![
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "1".into() },
            HanhDong::GoiCongCu { ten: "tinh_tong".into(), tham_so: "1".into() }, // y hệt
        ]};
        let kq = chay_vong_lap("nv", &nao, &khung);
        assert_eq!(kq.ly_do_dung, LyDoDung::LapVoHan);
        assert!(kq.so_buoc < 50, "phải dừng SỚM, không chạy hết 50 lượt");
    }

    #[test]
    fn do_thi_truy_xuat_dung_do_sau() {
        let mut g = DoThiTriThuc::moi();
        g.them_thuc_the("A", "a"); g.them_thuc_the("B", "b");
        g.them_thuc_the("C", "c"); g.them_thuc_the("D", "d");
        g.them_quan_he("A", "r1", "B");
        g.them_quan_he("B", "r2", "C");
        g.them_quan_he("C", "r3", "D");

        let sau1 = g.truy_xuat_lan_toa("A", 1);
        assert!(sau1.iter().any(|s| s.starts_with("B:")));
        assert!(!sau1.iter().any(|s| s.starts_with("C:")), "độ sâu 1 không được tới C");

        let sau2 = g.truy_xuat_lan_toa("A", 2);
        assert!(sau2.iter().any(|s| s.starts_with("C:")), "độ sâu 2 phải tới được C");
        assert!(!sau2.iter().any(|s| s.starts_with("D:")));
    }

    #[test]
    fn do_thi_khong_lap_vo_han_khi_co_chu_trinh() {
        let mut g = DoThiTriThuc::moi();
        g.them_thuc_the("A", "a"); g.them_thuc_the("B", "b");
        g.them_quan_he("A", "r", "B");
        g.them_quan_he("B", "r", "A"); // chu trình
        let kq = g.truy_xuat_lan_toa("A", 10);
        assert!(kq.len() < 10, "phải dừng nhờ tập đã thăm, không lặp vô hạn");
    }
}
```

---

## Từ mô hình giả tới mô hình thật

Mã trên dùng `BoNaoGia` để mọi thứ tất định và kiểm thử được. Khi nối vào mô hình thật, bạn **chỉ thay đúng một cài đặt trait**:

```rust
pub struct BoNaoThat { pub khoa_api: String, pub mo_hinh: String }

impl BoNao for BoNaoThat {
    fn quyet_dinh(&self, nhiem_vu: &str, lich_su: &[String]) -> HanhDong {
        // 1. Dựng ngữ cảnh bằng `dong_goi_ngu_canh` (tôn trọng ngân sách token)
        // 2. Gửi HTTP tới nhà cung cấp mô hình (reqwest + serde_json)
        // 3. Phân tích phản hồi thành HanhDong::GoiCongCu hoặc HanhDong::TraLoi
        todo!("gọi mô hình thật")
    }
}
```

Toàn bộ phần còn lại — bộ khung, vòng lặp, đồ thị, và **tất cả bài kiểm thử** — giữ nguyên không đổi. Đó chính là lợi ích của việc đặt ranh giới bằng `trait` (Chương 12) và tiêm phụ thuộc (Chương 14).

**Hệ sinh thái Rust cho AI** đáng theo dõi:

| Crate | Vai trò |
|---|---|
| [`rig`](https://github.com/0xPlaygrounds/rig) | Khung xây tác tử LLM: nhà cung cấp mô hình, công cụ, RAG, kho vector |
| `async-openai` / `anthropic-sdk` | Client cho từng nhà cung cấp |
| `qdrant-client`, `lancedb` | Kho vector cho truy xuất theo độ tương tự |
| `tiktoken-rs` | Đếm token chính xác — cần cho `dong_goi_ngu_canh` phiên bản thật |
| `tokio` + `reqwest` | Bất đồng bộ và HTTP (Chương 49) |

> **Vì sao dùng Rust cho tác tử AI?** Ba lý do rất thực tế: (1) một tiến trình tác tử Rust tốn ~15MB RAM thay vì 500MB, quan trọng khi chạy hàng nghìn tác tử song song (Chương 48); (2) hệ thống kiểu biến "công cụ" thành hợp đồng kiểm tra được lúc biên dịch, thay vì dictionary lỏng lẻo; (3) `tokio` cho phép chạy hàng nghìn tác tử đồng thời trên một máy (Chương 49).

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Ngữ cảnh là tài nguyên có hạn** — bài toán chọn ngữ cảnh là bài toán xếp ba lô, ưu tiên theo *mật độ giá trị* chứ không theo điểm thô. Ghim cứng những gì không được phép mất.
2. **Bộ khung là hợp đồng kiểu**: tác tử chỉ làm được những gì `trait CongCu` cho phép. Danh sách trắng, lỗi có nội dung, mô tả rõ ràng.
3. **Vòng lặp phải có ba cái phanh**: hoàn thành, hết ngân sách, phát hiện lặp. Thiếu cái thứ ba là thiếu cái quan trọng nhất.
4. **Đồ thị tri thức giải được lớp câu hỏi nhiều bước** mà tìm kiếm theo độ tương tự bó tay — nhưng bắt buộc phải giới hạn độ sâu và giữ tập đã thăm.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Công cụ mới trong bộ khung)**
Viết `CongCuThoiTiet` trả về nhiệt độ cho một thành phố từ một `HashMap` cố định, và trả lỗi rõ ràng cho thành phố không có. Đăng ký vào `BoKhung` rồi viết test chứng minh tác tử gọi được nó.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct CongCuThoiTiet { pub du_lieu: HashMap<String, i32> }

impl CongCu for CongCuThoiTiet {
    fn ten(&self) -> &str { "thoi_tiet" }
    fn mo_ta(&self) -> &str { "Trả về nhiệt độ (°C) của một thành phố. Ví dụ: \"Hà Nội\"" }
    fn chay(&self, tham_so: &str) -> KetQuaCongCu {
        match self.du_lieu.get(tham_so.trim()) {
            Some(t) => KetQuaCongCu::Xong(format!("{}°C", t)),
            None => KetQuaCongCu::Loi(format!("Chưa có dữ liệu cho {:?}", tham_so.trim())),
        }
    }
}

#[cfg(test)]
mod bt1 {
    use super::*;
    #[test]
    fn tac_tu_goi_duoc_cong_cu_thoi_tiet() {
        let mut dl = HashMap::new();
        dl.insert("Hà Nội".to_string(), 28);
        let khung = BoKhung::moi(3).dang_ky(Box::new(CongCuThoiTiet { du_lieu: dl }));
        assert_eq!(khung.goi("thoi_tiet", "Hà Nội"), KetQuaCongCu::Xong("28°C".into()));
        assert!(matches!(khung.goi("thoi_tiet", "Sao Hỏa"), KetQuaCongCu::Loi(_)));
    }
}
```
</details>

**Bài tập 2 (Phanh thứ tư: ngân sách token)**
Thêm vào `BoKhung` một trường `token_toi_da: usize` và vào `KetQuaVongLap` một biến thể `LyDoDung::HetToken`. Mỗi lượt gọi công cụ cộng dồn độ dài kết quả vào bộ đếm; vượt ngưỡng thì dừng.

<details>
<summary><b>Gợi ý</b></summary>

Đây là *cái phanh mà các đội thực chiến hay quên nhất*: tác tử có thể dừng đúng 5 lượt nhưng mỗi lượt kéo về 100.000 token. Đếm lượt là chưa đủ, phải đếm cả **khối lượng**.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
// Thêm vào enum:  LyDoDung::HetToken
// Trong chay_vong_lap, sau mỗi lời gọi công cụ:
//     da_dung_token += ket_qua_text.len() / 4;   // xấp xỉ: 4 ký tự ~ 1 token
//     if da_dung_token > khung.token_toi_da {
//         return KetQuaVongLap { tra_loi: None, so_buoc: buoc,
//                                ly_do_dung: LyDoDung::HetToken, nhat_ky: lich_su };
//     }
```

Trong sản phẩm thật, thay phép chia 4 bằng `tiktoken-rs` để đếm token chính xác theo đúng bộ mã hóa của mô hình.
</details>

**Bài tập 3 (Tư duy: chọn chiến lược truy xuất)**
Với mỗi câu hỏi, chọn **tìm kiếm theo độ tương tự (RAG phẳng)** hay **truy xuất theo đồ thị (GraphRAG)**, và giải thích:
1. "Chính sách đổi trả hàng của công ty là gì?"
2. "Đơn hàng ORD-88 do nhân viên nào ở kho nào xử lý?"
3. "Tóm tắt các khiếu nại về sản phẩm tai nghe."
4. "Nếu nhà cung cấp X ngừng hoạt động, những đơn hàng nào bị ảnh hưởng?"

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **RAG phẳng.** Câu trả lời nằm gọn trong một tài liệu chính sách; tương tự ngữ nghĩa là đủ.
2. **GraphRAG.** Phải đi nhiều bước quan hệ: `Đơn hàng → Vận đơn → Kho → Nhân viên`. Không tài liệu đơn lẻ nào chứa cả chuỗi này.
3. **RAG phẳng** (có lọc). Gom nhiều tài liệu tương tự rồi tóm tắt — đúng thế mạnh của tìm kiếm theo vector.
4. **GraphRAG.** Đây là câu hỏi *lan tỏa ngược*: `Nhà cung cấp → Sản phẩm → Đơn hàng`. Tìm kiếm tương tự sẽ trả về tài liệu *nói về* nhà cung cấp X, chứ không liệt kê được các đơn hàng bị ảnh hưởng.

**Quy tắc rút ra**: nếu câu hỏi chứa chữ *"nào"*, *"ảnh hưởng"*, *"liên quan tới"* và câu trả lời đòi bắc cầu qua nhiều thực thể — hãy nghĩ tới đồ thị. Nếu câu trả lời nằm gọn trong một đoạn văn — RAG phẳng nhanh hơn và rẻ hơn nhiều.
</details>
