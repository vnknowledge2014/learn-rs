# Chương 62: Phát triển Frontend với Rust & WebAssembly — Hệ phản ứng & Virtual DOM (Frontend Development)

## Giới thiệu & Mục tiêu học tập

Rust không chỉ chạy trên máy chủ. Nhờ **WebAssembly (WASM)** — một định dạng nhị phân chạy được trong mọi trình duyệt với tốc độ gần bằng mã máy — Rust có thể viết **giao diện web** chạy ngay trong trình duyệt, thay cho (hoặc cùng với) JavaScript. Các framework như [Leptos](https://leptos.dev/), [Yew](https://yew.rs/), [Dioxus](https://dioxuslabs.com/) đang biến điều này thành hiện thực, với hiệu năng vượt trội và an toàn kiểu ngay ở tầng giao diện.

Nhưng đằng sau mọi framework UI hiện đại — dù Leptos, React, Solid, hay Svelte — chỉ có **hai ý tưởng cốt lõi**:
1. **Hệ phản ứng (Reactivity)**: khi trạng thái đổi, giao diện *tự động* cập nhật. Bạn không phải viết mã "khi số đổi thì tìm thẻ h1 và sửa nội dung".
2. **Virtual DOM + Diff**: giao diện được mô tả bằng một cây ảo nhẹ; framework so cây mới với cây cũ và chỉ cập nhật *phần thay đổi* lên DOM thật (vốn rất tốn kém).

Chương này xây cả hai từ đầu — chạy offline, kiểm thử đầy đủ — để bạn hiểu *cơ chế* trước khi dùng framework thật. Rồi chỉ ra mã Leptos tương ứng và cách kết hợp Rust với **Tauri 2.0 + Svelte** cho ứng dụng đa nền tảng.

Mục tiêu học tập:
- Hiểu **tín hiệu (signal)** và **giá trị dẫn xuất (derived)** — nền của reactivity.
- Nắm **Virtual DOM** và thuật toán **diff** — vì sao nó làm UI nhanh.
- Thấy **giao diện là hàm thuần túy của trạng thái** (declarative UI) — đúng tinh thần Chương 13.
- Chống **XSS ngay trong tầng kết xuất** (nối với Chương 57).
- Biết hệ sinh thái frontend Rust: Leptos, Yew, Dioxus, và mô hình Tauri + Svelte.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: BẢNG ĐIỆN TỬ SÂN BAY VÀ NGƯỜI SỬA BIỂN THỦ CÔNG              │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  CÁCH CŨ (thao tác DOM thủ công, như jQuery):                                    │
│    Chuyến bay đổi giờ → bạn TỰ đi tìm đúng dòng, TỰ xóa chữ cũ, TỰ viết chữ mới. │
│    Sân bay có 500 chuyến → 500 lần tìm-xóa-viết. Dễ sai, dễ sót.                 │
│                                                                                  │
│  HỆ PHẢN ỨNG (signals):                                                          │
│    Bạn chỉ cập nhật DỮ LIỆU ("chuyến VN123 giờ mới = 14:30"). Cái bảng          │
│    TỰ ĐỘNG hiện đúng — vì mỗi ô đã "đăng ký lắng nghe" dữ liệu của nó.          │
│                                                                                  │
│  VIRTUAL DOM + DIFF:                                                             │
│    Thay vì thay cả tấm bảng mỗi lần, hệ thống VẼ RA tấm bảng mới trên giấy       │
│    nháp (rẻ), SO với tấm đang treo, rồi chỉ dán đè ĐÚNG những ô khác nhau        │
│    lên bảng thật (đắt). Đổi 1 chuyến → chỉ sửa 1 ô, không đụng 499 ô kia.        │
│                                                                                  │
│  GIAO DIỆN = HÀM CỦA TRẠNG THÁI:                                                 │
│    "Tấm bảng phải trông thế nào?" = một HÀM của "danh sách chuyến bay hiện tại". │
│    Bạn mô tả KẾT QUẢ mong muốn, framework lo cách đạt được nó (khai báo).        │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Hệ phản ứng — tín hiệu và giá trị dẫn xuất

**Tín hiệu (signal)** là một ô trạng thái "thông minh": đọc được, ghi được, và khi ghi thì *thông báo* cho những ai phụ thuộc vào nó. Trong mã dưới, `TinHieu<T>` dùng `Rc<RefCell<T>>` (khả biến nội tại, Chương 27) và một số **phiên bản** tăng mỗi lần đổi.

Một chi tiết tối ưu quan trọng: `dat` chỉ tăng phiên bản khi giá trị **thực sự khác** — nhờ vậy đặt lại cùng giá trị không kích hoạt render thừa. Đây là lý do framework hiện đại nhanh: chúng làm càng ít việc càng tốt.

**Giá trị dẫn xuất (derived/computed)** tự tính lại từ các tín hiệu nguồn. "Tổng tiền" dẫn xuất từ "giỏ hàng": đổi giỏ, tổng tự đúng. Bạn không bao giờ phải nhớ "khi sửa giỏ thì cập nhật tổng" — đây chính là *minh bạch tham chiếu* (Chương 13) ở tầng giao diện.

### 2. Virtual DOM — vì sao cần một tầng ảo

Thao tác lên DOM thật của trình duyệt **rất tốn kém** (kích hoạt tính toán lại bố cục, vẽ lại). Nếu mỗi lần trạng thái đổi mà dựng lại toàn bộ DOM, giao diện sẽ giật.

Giải pháp: mô tả giao diện bằng một **cây ảo nhẹ** (`NutAo` — chỉ là struct/enum trong bộ nhớ, dựng cực rẻ). Khi trạng thái đổi, dựng cây ảo *mới*, **so** với cây *cũ* bằng thuật toán **diff**, rồi chỉ áp những **bản vá tối thiểu** lên DOM thật.

### 3. Thuật toán Diff — trái tim của tốc độ

`diff` so hai cây và sinh danh sách bản vá:
- Hai **văn bản** khác nội dung → một bản vá "đổi văn bản".
- Hai **thẻ cùng tên** → so thuộc tính và đệ quy so các con.
- **Khác loại/khác tên thẻ** → thay thế cả nút.

Điểm đắt giá minh họa trong test `diff_component_dem_chi_va_van_ban`: khi bộ đếm đổi từ 3 sang 4, chỉ có **một** bản vá (đổi nội dung `<h1>`) — hai nút `<button>` giữ nguyên, không bị dựng lại. Đây chính xác là điều làm React/Leptos mượt: **cập nhật ngoại khoa, không phẫu thuật toàn thân**.

> **Ghi chú**: thuật toán diff ở đây so con *theo vị trí* (O(n)). Framework thật dùng thêm *khóa (key)* để nhận diện phần tử khi danh sách được sắp xếp lại — nếu không, đảo thứ tự một danh sách sẽ sinh nhiều bản vá không cần thiết. Đây là lý do React cảnh báo "mỗi phần tử trong list cần một `key` duy nhất".

### 4. Giao diện là hàm thuần túy của trạng thái

`view_dem` là một **hàm thuần túy**: nhận trạng thái, trả về cây ảo. Không tác dụng phụ, không thao tác DOM trực tiếp. Đây là **UI khai báo (declarative)**: bạn mô tả *giao diện trông thế nào ứng với trạng thái này*, framework lo phần *làm sao đạt được nó*.

So sánh với UI **mệnh lệnh** (jQuery): "tìm thẻ #dem, xóa chữ, viết chữ mới". Khai báo thắng vì cùng một lý do lập trình hàm thắng lập trình mệnh lệnh (Chương 13): ít trạng thái ẩn hơn, dễ suy luận hơn, ít lỗi hơn.

### 5. Chống XSS ngay trong kết xuất

`thanh_html` **thoát ký tự** mọi văn bản trước khi nhúng (Chương 57). Đây là "an toàn theo mặc định": framework thoát tự động, bạn phải *chủ động* yêu cầu "tin tưởng" thì nó mới không thoát. Nhờ vậy XSS lưu trữ — kẻ tấn công nhét `<script>` vào bình luận — bị chặn ngay ở tầng render, không cần lập trình viên nhớ thoát thủ công.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch62
cargo test -p ch62
```

```rust
//! Chương 62 — Frontend & WASM: hai trái tim của framework UI hiện đại —
//! HỆ PHẢN ỨNG (signals) và VIRTUAL DOM (diff). Lõi thuần túy, kiểm thử được.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// ============================================================================
// 1. HỆ PHẢN ỨNG (Reactivity) — nền của Leptos/SolidJS/Svelte
// ============================================================================

/// Một "tín hiệu" (signal): ô trạng thái mà khi thay đổi sẽ tự động thông báo
/// cho những ai đang lắng nghe. Đây là cơ chế khiến UI tự cập nhật khi dữ liệu đổi.
#[derive(Clone)]
pub struct TinHieu<T> {
    gia_tri: Rc<RefCell<T>>,
    phien_ban: Rc<RefCell<u64>>, // tăng mỗi lần đặt giá trị -> phát hiện thay đổi
}

impl<T: Clone + PartialEq> TinHieu<T> {
    pub fn moi(gia_tri: T) -> Self {
        TinHieu {
            gia_tri: Rc::new(RefCell::new(gia_tri)),
            phien_ban: Rc::new(RefCell::new(0)),
        }
    }
    pub fn lay(&self) -> T {
        self.gia_tri.borrow().clone()
    }
    /// Đặt giá trị mới. Chỉ tăng phiên bản nếu giá trị THỰC SỰ đổi (tránh render thừa).
    pub fn dat(&self, moi: T) {
        if *self.gia_tri.borrow() != moi {
            *self.gia_tri.borrow_mut() = moi;
            *self.phien_ban.borrow_mut() += 1;
        }
    }
    pub fn cap_nhat(&self, f: impl FnOnce(&T) -> T) {
        let moi = f(&self.gia_tri.borrow());
        self.dat(moi);
    }
    pub fn phien_ban(&self) -> u64 {
        *self.phien_ban.borrow()
    }
}

/// Giá trị DẪN XUẤT (derived/computed): tự tính lại từ các tín hiệu nguồn.
/// Ví dụ: "tổng tiền" dẫn xuất từ "giỏ hàng". Đổi giỏ -> tổng tự cập nhật.
pub struct DanXuat<T> {
    tinh: Box<dyn Fn() -> T>,
}
impl<T> DanXuat<T> {
    pub fn moi(tinh: impl Fn() -> T + 'static) -> Self {
        DanXuat { tinh: Box::new(tinh) }
    }
    pub fn lay(&self) -> T {
        (self.tinh)()
    }
}

// ============================================================================
// 2. VIRTUAL DOM — cây mô tả giao diện, và thuật toán DIFF
// ============================================================================

/// Một nút trong cây giao diện ảo. Framework dựng cây này từ trạng thái,
/// so nó với cây cũ, rồi chỉ cập nhật phần THAY ĐỔI lên DOM thật (tốn kém).
#[derive(Debug, Clone, PartialEq)]
pub enum NutAo {
    /// Thẻ HTML: tên thẻ, thuộc tính, các nút con.
    The {
        ten: String,
        thuoc_tinh: Vec<(String, String)>,
        con: Vec<NutAo>,
    },
    /// Nút văn bản.
    Van(String),
}

impl NutAo {
    pub fn the(ten: &str, thuoc_tinh: Vec<(&str, &str)>, con: Vec<NutAo>) -> Self {
        NutAo::The {
            ten: ten.to_string(),
            thuoc_tinh: thuoc_tinh.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            con,
        }
    }
    pub fn van(s: &str) -> Self {
        NutAo::Van(s.to_string())
    }

    /// Kết xuất thành chuỗi HTML (như server-side rendering).
    /// Chú ý: THOÁT ký tự để chống XSS (Chương 57)!
    pub fn thanh_html(&self) -> String {
        match self {
            NutAo::Van(s) => thoat_html(s),
            NutAo::The { ten, thuoc_tinh, con } => {
                let tt: String = thuoc_tinh.iter()
                    .map(|(k, v)| format!(" {}=\"{}\"", k, thoat_html(v)))
                    .collect();
                let ben_trong: String = con.iter().map(|c| c.thanh_html()).collect();
                format!("<{}{}>{}</{}>", ten, tt, ben_trong, ten)
            }
        }
    }
}

fn thoat_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Một bản vá (patch) mô tả một thay đổi cần áp lên DOM thật.
#[derive(Debug, Clone, PartialEq)]
pub enum BanVa {
    ThayThe { duong_dan: Vec<usize>, nut_moi: NutAo },
    DoiVanBan { duong_dan: Vec<usize>, van_moi: String },
    DoiThuocTinh { duong_dan: Vec<usize>, ten: String, gia_tri: String },
    ThemCon { duong_dan: Vec<usize>, nut: NutAo },
    XoaCon { duong_dan: Vec<usize>, chi_so: usize },
}

/// THUẬT TOÁN DIFF: so hai cây ảo, sinh danh sách bản vá TỐI THIỂU.
/// Đây là điều khiến React/Leptos nhanh: không dựng lại cả DOM, chỉ vá chỗ đổi.
pub fn diff(cu: &NutAo, moi: &NutAo, duong_dan: Vec<usize>) -> Vec<BanVa> {
    match (cu, moi) {
        // Hai văn bản khác nội dung -> vá văn bản
        (NutAo::Van(a), NutAo::Van(b)) => {
            if a != b {
                vec![BanVa::DoiVanBan { duong_dan, van_moi: b.clone() }]
            } else {
                vec![]
            }
        }
        // Hai thẻ cùng tên -> so thuộc tính và con
        (NutAo::The { ten: ta, thuoc_tinh: tta, con: ca },
         NutAo::The { ten: tb, thuoc_tinh: ttb, con: cb }) if ta == tb => {
            let mut va = Vec::new();
            // Thuộc tính thay đổi hoặc thêm
            let map_cu: HashMap<_, _> = tta.iter().cloned().collect();
            for (k, v) in ttb {
                if map_cu.get(k) != Some(v) {
                    va.push(BanVa::DoiThuocTinh {
                        duong_dan: duong_dan.clone(), ten: k.clone(), gia_tri: v.clone(),
                    });
                }
            }
            // So các con theo vị trí
            let chung = ca.len().min(cb.len());
            for i in 0..chung {
                let mut dd = duong_dan.clone();
                dd.push(i);
                va.extend(diff(&ca[i], &cb[i], dd));
            }
            // Con thừa ở cây mới -> thêm; thừa ở cây cũ -> xóa
            for i in chung..cb.len() {
                va.push(BanVa::ThemCon { duong_dan: duong_dan.clone(), nut: cb[i].clone() });
            }
            for i in (chung..ca.len()).rev() {
                va.push(BanVa::XoaCon { duong_dan: duong_dan.clone(), chi_so: i });
            }
            va
        }
        // Khác loại/khác tên thẻ -> thay thế cả nút
        _ => vec![BanVa::ThayThe { duong_dan, nut_moi: moi.clone() }],
    }
}

// ============================================================================
// 3. COMPONENT — hàm thuần túy: trạng thái -> cây ảo (như view của Leptos)
// ============================================================================

#[derive(Clone)]
pub struct TrangThaiDem {
    pub so: TinHieu<i64>,
}

/// Component đếm: một HÀM THUẦN TÚY nhận trạng thái, trả về cây giao diện ảo.
/// Đây là bản chất của UI khai báo (declarative): giao diện là HÀM của trạng thái.
pub fn view_dem(tt: &TrangThaiDem) -> NutAo {
    NutAo::the("div", vec![("class", "dem")], vec![
        NutAo::the("h1", vec![], vec![NutAo::van(&format!("Đếm: {}", tt.so.lay()))]),
        NutAo::the("button", vec![("id", "tang")], vec![NutAo::van("Tăng")]),
        NutAo::the("button", vec![("id", "giam")], vec![NutAo::van("Giảm")]),
    ])
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   FRONTEND: HỆ PHẢN ỨNG (SIGNALS) + VIRTUAL DOM (DIFF)         ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n1. HỆ PHẢN ỨNG");
    let so = TinHieu::moi(0i64);
    let tong = DanXuat::moi({
        let so = so.clone();
        move || so.lay() * 1000 // "tổng tiền" dẫn xuất từ "số lượng"
    });
    println!("   số = {}, tổng dẫn xuất = {}", so.lay(), tong.lay());
    so.dat(5);
    println!("   sau khi đặt số = 5: tổng tự cập nhật = {}", tong.lay());
    println!("   phiên bản tín hiệu: {}", so.phien_ban());
    so.dat(5); // đặt lại cùng giá trị -> KHÔNG tăng phiên bản
    println!("   đặt lại cùng giá trị 5: phiên bản vẫn = {} (bỏ render thừa)", so.phien_ban());

    println!("\n2. COMPONENT -> VIRTUAL DOM -> HTML");
    let tt = TrangThaiDem { so: TinHieu::moi(3) };
    let cay = view_dem(&tt);
    println!("   {}", cay.thanh_html());

    println!("\n3. DIFF — chỉ vá chỗ THAY ĐỔI");
    let tt2 = TrangThaiDem { so: TinHieu::moi(4) }; // số đổi 3 -> 4
    let cay_moi = view_dem(&tt2);
    let ban_va = diff(&cay, &cay_moi, vec![]);
    println!("   Số bản vá cần áp lên DOM thật: {} (chỉ đổi văn bản, không dựng lại cả cây!)", ban_va.len());
    for v in &ban_va {
        println!("     {:?}", v);
    }

    println!("\n4. CHỐNG XSS TRONG KẾT XUẤT (Chương 57)");
    let doc_hai = NutAo::the("div", vec![], vec![NutAo::van("<script>hack()</script>")]);
    println!("   Đầu vào độc: <script>hack()</script>");
    println!("   Kết xuất an toàn: {}", doc_hai.thanh_html());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   GIAO DIỆN = HÀM CỦA TRẠNG THÁI · DIFF = CHỈ VÁ CHỖ ĐỔI       ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn tin_hieu_luu_va_doi_gia_tri() {
        let s = TinHieu::moi(10i64);
        assert_eq!(s.lay(), 10);
        s.dat(20);
        assert_eq!(s.lay(), 20);
        s.cap_nhat(|x| x + 5);
        assert_eq!(s.lay(), 25);
    }

    #[test]
    fn tin_hieu_bo_qua_thay_doi_thua() {
        let s = TinHieu::moi(1i64);
        assert_eq!(s.phien_ban(), 0);
        s.dat(2);
        assert_eq!(s.phien_ban(), 1);
        s.dat(2); // cùng giá trị -> không tăng phiên bản
        assert_eq!(s.phien_ban(), 1, "đặt cùng giá trị không được kích hoạt render");
        s.dat(3);
        assert_eq!(s.phien_ban(), 2);
    }

    #[test]
    fn dan_xuat_tu_cap_nhat_theo_nguon() {
        let so = TinHieu::moi(2i64);
        let gap_doi = DanXuat::moi({ let so = so.clone(); move || so.lay() * 2 });
        assert_eq!(gap_doi.lay(), 4);
        so.dat(10);
        assert_eq!(gap_doi.lay(), 20); // tự cập nhật, không cần gọi lại thủ công
    }

    #[test]
    fn ket_xuat_html_dung() {
        let c = NutAo::the("div", vec![("class", "x")], vec![NutAo::van("chào")]);
        assert_eq!(c.thanh_html(), "<div class=\"x\">chào</div>");
    }

    #[test]
    fn ket_xuat_thoat_xss() {
        let c = NutAo::van("<script>alert(1)</script>");
        let html = c.thanh_html();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn diff_van_ban_chi_sinh_1_ban_va() {
        let cu = NutAo::van("Đếm: 3");
        let moi = NutAo::van("Đếm: 4");
        let va = diff(&cu, &moi, vec![]);
        assert_eq!(va.len(), 1);
        assert!(matches!(va[0], BanVa::DoiVanBan { .. }));
    }

    #[test]
    fn diff_khong_doi_thi_khong_co_ban_va() {
        let c = view_dem(&TrangThaiDem { so: TinHieu::moi(5) });
        let va = diff(&c, &c.clone(), vec![]);
        assert!(va.is_empty(), "cây giống hệt không được sinh bản vá");
    }

    #[test]
    fn diff_component_dem_chi_va_van_ban() {
        let a = view_dem(&TrangThaiDem { so: TinHieu::moi(3) });
        let b = view_dem(&TrangThaiDem { so: TinHieu::moi(4) });
        let va = diff(&a, &b, vec![]);
        // Chỉ số trong <h1> đổi -> đúng 1 bản vá đổi văn bản, các nút button giữ nguyên
        assert_eq!(va.len(), 1);
        match &va[0] {
            BanVa::DoiVanBan { van_moi, .. } => assert_eq!(van_moi, "Đếm: 4"),
            other => panic!("phải là DoiVanBan, nhận {:?}", other),
        }
    }

    #[test]
    fn diff_them_va_xoa_con() {
        let cu = NutAo::the("ul", vec![], vec![NutAo::van("a")]);
        let moi = NutAo::the("ul", vec![], vec![NutAo::van("a"), NutAo::van("b")]);
        let them = diff(&cu, &moi, vec![]);
        assert!(them.iter().any(|v| matches!(v, BanVa::ThemCon { .. })));
        let xoa = diff(&moi, &cu, vec![]);
        assert!(xoa.iter().any(|v| matches!(v, BanVa::XoaCon { .. })));
    }

    #[test]
    fn diff_khac_the_thi_thay_the() {
        let cu = NutAo::the("div", vec![], vec![]);
        let moi = NutAo::the("span", vec![], vec![]);
        let va = diff(&cu, &moi, vec![]);
        assert!(matches!(va[0], BanVa::ThayThe { .. }));
    }
}
```

---

## Chuyển sang framework thật: Leptos

Cùng bộ đếm viết bằng [Leptos](https://leptos.dev/) — chú ý `create_signal` chính là `TinHieu`, và `view!` dựng cây ảo:

```rust
// Cargo.toml:  leptos = { version = "0.7", features = ["csr"] }
use leptos::prelude::*;

#[component]
fn BoDem() -> impl IntoView {
    // create_signal = TinHieu ở trên: (bộ đọc, bộ ghi)
    let (so, dat_so) = signal(0i64);

    view! {
        <div class="dem">
            // Nội dung tự cập nhật khi `so` đổi — đúng cơ chế reactivity.
            <h1>"Đếm: " {so}</h1>
            <button on:click=move |_| dat_so.update(|n| *n += 1)>"Tăng"</button>
            <button on:click=move |_| dat_so.update(|n| *n -= 1)>"Giảm"</button>
        </div>
    }
}

fn main() {
    leptos::mount::mount_to_body(BoDem);
}
```

Biên dịch sang WASM bằng `trunk build --release`, và bạn có một ứng dụng web viết 100% bằng Rust, chạy trong trình duyệt với tốc độ gần mã máy, an toàn kiểu từ backend tới frontend.

## Kết hợp Rust với Svelte qua Tauri (xem tiếp Chương 63)

Một mô hình phổ biến khác: giao diện bằng **Svelte/React/Vue** (JavaScript quen thuộc), lõi nghiệp vụ bằng **Rust**, ghép qua **Tauri**. Frontend gọi hàm Rust qua cơ chế "command":

```svelte
<!-- Svelte (frontend) -->
<script>
  import { invoke } from '@tauri-apps/api/core';
  let ket_qua = '';
  async function tinh() {
    // Gọi thẳng hàm Rust từ JavaScript!
    ket_qua = await invoke('tinh_tong', { a: 3, b: 4 });
  }
</script>
<button on:click={tinh}>Tính</button>
<p>{ket_qua}</p>
```

```rust
// Rust (lõi Tauri)
#[tauri::command]
fn tinh_tong(a: i64, b: i64) -> i64 { a + b }
```

Đây là "lõi thuần túy (Rust), vỏ giao diện (Svelte)" — chủ đề đầy đủ của **Chương 63**.

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Reactivity = tín hiệu + giá trị dẫn xuất.** Cập nhật dữ liệu, giao diện tự đúng. Bỏ qua thay đổi thừa để tránh render lãng phí.
2. **Virtual DOM + diff = tốc độ.** Dựng cây ảo rẻ, so với cây cũ, chỉ vá chỗ đổi lên DOM thật đắt.
3. **Giao diện là hàm thuần túy của trạng thái** (khai báo) — ít trạng thái ẩn, dễ suy luận, đúng tinh thần Chương 13.
4. **Thoát ký tự theo mặc định** chặn XSS ngay ở tầng render (Chương 57). Rust + WASM cho frontend an toàn kiểu, tốc độ cao.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Danh sách việc cần làm)**
Viết `view_todo(muc: &[(&str, bool)]) -> NutAo` dựng một `<ul>` với mỗi việc là một `<li>`, gạch ngang (thuộc tính `class="xong"`) nếu đã hoàn thành. Test số bản vá khi đánh dấu một việc là xong.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn view_todo(muc: &[(&str, bool)]) -> NutAo {
    let items: Vec<NutAo> = muc.iter().map(|(ten, xong)| {
        let lop = if *xong { "xong" } else { "chua" };
        NutAo::the("li", vec![("class", lop)], vec![NutAo::van(ten)])
    }).collect();
    NutAo::the("ul", vec![], items)
}

#[cfg(test)]
mod bt1 {
    use super::*;
    #[test]
    fn danh_dau_xong_chi_va_thuoc_tinh() {
        let a = view_todo(&[("Học Rust", false), ("Uống nước", true)]);
        let b = view_todo(&[("Học Rust", true), ("Uống nước", true)]); // việc 1 -> xong
        let va = diff(&a, &b, vec![]);
        // Chỉ đổi class của <li> đầu -> 1 bản vá đổi thuộc tính
        assert_eq!(va.len(), 1);
        assert!(matches!(va[0], BanVa::DoiThuocTinh { .. }));
    }
}
```
</details>

**Bài tập 2 (Bộ nhớ hóa giá trị dẫn xuất)**
`DanXuat` hiện tính lại mỗi lần `lay()`. Thêm cache: chỉ tính lại khi phiên bản của tín hiệu nguồn thay đổi (ghi nhớ, Chương 60). Đây chính là tối ưu "memo" của Leptos/React.

<details>
<summary><b>Gợi ý</b></summary>

Lưu `(phiên_bản_nguồn, giá_trị_đã_tính)` trong một `RefCell`. Khi `lay()`, nếu phiên bản nguồn không đổi, trả giá trị cache; nếu đổi, tính lại và cập nhật cache. Điều kiện tiên quyết để cache đúng: hàm tính phải THUẦN TÚY (Chương 13).
</details>

**Bài tập 3 (Tư duy: chọn kiến trúc frontend)**
Với mỗi dự án, chọn: (a) Rust+WASM thuần (Leptos), (b) JS frontend + Rust core qua Tauri, (c) chỉ JavaScript. Giải thích:
1. Công cụ chỉnh sửa ảnh nặng tính toán chạy trong trình duyệt.
2. Trang landing marketing đơn giản.
3. Ứng dụng desktop cần cả giao diện đẹp lẫn xử lý dữ liệu lớn.
4. Đội đã thạo React, cần thêm một module tính toán khoa học tốc độ cao.

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **(a) hoặc lai**: phần tính toán ảnh nặng nên viết Rust→WASM để nhanh; giao diện có thể Leptos hoặc JS gọi WASM.
2. **(c) JavaScript** (hoặc chỉ HTML tĩnh). WASM là thừa thãi cho trang đơn giản — tải WASM còn nặng hơn.
3. **(b) Tauri**: JS/Svelte cho giao diện, Rust cho xử lý dữ liệu — đúng Chương 63.
4. **Lai**: giữ React, viết module khoa học bằng Rust→WASM và gọi từ JS. Không cần viết lại cả frontend.

Nguyên tắc: **WASM/Rust tỏa sáng ở phần tính toán nặng**; giao diện thuần tương tác thì JS vẫn tiện. Nhiều dự án dùng cả hai đúng chỗ mạnh của mỗi bên.
</details>
