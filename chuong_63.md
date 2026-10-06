# Chương 63: Ứng dụng Desktop & Đa nền tảng — Tauri 2.0, gpui & wgpu (Desktop & Cross-Platform Apps)

## Giới thiệu & Mục tiêu học tập

Rust không dừng ở máy chủ và trình duyệt — nó còn là nền cho **ứng dụng desktop hiệu năng cao chạy trên mọi hệ điều hành**. Trình soạn thảo mã [Zed](https://zed.dev/) (nổi tiếng về tốc độ, viết 100% bằng Rust), hàng nghìn ứng dụng [Tauri](https://tauri.app/), và các engine đồ họa dựa trên [wgpu](https://github.com/gfx-rs/wgpu) đều chứng minh điều đó.

Chương này trình bày ba con đường xây ứng dụng desktop bằng Rust, mỗi con đường một triết lý:

| Công cụ | Triết lý | Ví dụ nổi bật |
|---|---|---|
| **[Tauri 2.0](https://github.com/tauri-apps/tauri)** | Giao diện = web (HTML/CSS/JS), lõi = Rust | Hàng nghìn app; thay thế Electron nhẹ hơn 10× |
| **[gpui](https://gpui.rs/)** | Giao diện native vẽ bằng GPU, 100% Rust | Trình soạn thảo Zed |
| **[wgpu](https://github.com/gfx-rs/wgpu)** | Đồ họa đa nền tảng cấp thấp (WebGPU API trên Vulkan/Metal/DX12) | Bevy, Firefox (WebGPU), ứng dụng 3D |

Điểm chung của cả ba — và là bài học trung tâm của chương — là **kiến trúc trạng thái**: mọi ứng dụng tương tác phức tạp đều cần một cách quản lý trạng thái có kỷ luật. Chương này xây **Kiến trúc Elm** (Model–Message–update) mà Elm, Redux và Iced (framework GUI Rust) đều dùng, cùng **cầu IPC** kiểu Tauri — tất cả thuần túy và kiểm thử được.

Mục tiêu học tập:
- Nắm **Kiến trúc Elm/Redux**: mọi thay đổi đi qua một hàm `update` thuần túy.
- Hiểu vì sao `update` thuần túy cho **undo/redo, ghi nhật ký, phát lại** gần như miễn phí (nối Chương 54).
- Thiết kế **cầu IPC** an toàn giữa giao diện và lõi Rust (như Tauri command).
- Biết ba con đường: **Tauri 2 + Svelte**, **gpui**, **wgpu** — và khi nào chọn cái nào.
- Áp dụng bảo mật (Chương 57) ở ranh giới IPC: chặn path traversal, không cho webview làm mọi thứ.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: MỘT NHÀ HÀNG VỚI PHÒNG ĂN VÀ BẾP TÁCH BIỆT                   │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   PHÒNG ĂN (Frontend)          │  CỬA SỔ BẾP (IPC)  │  BẾP (Backend Rust)        │
│   ─────────────────────        │  ────────────────  │  ──────────────────        │
│   · Trang trí đẹp (HTML/CSS)   │  Khách gọi món     │  · Nấu nướng thật (logic)  │
│   · Khách xem thực đơn         │  qua phiếu gọi:    │  · Đọc kho (hệ thống tệp)  │
│   · KHÔNG có dao, lửa, kho     │  invoke("nấu_phở", │  · Có DAO, LỬA, KHO        │
│     → an toàn cho khách        │    {topping})      │  · Kiểm phiếu: món hợp lệ? │
│                                │  ───────────►      │    ai gọi có quyền?        │
│                                │  ◄───────────      │                            │
│                                │  Bếp trả món ra    │                            │
│                                                                                  │
│   Vì sao TÁCH? → Giao diện (phòng ăn) không nên có quyền truy cập hệ thống       │
│   (dao, lửa). Nếu một thực khách xấu (mã độc trong webview) lẻn vào, họ CHỈ có   │
│   thể gọi các món TRONG THỰC ĐƠN (lệnh đã đăng ký), không thể xông vào bếp.      │
│                                                                                  │
│   Đây là mô hình BẢO MẬT của Tauri: webview bị cô lập, mọi thao tác hệ thống     │
│   phải đi qua "cửa sổ bếp" (IPC) nơi lõi Rust kiểm duyệt từng lời gọi.           │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Kiến trúc Elm — mọi thay đổi qua một cửa

Ứng dụng tương tác phức tạp dễ trở thành mớ hỗn độn khi trạng thái bị sửa từ khắp nơi. **Kiến trúc Elm** (còn gọi TEA, và là nền của Redux) áp một kỷ luật:

```
        ┌──────────────┐
        │    Model     │  toàn bộ trạng thái ứng dụng ở MỘT chỗ
        └──────┬───────┘
               │ view (hàm thuần túy)
               ▼
        [Giao diện]  ──sinh ra──►  Message
               ▲                          │
               │                          ▼
               │              ┌────────────────────────┐
               └──────────────│  update(model, msg)    │  hàm THUẦN TÚY:
                cập nhật      │  -> model mới          │  trạng thái cũ + thông điệp
                              └────────────────────────┘  = trạng thái mới
```

Ba quy tắc:
1. **Trạng thái tập trung** ở một `Model`.
2. **Mọi thay đổi là một `Message`** — liệt kê bằng enum (kiểu tổng, Chương 20). Không có hành động nào ngoài danh sách.
3. **Chỉ `update` được sửa trạng thái**, và nó **thuần túy**: `(model, msg) -> model`.

Lợi ích không phải lý thuyết. Vì `update` thuần túy:
- **Undo/redo**: chỉ cần lưu danh sách trạng thái hoặc thông điệp.
- **Ghi nhật ký & phát lại**: ghi chuỗi thông điệp, phát lại để dựng đúng trạng thái — chính là *event sourcing* ở Chương 54. Test `pure_update_enables_replay` chứng minh điều này.
- **Kiểm thử tầm thường**: gọi `update` với thông điệp, kiểm trạng thái ra. Không cần giao diện.

Đây là lý do Elm, Redux, Iced dùng trực tiếp mô hình này, và nhiều framework khác dùng biến thể lỏng hơn. gpui (của Zed) chẳng hạn không có một hàm `update` toàn cục: trạng thái nằm trong các *entity*, được sửa qua `Context` rồi gọi `cx.notify()` để báo cần vẽ lại — nhưng kỷ luật "sửa trạng thái qua một kênh có kiểm soát" vẫn giữ nguyên.

### 2. Cầu IPC — và vì sao nó là ranh giới bảo mật

Trong Tauri, giao diện chạy trong một **webview bị cô lập** — nó KHÔNG có quyền truy cập hệ thống tệp, mạng thô, hay tiến trình. Mọi thao tác cần đặc quyền phải gọi xuống lõi Rust qua **command**:

```
Frontend:  invoke("save_file", { name: "note.txt" })   ──►   Rust: #[tauri::command] fn save_file(...)
```

Đây không chỉ là cách giao tiếp — nó là **mô hình bảo mật**. Webview có thể chứa mã độc (quảng cáo bên thứ ba, XSS ở Chương 57), nhưng nó chỉ gọi được **các lệnh đã đăng ký**, và lõi Rust **kiểm duyệt từng lời gọi**. Test `ipc_blocks_path_traversal` cho thấy: dù frontend gửi `../../etc/passwd`, `notes/../../secret` hay `/etc/passwd`, lõi Rust chặn — webview không bao giờ ghi được ra ngoài thư mục app. Chú ý cách kiểm: `safe_relative_path` phân tích đường dẫn bằng `Path::components()` và chỉ chấp nhận các đoạn tên thường (`Component::Normal`), thay vì dò chuỗi `".."` — dò chuỗi dễ sót biến thể. Đây là "danh sách trắng" (Chương 57) áp vào ranh giới desktop.

### 3. Ba con đường xây desktop bằng Rust

**Tauri 2** — giao diện web, lõi Rust:
- *Ưu*: dùng lại kỹ năng web (React/Svelte/Vue); app nhẹ (~3–10MB thay vì ~150MB của Electron, vì dùng webview hệ điều hành thay vì đóng gói cả Chromium).
- *Nhược*: giao diện vẫn là web, phụ thuộc webview của từng OS.
- Tauri 2 thêm: hỗ trợ **di động** (iOS/Android), hệ thống quyền chi tiết hơn (*capabilities*: tệp cấu hình liệt kê lệnh/plugin nào cửa sổ nào được gọi).

**gpui** — giao diện native vẽ bằng GPU:
- *Ưu*: rất nhanh (Zed nhắm 120fps), 100% Rust, kiểm soát hoàn toàn. gpui có **renderer riêng** viết thẳng trên API đồ hoạ của từng nền tảng (Metal trên macOS, DirectX trên Windows, thư viện Blade trên Vulkan cho Linux), **không** xây trên wgpu.
- *Nhược*: phải tự dựng nhiều thứ; hệ sinh thái non trẻ hơn.
- Dùng khi hiệu năng giao diện là tối quan trọng (trình soạn thảo, công cụ đồ họa).

**wgpu** — đồ họa đa nền tảng cấp thấp:
- Là *hiện thực của API WebGPU trên Vulkan/Metal/DirectX 12/OpenGL* (và chạy được trong trình duyệt qua WebGPU/WebGL). Bevy và nhiều engine/ứng dụng đồ hoạ xây trên nó; Firefox dùng nó để hiện thực WebGPU.
- Dùng khi cần vẽ 2D/3D tùy biến hoàn toàn (game, mô phỏng, trực quan hóa dữ liệu).

### 4. "Một lõi, nhiều nền tảng" — sức mạnh thực sự

Điểm mạnh chung: **lõi nghiệp vụ Rust viết một lần, chạy mọi nơi**. Cùng một `Model` + `update` có thể phục vụ:
- App desktop (Tauri/gpui),
- App web (WASM, Chương 62),
- App di động (Tauri 2),
- Thậm chí server (Chương 61).

Đây là đỉnh cao của "lõi thuần túy, vỏ mệnh lệnh" (Chương 20): lõi không biết nó đang chạy trên nền nào; chỉ lớp vỏ mỏng thay đổi theo nền tảng.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch63
cargo test -p ch63
```

```rust
//! Chương 63 — Ứng dụng Desktop & Đa nền tảng: kiến trúc trạng thái (Elm/Redux),
//! cầu IPC frontend↔backend (như Tauri command). Lõi thuần túy, kiểm thử được.

use std::collections::HashMap;
use std::path::{Component, Path, PathBuf};

// ============================================================================
// 1. KIẾN TRÚC TRẠNG THÁI (The Elm Architecture) — Model · Message · update
// ============================================================================
// Mô hình quản lý trạng thái của Elm, Redux và Iced (framework GUI Rust):
// mọi thay đổi đi qua MỘT hàm `update` thuần túy. Không sửa trạng thái lung tung.

#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    pub tasks: Vec<Task>,
    pub filter: Filter,
    pub next_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Task {
    pub id: u64,
    pub title: String,
    pub done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Filter {
    All,
    Active,
    Completed,
}

/// Mọi thứ CÓ THỂ xảy ra trong ứng dụng, liệt kê bằng enum (kiểu tổng, Chương 20).
/// Không có hành động nào ngoài danh sách này — trạng thái thay đổi có kiểm soát.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AddTask(String),
    Toggle(u64),
    Remove(u64),
    SetFilter(Filter),
    ClearCompleted,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            tasks: Vec::new(),
            filter: Filter::All,
            next_id: 1,
        }
    }
}

impl Model {
    pub fn new() -> Self {
        Self::default()
    }

    /// HÀM `update` THUẦN TÚY: (trạng thái cũ, thông điệp) -> trạng thái mới.
    /// Đây là trái tim của kiến trúc: mọi thay đổi phải đi qua đây, nên dễ
    /// suy luận, dễ kiểm thử, dễ ghi lại (undo/redo, ghi nhật ký, phát lại).
    pub fn update(mut self, msg: Message) -> Self {
        match msg {
            Message::AddTask(title) => {
                let t = title.trim();
                if !t.is_empty() {
                    self.tasks.push(Task {
                        id: self.next_id,
                        title: t.to_string(),
                        done: false,
                    });
                    self.next_id += 1;
                }
            }
            Message::Toggle(id) => {
                if let Some(task) = self.tasks.iter_mut().find(|t| t.id == id) {
                    task.done = !task.done;
                }
            }
            Message::Remove(id) => {
                self.tasks.retain(|t| t.id != id);
            }
            Message::SetFilter(filter) => {
                self.filter = filter;
            }
            Message::ClearCompleted => {
                self.tasks.retain(|t| !t.done);
            }
        }
        self
    }

    /// Dẫn xuất: danh sách hiển thị theo bộ lọc hiện tại (view thuần túy).
    pub fn visible(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|t| match self.filter {
                Filter::All => true,
                Filter::Active => !t.done,
                Filter::Completed => t.done,
            })
            .collect()
    }

    pub fn pending_count(&self) -> usize {
        self.tasks.iter().filter(|t| !t.done).count()
    }
}

// ============================================================================
// 2. CẦU IPC — frontend gọi backend (như Tauri command)
// ============================================================================
// Trong Tauri, giao diện (JS/Svelte) gọi hàm Rust qua `invoke("command_name", args)`.
// Ta mô phỏng cầu đó: một bộ điều phối nhận tên lệnh + tham số, trả kết quả.

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    Ok(String),
    Failed(String),
}

pub trait BackendCommand {
    fn name(&self) -> &str;
    fn run(&self, args: &HashMap<String, String>) -> CommandResult;
}

/// Ví dụ lệnh: đọc thông tin hệ thống (backend làm việc mà webview không làm được).
pub struct SystemInfoCommand;
impl BackendCommand for SystemInfoCommand {
    fn name(&self) -> &str {
        "system_info"
    }
    fn run(&self, _: &HashMap<String, String>) -> CommandResult {
        CommandResult::Ok(format!(
            "os={};arch={}",
            std::env::consts::OS,
            std::env::consts::ARCH
        ))
    }
}

/// Kiểm một đường dẫn do frontend gửi: chỉ chấp nhận đường dẫn TƯƠNG ĐỐI gồm
/// toàn các đoạn tên thường. Phân tích theo `Path::components()` thay vì so chuỗi,
/// nên bắt được cả `a/../../x`, `/etc/passwd`, `./`... (Chương 57).
pub fn safe_relative_path(name: &str) -> Option<PathBuf> {
    // `\` là dấu phân cách trên Windows; NUL cắt chuỗi ở tầng hệ điều hành.
    if name.is_empty() || name.contains('\\') || name.contains('\0') {
        return None;
    }
    let mut clean = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            // RootDir (`/`), Prefix (`C:`), ParentDir (`..`), CurDir (`.`) đều bị từ chối
            _ => return None,
        }
    }
    if clean.as_os_str().is_empty() {
        None
    } else {
        Some(clean)
    }
}

/// Ví dụ lệnh: lưu tệp (thao tác hệ thống — chỉ backend được phép, vì bảo mật).
pub struct SaveFileCommand;
impl BackendCommand for SaveFileCommand {
    fn name(&self) -> &str {
        "save_file"
    }
    fn run(&self, args: &HashMap<String, String>) -> CommandResult {
        let Some(name) = args.get("name") else {
            return CommandResult::Failed("thiếu tên tệp".into());
        };
        // Chặn path traversal — webview không được ghi ra ngoài thư mục app!
        match safe_relative_path(name) {
            Some(path) => CommandResult::Ok(format!("đã lưu app_data/{}", path.display())),
            None => CommandResult::Failed("đường dẫn không an toàn".into()),
        }
    }
}

/// Cầu IPC: đăng ký lệnh và điều phối lời gọi từ frontend.
#[derive(Default)]
pub struct IpcBridge {
    commands: Vec<Box<dyn BackendCommand>>,
}
impl IpcBridge {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn register(mut self, command: Box<dyn BackendCommand>) -> Self {
        self.commands.push(command);
        self
    }
    /// invoke(name, args) — y hệt `invoke` của Tauri: chỉ lệnh ĐÃ ĐĂNG KÝ mới chạy được.
    pub fn invoke(&self, name: &str, args: HashMap<String, String>) -> CommandResult {
        match self.commands.iter().find(|c| c.name() == name) {
            Some(c) => c.run(&args),
            None => CommandResult::Failed(format!("lệnh {:?} không được đăng ký", name)),
        }
    }
}

fn args(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   DESKTOP & ĐA NỀN TẢNG: KIẾN TRÚC TRẠNG THÁI + CẦU IPC        ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n1. KIẾN TRÚC TRẠNG THÁI (Elm/Redux) — mọi thay đổi qua `update`");
    let m = Model::new()
        .update(Message::AddTask("Học Tauri".into()))
        .update(Message::AddTask("Viết ứng dụng".into()))
        .update(Message::AddTask("Đóng gói đa nền tảng".into()))
        .update(Message::Toggle(1)); // đánh dấu việc #1 xong

    println!(
        "   Tổng công việc: {}, chưa xong: {}",
        m.tasks.len(),
        m.pending_count()
    );
    let m = m.update(Message::SetFilter(Filter::Active));
    println!(
        "   Lọc 'chưa xong': {:?}",
        m.visible().iter().map(|t| &t.title).collect::<Vec<_>>()
    );

    println!("\n2. CẦU IPC — frontend (Svelte/JS) gọi backend (Rust)");
    let bridge = IpcBridge::new()
        .register(Box::new(SystemInfoCommand))
        .register(Box::new(SaveFileCommand));

    println!(
        "   invoke('system_info'): {:?}",
        bridge.invoke("system_info", HashMap::new())
    );
    for name in [
        "notes/ghi_chu.txt",
        "../../etc/passwd",
        "a/../../x",
        "/etc/shadow",
    ] {
        println!(
            "   invoke('save_file', {{name: '{name}'}}): {:?}",
            bridge.invoke("save_file", args(&[("name", name)]))
        );
    }
    println!(
        "   invoke('unknown'): {:?}",
        bridge.invoke("unknown", HashMap::new())
    );

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   MỘT LÕI RUST · NHIỀU NỀN TẢNG · GIAO DIỆN WEB HAY NATIVE      ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_task_increments_id() {
        let m = Model::new()
            .update(Message::AddTask("A".into()))
            .update(Message::AddTask("B".into()));
        assert_eq!(m.tasks.len(), 2);
        assert_eq!(m.tasks[0].id, 1);
        assert_eq!(m.tasks[1].id, 2);
    }

    #[test]
    fn empty_title_is_ignored() {
        let m = Model::new()
            .update(Message::AddTask("   ".into()))
            .update(Message::AddTask("".into()));
        assert_eq!(m.tasks.len(), 0);
    }

    #[test]
    fn toggle_state() {
        let m = Model::new().update(Message::AddTask("X".into()));
        assert!(!m.tasks[0].done);
        let m = m.update(Message::Toggle(1));
        assert!(m.tasks[0].done);
        let m = m.update(Message::Toggle(1)); // bật lại
        assert!(!m.tasks[0].done);
    }

    #[test]
    fn remove_and_clear_completed() {
        let m = Model::new()
            .update(Message::AddTask("A".into()))
            .update(Message::AddTask("B".into()))
            .update(Message::AddTask("C".into()))
            .update(Message::Toggle(1))
            .update(Message::Toggle(3));
        // Xóa 1 việc cụ thể
        let m2 = m.clone().update(Message::Remove(2));
        assert_eq!(m2.tasks.len(), 2);
        // Xóa mọi việc đã xong (1 và 3)
        let m3 = m.update(Message::ClearCompleted);
        assert_eq!(m3.tasks.len(), 1);
        assert_eq!(m3.tasks[0].title, "B");
    }

    #[test]
    fn filter_shows_correct_items() {
        let m = Model::new()
            .update(Message::AddTask("A".into()))
            .update(Message::AddTask("B".into()))
            .update(Message::Toggle(1)); // A xong
        let count = |f| m.clone().update(Message::SetFilter(f)).visible().len();
        assert_eq!(count(Filter::All), 2);
        assert_eq!(count(Filter::Completed), 1);
        assert_eq!(count(Filter::Active), 1);
    }

    #[test]
    fn pure_update_enables_replay() {
        // Vì update thuần túy, ta có thể PHÁT LẠI một chuỗi thông điệp để dựng
        // lại đúng trạng thái — nền của undo/redo và event sourcing (Chương 54).
        let history = vec![
            Message::AddTask("A".into()),
            Message::AddTask("B".into()),
            Message::Toggle(1),
        ];
        let replay = |list: &[Message]| {
            list.iter()
                .cloned()
                .fold(Model::new(), |m, msg| m.update(msg))
        };
        // Phát lại hai lần cho CÙNG kết quả (tất định)
        assert_eq!(replay(&history), replay(&history));
        assert!(replay(&history).tasks[0].done);
    }

    #[test]
    fn ipc_dispatches_commands() {
        let bridge = IpcBridge::new()
            .register(Box::new(SystemInfoCommand))
            .register(Box::new(SaveFileCommand));
        assert!(matches!(
            bridge.invoke("system_info", HashMap::new()),
            CommandResult::Ok(_)
        ));
        assert!(matches!(
            bridge.invoke("no_such_command", HashMap::new()),
            CommandResult::Failed(_)
        ));
    }

    #[test]
    fn ipc_blocks_path_traversal() {
        let bridge = IpcBridge::new().register(Box::new(SaveFileCommand));
        let save = |name: &str| bridge.invoke("save_file", args(&[("name", name)]));
        assert!(matches!(save("note.txt"), CommandResult::Ok(_)));
        assert!(matches!(save("notes/2024/a.txt"), CommandResult::Ok(_)));
        // Cầu IPC chặn — webview KHÔNG được ghi ra ngoài thư mục app (bảo mật)
        for bad in [
            "../../../etc/passwd",
            "notes/../../secret",
            "/etc/passwd",
            "..\\..\\windows\\system32",
            "./",
            "",
            "a\0b",
        ] {
            assert!(
                matches!(save(bad), CommandResult::Failed(_)),
                "phải chặn {bad:?}"
            );
        }
    }

    #[test]
    fn safe_path_normalizes_harmless_forms() {
        // `a//b` và `a/b/` vẫn là đường dẫn hợp lệ bên trong thư mục app
        assert_eq!(safe_relative_path("a//b"), Some(PathBuf::from("a/b")));
        assert_eq!(safe_relative_path("a/b/"), Some(PathBuf::from("a/b")));
    }
}
```

---

## Mã Tauri 2 + Svelte (minh hoạ)

Cùng lõi trên, đóng gói bằng Tauri 2 với giao diện Svelte. Các đoạn mã Tauri/Svelte/gpui dưới đây là **mã minh hoạ, cần một dự án riêng** (`cargo create-tauri-app`, hoặc một crate có `gpui`) — chúng không nằm trong workspace `code/` và không được biên dịch trong CI. API đã đối chiếu với tài liệu Tauri 2 và gpui 0.2 hiện hành:

```rust
// src-tauri/src/lib.rs  (lõi Rust)
mod core; // chép Model/Message/update từ code/ch63 vào src-tauri/src/core.rs
use core::{Message, Model};
use std::sync::Mutex;

// Lệnh backend: giao diện gọi qua invoke("add_task", { title })
#[tauri::command]
fn add_task(
    title: String,
    state: tauri::State<'_, Mutex<Model>>, // trạng thái chia sẻ (Chương 61)
) -> Result<usize, String> {
    let mut m = state.lock().map_err(|e| e.to_string())?;
    *m = m.clone().update(Message::AddTask(title));
    Ok(m.pending_count())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)] // Tauri 2: chạy cả trên di động!
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(Model::new()))
        .invoke_handler(tauri::generate_handler![add_task])
        .run(tauri::generate_context!())
        .expect("lỗi khởi chạy ứng dụng Tauri");
}
```

```svelte
<!-- src/App.svelte  (giao diện Svelte) -->
<script>
  import { invoke } from '@tauri-apps/api/core';
  let title = $state('');
  let remaining = $state(0);

  async function addTask() {
    // Gọi thẳng lệnh Rust `add_task` — tên lệnh và tên tham số phải khớp
    // chữ ký Rust (Tauri tự đổi tham số snake_case sang camelCase phía JS).
    remaining = await invoke('add_task', { title });
    title = '';
  }
</script>

<input bind:value={title} placeholder="Việc mới..." />
<button onclick={addTask}>Thêm</button>
<p>Còn lại: {remaining} việc chưa xong</p>
```

Svelte 5 dùng *rune* `$state` và thuộc tính `onclick` (cú pháp `on:click` cũ vẫn chạy ở chế độ tương thích). Chạy `cargo tauri dev` để phát triển, `cargo tauri build` để đóng gói ra `.dmg` (macOS), `.msi` (Windows), `.deb`/`.AppImage` (Linux) — **cùng một mã nguồn**.

## Mã gpui (giao diện native, như Zed)

```rust
// Cargo.toml: gpui = "0.2"   — gpui: giao diện vẽ bằng GPU, không dùng webview
use gpui::*;

struct Counter {
    count: i64,
}

impl Render for Counter {
    // gpui ≥ 0.2: render nhận `Window` + `Context<Self>` (thay cho `ViewContext<Self>` cũ)
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap_2()
            .child(format!("Đếm: {}", self.count))
            .child(
                div()
                    .id("inc") // phần tử cần `id` mới nhận được on_click
                    .child("Tăng")
                    .on_click(cx.listener(|this, _event, _window, cx| {
                        this.count += 1; // cập nhật trạng thái của entity
                        cx.notify(); // báo cần render lại
                    })),
            )
    }
}

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.open_window(WindowOptions::default(), |_window, cx| {
            cx.new(|_cx| Counter { count: 0 })
        })
        .unwrap();
    });
}
```

`cx.notify()` giống cơ chế "phiên bản tín hiệu" ở Chương 62 — báo cho hệ thống render biết entity đã đổi, cần vẽ lại. Khác với Leptos, gpui *chạy lại* `render` của view đó rồi dựng lại cây phần tử cho khung hình mới (kiểu chế độ tức thời, immediate mode, trên GPU); khác với tín hiệu, việc gọi `notify` là do bạn tự nhớ. Cùng một ý tưởng reactivity, khác cách hiện thực.

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0308: mismatched types` (expected `Model`, found `()`) | Hàm `update` quên trả về mô hình mới (thiếu `self` ở cuối) | Kiến trúc Elm đòi `update` là hàm **thuần tuý** trả về trạng thái mới, không sửa tại chỗ |
| `E0382: borrow of moved value` | Dùng lại mô hình sau khi đã chuyển vào `update` | Nhận `Model` và trả `Model`, hoặc nhận `&Model` rồi `clone()` |
| `E0277`: `dyn std::error::Error` cannot be sent between threads safely | Trả `Result<_, Box<dyn Error>>` từ `thread::spawn` của tác vụ nền | `Box<dyn Error + Send + Sync>` |
| `E0502`: cannot borrow `m.tasks` as mutable because it is also borrowed as immutable | Sửa danh sách việc khi kết quả `visible()` (một `Vec<&Task>`) vẫn còn được dùng | Dùng xong kết quả lọc rồi mới ghi; mọi thao tác ghi đi qua `update` |
| IPC nhận đường dẫn ra ngoài thư mục cho phép (lỗi logic, không phải lỗi biên dịch) | Chỉ dò chuỗi `".."` mà không phân tích đường dẫn | Duyệt `Path::components()`, chỉ nhận `Component::Normal` — xem `safe_relative_path` và `ipc_blocks_path_traversal` |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Kiến trúc Elm: mọi thay đổi qua một `update` thuần túy.** Trạng thái tập trung, thay đổi là enum thông điệp, chỉ `update` được sửa. Cho undo/redo và phát lại gần như miễn phí.
2. **Cầu IPC là ranh giới bảo mật.** Webview bị cô lập, chỉ gọi được lệnh đã đăng ký; lõi Rust kiểm duyệt từng lời gọi (chặn path traversal, Chương 57).
3. **Ba con đường**: Tauri (giao diện web, nhẹ, đa nền tảng cả di động) · gpui (native GPU, nhanh nhất) · wgpu (đồ họa cấp thấp).
4. **Một lõi Rust, nhiều nền tảng.** Cùng `Model`+`update` phục vụ desktop, web, di động, server — đỉnh cao của "lõi thuần túy, vỏ mệnh lệnh" (Chương 20).

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Undo/Redo)**
Dùng tính thuần túy của `update`, viết `History` lưu chuỗi `Model` cho phép `undo()` và `redo()`. Test một chuỗi thao tác rồi hoàn tác.

<details>
<summary><b>Gợi ý</b></summary>

Lưu `Vec<Model>` và một con trỏ vị trí hiện tại. `update` mới thì cắt bỏ phần "tương lai" (redo cũ) và thêm trạng thái mới; `undo` lùi con trỏ; `redo` tiến con trỏ. Vì mỗi `Model` là một ảnh chụp bất biến, không có gì bị hỏng — đây chính là lợi ích của trạng thái bất biến (Chương 13).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct History {
    states: Vec<Model>,
    cursor: usize,
}

impl Default for History {
    fn default() -> Self {
        History {
            states: vec![Model::new()],
            cursor: 0,
        }
    }
}

impl History {
    pub fn current(&self) -> &Model {
        &self.states[self.cursor]
    }
    pub fn apply(&mut self, msg: Message) {
        let new = self.current().clone().update(msg);
        self.states.truncate(self.cursor + 1); // bỏ nhánh redo cũ
        self.states.push(new);
        self.cursor += 1;
    }
    pub fn undo(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
        }
    }
    pub fn redo(&mut self) {
        if self.cursor + 1 < self.states.len() {
            self.cursor += 1;
        }
    }
}

#[cfg(test)]
mod exercise1 {
    use super::*;
    #[test]
    fn undo_redo_works() {
        let mut h = History::default();
        h.apply(Message::AddTask("A".into()));
        h.apply(Message::AddTask("B".into()));
        assert_eq!(h.current().tasks.len(), 2);
        h.undo();
        assert_eq!(h.current().tasks.len(), 1); // quay về sau khi thêm A
        h.redo();
        assert_eq!(h.current().tasks.len(), 2);
        h.undo();
        h.apply(Message::AddTask("C".into())); // nhánh mới xoá redo cũ
        h.redo();
        assert_eq!(h.current().tasks[1].title, "C");
    }
}
```
</details>

**Bài tập 2 (Lệnh IPC mới có kiểm quyền)**
Thêm lệnh `read_file` chỉ cho đọc tệp trong thư mục app (chặn `..` và đường dẫn tuyệt đối). Test rằng đường dẫn nguy hiểm bị từ chối.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct ReadFileCommand;
impl BackendCommand for ReadFileCommand {
    fn name(&self) -> &str {
        "read_file"
    }
    fn run(&self, args: &HashMap<String, String>) -> CommandResult {
        let Some(name) = args.get("name") else {
            return CommandResult::Failed("thiếu tên tệp".into());
        };
        // Dùng lại bộ kiểm đường dẫn của SaveFileCommand — một chỗ duy nhất để sửa
        match safe_relative_path(name) {
            Some(path) => CommandResult::Ok(format!("nội dung của {}", path.display())),
            None => CommandResult::Failed("đường dẫn không an toàn".into()),
        }
    }
}

#[cfg(test)]
mod exercise2 {
    use super::*;
    #[test]
    fn read_file_blocks_bad_path() {
        let bridge = IpcBridge::new().register(Box::new(ReadFileCommand));
        let read = |name: &str| bridge.invoke("read_file", args(&[("name", name)]));
        assert!(matches!(read("config.json"), CommandResult::Ok(_)));
        assert!(matches!(read("/etc/shadow"), CommandResult::Failed(_)));
        assert!(matches!(read("../secret"), CommandResult::Failed(_)));
    }
}
```
</details>

**Bài tập 3 (Tư duy: chọn công cụ desktop)**
Với mỗi ứng dụng, chọn Tauri / gpui / wgpu và giải thích:
1. Ứng dụng ghi chú đơn giản, cần chạy trên Windows, macOS, Linux.
2. Trình soạn thảo mã cần cuộn 100.000 dòng mượt 120fps.
3. Một game 2D indie.
4. Ứng dụng quản lý công việc nội bộ công ty, đội đã thạo React.

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **Tauri.** Giao diện đơn giản, ưu tiên nhẹ và đa nền tảng nhanh. Webview quá đủ.
2. **gpui.** Hiệu năng render là tối quan trọng; với webview rất khó giữ 120fps ổn định khi cuộn tệp 100k dòng. Đây chính là lý do Zed dùng gpui.
3. **wgpu** (hoặc engine dựng trên nó như Bevy). Game cần vẽ 2D/3D tùy biến, kiểm soát vòng lặp render.
4. **Tauri + React.** Dùng lại kỹ năng React sẵn có, lõi nghiệp vụ Rust; đóng gói nhanh cho cả công ty.

Nguyên tắc: **Tauri cho tốc độ phát triển và đa nền tảng; gpui/wgpu cho hiệu năng giao diện tối đa.** Phần lớn ứng dụng nghiệp vụ chọn Tauri; chỉ công cụ đòi hỏi render khắc nghiệt mới cần gpui/wgpu.
</details>

---

*Tới đây bạn đã đi một chặng dài: từ bóng bán dẫn (Chương 01) tới ứng dụng đa nền tảng chạy trên mọi thiết bị — tất cả bằng một ngôn ngữ duy nhất: Rust. Chương 64 đi xuống tầng thấp nhất của phần mềm: hệ điều hành. Quay lại [Mục lục](./SUMMARY.md).*
