#![allow(dead_code, unused_variables)]
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
