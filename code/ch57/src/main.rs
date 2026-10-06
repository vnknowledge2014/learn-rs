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
