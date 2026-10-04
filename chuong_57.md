# Chương 57: Bảo mật ứng dụng Web — OSWE: SQLi, XSS, IDOR, SSRF, Xác thực & Path Traversal (Web Application Security)

## Giới thiệu & Mục tiêu học tập

Chủ đề 7 (Chương 37–42) đã dạy bạn tấn công **tầng bộ nhớ** theo tinh thần OSCP: buffer overflow, use-after-free, format string. Nhưng phần lớn ứng dụng ngày nay là **ứng dụng web**, và lỗ hổng web thuộc một thế giới hoàn toàn khác — chúng không nằm ở con trỏ, mà ở chỗ **lập trình viên tin tưởng dữ liệu người dùng**.

Chương này theo tinh thần chứng chỉ **OSWE (Offensive Security Web Expert)**: hiểu lỗ hổng bằng cách nhìn nó từ góc kẻ tấn công, rồi **viết mã phòng thủ chặn đứng nó**. Mỗi lỗ hổng trong chương đều có hai bản: bản `❌ DÍNH LỖI` và bản `✅ SỬA`, kèm test chứng minh bản sửa thực sự chặn được đòn tấn công.

> **Đây là giáo dục bảo mật phòng thủ.** Mục tiêu là để bạn *viết ứng dụng an toàn*, không phải tấn công hệ thống người khác. Mọi ví dụ đều là mô phỏng offline, không nhắm vào mục tiêu thật.

Điểm mạnh của Rust ở đây rất rõ: **hệ thống kiểu biến nhiều lỗ hổng thành lỗi biên dịch hoặc thành bất khả thi về mặt thiết kế**. Một câu SQL tham số hóa không thể bị tiêm; một hàm đòi id người gọi thì không thể gọi mà quên cung cấp danh tính; một kiểu bọc đã kiểm chứng (Chương 20) bảo đảm dữ liệu đúng *định dạng* — dù vẫn phải thoát ký tự khi xuất ra HTML, vì một email hợp lệ theo chuẩn vẫn có thể chứa `'` hay `<`.

Mục tiêu học tập:
- Hiểu **SQL Injection** và vì sao *tham số hóa* (không phải "lọc ký tự") mới là lời giải đúng.
- Chặn **XSS** bằng thoát ký tự HTML đúng ngữ cảnh.
- Chặn **IDOR** bằng cách bắt buộc kiểm tra quyền sở hữu trong chữ ký hàm.
- Chặn **SSRF** bằng danh sách trắng host và chặn dải mạng nội bộ (đặc biệt là metadata đám mây).
- Xác thực an toàn: **so sánh thời gian bất biến**, chính sách mật khẩu.
- Chặn **Path Traversal** — không cho `../` thoát khỏi thư mục gốc (kiểm theo `Path::components`).
- Nắm **Top 10 OWASP** dưới góc nhìn Rust.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: MỘT TÒA NHÀ VĂN PHÒNG VÀ CÁC KIỂU ĐỘT NHẬP                    │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  SQL INJECTION  = Đưa lễ tân một tờ giấy ghi tên khách, nhưng viết lén thêm      │
│                   "...VÀ mở luôn két sắt". Lễ tân ĐỌC CẢ CÂU như mệnh lệnh.      │
│                   → Sửa: lễ tân chỉ điền TÊN vào ô có sẵn, phần thừa là vô nghĩa.│
│                                                                                  │
│  XSS            = Dán một tờ thông báo có mực tàng hình chứa lệnh, ai đọc cũng   │
│                   bị sai khiến. → Sửa: đóng dấu "đây chỉ là chữ, không phải lệnh"│
│                                                                                  │
│  IDOR           = Thẻ phòng 301 nhưng bấm được cả cửa phòng 302. → Sửa: cửa      │
│                   kiểm tra thẻ CÓ ĐÚNG CHỦ phòng đó không.                       │
│                                                                                  │
│  SSRF           = Nhờ nhân viên nội bộ "ra ngoài mua giúp", nhưng đưa địa chỉ    │
│                   là KÉT SẮT CÔNG TY. Nhân viên có chìa khóa nội bộ! → Sửa: chỉ  │
│                   cho mua ở danh sách cửa hàng được duyệt.                       │
│                                                                                  │
│  PATH TRAVERSAL = Xin xem "hồ sơ của tôi" nhưng ghi "../../hồ sơ giám đốc".      │
│                   → Sửa: chặn mọi thành phần ".." trong đường dẫn.             │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. SQL Injection — tham số hóa, đừng "lọc"

Sai lầm phổ biến nhất là cố **lọc ký tự xấu** (loại bỏ dấu nháy, từ khóa `DROP`...). Cách này luôn thua, vì kẻ tấn công có vô số cách né bộ lọc (mã hóa, viết hoa lẫn lộn, ký tự Unicode tương đương).

Lời giải **đúng và duy nhất**: **tách rời cú pháp khỏi dữ liệu**. Câu SQL có cú pháp cố định với các chỗ trống `?`, còn dữ liệu người dùng chỉ là *giá trị* điền vào. Trình điều khiển cơ sở dữ liệu bảo đảm giá trị **không bao giờ** được diễn giải là cú pháp:

```
❌  "SELECT * FROM users WHERE name = '" + input + "'"     ← input trở thành CÚ PHÁP
✅  "SELECT * FROM users WHERE name = ?"  , [input]         ← input chỉ là GIÁ TRỊ
```

Trong Rust, các thư viện như `sqlx` còn đi xa hơn: `sqlx::query!` **kiểm tra câu SQL với cơ sở dữ liệu thật lúc biên dịch**, nên vừa chống tiêm vừa bắt lỗi sai tên cột trước khi chạy.

### 2. XSS — thoát ký tự theo đúng ngữ cảnh

XSS xảy ra khi dữ liệu người dùng được nhúng vào HTML mà không thoát. Kẻ tấn công gửi `<script>...</script>`, trình duyệt nạn nhân **thực thi** nó. Ba loại: phản chiếu (reflected), lưu trữ (stored, nguy hiểm nhất), và DOM-based.

Tuyến phòng thủ số 1 là **thoát ký tự** (`<` thành `&lt;`...). Nhưng phải thoát **đúng ngữ cảnh**: dữ liệu nhúng vào thân HTML, vào thuộc tính, vào URL, vào JavaScript đều có quy tắc thoát khác nhau. Trong Rust, các engine template như `askama` và `maud` **thoát tự động theo mặc định** — bạn phải chủ động yêu cầu "tin tưởng" thì nó mới không thoát. Đây là thiết kế "an toàn theo mặc định" (secure by default).

### 3. IDOR — kiểm tra quyền sở hữu, đừng chỉ kiểm tra tồn tại

IDOR là lỗ hổng **kiểm soát truy cập**: hệ thống tra đối tượng theo id nhưng quên hỏi *"người gọi có quyền với đối tượng này không?"*. Đổi `?id=100` thành `?id=101` là xem được dữ liệu người khác.

Rust cho một mẹo thiết kế mạnh: **đưa id người gọi vào chữ ký hàm bắt buộc**. So sánh:

```text
// ❌ dễ quên kiểm quyền
fn view_invoice_vulnerable(store: &[Invoice], id: u64) -> Option<&Invoice>
// ✅ KHÔNG THỂ gọi mà không có người gọi
fn view_invoice_safe(store: &[Invoice], id: u64, caller: u64) -> Result<&Invoice, AccessError>
```

Với chữ ký thứ hai, lập trình viên *không thể quên* cung cấp danh tính người gọi — trình biên dịch báo lỗi nếu thiếu (E0061). Việc *so sánh* quyền vẫn phải viết đúng bên trong hàm, nhưng nó nằm ở một chỗ duy nhất, dễ rà soát và có test.

### 4. SSRF — danh sách trắng, và đừng quên metadata đám mây

SSRF khiến **máy chủ** đi lấy một URL do kẻ tấn công chọn. Nguy hiểm vì máy chủ thường có quyền truy cập mạng nội bộ mà người ngoài không có. Mục tiêu khét tiếng nhất: **địa chỉ metadata đám mây `169.254.169.254`** — nơi AWS/GCP để lộ khóa truy cập tạm thời của máy chủ. Một SSRF tới địa chỉ này có thể chiếm luôn tài khoản đám mây.

Quy tắc phòng thủ:
1. **Danh sách trắng host**, không danh sách đen. Chỉ cho phép những host bạn *biết* là an toàn. Đây là tuyến phòng thủ chính.
2. **Chặn mọi dải mạng nội bộ**: `127.x`, `10.x`, `192.168.x`, `172.16–31.x`, đặc biệt `169.254.x`, và cả IPv6 (`::1`, `fc00::/7`, `fe80::/10`, `::ffff:a.b.c.d`). Phân tích bằng `std::net::IpAddr`, đừng so tiền tố chuỗi.
3. Chỉ cho `http`/`https`, chặn `file://`, `gopher://`...
4. **Cẩn thận khi tách host**: `https://api.good.vn:443@evil.com/` thực ra gửi tới `evil.com` (phần trước `@` là thông tin đăng nhập). Bộ tách tự chế rất dễ sai — thực tế dùng crate `url`, và kiểm lại IP *sau khi phân giải DNS* lúc kết nối (chống DNS rebinding).

### 5. Xác thực — đừng tự chế thuật toán mã hóa

Hai quy tắc sống còn:
- **Băm mật khẩu bằng thuật toán chuyên dụng chậm** (`argon2`, `bcrypt` — có crate Rust sẵn), **không bao giờ** dùng SHA-256 trần cho mật khẩu.
- **So sánh bí mật theo thời gian bất biến**. So sánh `==` thông thường dừng ngay ở byte sai đầu tiên, để lộ thông tin qua thời gian phản hồi (tấn công kênh kề, Chương 42). Hàm `constant_time_eq` trong mã dưới luôn duyệt hết mọi byte — nhưng trình biên dịch không *cam kết* giữ nguyên tính chất đó sau tối ưu hóa, nên sản phẩm thật dùng crate `subtle` (`ConstantTimeEq`).

### 6. Top 10 OWASP dưới góc nhìn Rust

| OWASP 2021 | Rust giúp thế nào |
|---|---|
| A01 Kiểm soát truy cập hỏng (IDOR) | Đưa danh tính vào chữ ký hàm; typestate cho phiên đăng nhập |
| A02 Lỗi mã hóa | Crate `argon2`, `ring`, `rustls` — đừng tự viết |
| A03 Injection (SQL/XSS) | `sqlx` (kiểm tra lúc biên dịch), template thoát tự động |
| A04 Thiết kế không an toàn | Kiểu bọc + smart constructor (Chương 20) biến trạng thái sai thành bất khả biểu diễn |
| A05 Cấu hình sai | `[profile.release]` gia cố (Chương 42) |
| A08 Toàn vẹn dữ liệu (deserialization) | `serde` an toàn kiểu — không có deserialization tùy tiện như Java/Python |
| A10 SSRF | Danh sách trắng như mã dưới |

> **Điểm mấu chốt**: rất nhiều lỗ hổng của A04/A08 đến từ việc ngôn ngữ động cho phép "dữ liệu biến thành mã" (eval, pickle, deserialization đa hình). Rust và thư viện chuẩn **không có** những cơ chế đó (và `serde` chỉ giải mã vào kiểu bạn khai báo) — cả một lớp lỗ hổng gần như biến mất, trừ khi bạn tự dựng lại nó.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch57
cargo test -p ch57
```

```rust
//! Chương 57 — OSWE: Bảo mật ứng dụng Web. Mỗi lỗ hổng có bản DÍNH LỖI và bản SỬA,
//! kèm test chứng minh bản sửa chặn được đòn tấn công. Toàn bộ chạy offline.

use std::path::PathBuf;

// ============================================================================
// 1. SQL INJECTION — và cách kiểu dữ liệu chặn nó
// ============================================================================

/// ❌ DÍNH LỖI: ghép chuỗi thẳng vào câu SQL. Kẻ tấn công gửi
/// `' OR '1'='1` để vượt qua điều kiện.
pub fn build_vulnerable_sql(username: &str) -> String {
    format!("SELECT * FROM users WHERE username = '{}'", username)
}

/// ✅ SỬA: dùng tham số hóa (placeholder). Dữ liệu người dùng KHÔNG BAO GIỜ
/// trở thành một phần cú pháp SQL — nó chỉ là *giá trị* điền vào chỗ `?`.
#[derive(Debug, PartialEq)]
pub struct SafeSql {
    pub template: String,   // "... WHERE username = ?"
    pub param: Vec<String>, // giá trị điền vào, tách RỜI khỏi cú pháp
}
pub fn build_safe_sql(username: &str) -> SafeSql {
    SafeSql {
        template: "SELECT * FROM users WHERE username = ?".to_string(),
        param: vec![username.to_string()],
    }
}

/// Kiểm tra một câu đã tham số hóa đúng cách: có chỗ trống `?` và số tham số
/// khớp số chỗ trống. Khi đó trình điều khiển CSDL gửi cú pháp và giá trị TÁCH
/// RIÊNG — giá trị không bao giờ được diễn giải thành cú pháp.
pub fn is_parameterized(query: &SafeSql) -> bool {
    // Với câu tham số hóa, dù tham số chứa gì thì cú pháp vẫn cố định.
    query.template.matches('?').count() == query.param.len() && query.template.contains('?')
}

// ============================================================================
// 2. XSS (Cross-Site Scripting) — thoát ký tự HTML
// ============================================================================

/// ✅ Thoát các ký tự nguy hiểm trước khi nhúng dữ liệu người dùng vào HTML.
/// Đây là tuyến phòng thủ số 1 chống XSS phản chiếu và lưu trữ.
pub fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// ❌ DÍNH LỖI: nhúng thẳng đầu vào vào HTML.
pub fn render_comment_vulnerable(comment: &str) -> String {
    format!("<div class=\"cmt\">{}</div>", comment)
}
/// ✅ SỬA: thoát trước khi nhúng.
pub fn render_comment_safe(comment: &str) -> String {
    format!("<div class=\"cmt\">{}</div>", escape_html(comment))
}

// ============================================================================
// 3. IDOR (Insecure Direct Object Reference) — kiểm tra quyền sở hữu
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Invoice {
    pub id: u64,
    pub owner: u64, // id người dùng sở hữu
    pub amount: u64,
}

#[derive(Debug, PartialEq)]
pub enum AccessError {
    NotFound,
    Forbidden, // đây là lỗ hổng IDOR nếu quên kiểm tra
}

/// ❌ DÍNH LỖI: chỉ tra theo id, KHÔNG kiểm tra người gọi có sở hữu không.
/// Kẻ tấn công đổi `?id=123` thành `?id=124` để xem hóa đơn người khác.
pub fn view_invoice_vulnerable(store: &[Invoice], id: u64) -> Option<&Invoice> {
    store.iter().find(|h| h.id == id)
}

/// ✅ SỬA: bắt buộc truyền id người gọi và kiểm tra quyền sở hữu.
pub fn view_invoice_safe(store: &[Invoice], id: u64, caller: u64) -> Result<&Invoice, AccessError> {
    let invoice = store
        .iter()
        .find(|h| h.id == id)
        .ok_or(AccessError::NotFound)?;
    if invoice.owner != caller {
        return Err(AccessError::Forbidden);
    }
    Ok(invoice)
}

// ============================================================================
// 4. SSRF (Server-Side Request Forgery) — danh sách trắng, không danh sách đen
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum UrlError {
    NotHttp,
    HasCredentials, // dạng `https://host-hợp-lệ@host-thật/` — kỹ thuật vượt bộ lọc kinh điển
    PointsToPrivateNetwork, // chặn loopback, 10.x, 172.16–31.x, 192.168.x, 169.254.x (metadata), IPv6 nội bộ
    HostNotAllowed,
}

/// ✅ Kiểm tra URL trước khi máy chủ đi lấy nội dung (chống SSRF).
/// Quy tắc: DANH SÁCH TRẮNG host cho phép, và chặn mọi địa chỉ mạng nội bộ.
/// (Bản giáo khoa: thực tế hãy dùng bộ phân tích URL chuẩn như crate `url`, và
/// kiểm tra lại ĐỊA CHỈ IP SAU KHI PHÂN GIẢI DNS ngay lúc kết nối — chống DNS rebinding.)
pub fn is_safe_url(url: &str, allowed_hosts: &[&str]) -> Result<(), UrlError> {
    let lower = url.to_ascii_lowercase();
    let after_scheme = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
        .ok_or(UrlError::NotHttp)?;

    // Phần "authority" kết thúc ở '/', '?' hoặc '#' đầu tiên
    let authority = after_scheme.split(['/', '?', '#']).next().unwrap_or("");
    // `https://api.good.vn:443@evil.com/` thực chất gửi tới evil.com!
    if authority.contains('@') {
        return Err(UrlError::HasCredentials);
    }
    // Tách cổng; IPv6 nằm trong ngoặc vuông: [::1]:8080
    let host = match authority.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or(""),
        None => authority.split(':').next().unwrap_or(""),
    };
    let host = host.trim_end_matches('.'); // "api.good.vn." cũng là api.good.vn

    // Chặn địa chỉ mạng nội bộ / loopback / metadata đám mây
    if is_internal_address(host) {
        return Err(UrlError::PointsToPrivateNetwork);
    }
    if !allowed_hosts.iter().any(|h| h.eq_ignore_ascii_case(host)) {
        return Err(UrlError::HostNotAllowed);
    }
    Ok(())
}

/// Host có trỏ vào mạng nội bộ không. Phân tích bằng `std::net::IpAddr` thay vì so
/// tiền tố chuỗi, nên bắt được cả IPv6 và IPv4 ánh xạ trong IPv6 (`::ffff:169.254.169.254`).
/// Các dạng viết IP lạ (`2852039166`, `0x7f.1`) không phân tích được thành IP ->
/// bị coi là tên miền và rơi vào kiểm tra danh sách trắng.
pub fn is_internal_address(host: &str) -> bool {
    use std::net::IpAddr;
    if host == "localhost" || host.ends_with(".localhost") {
        return true;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(v4)) => is_internal_v4(v4),
        Ok(IpAddr::V6(v6)) => {
            if let Some(v4) = v6.to_ipv4_mapped() {
                return is_internal_v4(v4);
            }
            let first = v6.segments()[0];
            v6.is_loopback()
                || v6.is_unspecified()
                || (first & 0xfe00) == 0xfc00 // fc00::/7 — địa chỉ cục bộ duy nhất
                || (first & 0xffc0) == 0xfe80 // fe80::/10 — link-local
        }
        Err(_) => false, // là tên miền
    }
}

fn is_internal_v4(ip: std::net::Ipv4Addr) -> bool {
    ip.is_loopback()          // 127.0.0.0/8
        || ip.is_private()    // 10/8, 172.16/12, 192.168/16
        || ip.is_link_local() // 169.254/16 — metadata AWS/GCP, mục tiêu SSRF phổ biến nhất
        || ip.is_unspecified() // 0.0.0.0
}

// ============================================================================
// 5. XÁC THỰC — so sánh thời gian bất biến & băm mật khẩu (không tự chế crypto)
// ============================================================================

/// ✅ So sánh chuỗi bí mật theo THỜI GIAN BẤT BIẾN (chống tấn công kênh kề).
/// Luôn duyệt hết mọi byte, không dừng sớm khi gặp byte sai (xem Chương 42).
/// Lưu ý: (1) độ dài vẫn bị lộ — chấp nhận được khi so mã băm/token có độ dài cố
/// định; (2) trình tối ưu hóa KHÔNG cam kết giữ mã này không rẽ nhánh. Trong sản
/// phẩm thật hãy dùng `subtle::ConstantTimeEq` (crate `subtle`), được viết riêng
/// để chống trình biên dịch "tối ưu hộ".
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff: u8 = 0;
    for (x, y) in a.iter().zip(b) {
        diff |= x ^ y; // gộp mọi khác biệt, không rẽ nhánh sớm
    }
    diff == 0
}

/// Kiểm tra ĐỘ MẠNH mật khẩu — chính sách tối thiểu.
#[derive(Debug, PartialEq)]
pub enum PasswordError {
    TooShort,
    MissingUppercase,
    MissingDigit,
    MissingSymbol,
}
pub fn check_strength(password: &str) -> Result<(), Vec<PasswordError>> {
    let mut errors = Vec::new();
    if password.chars().count() < 12 {
        errors.push(PasswordError::TooShort);
    }
    if !password.chars().any(|c| c.is_uppercase()) {
        errors.push(PasswordError::MissingUppercase);
    }
    if !password.chars().any(|c| c.is_ascii_digit()) {
        errors.push(PasswordError::MissingDigit);
    }
    if !password.chars().any(|c| !c.is_alphanumeric()) {
        errors.push(PasswordError::MissingSymbol);
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

// ============================================================================
// 6. PATH TRAVERSAL — chặn ../../etc/passwd
// ============================================================================

/// ✅ Kiểm tra đường dẫn tệp do người dùng cung cấp theo TỪNG THÀNH PHẦN
/// (`Path::components`), thay vì tìm chuỗi con: chỉ cho phép tên thường —
/// cấm `..`, đường dẫn tuyệt đối, tiền tố ổ đĩa Windows và byte NUL.
/// (Nếu thư mục gốc có thể chứa symlink do người dùng tạo, còn phải
/// `canonicalize` rồi kiểm tra kết quả vẫn nằm trong thư mục gốc.)
pub fn safe_path(root: &str, requested: &str) -> Result<PathBuf, String> {
    use std::path::{Component, Path};
    let blocked = || format!("Đường dẫn nguy hiểm bị chặn: {:?}", requested);
    if requested.is_empty() || requested.contains('\0') || requested.contains('\\') {
        return Err(blocked());
    }
    let mut result = PathBuf::from(root);
    for component in Path::new(requested).components() {
        match component {
            Component::Normal(part) => result.push(part),
            Component::CurDir => {}
            // ParentDir (..), RootDir (/), Prefix (C:) đều bị cấm
            _ => return Err(blocked()),
        }
    }
    Ok(result)
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   OSWE — BẢO MẬT ỨNG DỤNG WEB: 6 LỖ HỔNG KINH ĐIỂN & CÁCH SỬA  ");
    println!("═══════════════════════════════════════════════════════════════");

    let payload = "admin' OR '1'='1";
    println!("\n1. SQL INJECTION");
    println!("   Đầu vào tấn công: {:?}", payload);
    println!("   ❌ Ghép chuỗi : {}", build_vulnerable_sql(payload));
    let safe = build_safe_sql(payload);
    println!(
        "   ✅ Tham số hóa: {} | tham số = {:?}",
        safe.template, safe.param
    );
    println!("      → Đầu vào chỉ là GIÁ TRỊ, không thể trở thành cú pháp.");

    println!("\n2. XSS");
    let xss = "<script>steal(document.cookie)</script>";
    println!("   Đầu vào: {}", xss);
    println!("   ✅ Sau khi thoát: {}", escape_html(xss));

    println!("\n3. IDOR");
    let store = vec![
        Invoice {
            id: 100,
            owner: 1,
            amount: 500,
        },
        Invoice {
            id: 101,
            owner: 2,
            amount: 999,
        },
    ];
    println!("   Người dùng #1 xem hóa đơn #101 (của người #2):");
    println!(
        "   ❌ Bản lỗi cho xem: {:?}",
        view_invoice_vulnerable(&store, 101).map(|h| h.amount)
    );
    println!(
        "   ✅ Bản sửa chặn  : {:?}",
        view_invoice_safe(&store, 101, 1)
    );

    println!("\n4. SSRF");
    let allowed_hosts = ["api.partner.vn", "cdn.company.vn"];
    for u in [
        "https://api.partner.vn/data",
        "http://169.254.169.254/latest/meta-data/",
        "https://evil.com",
    ] {
        println!("   {:>45} -> {:?}", u, is_safe_url(u, &allowed_hosts));
    }

    println!("\n5. XÁC THỰC");
    println!(
        "   So sánh token bất biến: {}",
        constant_time_eq(b"secret123", b"secret123")
    );
    println!("   Mật khẩu 'abc': {:?}", check_strength("abc").is_err());
    println!(
        "   Mật khẩu 'Rust@2026!Secure': {:?}",
        check_strength("Rust@2026!Secure")
    );

    println!("\n6. PATH TRAVERSAL");
    println!("   {:?}", safe_path("/var/www/uploads", "avatar.png"));
    println!("   {:?}", safe_path("/var/www/uploads", "../../etc/passwd"));

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   ĐỪNG TIN DỮ LIỆU NGƯỜI DÙNG · DÙNG DANH SÁCH TRẮNG · KIỂM QUYỀN ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameterized_sql_resists_injection() {
        let payload = "admin' OR '1'='1; DROP TABLE users;--";
        let safe = build_safe_sql(payload);
        // Cú pháp cố định, chỉ 1 chỗ ?; toàn bộ đòn tấn công nằm trong THAM SỐ
        assert_eq!(safe.param, vec![payload.to_string()]);
        assert!(safe.template.matches('?').count() == 1);
        assert!(!safe.template.contains("OR")); // đầu vào KHÔNG lọt vào cú pháp
    }

    #[test]
    fn xss_escaping_covers_all_dangerous_chars() {
        let out = escape_html("<script>alert('x')</script>");
        assert!(!out.contains('<'));
        assert!(!out.contains('>'));
        assert!(out.contains("&lt;script&gt;"));
        // Bản sửa KHÔNG chứa thẻ script thực thi được
        assert!(!render_comment_safe("<img onerror=hack()>").contains("<img"));
    }

    #[test]
    fn idor_blocks_cross_user_access() {
        let store = vec![
            Invoice {
                id: 100,
                owner: 1,
                amount: 500,
            },
            Invoice {
                id: 101,
                owner: 2,
                amount: 999,
            },
        ];
        // Người #1 xem hóa đơn của chính mình -> OK
        assert!(view_invoice_safe(&store, 100, 1).is_ok());
        // Người #1 xem hóa đơn người #2 -> BỊ CHẶN
        assert_eq!(
            view_invoice_safe(&store, 101, 1),
            Err(AccessError::Forbidden)
        );
        // Hóa đơn không tồn tại
        assert_eq!(
            view_invoice_safe(&store, 999, 1),
            Err(AccessError::NotFound)
        );
    }

    #[test]
    fn ssrf_blocks_cloud_metadata_and_private_ranges() {
        let allowed = ["api.good.vn"];
        assert!(is_safe_url("https://api.good.vn/x", &allowed).is_ok());
        // Địa chỉ metadata đám mây — mục tiêu SSRF nguy hiểm nhất
        assert_eq!(
            is_safe_url("http://169.254.169.254/", &allowed),
            Err(UrlError::PointsToPrivateNetwork)
        );
        assert_eq!(
            is_safe_url("http://127.0.0.1:8080/admin", &allowed),
            Err(UrlError::PointsToPrivateNetwork)
        );
        assert_eq!(
            is_safe_url("http://10.0.0.5/", &allowed),
            Err(UrlError::PointsToPrivateNetwork)
        );
        assert_eq!(
            is_safe_url("http://172.16.0.1/", &allowed),
            Err(UrlError::PointsToPrivateNetwork)
        );
        assert_eq!(is_safe_url("http://172.15.0.1/", &["172.15.0.1"]), Ok(())); // 172.15 KHÔNG nội bộ
        // Host lạ không trong danh sách trắng
        assert_eq!(
            is_safe_url("https://evil.com/", &allowed),
            Err(UrlError::HostNotAllowed)
        );
        // Không phải http(s)
        assert_eq!(
            is_safe_url("file:///etc/passwd", &allowed),
            Err(UrlError::NotHttp)
        );
    }

    #[test]
    fn ssrf_parser_tricks_are_blocked() {
        let allowed = ["api.good.vn"];
        // Bản cũ tách host ở ':' đầu tiên -> thấy "api.good.vn" -> CHO QUA, dù
        // yêu cầu thật đi tới evil.com (phần trước '@' chỉ là thông tin đăng nhập)
        assert_eq!(
            is_safe_url("https://api.good.vn:443@evil.com/", &allowed),
            Err(UrlError::HasCredentials)
        );
        assert_eq!(
            is_safe_url("http://[::1]:8080/", &allowed),
            Err(UrlError::PointsToPrivateNetwork)
        );
        assert_eq!(
            is_safe_url("http://[::ffff:169.254.169.254]/", &allowed),
            Err(UrlError::PointsToPrivateNetwork)
        );
        assert_eq!(
            is_safe_url("http://2852039166/", &allowed), // = 169.254.169.254 dạng thập phân
            Err(UrlError::HostNotAllowed)
        );
        assert!(is_safe_url("HTTPS://API.GOOD.VN/x", &allowed).is_ok());
        assert!(is_safe_url("https://api.good.vn./x", &allowed).is_ok());
    }

    #[test]
    fn constant_time_compare_is_correct() {
        assert!(constant_time_eq(b"token-abc", b"token-abc"));
        assert!(!constant_time_eq(b"token-abc", b"token-xyz"));
        assert!(!constant_time_eq(b"ngan", b"dai-hon-nhieu")); // độ dài khác
    }

    #[test]
    fn password_strength() {
        assert!(check_strength("abc").is_err());
        assert!(check_strength("khongcosohoa!X").is_err()); // thiếu số
        assert!(check_strength("Rust@2026!Secure").is_ok());
        let error = check_strength("short").unwrap_err();
        assert!(error.contains(&PasswordError::TooShort));
    }

    #[test]
    fn path_traversal_is_blocked() {
        assert!(safe_path("/uploads", "anh.png").is_ok());
        assert!(safe_path("/uploads", "../../etc/passwd").is_err());
        assert!(safe_path("/uploads", "/etc/passwd").is_err());
        assert!(safe_path("/uploads", "a/../../secret").is_err());
        assert!(safe_path("/uploads", "..\\..\\windows").is_err());
        // Kiểm theo thành phần: tên tệp chứa ".." hợp lệ không bị chặn oan
        assert_eq!(
            safe_path("/uploads", "a/ban..sao.txt"),
            Ok(PathBuf::from("/uploads/a/ban..sao.txt"))
        );
    }
}
```

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0308: expected &str, found String` | Trộn `String` và `&str` khi ghép câu truy vấn | Nhận `&str` ở tham số, gọi `.as_str()` hoặc `&s` khi truyền |
| `E0502: cannot borrow as mutable` | Vừa đọc vừa ghi cùng một bộ đệm khi thoát ký tự | Dựng chuỗi kết quả **mới** thay vì sửa tại chỗ |
| `E0716: temporary value dropped while borrowed` | `&format!(...)` truyền thẳng vào hàm giữ tham chiếu | Gán ra biến trước: `let s = format!(...); f(&s);` |
| Escape XSS vẫn lọt | Thoát `<` `>` mà quên `&`, `"`, `'` | Thoát `&` **đầu tiên**, nếu không sẽ thoát chồng lên chính dấu vừa sinh |
| So sánh bí mật vẫn rò thời gian | Dùng `==` trên `String` | So sánh từng byte, không thoát sớm — thấy ở `constant_time_eq`; sản phẩm thật dùng crate `subtle` |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Đừng tin dữ liệu người dùng — bao giờ.** Mọi lỗ hổng trong chương đều bắt nguồn từ việc tin dữ liệu bên ngoài. Kiểm chứng ở cổng vào bằng kiểu bọc (Chương 20).
2. **Tách cú pháp khỏi dữ liệu** (SQL tham số hóa) và **thoát theo ngữ cảnh** (XSS). "Lọc ký tự xấu" luôn thua.
3. **Danh sách trắng, không danh sách đen** — cho SSRF, path, host. Và **kiểm tra quyền sở hữu**, không chỉ sự tồn tại (IDOR).
4. **Đừng tự chế crypto.** Dùng `argon2`, `rustls`, `ring`; so sánh bí mật theo thời gian bất biến.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Chống XSS trong thuộc tính)**
`escape_html` an toàn cho *thân* HTML. Nhưng nhúng vào một *thuộc tính* (`<a title="...">`) cần thoát thêm. Viết `escape_attribute` và test với đầu vào `" onmouseover="hack()`.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn escape_attribute(s: &str) -> String {
    // Trong thuộc tính, dấu nháy kép là ký tự thoát ra ngoài nguy hiểm nhất.
    // escape_html đã thoát cả " thành &quot; và ' thành &#x27; — đủ cho thuộc tính
    // CÓ BỌC NHÁY. Thuộc tính không bọc nháy (title=...) thì khoảng trắng cũng
    // phá được: luôn bọc giá trị thuộc tính trong nháy kép.
    escape_html(s)
}

#[cfg(test)]
mod exercise_1 {
    use super::*;
    #[test]
    fn attribute_payload_cannot_break_out() {
        let payload = "\" onmouseover=\"hack()";
        let out = format!("<a title=\"{}\">", escape_attribute(payload));
        assert!(!out.contains("onmouseover=\"hack"));
        assert!(out.contains("&quot;"));
    }
}
```
</details>

**Bài tập 2 (Giới hạn tần suất — chống dò mật khẩu)**
Viết `LoginCounter` cho phép tối đa 5 lần đăng nhập sai trong "cửa sổ" hiện tại, sau đó khóa. Dùng `HashMap<String, u32>`. Test rằng lần thứ 6 bị chặn.

<details>
<summary><b>Lời giải</b></summary>

```rust
use std::collections::HashMap;

pub struct LoginCounter {
    failures: HashMap<String, u32>,
    limit: u32,
}
impl LoginCounter {
    pub fn new(limit: u32) -> Self {
        LoginCounter { failures: HashMap::new(), limit }
    }
    pub fn try_login(&mut self, account: &str, correct: bool) -> Result<(), String> {
        let count = self.failures.entry(account.to_string()).or_insert(0);
        if *count >= self.limit {
            return Err("Tài khoản tạm khóa do quá nhiều lần sai".into());
        }
        if correct {
            *count = 0;
            Ok(())
        } else {
            *count += 1;
            Err("Sai mật khẩu".into())
        }
    }
}

#[cfg(test)]
mod exercise_2 {
    use super::*;
    #[test]
    fn locks_after_five_failures() {
        let mut counter = LoginCounter::new(5);
        for _ in 0..5 { let _ = counter.try_login("an", false); }
        // Lần thứ 6 bị chặn dù có nhập đúng
        assert!(counter.try_login("an", true).unwrap_err().contains("tạm khóa"));
    }
}
```

Trong sản phẩm thật, thêm yếu tố thời gian (cửa sổ trượt) và dùng Redis để đếm phân tán giữa nhiều máy chủ (Chương 52).
</details>

**Bài tập 3 (Tư duy: phân loại lỗ hổng)**
Với mỗi tình huống, chỉ ra lỗ hổng và cách sửa:
1. API `/api/user/123/profile` cho ai đăng nhập cũng xem được profile bất kỳ.
2. Ô tìm kiếm hiển thị lại từ khóa: `Kết quả cho: <từ khóa người dùng>`.
3. Tính năng "tải ảnh từ URL" cho nhập URL bất kỳ để server đi lấy.
4. Form đổi mật khẩu không hỏi mật khẩu cũ.

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **IDOR** (A01). Sửa: kiểm tra `user_id trong URL == user_id của phiên đăng nhập`, hoặc quyền admin.
2. **XSS phản chiếu** (A03). Sửa: `escape_html` từ khóa trước khi hiển thị.
3. **SSRF** (A10). Sửa: danh sách trắng host + chặn dải nội bộ như `is_safe_url`.
4. **Kiểm soát truy cập hỏng / CSRF** (A01). Sửa: bắt buộc xác nhận mật khẩu cũ, dùng token CSRF, và cân nhắc xác thực hai yếu tố cho thao tác nhạy cảm.

Nguyên tắc chung: mỗi lần dữ liệu **vào** (input) hay **ra** (output) qua một ranh giới tin cậy, hãy hỏi *"nếu đây là kẻ tấn công thì sao?"*.
</details>
