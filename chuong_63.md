# Chương 63: Ứng dụng Desktop & Đa nền tảng — Tauri 2.0, gpui & wgpu (Desktop & Cross-Platform Apps)

## Giới thiệu & Mục tiêu học tập

Rust không dừng ở máy chủ và trình duyệt — nó còn là nền cho **ứng dụng desktop hiệu năng cao chạy trên mọi hệ điều hành**. Trình soạn thảo mã [Zed](https://zed.dev/) (nhanh nhất thế giới, viết 100% bằng Rust), hàng nghìn ứng dụng [Tauri](https://tauri.app/), và các engine đồ họa dựa trên [wgpu](https://github.com/gfx-rs/wgpu) đều chứng minh điều đó.

Chương cuối này trình bày ba con đường xây ứng dụng desktop bằng Rust, mỗi con đường một triết lý:

| Công cụ | Triết lý | Ví dụ nổi bật |
|---|---|---|
| **[Tauri 2.0](https://github.com/tauri-apps/tauri)** | Giao diện = web (HTML/CSS/JS), lõi = Rust | Hàng nghìn app; thay thế Electron nhẹ hơn 10× |
| **[gpui](https://gpui.rs/)** | Giao diện native vẽ bằng GPU, 100% Rust | Trình soạn thảo Zed |
| **[wgpu](https://github.com/gfx-rs/wgpu)** | Đồ họa đa nền tảng cấp thấp (nền của gpui) | Game engine, ứng dụng 3D |

Điểm chung của cả ba — và là bài học trung tâm của chương — là **kiến trúc trạng thái**: mọi ứng dụng tương tác phức tạp đều cần một cách quản lý trạng thái có kỷ luật. Chương này xây **Kiến trúc Elm** (Model–Message–update) mà cả Redux, Elm và gpui đều dùng, cùng **cầu IPC** kiểu Tauri — tất cả thuần túy và kiểm thử được.

Mục tiêu học tập:
- Nắm **Kiến trúc Elm/Redux**: mọi thay đổi đi qua một hàm `update` thuần túy.
- Hiểu vì sao `update` thuần túy cho **undo/redo, ghi nhật ký, phát lại** gần như miễn phí (nối Chương 54).
- Thiết kế **cầu IPC** an toàn giữa giao diện và lõi Rust (như Tauri command).
- Biết ba con đường: **Tauri 2.0 + Svelte**, **gpui**, **wgpu** — và khi nào chọn cái nào.
- Áp dụng bảo mật (Chương 57) ở ranh giới IPC: chặn path traversal, không cho webview làm mọi thứ.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: MỘT NHÀ HÀNG VỚI PHÒNG ĂN VÀ BẾP TÁCH BIỆT                    │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   PHÒNG ĂN (Frontend)          │  CỬA SỔ BẾP (IPC)  │   BẾP (Backend Rust)       │
│   ─────────────────────        │   ──────────────    │   ──────────────────       │
│   · Trang trí đẹp (HTML/CSS)   │  Khách gọi món      │  · Nấu nướng thật (logic)  │
│   · Khách xem thực đơn         │  qua phiếu gọi:     │  · Đọc kho (hệ thống tệp)  │
│   · KHÔNG có dao, lửa, kho     │  invoke("nấu_phở",  │  · Có DAO, LỬA, KHO        │
│     → an toàn cho khách        │    {topping})       │  · Kiểm phiếu: món hợp lệ? │
│                                │  ───────────►       │    ai gọi có quyền?        │
│                                │  ◄───────────       │                            │
│                                │  Bếp trả món ra      │                            │
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
        │   OpenImage     │  toàn bộ trạng thái ứng dụng ở MỘT chỗ
        │  (Model)     │
        └──────┬───────┘
               │ view (hàm thuần túy)
               ▼
        [Giao diện]  ──sinh ra──►  ThongMessage (Message)
               ▲                          │
               │                          ▼
               │              ┌───────────────────────┐
               └──────────────│  update(model, msg)   │  hàm THUẦN TÚY:
                cập nhật      │  -> model mới          │  trạng thái cũ + thông điệp
                              └───────────────────────┘  = trạng thái mới
```

Ba quy tắc:
1. **Trạng thái tập trung** ở một `OpenImage`.
2. **Mọi thay đổi là một `ThongMessage`** — liệt kê bằng enum (kiểu tổng, Chương 20). Không có hành động nào ngoài danh sách.
3. **Chỉ `update` được sửa trạng thái**, và nó **thuần túy**: `(model, msg) -> model`.

Lợi ích không phải lý thuyết. Vì `update` thuần túy:
- **Undo/redo**: chỉ cần lưu danh sách trạng thái hoặc thông điệp.
- **Ghi nhật ký & phát lại**: ghi chuỗi thông điệp, phát lại để dựng đúng trạng thái — chính là *event sourcing* ở Chương 54. Test `pure_update_enables_replay` chứng minh điều này.
- **Kiểm thử tầm thường**: gọi `update` với thông điệp, kiểm trạng thái ra. Không cần giao diện.

Đây là lý do gpui (của Zed) và mọi framework UI nghiêm túc đều dùng biến thể của mô hình này.

### 2. Cầu IPC — và vì sao nó là ranh giới bảo mật

Trong Tauri, giao diện chạy trong một **webview bị cô lập** — nó KHÔNG có quyền truy cập hệ thống tệp, mạng thô, hay tiến trình. Mọi thao tác cần đặc quyền phải gọi xuống lõi Rust qua **command**:

```
Frontend:  invoke("luu_tep", { ten: "note.txt" })   ──►   Rust: #[tauri::command] fn luu_tep(...)
```

Đây không chỉ là cách giao tiếp — nó là **mô hình bảo mật**. Webview có thể chứa mã độc (quảng cáo bên thứ ba, XSS ở Chương 57), nhưng nó chỉ gọi được **các lệnh đã đăng ký**, và lõi Rust **kiểm duyệt từng lời gọi**. Test `ipc_blocks_path_traversal` cho thấy: dù frontend gửi `../../etc/passwd`, lõi Rust chặn — webview không bao giờ ghi được ra ngoài thư mục app. Đây là "danh sách trắng" (Chương 57) áp vào ranh giới desktop.

### 3. Ba con đường xây desktop bằng Rust

**Tauri 2.0** — giao diện web, lõi Rust:
- *Ưu*: dùng lại kỹ năng web (React/Svelte/Vue); app nhẹ (~3–10MB thay vì ~150MB của Electron, vì dùng webview hệ điều hành thay vì đóng gói cả Chromium).
- *Nhược*: giao diện vẫn là web, phụ thuộc webview của từng OS.
- Tauri 2.0 thêm: hỗ trợ **di động** (iOS/Android), hệ thống quyền chi tiết hơn.

**gpui** — giao diện native vẽ bằng GPU:
- *Ưu*: nhanh nhất (Zed render 120fps), 100% Rust, kiểm soát hoàn toàn.
- *Nhược*: phải tự dựng nhiều thứ; hệ sinh thái non trẻ hơn.
- Dùng khi hiệu năng giao diện là tối quan trọng (trình soạn thảo, công cụ đồ họa).

**wgpu** — đồ họa đa nền tảng cấp thấp:
- Là *lớp trừu tượng trên Vulkan/Metal/DirectX/WebGPU*. gpui và nhiều game engine xây trên nó.
- Dùng khi cần vẽ 2D/3D tùy biến hoàn toàn (game, mô phỏng, trực quan hóa dữ liệu).

### 4. "Một lõi, nhiều nền tảng" — sức mạnh thực sự

Điểm mạnh chung: **lõi nghiệp vụ Rust viết một lần, chạy mọi nơi**. Cùng một `OpenImage` + `update` có thể phục vụ:
- App desktop (Tauri/gpui),
- App web (WASM, Chương 62),
- App di động (Tauri 2.0),
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

// ============================================================================
// 1. KIẾN TRÚC TRẠNG THÁI (The Elm Architecture) — Model · Message · update
// ============================================================================
// Đây là mô hình quản lý trạng thái mà Redux, Elm, và gpui (của Zed) đều dùng:
// mọi thay đổi đi qua MỘT hàm `update` thuần túy. Không sửa trạng thái lung tung.

#[derive(Debug, Clone, PartialEq)]
pub struct OpenImage {
    pub job: Vec<WorkPort>,
    pub filter: Filter,
    pub next_id: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkPort {
    pub id: u64,
    pub title: String,
    pub done: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Filter { TatCa, ChuaXong, DaXong }

/// Mọi thứ CÓ THỂ xảy ra trong ứng dụng, liệt kê bằng enum (kiểu tổng, Chương 20).
/// Không có hành động nào ngoài danh sách này — trạng thái thay đổi có kiểm soát.
#[derive(Debug, Clone, PartialEq)]
pub enum ThongMessage {
    ThemViec(String),
    BatTat(u64),
    Xoa(u64),
    DoiBoLoc(Filter),
    XoaDaXong,
}

impl OpenImage {
    pub fn new() -> Self {
        OpenImage { job: Vec::new(), filter: Filter::TatCa, next_id: 1 }
    }

    /// HÀM `update` THUẦN TÚY: (trạng thái cũ, thông điệp) -> trạng thái mới.
    /// Đây là trái tim của kiến trúc: mọi thay đổi phải đi qua đây, nên dễ
    /// suy luận, dễ kiểm thử, dễ ghi lại (undo/redo, ghi nhật ký, phát lại).
    pub fn update(mut self, td: ThongMessage) -> Self {
        match td {
            ThongMessage::ThemViec(title) => {
                let t = title.trim();
                if !t.is_empty() {
                    self.job.push(WorkPort {
                        id: self.next_id, title: t.to_string(), done: false,
                    });
                    self.next_id += 1;
                }
            }
            ThongMessage::BatTat(id) => {
                if let Some(cv) = self.job.iter_mut().find(|c| c.id == id) {
                    cv.done = !cv.done;
                }
            }
            ThongMessage::Xoa(id) => {
                self.job.retain(|c| c.id != id);
            }
            ThongMessage::DoiBoLoc(bl) => {
                self.filter = bl;
            }
            ThongMessage::XoaDaXong => {
                self.job.retain(|c| !c.done);
            }
        }
        self
    }

    /// Dẫn xuất: danh sách hiển thị theo bộ lọc hiện tại (view thuần túy).
    pub fn display(&self) -> Vec<&WorkPort> {
        self.job.iter().filter(|c| match self.filter {
            Filter::TatCa => true,
            Filter::ChuaXong => !c.done,
            Filter::DaXong => c.done,
        }).collect()
    }

    pub fn pending_count(&self) -> usize {
        self.job.iter().filter(|c| !c.done).count()
    }
}

// ============================================================================
// 2. CẦU IPC — frontend gọi backend (như Tauri command)
// ============================================================================
// Trong Tauri, giao diện (JS/Svelte) gọi hàm Rust qua `invoke("ten", param)`.
// Ta mô phỏng cầu đó: một bộ điều phối nhận tên lệnh + tham số, trả kết quả JSON.

#[derive(Debug, PartialEq)]
pub enum ResultOrder {
    Ok(String),
    Loi(String),
}

pub trait BackendCommand {
    fn name(&self) -> &str;
    fn run(&self, param: &HashMap<String, String>) -> ResultOrder;
}

/// Ví dụ lệnh: đọc thông tin hệ thống (backend làm việc mà webview không làm được).
pub struct SystemInfoRequest;
impl BackendCommand for SystemInfoRequest {
    fn name(&self) -> &str { "thong_tin_he_thong" }
    fn run(&self, _: &HashMap<String, String>) -> ResultOrder {
        ResultOrder::Ok("os=cross-platform;kien_truc=x86_64".to_string())
    }
}

/// Ví dụ lệnh: lưu tệp (thao tác hệ thống — chỉ backend được phép, vì bảo mật).
pub struct OrderSaveFile;
impl BackendCommand for OrderSaveFile {
    fn name(&self) -> &str { "luu_tep" }
    fn run(&self, param: &HashMap<String, String>) -> ResultOrder {
        let name = match param.get("ten") {
            Some(t) if !t.is_empty() => t,
            _ => return ResultOrder::Loi("thiếu tên tệp".into()),
        };
        // Chặn path traversal (Chương 57) — webview không được ghi ra ngoài thư mục app!
        if name.contains("..") || name.starts_with('/') {
            return ResultOrder::Loi("đường dẫn không an toàn".into());
        }
        ResultOrder::Ok(format!("đã lưu {}", name))
    }
}

/// Cầu IPC: đăng ký lệnh và điều phối lời gọi từ frontend.
pub struct IpcBridge {
    order: Vec<Box<dyn BackendCommand>>,
}
impl IpcBridge {
    pub fn new() -> Self { IpcBridge { order: Vec::new() } }
    pub fn register(mut self, l: Box<dyn BackendCommand>) -> Self {
        self.order.push(l);
        self
    }
    /// invoke(ten, param) — y hệt `invoke` của Tauri.
    pub fn invoke(&self, name: &str, param: HashMap<String, String>) -> ResultOrder {
        match self.order.iter().find(|l| l.name() == name) {
            Some(l) => l.run(&param),
            None => ResultOrder::Loi(format!("lệnh {:?} không được đăng ký", name)),
        }
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   DESKTOP & ĐA NỀN TẢNG: KIẾN TRÚC TRẠNG THÁI + CẦU IPC        ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n1. KIẾN TRÚC TRẠNG THÁI (Elm/Redux) — mọi thay đổi qua `update`");
    let m = OpenImage::new()
        .update(ThongMessage::ThemViec("Học Tauri".into()))
        .update(ThongMessage::ThemViec("Viết ứng dụng".into()))
        .update(ThongMessage::ThemViec("Đóng gói đa nền tảng".into()))
        .update(ThongMessage::BatTat(1)); // đánh dấu việc #1 xong

    println!("   Tổng công việc: {}, chưa xong: {}", m.job.len(), m.pending_count());
    let m = m.update(ThongMessage::DoiBoLoc(Filter::ChuaXong));
    println!("   Lọc 'chưa xong': {:?}", m.display().iter().map(|c| &c.title).collect::<Vec<_>>());

    println!("\n2. CẦU IPC — frontend (Svelte/JS) gọi backend (Rust)");
    let cau = IpcBridge::new()
        .register(Box::new(SystemInfoRequest))
        .register(Box::new(OrderSaveFile));

    println!("   invoke('thong_tin_he_thong'): {:?}", cau.invoke("thong_tin_he_thong", HashMap::new()));
    let mut ts = HashMap::new();
    ts.insert("ten".to_string(), "ghi_chu.txt".to_string());
    println!("   invoke('luu_tep', {{name: 'ghi_chu.txt'}}): {:?}", cau.invoke("luu_tep", ts.clone()));
    ts.insert("ten".to_string(), "../../etc/passwd".to_string());
    println!("   invoke('luu_tep', {{name: '../../etc/passwd'}}): {:?}", cau.invoke("luu_tep", ts));
    println!("   invoke('lenh_la'): {:?}", cau.invoke("lenh_la", HashMap::new()));

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   MỘT LÕI RUST · NHIỀU NỀN TẢNG · GIAO DIỆN WEB HAY NATIVE      ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_task_increments_id() {
        let m = OpenImage::new()
            .update(ThongMessage::ThemViec("A".into()))
            .update(ThongMessage::ThemViec("B".into()));
        assert_eq!(m.job.len(), 2);
        assert_eq!(m.job[0].id, 1);
        assert_eq!(m.job[1].id, 2);
    }

    #[test]
    fn add_work_empty_is_unit_qua() {
        let m = OpenImage::new()
            .update(ThongMessage::ThemViec("   ".into()))
            .update(ThongMessage::ThemViec("".into()));
        assert_eq!(m.job.len(), 0);
    }

    #[test]
    fn toggle_state() {
        let m = OpenImage::new().update(ThongMessage::ThemViec("X".into()));
        assert!(!m.job[0].done);
        let m = m.update(ThongMessage::BatTat(1));
        assert!(m.job[0].done);
        let m = m.update(ThongMessage::BatTat(1)); // bật lại
        assert!(!m.job[0].done);
    }

    #[test]
    fn remove_and_clear_completed() {
        let m = OpenImage::new()
            .update(ThongMessage::ThemViec("A".into()))
            .update(ThongMessage::ThemViec("B".into()))
            .update(ThongMessage::ThemViec("C".into()))
            .update(ThongMessage::BatTat(1))
            .update(ThongMessage::BatTat(3));
        // Xóa 1 việc cụ thể
        let m2 = m.clone().update(ThongMessage::Xoa(2));
        assert_eq!(m2.job.len(), 2);
        // Xóa mọi việc đã xong (1 và 3)
        let m3 = m.update(ThongMessage::XoaDaXong);
        assert_eq!(m3.job.len(), 1);
        assert_eq!(m3.job[0].title, "B");
    }

    #[test]
    fn filter_shows_correct_items() {
        let m = OpenImage::new()
            .update(ThongMessage::ThemViec("A".into()))
            .update(ThongMessage::ThemViec("B".into()))
            .update(ThongMessage::BatTat(1)); // A xong
        assert_eq!(m.clone().update(ThongMessage::DoiBoLoc(Filter::TatCa)).display().len(), 2);
        assert_eq!(m.clone().update(ThongMessage::DoiBoLoc(Filter::DaXong)).display().len(), 1);
        assert_eq!(m.update(ThongMessage::DoiBoLoc(Filter::ChuaXong)).display().len(), 1);
    }

    #[test]
    fn pure_update_enables_replay() {
        // Vì update thuần túy, ta có thể PHÁT LẠI một chuỗi thông điệp để dựng
        // lại đúng trạng thái — nền của undo/redo và event sourcing (Chương 54).
        let history = vec![
            ThongMessage::ThemViec("A".into()),
            ThongMessage::ThemViec("B".into()),
            ThongMessage::BatTat(1),
        ];
        let dung = |list: &[ThongMessage]| list.iter().cloned().fold(OpenImage::new(), |m, td| m.update(td));
        // Phát lại hai lần cho CÙNG kết quả (tất định)
        assert_eq!(dung(&history), dung(&history));
    }

    #[test]
    fn ipc_dispatches_commands() {
        let cau = IpcBridge::new()
            .register(Box::new(SystemInfoRequest))
            .register(Box::new(OrderSaveFile));
        assert!(matches!(cau.invoke("thong_tin_he_thong", HashMap::new()), ResultOrder::Ok(_)));
        assert!(matches!(cau.invoke("lenh_khong_co", HashMap::new()), ResultOrder::Loi(_)));
    }

    #[test]
    fn ipc_blocks_path_traversal() {
        let cau = IpcBridge::new().register(Box::new(OrderSaveFile));
        let mut ok = HashMap::new();
        ok.insert("ten".into(), "note.txt".to_string());
        assert!(matches!(cau.invoke("luu_tep", ok), ResultOrder::Ok(_)));

        let mut xau = HashMap::new();
        xau.insert("ten".into(), "../../../etc/passwd".to_string());
        // Cầu IPC chặn — webview KHÔNG được ghi ra ngoài thư mục app (bảo mật)
        assert!(matches!(cau.invoke("luu_tep", xau), ResultOrder::Loi(_)));
    }
}
```

---

## Mã Tauri 2.0 + Svelte thật

Cùng lõi trên, đóng gói bằng Tauri với giao diện Svelte:

```rust
// src-tauri/src/lib.rs  (lõi Rust)
use std::sync::Mutex;

// Lệnh backend: giao diện gọi qua invoke("them_viec", {...})
#[tauri::command]
fn them_viec(
    title: String,
    state: tauri::State<Mutex<OpenImage>>,  // trạng thái chia sẻ (Chương 61)
) -> Result<usize, String> {
    let mut m = state.lock().unwrap();
    *m = m.clone().update(ThongMessage::ThemViec(title));
    Ok(m.pending_count())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]  // Tauri 2.0: chạy cả trên di động!
pub fn run() {
    tauri::Builder::default()
        .manage(Mutex::new(OpenImage::new()))
        .invoke_handler(tauri::generate_handler![them_viec])
        .run(tauri::generate_context!())
        .expect("lỗi khởi chạy ứng dụng Tauri");
}
```

```svelte
<!-- src/App.svelte  (giao diện Svelte) -->
<script>
  import { invoke } from '@tauri-apps/api/core';
  let title = '';
  let remaining = 0;

  async function them() {
    // Gọi thẳng hàm Rust — Tauri lo phần serialize/IPC
    remaining = await invoke('them_viec', { tieuDe: title });
    title = '';
  }
</script>

<input bind:value={title} placeholder="Việc mới..." />
<button on:click={them}>Thêm</button>
<p>Còn lại: {remaining} việc chưa xong</p>
```

Chạy `cargo tauri dev` để phát triển, `cargo tauri build` để đóng gói ra `.dmg` (macOS), `.msi` (Windows), `.deb`/`.AppImage` (Linux) — **cùng một mã nguồn**.

## Mã gpui (giao diện native, như Zed)

```rust
// gpui: giao diện vẽ bằng GPU, không dùng webview
use gpui::*;

struct BoDem { so: i64 }

impl Render for BoDem {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex().flex_col().gap_2()
            .child(format!("Đếm: {}", self.so))
            .child(
                div().id("tang").child("Tăng")
                    .on_click(cx.listener(|this, _, cx| {
                        this.so += 1;       // cập nhật trạng thái
                        cx.notify();        // báo cần render lại (như phiên bản tín hiệu ở Ch62)
                    }))
            )
    }
}
```

`cx.notify()` chính là cơ chế "phiên bản tín hiệu" ở Chương 62 — báo cho hệ thống render biết trạng thái đã đổi, cần vẽ lại. Cùng một ý tưởng reactivity, khác cách hiện thực.

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Kiến trúc Elm: mọi thay đổi qua một `update` thuần túy.** Trạng thái tập trung, thay đổi là enum thông điệp, chỉ `update` được sửa. Cho undo/redo và phát lại gần như miễn phí.
2. **Cầu IPC là ranh giới bảo mật.** Webview bị cô lập, chỉ gọi được lệnh đã đăng ký; lõi Rust kiểm duyệt từng lời gọi (chặn path traversal, Chương 57).
3. **Ba con đường**: Tauri (giao diện web, nhẹ, đa nền tảng cả di động) · gpui (native GPU, nhanh nhất) · wgpu (đồ họa cấp thấp).
4. **Một lõi Rust, nhiều nền tảng.** Cùng `OpenImage`+`update` phục vụ desktop, web, di động, server — đỉnh cao của "lõi thuần túy, vỏ mệnh lệnh" (Chương 20).

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Undo/Redo)**
Dùng tính thuần túy của `update`, viết `LichSu` lưu chuỗi `OpenImage` cho phép `hoan_tac()` và `lam_lai()`. Test một chuỗi thao tác rồi hoàn tác.

<details>
<summary><b>Gợi ý</b></summary>

Lưu `Vec<OpenImage>` và một con trỏ vị trí hiện tại. `update` mới thì cắt bỏ phần "tương lai" (redo cũ) và thêm trạng thái mới; `hoan_tac` lùi con trỏ; `lam_lai` tiến con trỏ. Vì mỗi `OpenImage` là một ảnh chụp bất biến, không có gì bị hỏng — đây chính là lợi ích của trạng thái bất biến (Chương 13).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct LichSu {
    state: Vec<OpenImage>,
    pos_value: usize,
}
impl LichSu {
    pub fn new() -> Self { LichSu { state: vec![OpenImage::new()], pos_value: 0 } }
    pub fn current(&self) -> &OpenImage { &self.state[self.pos_value] }
    pub fn thuc_hien(&mut self, td: ThongMessage) {
        let new = self.current().clone().update(td);
        self.state.truncate(self.pos_value + 1); // bỏ nhánh redo cũ
        self.state.push(new);
        self.pos_value += 1;
    }
    pub fn hoan_tac(&mut self) { if self.pos_value > 0 { self.pos_value -= 1; } }
    pub fn lam_lai(&mut self) { if self.pos_value + 1 < self.state.len() { self.pos_value += 1; } }
}

#[cfg(test)]
mod bt1 {
    use super::*;
    #[test]
    fn undo_redo_hoat_dong() {
        let mut ls = LichSu::new();
        ls.thuc_hien(ThongMessage::ThemViec("A".into()));
        ls.thuc_hien(ThongMessage::ThemViec("B".into()));
        assert_eq!(ls.current().job.len(), 2);
        ls.hoan_tac();
        assert_eq!(ls.current().job.len(), 1); // quay về sau khi thêm A
        ls.lam_lai();
        assert_eq!(ls.current().job.len(), 2);
    }
}
```
</details>

**Bài tập 2 (Lệnh IPC mới có kiểm quyền)**
Thêm lệnh `doc_tep` chỉ cho đọc tệp trong thư mục app (chặn `..` và đường dẫn tuyệt đối). Test rằng đường dẫn nguy hiểm bị từ chối.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub struct LenhDocTep;
impl BackendCommand for LenhDocTep {
    fn name(&self) -> &str { "doc_tep" }
    fn run(&self, param: &HashMap<String, String>) -> ResultOrder {
        let name = match param.get("ten") { Some(t) if !t.is_empty() => t,
            _ => return ResultOrder::Loi("thiếu tên".into()) };
        if name.contains("..") || name.starts_with('/') || name.contains('\0') {
            return ResultOrder::Loi("đường dẫn không an toàn".into());
        }
        ResultOrder::Ok(format!("nội dung của {}", name))
    }
}
#[cfg(test)]
mod bt2 {
    use super::*;
    #[test]
    fn doc_tep_chan_duong_dan_xau() {
        let cau = IpcBridge::new().register(Box::new(LenhDocTep));
        let mut ok = HashMap::new(); ok.insert("ten".into(), "config.json".to_string());
        assert!(matches!(cau.invoke("doc_tep", ok), ResultOrder::Ok(_)));
        let mut xau = HashMap::new(); xau.insert("ten".into(), "/etc/shadow".to_string());
        assert!(matches!(cau.invoke("doc_tep", xau), ResultOrder::Loi(_)));
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
2. **gpui.** Hiệu năng render là tối quan trọng; webview không đạt 120fps với 100k dòng. Đây chính là lý do Zed dùng gpui.
3. **wgpu** (hoặc engine dựng trên nó như Bevy). Game cần vẽ 2D/3D tùy biến, kiểm soát vòng lặp render.
4. **Tauri + React.** Dùng lại kỹ năng React sẵn có, lõi nghiệp vụ Rust; đóng gói nhanh cho cả công ty.

Nguyên tắc: **Tauri cho tốc độ phát triển và đa nền tảng; gpui/wgpu cho hiệu năng giao diện tối đa.** Phần lớn ứng dụng nghiệp vụ chọn Tauri; chỉ công cụ đòi hỏi render khắc nghiệt mới cần gpui/wgpu.
</details>

---

*Đây là chương cuối của phần mở rộng. Bạn đã đi trọn hành trình: từ bóng bán dẫn (Chương 01) tới ứng dụng đa nền tảng chạy trên mọi thiết bị (Chương 63) — tất cả bằng một ngôn ngữ duy nhất: Rust. Quay lại [Mục lục](./SUMMARY.md).*
