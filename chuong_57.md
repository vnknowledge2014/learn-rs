# Chương 57: Bảo mật ứng dụng Web — OSWE: SQLi, XSS, IDOR, SSRF, Xác thực & Path Traversal (Web Application Security)

## Giới thiệu & Mục tiêu học tập

Chủ đề 7 (Chương 37–42) đã dạy bạn tấn công **tầng bộ nhớ** theo tinh thần OSCP: buffer overflow, use-after-free, format string. Nhưng phần lớn ứng dụng ngày nay là **ứng dụng web**, và lỗ hổng web thuộc một thế giới hoàn toàn khác — chúng không nằm ở con trỏ, mà ở chỗ **lập trình viên tin tưởng dữ liệu người dùng**.

Chương này theo tinh thần chứng chỉ **OSWE (Offensive Security Web Expert)**: hiểu lỗ hổng bằng cách nhìn nó từ góc kẻ tấn công, rồi **viết mã phòng thủ chặn đứng nó**. Mỗi lỗ hổng trong chương đều có hai bản: bản `❌ DÍNH LỖI` và bản `✅ SỬA`, kèm test chứng minh bản sửa thực sự chặn được đòn tấn công.

> **Đây là giáo dục bảo mật phòng thủ.** Mục tiêu là để bạn *viết ứng dụng an toàn*, không phải tấn công hệ thống người khác. Mọi ví dụ đều là mô phỏng offline, không nhắm vào mục tiêu thật.

Điểm mạnh của Rust ở đây rất rõ: **hệ thống kiểu biến nhiều lỗ hổng thành lỗi biên dịch hoặc thành bất khả thi về mặt thiết kế**. Một `Email` đã qua kiểm chứng (Chương 20) không thể chứa payload; một câu SQL tham số hóa không thể bị tiêm; một hàm đòi id người gọi thì không thể quên kiểm tra quyền.

Mục tiêu học tập:
- Hiểu **SQL Injection** và vì sao *tham số hóa* (không phải "lọc ký tự") mới là lời giải đúng.
- Chặn **XSS** bằng thoát ký tự HTML đúng ngữ cảnh.
- Chặn **IDOR** bằng cách bắt buộc kiểm tra quyền sở hữu trong chữ ký hàm.
- Chặn **SSRF** bằng danh sách trắng host và chặn dải mạng nội bộ (đặc biệt là metadata đám mây).
- Xác thực an toàn: **so sánh thời gian bất biến**, chính sách mật khẩu.
- Chặn **Path Traversal** — không cho `../` thoát khỏi thư mục gốc.
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
│                   → Sửa: chặn mọi ".." trong đường dẫn.                          │
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

```rust
fn xem_hoa_don(id: u64) -> Option<Invoice>              // ❌ dễ quên kiểm quyền
fn xem_hoa_don(id: u64, caller: u64) -> Result<..>  // ✅ KHÔNG THỂ gọi mà không có người gọi
```

Với chữ ký thứ hai, lập trình viên *không thể quên* — muốn gọi hàm là phải cung cấp danh tính người gọi, và trình biên dịch nhắc nếu thiếu.

### 4. SSRF — danh sách trắng, và đừng quên metadata đám mây

SSRF khiến **máy chủ** đi lấy một URL do kẻ tấn công chọn. Nguy hiểm vì máy chủ thường có quyền truy cập mạng nội bộ mà người ngoài không có. Mục tiêu khét tiếng nhất: **địa chỉ metadata đám mây `169.254.169.254`** — nơi AWS/GCP để lộ khóa truy cập tạm thời của máy chủ. Một SSRF tới địa chỉ này có thể chiếm luôn tài khoản đám mây.

Quy tắc phòng thủ:
1. **Danh sách trắng host**, không danh sách đen. Chỉ cho phép những host bạn *biết* là an toàn.
2. **Chặn mọi dải mạng nội bộ**: `127.x`, `10.x`, `192.168.x`, `172.16–31.x`, và đặc biệt `169.254.x`.
3. Chỉ cho `http`/`https`, chặn `file://`, `gopher://`...

### 5. Xác thực — đừng tự chế thuật toán mã hóa

Hai quy tắc sống còn:
- **Băm mật khẩu bằng thuật toán chuyên dụng chậm** (`argon2`, `bcrypt` — có crate Rust sẵn), **không bao giờ** dùng SHA-256 trần cho mật khẩu.
- **So sánh bí mật theo thời gian bất biến**. So sánh `==` thông thường dừng ngay ở byte sai đầu tiên, để lộ thông tin qua thời gian phản hồi (tấn công kênh kề, Chương 42). Hàm `so_sanh_bat_bien` trong mã dưới luôn duyệt hết mọi byte.

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

> **Điểm mấu chốt**: rất nhiều lỗ hổng của A04/A08 đến từ việc ngôn ngữ động cho phép "dữ liệu biến thành mã" (eval, pickle, deserialization đa hình). Rust **không có** những cơ chế đó — một cả lớp lỗ hổng đơn giản là không tồn tại.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch57
cargo test -p ch57
```

```rust
#![allow(dead_code, unused_variables)]
//! Chương 57 — OSWE: Bảo mật ứng dụng Web. Mỗi lỗ hổng có bản DÍNH LỖI và bản SỬA,
//! kèm test chứng minh bản sửa chặn được đòn tấn công. Toàn bộ chạy offline.


// ============================================================================
// 1. SQL INJECTION — và cách kiểu dữ liệu chặn nó
// ============================================================================

/// ❌ DÍNH LỖI: ghép chuỗi thẳng vào câu SQL. Kẻ tấn công gửi
/// `' OR '1'='1` để vượt qua điều kiện.
pub fn build_vulnerable_sql(name_dang_import: &str) -> String {
    format!("SELECT * FROM users WHERE username = '{}'", name_dang_import)
}

/// ✅ SỬA: dùng tham số hóa (placeholder). Dữ liệu người dùng KHÔNG BAO GIỜ
/// trở thành một phần cú pháp SQL — nó chỉ là *giá trị* điền vào chỗ `?`.
#[derive(Debug, PartialEq)]
pub struct SafeSql {
    pub mau: String,           // "... WHERE username = ?"
    pub param: Vec<String>,  // giá trị điền vào, tách RỜI khỏi cú pháp
}
pub fn build_safe_sql(name_dang_import: &str) -> SafeSql {
    SafeSql {
        mau: "SELECT * FROM users WHERE username = ?".to_string(),
        param: vec![name_dang_import.to_string()],
    }
}

/// Mô phỏng cách trình điều khiển cơ sở dữ liệu thật xử lý: giá trị được
/// "thoát" và bọc, không bao giờ được diễn giải là cú pháp.
pub fn co_the_bi_tiem_sql(cau: &SafeSql) -> bool {
    // Với câu tham số hóa, dù tham số chứa gì thì cú pháp vẫn cố định.
    cau.mau.matches('?').count() == cau.param.len() && cau.mau.contains('?')
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
pub fn render_comment_vulnerable(binh_luan: &str) -> String {
    format!("<div class=\"cmt\">{}</div>", binh_luan)
}
/// ✅ SỬA: thoát trước khi nhúng.
pub fn render_comment_safe(binh_luan: &str) -> String {
    format!("<div class=\"cmt\">{}</div>", escape_html(binh_luan))
}

// ============================================================================
// 3. IDOR (Insecure Direct Object Reference) — kiểm tra quyền sở hữu
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Invoice {
    pub id: u64,
    pub owner: u64, // id người dùng sở hữu
    pub so_tien: u64,
}

#[derive(Debug, PartialEq)]
pub enum ErrorAccessCap {
    KhongTonTai,
    KhongCoQuyen, // đây là lỗ hổng IDOR nếu quên kiểm tra
}

/// ❌ DÍNH LỖI: chỉ tra theo id, KHÔNG kiểm tra người gọi có sở hữu không.
/// Kẻ tấn công đổi `?id=123` thành `?id=124` để xem hóa đơn người khác.
pub fn invoice_view_error<'a>(store: &'a [Invoice], id: u64) -> Option<&'a Invoice> {
    store.iter().find(|h| h.id == id)
}

/// ✅ SỬA: bắt buộc truyền id người gọi và kiểm tra quyền sở hữu.
pub fn invoice_view_safe<'a>(
    store: &'a [Invoice],
    id: u64,
    caller: u64,
) -> Result<&'a Invoice, ErrorAccessCap> {
    let hd = store.iter().find(|h| h.id == id).ok_or(ErrorAccessCap::KhongTonTai)?;
    if hd.owner != caller {
        return Err(ErrorAccessCap::KhongCoQuyen);
    }
    Ok(hd)
}

// ============================================================================
// 4. SSRF (Server-Side Request Forgery) — danh sách trắng, không danh sách đen
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum UrlError {
    KhongPhaiHttp,
    TroToiMangNoiBo, // chặn 127.0.0.1, 169.254.x (metadata đám mây), 10.x, 192.168.x
    HostKhongDuocPhep,
}

/// ✅ Kiểm tra URL trước khi máy chủ đi lấy nội dung (chống SSRF).
/// Quy tắc: DANH SÁCH TRẮNG host cho phép, và chặn mọi địa chỉ mạng nội bộ.
pub fn is_safe_url(url: &str, host_cho_phep: &[&str]) -> Result<(), UrlError> {
    let sau_scheme = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or(UrlError::KhongPhaiHttp)?;

    let host = sau_scheme.split(['/', ':']).next().unwrap_or("");

    // Chặn địa chỉ mạng nội bộ / loopback / metadata đám mây
    if is_unit_address(host) {
        return Err(UrlError::TroToiMangNoiBo);
    }
    if !host_cho_phep.contains(&host) {
        return Err(UrlError::HostKhongDuocPhep);
    }
    Ok(())
}

pub fn is_unit_address(host: &str) -> bool {
    host == "localhost"
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || host.starts_with("169.254.") // metadata AWS/GCP — mục tiêu SSRF phổ biến nhất
        || host == "0.0.0.0"
        || host == "[::1]"
        || {
            // 172.16.0.0 – 172.31.255.255
            host.strip_prefix("172.")
                .and_then(|r| r.split('.').next())
                .and_then(|o| o.parse::<u8>().ok())
                .map(|o| (16..=31).contains(&o))
                .unwrap_or(false)
        }
}

// ============================================================================
// 5. XÁC THỰC — so sánh thời gian bất biến & băm mật khẩu (không tự chế crypto)
// ============================================================================

/// ✅ So sánh chuỗi bí mật theo THỜI GIAN BẤT BIẾN (chống tấn công kênh kề).
/// Luôn duyệt hết mọi byte, không dừng sớm khi gặp byte sai (xem Chương 42).
pub fn so_sanh_bat_bien(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut other: u8 = 0;
    for i in 0..a.len() {
        other |= a[i] ^ b[i]; // gộp mọi khác biệt, không rẽ nhánh sớm
    }
    other == 0
}

/// Kiểm tra ĐỘ MẠNH mật khẩu — chính sách tối thiểu.
#[derive(Debug, PartialEq)]
pub enum ErrorPassword {
    QuaNgan,
    ThieuChuHoa,
    ThieuChuSo,
    ThieuKyTuDacBiet,
}
pub fn check_do_strong(mk: &str) -> Result<(), Vec<ErrorPassword>> {
    let mut error = Vec::new();
    if mk.chars().count() < 12 {
        error.push(ErrorPassword::QuaNgan);
    }
    if !mk.chars().any(|c| c.is_uppercase()) {
        error.push(ErrorPassword::ThieuChuHoa);
    }
    if !mk.chars().any(|c| c.is_ascii_digit()) {
        error.push(ErrorPassword::ThieuChuSo);
    }
    if !mk.chars().any(|c| !c.is_alphanumeric()) {
        error.push(ErrorPassword::ThieuKyTuDacBiet);
    }
    if error.is_empty() { Ok(()) } else { Err(error) }
}

// ============================================================================
// 6. PATH TRAVERSALL — chặn ../../etc/passwd
// ============================================================================

/// ✅ Chuẩn hóa và kiểm tra đường dẫn tệp do người dùng cung cấp.
/// Chặn `..` để không thoát ra khỏi thư mục gốc cho phép.
pub fn path_safe(root: &str, yeu_cau: &str) -> Result<String, String> {
    if yeu_cau.contains("..") || yeu_cau.starts_with('/') || yeu_cau.contains('\0') {
        return Err(format!("Đường dẫn nguy hiểm bị chặn: {:?}", yeu_cau));
    }
    Ok(format!("{}/{}", root.trim_end_matches('/'), yeu_cau))
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   OSWE — BẢO MẬT ỨNG DỤNG WEB: 6 LỖ HỔNG KINH ĐIỂN & CÁCH SỬA  ");
    println!("═══════════════════════════════════════════════════════════════");

    let doc = "admin' OR '1'='1";
    println!("\n1. SQL INJECTION");
    println!("   Đầu vào tấn công: {:?}", doc);
    println!("   ❌ Ghép chuỗi : {}", build_vulnerable_sql(doc));
    let an = build_safe_sql(doc);
    println!("   ✅ Tham số hóa: {} | tham số = {:?}", an.mau, an.param);
    println!("      → Đầu vào chỉ là GIÁ TRỊ, không thể trở thành cú pháp.");

    println!("\n2. XSS");
    let xss = "<script>steal(document.cookie)</script>";
    println!("   Đầu vào: {}", xss);
    println!("   ✅ Sau khi thoát: {}", escape_html(xss));

    println!("\n3. IDOR");
    let store = vec![
        Invoice { id: 100, owner: 1, so_tien: 500 },
        Invoice { id: 101, owner: 2, so_tien: 999 },
    ];
    println!("   Người dùng #1 xem hóa đơn #101 (của người #2):");
    println!("   ❌ Bản lỗi cho xem: {:?}", invoice_view_error(&store, 101).map(|h| h.so_tien));
    println!("   ✅ Bản sửa chặn  : {:?}", invoice_view_safe(&store, 101, 1));

    println!("\n4. SSRF");
    let wait_op = ["api.doitac.vn", "cdn.congty.vn"];
    for u in ["https://api.doitac.vn/data", "http://169.254.169.254/latest/meta-data/", "https://evil.com"] {
        println!("   {:>45} -> {:?}", u, is_safe_url(u, &wait_op));
    }

    println!("\n5. XÁC THỰC");
    println!("   So sánh token bất biến: {}", so_sanh_bat_bien(b"secret123", b"secret123"));
    println!("   Mật khẩu 'abc': {:?}", check_do_strong("abc").is_err());
    println!("   Mật khẩu 'Rust@2026!Secure': {:?}", check_do_strong("Rust@2026!Secure"));

    println!("\n6. PATH TRAVERSAL");
    println!("   {:?}", path_safe("/var/www/uploads", "avatar.png"));
    println!("   {:?}", path_safe("/var/www/uploads", "../../etc/passwd"));

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   ĐỪNG TIN DỮ LIỆU NGƯỜI DÙNG · DÙNG DANH SÁCH TRẮNG · KIỂM QUYỀN ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameterized_sql_resists_injection() {
        let doc = "admin' OR '1'='1; DROP TABLE users;--";
        let an = build_safe_sql(doc);
        // Cú pháp cố định, chỉ 1 chỗ ?; toàn bộ đòn tấn công nằm trong THAM SỐ
        assert_eq!(an.param, vec![doc.to_string()]);
        assert!(an.mau.matches('?').count() == 1);
        assert!(!an.mau.contains("OR")); // đầu vào KHÔNG lọt vào cú pháp
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
            Invoice { id: 100, owner: 1, so_tien: 500 },
            Invoice { id: 101, owner: 2, so_tien: 999 },
        ];
        // Người #1 xem hóa đơn của chính mình -> OK
        assert!(invoice_view_safe(&store, 100, 1).is_ok());
        // Người #1 xem hóa đơn người #2 -> BỊ CHẶN
        assert_eq!(invoice_view_safe(&store, 101, 1), Err(ErrorAccessCap::KhongCoQuyen));
        // Hóa đơn không tồn tại
        assert_eq!(invoice_view_safe(&store, 999, 1), Err(ErrorAccessCap::KhongTonTai));
    }

    #[test]
    fn ssrf_blocks_cloud_metadata_and_private_ranges() {
        let cp = ["api.tot.vn"];
        assert!(is_safe_url("https://api.tot.vn/x", &cp).is_ok());
        // Địa chỉ metadata đám mây — mục tiêu SSRF nguy hiểm nhất
        assert_eq!(is_safe_url("http://169.254.169.254/", &cp), Err(UrlError::TroToiMangNoiBo));
        assert_eq!(is_safe_url("http://127.0.0.1:8080/admin", &cp), Err(UrlError::TroToiMangNoiBo));
        assert_eq!(is_safe_url("http://10.0.0.5/", &cp), Err(UrlError::TroToiMangNoiBo));
        assert_eq!(is_safe_url("http://172.16.0.1/", &cp), Err(UrlError::TroToiMangNoiBo));
        assert_eq!(is_safe_url("http://172.15.0.1/", &["172.15.0.1"]), Ok(())); // 172.15 KHÔNG nội bộ
        // Host lạ không trong danh sách trắng
        assert_eq!(is_safe_url("https://evil.com/", &cp), Err(UrlError::HostKhongDuocPhep));
        // Không phải http(s)
        assert_eq!(is_safe_url("file:///etc/passwd", &cp), Err(UrlError::KhongPhaiHttp));
    }

    #[test]
    fn constant_time_compare_is_correct() {
        assert!(so_sanh_bat_bien(b"token-abc", b"token-abc"));
        assert!(!so_sanh_bat_bien(b"token-abc", b"token-xyz"));
        assert!(!so_sanh_bat_bien(b"ngan", b"dai-hon-nhieu")); // độ dài khác
    }

    #[test]
    fn password_strength() {
        assert!(check_do_strong("abc").is_err());
        assert!(check_do_strong("khongcosohoa!X").is_err()); // thiếu số
        assert!(check_do_strong("Rust@2026!Secure").is_ok());
        let error = check_do_strong("short").unwrap_err();
        assert!(error.contains(&ErrorPassword::QuaNgan));
    }

    #[test]
    fn path_traversal_is_blocked() {
        assert!(path_safe("/uploads", "anh.png").is_ok());
        assert!(path_safe("/uploads", "../../etc/passwd").is_err());
        assert!(path_safe("/uploads", "/etc/passwd").is_err());
        assert!(path_safe("/uploads", "a/../../secret").is_err());
    }
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Đừng tin dữ liệu người dùng — bao giờ.** Mọi lỗ hổng trong chương đều bắt nguồn từ việc tin dữ liệu bên ngoài. Kiểm chứng ở cổng vào bằng kiểu bọc (Chương 20).
2. **Tách cú pháp khỏi dữ liệu** (SQL tham số hóa) và **thoát theo ngữ cảnh** (XSS). "Lọc ký tự xấu" luôn thua.
3. **Danh sách trắng, không danh sách đen** — cho SSRF, path, host. Và **kiểm tra quyền sở hữu**, không chỉ sự tồn tại (IDOR).
4. **Đừng tự chế crypto.** Dùng `argon2`, `rustls`, `ring`; so sánh bí mật theo thời gian bất biến.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Chống XSS trong thuộc tính)**
`escape_html` an toàn cho *thân* HTML. Nhưng nhúng vào một *thuộc tính* (`<a title="...">`) cần thoát thêm. Viết `thoat_thuoc_tinh` và test với đầu vào `" onmouseover="hack()`.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn thoat_thuoc_tinh(s: &str) -> String {
    // Trong thuộc tính, dấu nháy kép là ký tự thoát ra ngoài nguy hiểm nhất
    escape_html(s) // đã thoát cả " thành &quot; và ' thành &#x27;
}

#[cfg(test)]
mod bt1 {
    use super::*;
    #[test]
    fn khong_thoat_ra_ngoai_thuoc_tinh() {
        let payload = "\" onmouseover=\"hack()";
        let out = format!("<a title=\"{}\">", thoat_thuoc_tinh(payload));
        assert!(!out.contains("onmouseover=\"hack"));
        assert!(out.contains("&quot;"));
    }
}
```
</details>

**Bài tập 2 (Giới hạn tần suất — chống dò mật khẩu)**
Viết `BoDemDangNhap` cho phép tối đa 5 lần đăng nhập sai trong "cửa sổ" hiện tại, sau đó khóa. Dùng `HashMap<String, u32>`. Test rằng lần thứ 6 bị chặn.

<details>
<summary><b>Lời giải</b></summary>

```rust
use std::collections::HashMap;

pub struct BoDemDangNhap {
    lan_sai: HashMap<String, u32>,
    gioi_han: u32,
}
impl BoDemDangNhap {
    pub fn new(gioi_han: u32) -> Self {
        BoDemDangNhap { lan_sai: HashMap::new(), gioi_han }
    }
    pub fn thu_dang_nhap(&mut self, account: &str, dung: bool) -> Result<(), String> {
        let count = self.lan_sai.entry(account.to_string()).or_insert(0);
        if *count >= self.gioi_han {
            return Err("Tài khoản tạm khóa do quá nhiều lần sai".into());
        }
        if dung {
            *count = 0;
            Ok(())
        } else {
            *count += 1;
            Err("Sai mật khẩu".into())
        }
    }
}

#[cfg(test)]
mod bt2 {
    use super::*;
    #[test]
    fn khoa_sau_5_lan_sai() {
        let mut bd = BoDemDangNhap::new(5);
        for _ in 0..5 { let _ = bd.thu_dang_nhap("an", false); }
        // Lần thứ 6 bị chặn dù có nhập đúng
        assert!(bd.thu_dang_nhap("an", true).unwrap_err().contains("tạm khóa"));
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
