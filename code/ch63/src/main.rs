#![allow(dead_code, unused_variables)]
//! Chương 63 — Ứng dụng Desktop & Đa nền tảng: kiến trúc trạng thái (Elm/Redux),
//! cầu IPC frontend↔backend (như Tauri command). Lõi thuần túy, kiểm thử được.

use std::collections::HashMap;

// ============================================================================
// 1. KIẾN TRÚC TRẠNG THÁI (The Elm Architecture) — Model · Message · update
// ============================================================================
// Đây là mô hình quản lý trạng thái mà Redux, Elm, và gpui (của Zed) đều dùng:
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
    TatCa,
    ChuaXong,
    DaXong,
}

/// Mọi thứ CÓ THỂ xảy ra trong ứng dụng, liệt kê bằng enum (kiểu tổng, Chương 20).
/// Không có hành động nào ngoài danh sách này — trạng thái thay đổi có kiểm soát.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    AddTask(String),
    BatTat(u64),
    Remove(u64),
    SetFilter(Filter),
    ClearCompleted,
}

impl Model {
    pub fn new() -> Self {
        Model {
            tasks: Vec::new(),
            filter: Filter::TatCa,
            next_id: 1,
        }
    }

    /// HÀM `update` THUẦN TÚY: (trạng thái cũ, thông điệp) -> trạng thái mới.
    /// Đây là trái tim của kiến trúc: mọi thay đổi phải đi qua đây, nên dễ
    /// suy luận, dễ kiểm thử, dễ ghi lại (undo/redo, ghi nhật ký, phát lại).
    pub fn update(mut self, td: Message) -> Self {
        match td {
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
            Message::BatTat(id) => {
                if let Some(cv) = self.tasks.iter_mut().find(|c| c.id == id) {
                    cv.done = !cv.done;
                }
            }
            Message::Remove(id) => {
                self.tasks.retain(|c| c.id != id);
            }
            Message::SetFilter(bl) => {
                self.filter = bl;
            }
            Message::ClearCompleted => {
                self.tasks.retain(|c| !c.done);
            }
        }
        self
    }

    /// Dẫn xuất: danh sách hiển thị theo bộ lọc hiện tại (view thuần túy).
    pub fn display(&self) -> Vec<&Task> {
        self.tasks
            .iter()
            .filter(|c| match self.filter {
                Filter::TatCa => true,
                Filter::ChuaXong => !c.done,
                Filter::DaXong => c.done,
            })
            .collect()
    }

    pub fn pending_count(&self) -> usize {
        self.tasks.iter().filter(|c| !c.done).count()
    }
}

// ============================================================================
// 2. CẦU IPC — frontend gọi backend (như Tauri command)
// ============================================================================
// Trong Tauri, giao diện (JS/Svelte) gọi hàm Rust qua `invoke("ten", param)`.
// Ta mô phỏng cầu đó: một bộ điều phối nhận tên lệnh + tham số, trả kết quả JSON.

#[derive(Debug, PartialEq)]
pub enum CommandResult {
    Ok(String),
    Failed(String),
}

pub trait BackendCommand {
    fn name(&self) -> &str;
    fn run(&self, param: &HashMap<String, String>) -> CommandResult;
}

/// Ví dụ lệnh: đọc thông tin hệ thống (backend làm việc mà webview không làm được).
pub struct SystemInfoCommand;
impl BackendCommand for SystemInfoCommand {
    fn name(&self) -> &str {
        "thong_tin_he_thong"
    }
    fn run(&self, _: &HashMap<String, String>) -> CommandResult {
        CommandResult::Ok("os=cross-platform;kien_truc=x86_64".to_string())
    }
}

/// Ví dụ lệnh: lưu tệp (thao tác hệ thống — chỉ backend được phép, vì bảo mật).
pub struct SaveFileCommand;
impl BackendCommand for SaveFileCommand {
    fn name(&self) -> &str {
        "luu_tep"
    }
    fn run(&self, param: &HashMap<String, String>) -> CommandResult {
        let name = match param.get("ten") {
            Some(t) if !t.is_empty() => t,
            _ => return CommandResult::Failed("thiếu tên tệp".into()),
        };
        // Chặn path traversal (Chương 57) — webview không được ghi ra ngoài thư mục app!
        if name.contains("..") || name.starts_with('/') {
            return CommandResult::Failed("đường dẫn không an toàn".into());
        }
        CommandResult::Ok(format!("đã lưu {}", name))
    }
}

/// Cầu IPC: đăng ký lệnh và điều phối lời gọi từ frontend.
pub struct IpcBridge {
    order: Vec<Box<dyn BackendCommand>>,
}
impl IpcBridge {
    pub fn new() -> Self {
        IpcBridge { order: Vec::new() }
    }
    pub fn register(mut self, l: Box<dyn BackendCommand>) -> Self {
        self.order.push(l);
        self
    }
    /// invoke(ten, param) — y hệt `invoke` của Tauri.
    pub fn invoke(&self, name: &str, param: HashMap<String, String>) -> CommandResult {
        match self.order.iter().find(|l| l.name() == name) {
            Some(l) => l.run(&param),
            None => CommandResult::Failed(format!("lệnh {:?} không được đăng ký", name)),
        }
    }
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
        .update(Message::BatTat(1)); // đánh dấu việc #1 xong

    println!(
        "   Tổng công việc: {}, chưa xong: {}",
        m.tasks.len(),
        m.pending_count()
    );
    let m = m.update(Message::SetFilter(Filter::ChuaXong));
    println!(
        "   Lọc 'chưa xong': {:?}",
        m.display().iter().map(|c| &c.title).collect::<Vec<_>>()
    );

    println!("\n2. CẦU IPC — frontend (Svelte/JS) gọi backend (Rust)");
    let cau = IpcBridge::new()
        .register(Box::new(SystemInfoCommand))
        .register(Box::new(SaveFileCommand));

    println!(
        "   invoke('thong_tin_he_thong'): {:?}",
        cau.invoke("thong_tin_he_thong", HashMap::new())
    );
    let mut ts = HashMap::new();
    ts.insert("ten".to_string(), "ghi_chu.txt".to_string());
    println!(
        "   invoke('luu_tep', {{name: 'ghi_chu.txt'}}): {:?}",
        cau.invoke("luu_tep", ts.clone())
    );
    ts.insert("ten".to_string(), "../../etc/passwd".to_string());
    println!(
        "   invoke('luu_tep', {{name: '../../etc/passwd'}}): {:?}",
        cau.invoke("luu_tep", ts)
    );
    println!(
        "   invoke('lenh_la'): {:?}",
        cau.invoke("lenh_la", HashMap::new())
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
    fn add_work_empty_is_unit_qua() {
        let m = Model::new()
            .update(Message::AddTask("   ".into()))
            .update(Message::AddTask("".into()));
        assert_eq!(m.tasks.len(), 0);
    }

    #[test]
    fn toggle_state() {
        let m = Model::new().update(Message::AddTask("X".into()));
        assert!(!m.tasks[0].done);
        let m = m.update(Message::BatTat(1));
        assert!(m.tasks[0].done);
        let m = m.update(Message::BatTat(1)); // bật lại
        assert!(!m.tasks[0].done);
    }

    #[test]
    fn remove_and_clear_completed() {
        let m = Model::new()
            .update(Message::AddTask("A".into()))
            .update(Message::AddTask("B".into()))
            .update(Message::AddTask("C".into()))
            .update(Message::BatTat(1))
            .update(Message::BatTat(3));
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
            .update(Message::BatTat(1)); // A xong
        assert_eq!(
            m.clone()
                .update(Message::SetFilter(Filter::TatCa))
                .display()
                .len(),
            2
        );
        assert_eq!(
            m.clone()
                .update(Message::SetFilter(Filter::DaXong))
                .display()
                .len(),
            1
        );
        assert_eq!(
            m.update(Message::SetFilter(Filter::ChuaXong))
                .display()
                .len(),
            1
        );
    }

    #[test]
    fn pure_update_enables_replay() {
        // Vì update thuần túy, ta có thể PHÁT LẠI một chuỗi thông điệp để dựng
        // lại đúng trạng thái — nền của undo/redo và event sourcing (Chương 54).
        let history = vec![
            Message::AddTask("A".into()),
            Message::AddTask("B".into()),
            Message::BatTat(1),
        ];
        let dung = |list: &[Message]| {
            list.iter()
                .cloned()
                .fold(Model::new(), |m, td| m.update(td))
        };
        // Phát lại hai lần cho CÙNG kết quả (tất định)
        assert_eq!(dung(&history), dung(&history));
    }

    #[test]
    fn ipc_dispatches_commands() {
        let cau = IpcBridge::new()
            .register(Box::new(SystemInfoCommand))
            .register(Box::new(SaveFileCommand));
        assert!(matches!(
            cau.invoke("thong_tin_he_thong", HashMap::new()),
            CommandResult::Ok(_)
        ));
        assert!(matches!(
            cau.invoke("lenh_khong_co", HashMap::new()),
            CommandResult::Failed(_)
        ));
    }

    #[test]
    fn ipc_blocks_path_traversal() {
        let cau = IpcBridge::new().register(Box::new(SaveFileCommand));
        let mut ok = HashMap::new();
        ok.insert("ten".into(), "note.txt".to_string());
        assert!(matches!(cau.invoke("luu_tep", ok), CommandResult::Ok(_)));

        let mut xau = HashMap::new();
        xau.insert("ten".into(), "../../../etc/passwd".to_string());
        // Cầu IPC chặn — webview KHÔNG được ghi ra ngoài thư mục app (bảo mật)
        assert!(matches!(
            cau.invoke("luu_tep", xau),
            CommandResult::Failed(_)
        ));
    }
}
