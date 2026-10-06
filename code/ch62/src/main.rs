//! Chương 62 — Frontend & WASM: hai cách giữ giao diện khớp với trạng thái —
//! HỆ PHẢN ỨNG MỊN (signals, như Leptos/SolidJS) và VIRTUAL DOM + DIFF (như React/Yew).
//! Lõi thuần túy, kiểm thử được.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

// ============================================================================
// 1. HỆ PHẢN ỨNG (Reactivity) — nền của Leptos/SolidJS/Svelte 5
// ============================================================================

type Subscriber<T> = Box<dyn Fn(&T)>;

/// Một "tín hiệu" (signal): ô trạng thái mà khi thay đổi sẽ tự động thông báo
/// cho những ai đang lắng nghe. Đây là cơ chế khiến UI tự cập nhật khi dữ liệu đổi.
pub struct Signal<T> {
    value: Rc<RefCell<T>>,
    version: Rc<RefCell<u64>>, // tăng mỗi lần giá trị đổi -> phát hiện thay đổi
    subscribers: Rc<RefCell<Vec<Subscriber<T>>>>,
}

// Cài Clone thủ công: `#[derive(Clone)]` sẽ đòi `T: Clone` không cần thiết.
// Clone một tín hiệu = thêm một tay cầm tới CÙNG ô trạng thái (Rc::clone).
impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Signal {
            value: Rc::clone(&self.value),
            version: Rc::clone(&self.version),
            subscribers: Rc::clone(&self.subscribers),
        }
    }
}

impl<T: Clone + PartialEq> Signal<T> {
    pub fn new(value: T) -> Self {
        Signal {
            value: Rc::new(RefCell::new(value)),
            version: Rc::new(RefCell::new(0)),
            subscribers: Rc::new(RefCell::new(Vec::new())),
        }
    }
    pub fn get(&self) -> T {
        self.value.borrow().clone()
    }
    /// Đặt giá trị mới. Chỉ tăng phiên bản và báo cho người nghe nếu giá trị
    /// THỰC SỰ đổi (tránh render thừa).
    pub fn set(&self, new: T) {
        if *self.value.borrow() == new {
            return;
        }
        *self.value.borrow_mut() = new;
        *self.version.borrow_mut() += 1;
        // Mượn chung (borrow) — người nghe chỉ được ĐỌC giá trị.
        let value = self.value.borrow();
        for f in self.subscribers.borrow().iter() {
            f(&value);
        }
    }
    pub fn update(&self, f: impl FnOnce(&T) -> T) {
        // Khoá mượn kết thúc ở cuối câu lệnh này, trước khi `set` mượn ghi.
        let new = f(&self.value.borrow());
        self.set(new);
    }
    pub fn version(&self) -> u64 {
        *self.version.borrow()
    }
    /// Đăng ký một "hiệu ứng" (effect): chạy ngay một lần, rồi chạy lại mỗi khi
    /// tín hiệu đổi. Leptos gắn mỗi `{count}` trong `view!` với một effect như vậy.
    pub fn subscribe(&self, f: impl Fn(&T) + 'static) {
        f(&self.value.borrow());
        self.subscribers.borrow_mut().push(Box::new(f));
    }
}

/// Giá trị DẪN XUẤT (derived/computed): tự tính lại từ các tín hiệu nguồn.
/// Ví dụ: "tổng tiền" dẫn xuất từ "giỏ hàng". Đổi giỏ -> tổng tự cập nhật.
pub struct Derived<T> {
    compute: Box<dyn Fn() -> T>,
}
impl<T> Derived<T> {
    pub fn new(compute: impl Fn() -> T + 'static) -> Self {
        Derived {
            compute: Box::new(compute),
        }
    }
    pub fn get(&self) -> T {
        (self.compute)()
    }
}

/// Một nút văn bản của "DOM thật" giả lập: đếm số lần bị ghi để thấy chi phí.
#[derive(Default)]
pub struct DomText {
    pub content: String,
    pub writes: u32,
}

/// PHẢN ỨNG MỊN (fine-grained): nối một tín hiệu THẲNG vào một nút DOM.
/// Không có cây ảo, không có diff — tín hiệu đổi thì đúng nút đó được ghi.
pub fn bind_text<T: Clone + PartialEq + 'static>(
    signal: &Signal<T>,
    node: Rc<RefCell<DomText>>,
    render: impl Fn(&T) -> String + 'static,
) {
    signal.subscribe(move |v| {
        let mut n = node.borrow_mut();
        n.content = render(v);
        n.writes += 1;
    });
}

// ============================================================================
// 2. VIRTUAL DOM — cây mô tả giao diện, và thuật toán DIFF (mô hình React/Yew/Dioxus)
// ============================================================================

/// Một nút trong cây giao diện ảo. Framework kiểu React dựng cây này từ trạng thái,
/// so nó với cây cũ, rồi chỉ cập nhật phần THAY ĐỔI lên DOM thật (tốn kém).
#[derive(Debug, Clone, PartialEq)]
pub enum VirtualNode {
    /// Thẻ HTML: tên thẻ, thuộc tính, các nút con.
    Element {
        tag: String,
        attrs: Vec<(String, String)>,
        children: Vec<VirtualNode>,
    },
    /// Nút văn bản.
    Text(String),
}

impl VirtualNode {
    pub fn element(tag: &str, attrs: Vec<(&str, &str)>, children: Vec<VirtualNode>) -> Self {
        VirtualNode::Element {
            tag: tag.to_string(),
            attrs: attrs
                .into_iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            children,
        }
    }
    pub fn text(s: &str) -> Self {
        VirtualNode::Text(s.to_string())
    }

    /// Kết xuất thành chuỗi HTML (như server-side rendering).
    /// Chú ý: THOÁT ký tự để chống XSS (Chương 57)!
    pub fn to_html(&self) -> String {
        match self {
            VirtualNode::Text(s) => escape_html(s),
            VirtualNode::Element {
                tag,
                attrs,
                children,
            } => {
                let attrs_html: String = attrs
                    .iter()
                    .map(|(k, v)| format!(" {}=\"{}\"", k, escape_html(v)))
                    .collect();
                let inner: String = children.iter().map(|c| c.to_html()).collect();
                format!("<{}{}>{}</{}>", tag, attrs_html, inner, tag)
            }
        }
    }
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

/// Một bản vá (patch) mô tả một thay đổi cần áp lên DOM thật.
#[derive(Debug, Clone, PartialEq)]
pub enum Patch {
    Replaced {
        path: Vec<usize>,
        node: VirtualNode,
    },
    TextChanged {
        path: Vec<usize>,
        text: String,
    },
    AttrChanged {
        path: Vec<usize>,
        name: String,
        value: String,
    },
    AttrRemoved {
        path: Vec<usize>,
        name: String,
    },
    ChildAdded {
        path: Vec<usize>,
        node: VirtualNode,
    },
    ChildRemoved {
        path: Vec<usize>,
        index: usize,
    },
}

/// THUẬT TOÁN DIFF: so hai cây ảo, sinh danh sách bản vá TỐI THIỂU.
/// Đây là điều khiến React/Yew nhanh: không dựng lại cả DOM, chỉ vá chỗ đổi.
pub fn diff(old: &VirtualNode, new: &VirtualNode, path: Vec<usize>) -> Vec<Patch> {
    match (old, new) {
        // Hai văn bản khác nội dung -> vá văn bản
        (VirtualNode::Text(a), VirtualNode::Text(b)) => {
            if a != b {
                vec![Patch::TextChanged {
                    path,
                    text: b.clone(),
                }]
            } else {
                vec![]
            }
        }
        // Hai thẻ cùng tên -> so thuộc tính và con
        (
            VirtualNode::Element {
                tag: tag_a,
                attrs: attrs_a,
                children: kids_a,
            },
            VirtualNode::Element {
                tag: tag_b,
                attrs: attrs_b,
                children: kids_b,
            },
        ) if tag_a == tag_b => {
            let mut patches = Vec::new();
            // Thuộc tính thay đổi hoặc được thêm
            let old_attrs: HashMap<_, _> = attrs_a.iter().cloned().collect();
            for (k, v) in attrs_b {
                if old_attrs.get(k) != Some(v) {
                    patches.push(Patch::AttrChanged {
                        path: path.clone(),
                        name: k.clone(),
                        value: v.clone(),
                    });
                }
            }
            // Thuộc tính bị bỏ ở cây mới -> phải xoá khỏi DOM thật
            for (k, _) in attrs_a {
                if !attrs_b.iter().any(|(kb, _)| kb == k) {
                    patches.push(Patch::AttrRemoved {
                        path: path.clone(),
                        name: k.clone(),
                    });
                }
            }
            // So các con theo vị trí
            for (i, (a, b)) in kids_a.iter().zip(kids_b).enumerate() {
                let mut child_path = path.clone();
                child_path.push(i);
                patches.extend(diff(a, b, child_path));
            }
            // Con thừa ở cây mới -> thêm; thừa ở cây cũ -> xóa (từ cuối lên)
            let shared = kids_a.len().min(kids_b.len());
            for node in &kids_b[shared..] {
                patches.push(Patch::ChildAdded {
                    path: path.clone(),
                    node: node.clone(),
                });
            }
            for index in (shared..kids_a.len()).rev() {
                patches.push(Patch::ChildRemoved {
                    path: path.clone(),
                    index,
                });
            }
            patches
        }
        // Khác loại/khác tên thẻ -> thay thế cả nút
        _ => vec![Patch::Replaced {
            path,
            node: new.clone(),
        }],
    }
}

// ============================================================================
// 3. COMPONENT — hàm thuần túy: trạng thái -> cây ảo (như hàm component của React/Yew)
// ============================================================================

#[derive(Clone)]
pub struct CounterState {
    pub count: Signal<i64>,
}

/// Component đếm: một HÀM THUẦN TÚY nhận trạng thái, trả về cây giao diện ảo.
/// Đây là bản chất của UI khai báo (declarative): giao diện là HÀM của trạng thái.
pub fn counter_view(state: &CounterState) -> VirtualNode {
    VirtualNode::element(
        "div",
        vec![("class", "counter")],
        vec![
            VirtualNode::element(
                "h1",
                vec![],
                vec![VirtualNode::text(&format!("Đếm: {}", state.count.get()))],
            ),
            VirtualNode::element(
                "button",
                vec![("id", "inc")],
                vec![VirtualNode::text("Tăng")],
            ),
            VirtualNode::element(
                "button",
                vec![("id", "dec")],
                vec![VirtualNode::text("Giảm")],
            ),
        ],
    )
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   FRONTEND: HỆ PHẢN ỨNG (SIGNALS) · VIRTUAL DOM (DIFF)         ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n1. HỆ PHẢN ỨNG");
    let count = Signal::new(0i64);
    let total = Derived::new({
        let count = count.clone();
        move || count.get() * 1000 // "tổng tiền" dẫn xuất từ "số lượng"
    });
    println!("   số = {}, tổng dẫn xuất = {}", count.get(), total.get());
    count.set(5);
    println!("   sau khi đặt số = 5: tổng tự cập nhật = {}", total.get());
    println!("   phiên bản tín hiệu: {}", count.version());
    count.set(5); // đặt lại cùng giá trị -> KHÔNG tăng phiên bản
    println!(
        "   đặt lại cùng giá trị 5: phiên bản vẫn = {} (bỏ render thừa)",
        count.version()
    );

    println!("\n2. PHẢN ỨNG MỊN (kiểu Leptos) — tín hiệu nối thẳng vào nút DOM");
    let h1 = Rc::new(RefCell::new(DomText::default()));
    bind_text(&count, Rc::clone(&h1), |n| format!("Đếm: {n}"));
    count.set(6);
    count.set(7);
    println!(
        "   <h1> = \"{}\" sau {} lần ghi — không dựng cây, không diff",
        h1.borrow().content,
        h1.borrow().writes
    );

    println!("\n3. COMPONENT -> VIRTUAL DOM -> HTML (kiểu React/Yew)");
    let state = CounterState {
        count: Signal::new(3),
    };
    let tree = counter_view(&state);
    println!("   {}", tree.to_html());

    println!("\n4. DIFF — chỉ vá chỗ THAY ĐỔI");
    state.count.set(4); // số đổi 3 -> 4: dựng LẠI cả cây ảo rồi so
    let new_tree = counter_view(&state);
    let patches = diff(&tree, &new_tree, vec![]);
    println!(
        "   Số bản vá cần áp lên DOM thật: {} (chỉ đổi văn bản, không dựng lại cả cây!)",
        patches.len()
    );
    for p in &patches {
        println!("     {:?}", p);
    }

    println!("\n5. CHỐNG XSS TRONG KẾT XUẤT (Chương 57)");
    let malicious = VirtualNode::element(
        "div",
        vec![],
        vec![VirtualNode::text("<script>hack()</script>")],
    );
    println!("   Đầu vào độc: <script>hack()</script>");
    println!("   Kết xuất an toàn: {}", malicious.to_html());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   GIAO DIỆN = HÀM CỦA TRẠNG THÁI · CHỈ CẬP NHẬT CHỖ ĐỔI         ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signal_stores_and_updates_value() {
        let s = Signal::new(10i64);
        assert_eq!(s.get(), 10);
        s.set(20);
        assert_eq!(s.get(), 20);
        s.update(|x| x + 5);
        assert_eq!(s.get(), 25);
    }

    #[test]
    fn signal_skips_redundant_updates() {
        let s = Signal::new(1i64);
        assert_eq!(s.version(), 0);
        s.set(2);
        assert_eq!(s.version(), 1);
        s.set(2); // cùng giá trị -> không tăng phiên bản
        assert_eq!(
            s.version(),
            1,
            "đặt cùng giá trị không được kích hoạt render"
        );
        s.set(3);
        assert_eq!(s.version(), 2);
    }

    #[test]
    fn derived_signal_tracks_its_source() {
        let count = Signal::new(2i64);
        let doubled = Derived::new({
            let count = count.clone();
            move || count.get() * 2
        });
        assert_eq!(doubled.get(), 4);
        count.set(10);
        assert_eq!(doubled.get(), 20); // tự cập nhật, không cần gọi lại thủ công
    }

    #[test]
    fn fine_grained_binding_writes_only_on_change() {
        let count = Signal::new(0i64);
        let node = Rc::new(RefCell::new(DomText::default()));
        bind_text(&count, Rc::clone(&node), |n| format!("Đếm: {n}"));
        assert_eq!(node.borrow().writes, 1); // lần kết xuất đầu
        count.set(1);
        count.set(1); // cùng giá trị -> không ghi DOM
        count.update(|n| n + 1);
        assert_eq!(node.borrow().content, "Đếm: 2");
        assert_eq!(node.borrow().writes, 3);
    }

    #[test]
    fn renders_correct_html() {
        let c = VirtualNode::element("div", vec![("class", "x")], vec![VirtualNode::text("chào")]);
        assert_eq!(c.to_html(), "<div class=\"x\">chào</div>");
    }

    #[test]
    fn render_escapes_xss() {
        let c = VirtualNode::text("<script>alert(1)</script>");
        let html = c.to_html();
        assert!(!html.contains("<script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn diff_text_yields_one_patch() {
        let old = VirtualNode::text("Đếm: 3");
        let new = VirtualNode::text("Đếm: 4");
        let patches = diff(&old, &new, vec![]);
        assert_eq!(patches.len(), 1);
        assert!(matches!(patches[0], Patch::TextChanged { .. }));
    }

    #[test]
    fn diff_of_identical_trees_is_empty() {
        let c = counter_view(&CounterState {
            count: Signal::new(5),
        });
        let patches = diff(&c, &c.clone(), vec![]);
        assert!(patches.is_empty(), "cây giống hệt không được sinh bản vá");
    }

    #[test]
    fn diff_detects_text_change_only() {
        let a = counter_view(&CounterState {
            count: Signal::new(3),
        });
        let b = counter_view(&CounterState {
            count: Signal::new(4),
        });
        let patches = diff(&a, &b, vec![]);
        // Chỉ số trong <h1> đổi -> đúng 1 bản vá đổi văn bản, các nút button giữ nguyên
        assert_eq!(patches.len(), 1);
        match &patches[0] {
            Patch::TextChanged { text, path } => {
                assert_eq!(text, "Đếm: 4");
                assert_eq!(path, &vec![0, 0]); // div > h1 > văn bản
            }
            other => panic!("phải là TextChanged, nhận {:?}", other),
        }
    }

    #[test]
    fn diff_detects_removed_attribute() {
        let old = VirtualNode::element("li", vec![("class", "done"), ("id", "a")], vec![]);
        let new = VirtualNode::element("li", vec![("id", "a")], vec![]);
        let patches = diff(&old, &new, vec![]);
        assert_eq!(
            patches,
            vec![Patch::AttrRemoved {
                path: vec![],
                name: "class".into()
            }]
        );
    }

    #[test]
    fn diff_detects_child_insert_and_remove() {
        let old = VirtualNode::element("ul", vec![], vec![VirtualNode::text("a")]);
        let new = VirtualNode::element(
            "ul",
            vec![],
            vec![VirtualNode::text("a"), VirtualNode::text("b")],
        );
        let added = diff(&old, &new, vec![]);
        assert!(added.iter().any(|p| matches!(p, Patch::ChildAdded { .. })));
        let removed = diff(&new, &old, vec![]);
        assert!(
            removed
                .iter()
                .any(|p| matches!(p, Patch::ChildRemoved { index: 1, .. }))
        );
    }

    #[test]
    fn diff_replaces_on_different_tag() {
        let old = VirtualNode::element("div", vec![], vec![]);
        let new = VirtualNode::element("span", vec![], vec![]);
        let patches = diff(&old, &new, vec![]);
        assert!(matches!(patches[0], Patch::Replaced { .. }));
    }
}
