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
