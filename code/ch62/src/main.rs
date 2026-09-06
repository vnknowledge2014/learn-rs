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
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    session_sell: Rc<RefCell<u64>>, // tăng mỗi lần đặt giá trị -> phát hiện thay đổi
}

impl<T: Clone + PartialEq> Signal<T> {
    pub fn new(value: T) -> Self {
        Signal {
            value: Rc::new(RefCell::new(value)),
            session_sell: Rc::new(RefCell::new(0)),
        }
    }
    pub fn lay(&self) -> T {
        self.value.borrow().clone()
    }
    /// Đặt giá trị mới. Chỉ tăng phiên bản nếu giá trị THỰC SỰ đổi (tránh render thừa).
    pub fn set(&self, new: T) {
        if *self.value.borrow() != new {
            *self.value.borrow_mut() = new;
            *self.session_sell.borrow_mut() += 1;
        }
    }
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        let new = f(&self.value.borrow());
        self.set(new);
    }
    pub fn session_sell(&self) -> u64 {
        *self.session_sell.borrow()
    }
}

/// Giá trị DẪN XUẤT (derived/computed): tự tính lại từ các tín hiệu nguồn.
/// Ví dụ: "tổng tiền" dẫn xuất từ "giỏ hàng". Đổi giỏ -> tổng tự cập nhật.
pub struct DeriveExport<T> {
    compute: Box<dyn Fn() -> T>,
}
impl<T> DeriveExport<T> {
    pub fn new(compute: impl Fn() -> T + 'static) -> Self {
        DeriveExport { compute: Box::new(compute) }
    }
    pub fn lay(&self) -> T {
        (self.compute)()
    }
}

// ============================================================================
// 2. VIRTUAL DOM — cây mô tả giao diện, và thuật toán DIFF
// ============================================================================

/// Một nút trong cây giao diện ảo. Framework dựng cây này từ trạng thái,
/// so nó với cây cũ, rồi chỉ cập nhật phần THAY ĐỔI lên DOM thật (tốn kém).
#[derive(Debug, Clone, PartialEq)]
pub enum VirtualNode {
    /// Thẻ HTML: tên thẻ, thuộc tính, các nút con.
    The {
        name: String,
        attribute: Vec<(String, String)>,
        con: Vec<VirtualNode>,
    },
    /// Nút văn bản.
    Van(String),
}

impl VirtualNode {
    pub fn the(name: &str, attribute: Vec<(&str, &str)>, con: Vec<VirtualNode>) -> Self {
        VirtualNode::The {
            name: name.to_string(),
            attribute: attribute.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            con,
        }
    }
    pub fn van(s: &str) -> Self {
        VirtualNode::Van(s.to_string())
    }

    /// Kết xuất thành chuỗi HTML (như server-side rendering).
    /// Chú ý: THOÁT ký tự để chống XSS (Chương 57)!
    pub fn to_html(&self) -> String {
        match self {
            VirtualNode::Van(s) => escape_html(s),
            VirtualNode::The { name, attribute, con } => {
                let tt: String = attribute.iter()
                    .map(|(k, v)| format!(" {}=\"{}\"", k, escape_html(v)))
                    .collect();
                let side_in: String = con.iter().map(|c| c.to_html()).collect();
                format!("<{}{}>{}</{}>", name, tt, side_in, name)
            }
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Một bản vá (patch) mô tả một thay đổi cần áp lên DOM thật.
#[derive(Debug, Clone, PartialEq)]
pub enum SellAnd {
    Replaced { path: Vec<usize>, nut_moi: VirtualNode },
    TextChanged { path: Vec<usize>, van_moi: String },
    AttrChanged { path: Vec<usize>, name: String, value: String },
    ThemCon { path: Vec<usize>, nut: VirtualNode },
    ChildRemoved { path: Vec<usize>, chi_so: usize },
}

/// THUẬT TOÁN DIFF: so hai cây ảo, sinh danh sách bản vá TỐI THIỂU.
/// Đây là điều khiến React/Leptos nhanh: không dựng lại cả DOM, chỉ vá chỗ đổi.
pub fn diff(cu: &VirtualNode, new: &VirtualNode, path: Vec<usize>) -> Vec<SellAnd> {
    match (cu, new) {
        // Hai văn bản khác nội dung -> vá văn bản
        (VirtualNode::Van(a), VirtualNode::Van(b)) => {
            if a != b {
                vec![SellAnd::TextChanged { path, van_moi: b.clone() }]
            } else {
                vec![]
            }
        }
        // Hai thẻ cùng tên -> so thuộc tính và con
        (VirtualNode::The { name: ta, attribute: tta, con: ca },
         VirtualNode::The { name: tb, attribute: ttb, con: cb }) if ta == tb => {
            let mut va = Vec::new();
            // Thuộc tính thay đổi hoặc thêm
            let map_cu: HashMap<_, _> = tta.iter().cloned().collect();
            for (k, v) in ttb {
                if map_cu.get(k) != Some(v) {
                    va.push(SellAnd::AttrChanged {
                        path: path.clone(), name: k.clone(), value: v.clone(),
                    });
                }
            }
            // So các con theo vị trí
            let chung = ca.len().min(cb.len());
            for i in 0..chung {
                let mut dd = path.clone();
                dd.push(i);
                va.extend(diff(&ca[i], &cb[i], dd));
            }
            // Con thừa ở cây mới -> thêm; thừa ở cây cũ -> xóa
            for i in chung..cb.len() {
                va.push(SellAnd::ThemCon { path: path.clone(), nut: cb[i].clone() });
            }
            for i in (chung..ca.len()).rev() {
                va.push(SellAnd::ChildRemoved { path: path.clone(), chi_so: i });
            }
            va
        }
        // Khác loại/khác tên thẻ -> thay thế cả nút
        _ => vec![SellAnd::Replaced { path, nut_moi: new.clone() }],
    }
}

// ============================================================================
// 3. COMPONENT — hàm thuần túy: trạng thái -> cây ảo (như view của Leptos)
// ============================================================================

#[derive(Clone)]
pub struct StateCount {
    pub so: Signal<i64>,
}

/// Component đếm: một HÀM THUẦN TÚY nhận trạng thái, trả về cây giao diện ảo.
/// Đây là bản chất của UI khai báo (declarative): giao diện là HÀM của trạng thái.
pub fn counter_view(tt: &StateCount) -> VirtualNode {
    VirtualNode::the("div", vec![("class", "dem")], vec![
        VirtualNode::the("h1", vec![], vec![VirtualNode::van(&format!("Đếm: {}", tt.so.lay()))]),
        VirtualNode::the("button", vec![("id", "tang")], vec![VirtualNode::van("Tăng")]),
        VirtualNode::the("button", vec![("id", "giam")], vec![VirtualNode::van("Giảm")]),
    ])
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   FRONTEND: HỆ PHẢN ỨNG (SIGNALS) + VIRTUAL DOM (DIFF)         ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n1. HỆ PHẢN ỨNG");
    let so = Signal::new(0i64);
    let tong = DeriveExport::new({
        let so = so.clone();
        move || so.lay() * 1000 // "tổng tiền" dẫn xuất từ "số lượng"
    });
    println!("   số = {}, tổng dẫn xuất = {}", so.lay(), tong.lay());
    so.set(5);
    println!("   sau khi đặt số = 5: tổng tự cập nhật = {}", tong.lay());
    println!("   phiên bản tín hiệu: {}", so.session_sell());
    so.set(5); // đặt lại cùng giá trị -> KHÔNG tăng phiên bản
    println!("   đặt lại cùng giá trị 5: phiên bản vẫn = {} (bỏ render thừa)", so.session_sell());

    println!("\n2. COMPONENT -> VIRTUAL DOM -> HTML");
    let tt = StateCount { so: Signal::new(3) };
    let cay = counter_view(&tt);
    println!("   {}", cay.to_html());

    println!("\n3. DIFF — chỉ vá chỗ THAY ĐỔI");
    let tt2 = StateCount { so: Signal::new(4) }; // số đổi 3 -> 4
    let cay_moi = counter_view(&tt2);
    let sell_and = diff(&cay, &cay_moi, vec![]);
    println!("   Số bản vá cần áp lên DOM thật: {} (chỉ đổi văn bản, không dựng lại cả cây!)", sell_and.len());
    for v in &sell_and {
        println!("     {:?}", v);
    }

    println!("\n4. CHỐNG XSS TRONG KẾT XUẤT (Chương 57)");
    let read_two = VirtualNode::the("div", vec![], vec![VirtualNode::van("<script>hack()</script>")]);
    println!("   Đầu vào độc: <script>hack()</script>");
    println!("   Kết xuất an toàn: {}", read_two.to_html());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   GIAO DIỆN = HÀM CỦA TRẠNG THÁI · DIFF = CHỈ VÁ CHỖ ĐỔI       ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_stores_and_updates_value() {
        let s = Signal::new(10i64);
        assert_eq!(s.lay(), 10);
        s.set(20);
        assert_eq!(s.lay(), 20);
        s.update(|x| x + 5);
        assert_eq!(s.lay(), 25);
    }

    #[test]
    fn signal_skips_redundant_updates() {
        let s = Signal::new(1i64);
        assert_eq!(s.session_sell(), 0);
        s.set(2);
        assert_eq!(s.session_sell(), 1);
        s.set(2); // cùng giá trị -> không tăng phiên bản
        assert_eq!(s.session_sell(), 1, "đặt cùng giá trị không được kích hoạt render");
        s.set(3);
        assert_eq!(s.session_sell(), 2);
    }

    #[test]
    fn derived_signal_tracks_its_source() {
        let so = Signal::new(2i64);
        let doubled = DeriveExport::new({ let so = so.clone(); move || so.lay() * 2 });
        assert_eq!(doubled.lay(), 4);
        so.set(10);
        assert_eq!(doubled.lay(), 20); // tự cập nhật, không cần gọi lại thủ công
    }

    #[test]
    fn renders_correct_html() {
        let c = VirtualNode::the("div", vec![("class", "x")], vec![VirtualNode::van("chào")]);
        assert_eq!(c.to_html(), "<div class=\"x\">chào</div>");
    }

    #[test]
    fn render_escapes_xss() {
        let c = VirtualNode::van("<script>alert(1)</script>");
        let html = c.to_html();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn diff_van_ban_chi_sinh_1_ban_va() {
        let cu = VirtualNode::van("Đếm: 3");
        let new = VirtualNode::van("Đếm: 4");
        let va = diff(&cu, &new, vec![]);
        assert_eq!(va.len(), 1);
        assert!(matches!(va[0], SellAnd::TextChanged { .. }));
    }

    #[test]
    fn diff_of_identical_trees_is_empty() {
        let c = counter_view(&StateCount { so: Signal::new(5) });
        let va = diff(&c, &c.clone(), vec![]);
        assert!(va.is_empty(), "cây giống hệt không được sinh bản vá");
    }

    #[test]
    fn diff_detects_attr_and_text_changes() {
        let a = counter_view(&StateCount { so: Signal::new(3) });
        let b = counter_view(&StateCount { so: Signal::new(4) });
        let va = diff(&a, &b, vec![]);
        // Chỉ số trong <h1> đổi -> đúng 1 bản vá đổi văn bản, các nút button giữ nguyên
        assert_eq!(va.len(), 1);
        match &va[0] {
            SellAnd::TextChanged { van_moi, .. } => assert_eq!(van_moi, "Đếm: 4"),
            other => panic!("phải là TextChanged, nhận {:?}", other),
        }
    }

    #[test]
    fn diff_detects_child_insert_and_remove() {
        let cu = VirtualNode::the("ul", vec![], vec![VirtualNode::van("a")]);
        let new = VirtualNode::the("ul", vec![], vec![VirtualNode::van("a"), VirtualNode::van("b")]);
        let them = diff(&cu, &new, vec![]);
        assert!(them.iter().any(|v| matches!(v, SellAnd::ThemCon { .. })));
        let remove = diff(&new, &cu, vec![]);
        assert!(remove.iter().any(|v| matches!(v, SellAnd::ChildRemoved { .. })));
    }

    #[test]
    fn diff_replaces_on_different_tag() {
        let cu = VirtualNode::the("div", vec![], vec![]);
        let new = VirtualNode::the("span", vec![], vec![]);
        let va = diff(&cu, &new, vec![]);
        assert!(matches!(va[0], SellAnd::Replaced { .. }));
    }
}
