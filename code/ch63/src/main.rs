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
