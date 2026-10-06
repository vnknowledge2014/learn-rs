# Chương 62: Phát triển Frontend với Rust & WebAssembly — Hệ phản ứng, Phản ứng mịn & Virtual DOM (Frontend Development)

## Giới thiệu & Mục tiêu học tập

Rust không chỉ chạy trên máy chủ. Nhờ **WebAssembly (WASM)** — một định dạng nhị phân chạy được trong mọi trình duyệt với tốc độ gần bằng mã máy — Rust có thể viết **giao diện web** chạy ngay trong trình duyệt, thay cho (hoặc cùng với) JavaScript. Các framework như [Leptos](https://leptos.dev/), [Yew](https://yew.rs/), [Dioxus](https://dioxuslabs.com/) đang biến điều này thành hiện thực, với hiệu năng vượt trội và an toàn kiểu ngay ở tầng giao diện.

Nhưng đằng sau mọi framework UI hiện đại — dù Leptos, React, Solid, hay Svelte — đều là cùng một mục tiêu: **khi trạng thái đổi, giao diện tự động cập nhật**, và chỉ cập nhật *đúng chỗ đổi* lên DOM thật (vốn rất tốn kém). Có hai trường phái chính để đạt điều đó:
1. **Virtual DOM + Diff** (React, Yew, Dioxus): mỗi lần trạng thái đổi, component chạy lại và dựng một cây ảo nhẹ; framework so cây mới với cây cũ rồi chỉ áp *phần khác nhau* lên DOM thật.
2. **Phản ứng mịn (fine-grained reactivity)** (Leptos, SolidJS, Svelte 5): component chỉ chạy **một lần** để tạo DOM thật; mỗi chỗ hiển thị dữ liệu được nối thẳng với **tín hiệu (signal)** của nó. Tín hiệu đổi → đúng nút DOM đó được ghi lại. **Không có cây ảo, không có diff.**

Chương này xây cả hai từ đầu — chạy offline, kiểm thử đầy đủ — để bạn hiểu *cơ chế* và so sánh được hai cách. Rồi chỉ ra mã Leptos tương ứng và cách kết hợp Rust với **Tauri 2 + Svelte** cho ứng dụng đa nền tảng.

Mục tiêu học tập:
- Hiểu **tín hiệu (signal)**, **giá trị dẫn xuất (derived)** và **hiệu ứng (effect)** — nền của reactivity.
- Phân biệt **phản ứng mịn** (Leptos) với **Virtual DOM + diff** (React/Yew) — mỗi cách trả giá ở đâu.
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
│  HỆ PHẢN ỨNG MỊN (signals, Leptos):                                              │
│    Bạn chỉ cập nhật DỮ LIỆU ("chuyến VN123 giờ mới = 14:30"). Cái bảng           │
│    TỰ ĐỘNG hiện đúng — vì mỗi ô đã "đăng ký lắng nghe" dữ liệu của nó.           │
│                                                                                  │
│  VIRTUAL DOM + DIFF (React/Yew):                                                 │
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

**Tín hiệu (signal)** là một ô trạng thái "thông minh": đọc được, ghi được, và khi ghi thì *thông báo* cho những ai phụ thuộc vào nó. Trong mã dưới, `Signal<T>` dùng `Rc<RefCell<T>>` (khả biến nội tại, Chương 27), một số **phiên bản** tăng mỗi lần đổi, và danh sách **người nghe** (`subscribe`) được gọi lại sau mỗi lần đổi.

Một chi tiết tối ưu quan trọng: `set` chỉ tăng phiên bản và báo người nghe khi giá trị **thực sự khác** — nhờ vậy đặt lại cùng giá trị không kích hoạt render thừa. Đây là lý do framework hiện đại nhanh: chúng làm càng ít việc càng tốt.

**Giá trị dẫn xuất (derived/computed)** tự tính lại từ các tín hiệu nguồn. "Tổng tiền" dẫn xuất từ "giỏ hàng": đổi giỏ, tổng tự đúng. Bạn không bao giờ phải nhớ "khi sửa giỏ thì cập nhật tổng" — đây chính là *minh bạch tham chiếu* (Chương 13) ở tầng giao diện.

### 2. Phản ứng mịn — nối tín hiệu thẳng vào DOM (cách của Leptos)

Thao tác lên DOM thật của trình duyệt **rất tốn kém** (kích hoạt tính toán lại bố cục, vẽ lại). Nếu mỗi lần trạng thái đổi mà dựng lại toàn bộ DOM, giao diện sẽ giật.

Leptos (và SolidJS, Svelte 5) giải bài toán này bằng **hiệu ứng (effect)**: khi component chạy lần đầu, mỗi chỗ `{count}` trong `view!` được biến thành một nút DOM thật *cộng* một effect đăng ký lắng nghe tín hiệu `count`. Về sau, `count` đổi thì chỉ effect đó chạy và ghi đúng nút văn bản đó — component **không chạy lại**, không có cây nào để so. Hàm `bind_text` trong mã mô phỏng đúng điều này: test `fine_grained_binding_writes_only_on_change` cho thấy nút DOM chỉ bị ghi khi giá trị thực sự đổi.

### 3. Virtual DOM — cách của React/Yew/Dioxus

Trường phái thứ hai giữ component là hàm chạy lại *toàn bộ* mỗi khi trạng thái đổi, nhưng không cho nó chạm vào DOM thật. Thay vào đó: mô tả giao diện bằng một **cây ảo nhẹ** (`VirtualNode` — chỉ là struct/enum trong bộ nhớ, dựng cực rẻ). Khi trạng thái đổi, dựng cây ảo *mới*, **so** với cây *cũ* bằng thuật toán **diff**, rồi chỉ áp những **bản vá tối thiểu** lên DOM thật.

Mã của chương giữ phần Virtual DOM này **để so sánh** với Leptos: đây là mô hình của React, Yew và Dioxus, *không phải* của Leptos.

### 4. Thuật toán Diff — trái tim của tốc độ (kiểu Virtual DOM)

`diff` so hai cây và sinh danh sách bản vá:
- Hai **văn bản** khác nội dung → một bản vá "đổi văn bản".
- Hai **thẻ cùng tên** → so thuộc tính (đổi, thêm, *và bị xoá*) rồi đệ quy so các con.
- **Khác loại/khác tên thẻ** → thay thế cả nút.

Điểm đắt giá minh họa trong test `diff_detects_text_change_only`: khi bộ đếm đổi từ 3 sang 4, chỉ có **một** bản vá (đổi nội dung `<h1>`) — hai nút `<button>` giữ nguyên, không bị dựng lại. Đây chính xác là điều làm React/Yew mượt: **cập nhật ngoại khoa, không phẫu thuật toàn thân**.

So với phản ứng mịn, cái giá phải trả là: *mỗi* lần đổi vẫn phải chạy lại cả component và so cả cây ảo (O(kích thước cây)), dù chỉ một chữ số thay đổi. Phản ứng mịn trả giá ở chỗ khác: phải theo dõi phụ thuộc giữa tín hiệu và effect lúc chạy. Cả hai cuối cùng đều chỉ ghi **một** nút DOM thật.

> **Ghi chú**: thuật toán diff ở đây so con *theo vị trí* (O(n)). Framework thật dùng thêm *khóa (key)* để nhận diện phần tử khi danh sách được sắp xếp lại — nếu không, đảo thứ tự một danh sách sẽ sinh nhiều bản vá không cần thiết. Đây là lý do React cảnh báo "mỗi phần tử trong list cần một `key` duy nhất".

### 5. Giao diện là hàm thuần túy của trạng thái

`counter_view` là một **hàm thuần túy**: nhận trạng thái, trả về cây ảo. Không tác dụng phụ, không thao tác DOM trực tiếp. Đây là **UI khai báo (declarative)**: bạn mô tả *giao diện trông thế nào ứng với trạng thái này*, framework lo phần *làm sao đạt được nó*.

So sánh với UI **mệnh lệnh** (jQuery): "tìm thẻ #counter, xóa chữ, viết chữ mới".

Trong Leptos, component cũng được viết khai báo y như vậy — khác biệt chỉ nằm ở *cách* framework hiện thực hoá nó (effect thay vì diff). Khai báo thắng vì cùng một lý do lập trình hàm thắng lập trình mệnh lệnh (Chương 13): ít trạng thái ẩn hơn, dễ suy luận hơn, ít lỗi hơn.

### 6. Chống XSS ngay trong kết xuất

`to_html` **thoát ký tự** mọi văn bản trước khi nhúng (Chương 57). Đây là "an toàn theo mặc định": framework thoát tự động, bạn phải *chủ động* yêu cầu "tin tưởng" thì nó mới không thoát. Nhờ vậy XSS lưu trữ — kẻ tấn công nhét `<script>` vào bình luận — bị chặn ngay ở tầng render, không cần lập trình viên nhớ thoát thủ công.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch62
cargo test -p ch62
```

```rust
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
```

---

## Chuyển sang framework thật: Leptos

Cùng bộ đếm viết bằng [Leptos](https://leptos.dev/) 0.8 (bản hiện hành). Chú ý: hàm `signal()` (từ Leptos 0.7; các bản cũ gọi là `create_signal`) chính là `Signal` ở trên, tách thành cặp (bộ đọc, bộ ghi). Còn `view!` **không** dựng cây ảo: nó tạo nút DOM thật một lần, và `{count}` trở thành một effect như `bind_text`.

> Đây là mã minh hoạ, cần một dự án riêng (biên dịch ra WASM bằng `trunk`), không nằm trong workspace `code/`. Đoạn mã đã được kiểm `cargo check` với `leptos 0.8` + feature `csr`.

```rust
// Cargo.toml:  leptos = { version = "0.8", features = ["csr"] }
use leptos::prelude::*;

#[component]
fn Counter() -> impl IntoView {
    // signal() = Signal ở trên, tách thành (bộ đọc, bộ ghi)
    let (count, set_count) = signal(0i64);

    // Hàm này chỉ chạy MỘT lần. `{count}` thành một effect ghi thẳng vào nút văn bản.
    view! {
        <div class="counter">
            <h1>"Đếm: " {count}</h1>
            <button on:click=move |_| set_count.update(|n| *n += 1)>"Tăng"</button>
            <button on:click=move |_| set_count.update(|n| *n -= 1)>"Giảm"</button>
        </div>
    }
}

fn main() {
    leptos::mount::mount_to_body(Counter);
}
```

Biên dịch sang WASM bằng `trunk build --release`, và bạn có một ứng dụng web viết 100% bằng Rust, chạy trong trình duyệt với tốc độ gần mã máy, an toàn kiểu từ backend tới frontend.

## Kết hợp Rust với Svelte qua Tauri (xem tiếp Chương 63)

Một mô hình phổ biến khác: giao diện bằng **Svelte/React/Vue** (JavaScript quen thuộc), lõi nghiệp vụ bằng **Rust**, ghép qua **Tauri 2**. Frontend gọi hàm Rust qua cơ chế "command" — tên truyền cho `invoke` phải trùng **đúng** tên hàm Rust (mã minh hoạ, cần dự án Tauri riêng):

```svelte
<!-- Svelte 5 (frontend) -->
<script>
  import { invoke } from '@tauri-apps/api/core';
  let result = $state('');
  async function compute() {
    // Gọi thẳng hàm Rust `sum_all` từ JavaScript!
    result = await invoke('sum_all', { a: 3, b: 4 });
  }
</script>
<button onclick={compute}>Tính</button>
<p>{result}</p>
```

```rust
// Rust (lõi Tauri 2)
#[tauri::command]
fn sum_all(a: i64, b: i64) -> i64 {
    a + b
}

// và đăng ký trong builder:
// tauri::Builder::default().invoke_handler(tauri::generate_handler![sum_all])
```

Đây là "lõi thuần túy (Rust), vỏ giao diện (Svelte)" — chủ đề đầy đủ của **Chương 63**.

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| *Không có mã lỗi* — panic lúc chạy `RefCell already borrowed` | Effect (`subscribe`) gọi `set` lên chính tín hiệu đang báo cho nó: `set` còn giữ `borrow()` khi gọi người nghe, nên `borrow_mut()` thất bại | `RefCell` kiểm tra mượn lúc **chạy**, trình biên dịch không bắt được. Đừng ghi vào nguồn trong effect của chính nó; tính giá trị mới ra biến, thoát vùng mượn rồi mới ghi |
| `E0277`: `Rc<RefCell<i64>>` cannot be sent between threads safely | Định đưa tín hiệu sang luồng khác (`thread::spawn(move \|\| s.get())`) | `Rc`/`RefCell` chỉ dùng một luồng; muốn đa luồng thì `Arc<Mutex<T>>` |
| `E0631: type mismatch in closure arguments` | `subscribe(\|x: i64\| ...)` trong khi người nghe nhận `&T` | Viết `\|x: &i64\|` hoặc bỏ chú thích kiểu để trình biên dịch suy luận |
| `E0373`: closure may outlive the current function, but it borrows `c` | `Derived::new(\|\| c.get() * 2)` thiếu `move` — `Derived` đòi closure `'static` | `let c = c.clone();` rồi `move \|\| c.get() * 2` |
| `E0382: borrow of moved value` | `move` tín hiệu vào closure rồi vẫn dùng biến cũ | `clone()` tín hiệu trước khi `move` (clone chỉ tăng bộ đếm `Rc`) |
| Giao diện không cập nhật | Đặt lại đúng giá trị cũ nên bị bỏ qua | Đúng như thiết kế — xem bài kiểm thử "signal_skips_redundant_updates" |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Reactivity = tín hiệu + giá trị dẫn xuất.** Cập nhật dữ liệu, giao diện tự đúng. Bỏ qua thay đổi thừa để tránh render lãng phí.
2. **Hai cách chỉ cập nhật chỗ đổi.** Phản ứng mịn (Leptos): tín hiệu nối thẳng vào nút DOM, không cây ảo. Virtual DOM + diff (React/Yew): dựng cây ảo rẻ, so với cây cũ, chỉ vá chỗ đổi lên DOM thật đắt.
3. **Giao diện là hàm thuần túy của trạng thái** (khai báo) — ít trạng thái ẩn, dễ suy luận, đúng tinh thần Chương 13.
4. **Thoát ký tự theo mặc định** chặn XSS ngay ở tầng render (Chương 57). Rust + WASM cho frontend an toàn kiểu, tốc độ cao.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Danh sách việc cần làm)**
Viết `view_todo(items: &[(&str, bool)]) -> VirtualNode` dựng một `<ul>` với mỗi việc là một `<li>`, gạch ngang (thuộc tính `class="done"`) nếu đã hoàn thành. Test số bản vá khi đánh dấu một việc là xong.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn view_todo(items: &[(&str, bool)]) -> VirtualNode {
    let lis: Vec<VirtualNode> = items
        .iter()
        .map(|(name, done)| {
            let class = if *done { "done" } else { "todo" };
            VirtualNode::element("li", vec![("class", class)], vec![VirtualNode::text(name)])
        })
        .collect();
    VirtualNode::element("ul", vec![], lis)
}

#[cfg(test)]
mod exercise1 {
    use super::*;
    #[test]
    fn marks_only_attr_change() {
        let a = view_todo(&[("Học Rust", false), ("Uống nước", true)]);
        let b = view_todo(&[("Học Rust", true), ("Uống nước", true)]); // việc 1 -> xong
        let patches = diff(&a, &b, vec![]);
        // Chỉ đổi class của <li> đầu -> 1 bản vá đổi thuộc tính, tại đường dẫn [0]
        assert_eq!(patches.len(), 1);
        assert!(matches!(&patches[0], Patch::AttrChanged { path, .. } if path == &vec![0]));
    }
}
```
</details>

**Bài tập 2 (Bộ nhớ hóa giá trị dẫn xuất)**
`Derived` hiện tính lại mỗi lần `get()`. Thêm cache: chỉ tính lại khi phiên bản của tín hiệu nguồn thay đổi (ghi nhớ, Chương 60). Đây chính là tối ưu "memo" của Leptos/React.

<details>
<summary><b>Gợi ý</b></summary>

Lưu `(phiên_bản_nguồn, giá_trị_đã_tính)` trong một `RefCell`. Khi `get()`, nếu phiên bản nguồn không đổi, trả giá trị cache; nếu đổi, tính lại và cập nhật cache. Điều kiện tiên quyết để cache đúng: hàm tính phải THUẦN TÚY (Chương 13).
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
