#![allow(dead_code, unused_variables)]
//! Chương 57 — OSWE: Bảo mật ứng dụng Web. Mỗi lỗ hổng có bản DÍNH LỖI và bản SỬA,
//! kèm test chứng minh bản sửa chặn được đòn tấn công. Toàn bộ chạy offline.


// ============================================================================
// 1. SQL INJECTION — và cách kiểu dữ liệu chặn nó
// ============================================================================

/// ❌ DÍNH LỖI: ghép chuỗi thẳng vào câu SQL. Kẻ tấn công gửi
/// `' OR '1'='1` để vượt qua điều kiện.
pub fn dung_cau_sql_dinh_loi(ten_dang_nhap: &str) -> String {
    format!("SELECT * FROM users WHERE username = '{}'", ten_dang_nhap)
}

/// ✅ SỬA: dùng tham số hóa (placeholder). Dữ liệu người dùng KHÔNG BAO GIỜ
/// trở thành một phần cú pháp SQL — nó chỉ là *giá trị* điền vào chỗ `?`.
#[derive(Debug, PartialEq)]
pub struct CauSqlAnToan {
    pub mau: String,           // "... WHERE username = ?"
    pub tham_so: Vec<String>,  // giá trị điền vào, tách RỜI khỏi cú pháp
}
pub fn dung_cau_sql_an_toan(ten_dang_nhap: &str) -> CauSqlAnToan {
    CauSqlAnToan {
        mau: "SELECT * FROM users WHERE username = ?".to_string(),
        tham_so: vec![ten_dang_nhap.to_string()],
    }
}

/// Mô phỏng cách trình điều khiển cơ sở dữ liệu thật xử lý: giá trị được
/// "thoát" và bọc, không bao giờ được diễn giải là cú pháp.
pub fn co_the_bi_tiem_sql(cau: &CauSqlAnToan) -> bool {
    // Với câu tham số hóa, dù tham số chứa gì thì cú pháp vẫn cố định.
    cau.mau.matches('?').count() == cau.tham_so.len() && cau.mau.contains('?')
}

// ============================================================================
// 2. XSS (Cross-Site Scripting) — thoát ký tự HTML
// ============================================================================

/// ✅ Thoát các ký tự nguy hiểm trước khi nhúng dữ liệu người dùng vào HTML.
/// Đây là tuyến phòng thủ số 1 chống XSS phản chiếu và lưu trữ.
pub fn thoat_html(dau_vao: &str) -> String {
    dau_vao
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#x27;")
}

/// ❌ DÍNH LỖI: nhúng thẳng đầu vào vào HTML.
pub fn render_binh_luan_dinh_loi(binh_luan: &str) -> String {
    format!("<div class=\"cmt\">{}</div>", binh_luan)
}
/// ✅ SỬA: thoát trước khi nhúng.
pub fn render_binh_luan_an_toan(binh_luan: &str) -> String {
    format!("<div class=\"cmt\">{}</div>", thoat_html(binh_luan))
}

// ============================================================================
// 3. IDOR (Insecure Direct Object Reference) — kiểm tra quyền sở hữu
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct HoaDon {
    pub id: u64,
    pub chu_so_huu: u64, // id người dùng sở hữu
    pub so_tien: u64,
}

#[derive(Debug, PartialEq)]
pub enum LoiTruyCap {
    KhongTonTai,
    KhongCoQuyen, // đây là lỗ hổng IDOR nếu quên kiểm tra
}

/// ❌ DÍNH LỖI: chỉ tra theo id, KHÔNG kiểm tra người gọi có sở hữu không.
/// Kẻ tấn công đổi `?id=123` thành `?id=124` để xem hóa đơn người khác.
pub fn xem_hoa_don_dinh_loi<'a>(kho: &'a [HoaDon], id: u64) -> Option<&'a HoaDon> {
    kho.iter().find(|h| h.id == id)
}

/// ✅ SỬA: bắt buộc truyền id người gọi và kiểm tra quyền sở hữu.
pub fn xem_hoa_don_an_toan<'a>(
    kho: &'a [HoaDon],
    id: u64,
    nguoi_goi: u64,
) -> Result<&'a HoaDon, LoiTruyCap> {
    let hd = kho.iter().find(|h| h.id == id).ok_or(LoiTruyCap::KhongTonTai)?;
    if hd.chu_so_huu != nguoi_goi {
        return Err(LoiTruyCap::KhongCoQuyen);
    }
    Ok(hd)
}

// ============================================================================
// 4. SSRF (Server-Side Request Forgery) — danh sách trắng, không danh sách đen
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum LoiUrl {
    KhongPhaiHttp,
    TroToiMangNoiBo, // chặn 127.0.0.1, 169.254.x (metadata đám mây), 10.x, 192.168.x
    HostKhongDuocPhep,
}

/// ✅ Kiểm tra URL trước khi máy chủ đi lấy nội dung (chống SSRF).
/// Quy tắc: DANH SÁCH TRẮNG host cho phép, và chặn mọi địa chỉ mạng nội bộ.
pub fn kiem_tra_url_an_toan(url: &str, host_cho_phep: &[&str]) -> Result<(), LoiUrl> {
    let sau_scheme = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .ok_or(LoiUrl::KhongPhaiHttp)?;

    let host = sau_scheme.split(['/', ':']).next().unwrap_or("");

    // Chặn địa chỉ mạng nội bộ / loopback / metadata đám mây
    if la_dia_chi_noi_bo(host) {
        return Err(LoiUrl::TroToiMangNoiBo);
    }
    if !host_cho_phep.contains(&host) {
        return Err(LoiUrl::HostKhongDuocPhep);
    }
    Ok(())
}

pub fn la_dia_chi_noi_bo(host: &str) -> bool {
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
    let mut khac: u8 = 0;
    for i in 0..a.len() {
        khac |= a[i] ^ b[i]; // gộp mọi khác biệt, không rẽ nhánh sớm
    }
    khac == 0
}

/// Kiểm tra ĐỘ MẠNH mật khẩu — chính sách tối thiểu.
#[derive(Debug, PartialEq)]
pub enum LoiMatKhau {
    QuaNgan,
    ThieuChuHoa,
    ThieuChuSo,
    ThieuKyTuDacBiet,
}
pub fn kiem_tra_do_manh(mk: &str) -> Result<(), Vec<LoiMatKhau>> {
    let mut loi = Vec::new();
    if mk.chars().count() < 12 {
        loi.push(LoiMatKhau::QuaNgan);
    }
    if !mk.chars().any(|c| c.is_uppercase()) {
        loi.push(LoiMatKhau::ThieuChuHoa);
    }
    if !mk.chars().any(|c| c.is_ascii_digit()) {
        loi.push(LoiMatKhau::ThieuChuSo);
    }
    if !mk.chars().any(|c| !c.is_alphanumeric()) {
        loi.push(LoiMatKhau::ThieuKyTuDacBiet);
    }
    if loi.is_empty() { Ok(()) } else { Err(loi) }
}

// ============================================================================
// 6. PATH TRAVERSALL — chặn ../../etc/passwd
// ============================================================================

/// ✅ Chuẩn hóa và kiểm tra đường dẫn tệp do người dùng cung cấp.
/// Chặn `..` để không thoát ra khỏi thư mục gốc cho phép.
pub fn duong_dan_an_toan(goc: &str, yeu_cau: &str) -> Result<String, String> {
    if yeu_cau.contains("..") || yeu_cau.starts_with('/') || yeu_cau.contains('\0') {
        return Err(format!("Đường dẫn nguy hiểm bị chặn: {:?}", yeu_cau));
    }
    Ok(format!("{}/{}", goc.trim_end_matches('/'), yeu_cau))
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   OSWE — BẢO MẬT ỨNG DỤNG WEB: 6 LỖ HỔNG KINH ĐIỂN & CÁCH SỬA  ");
    println!("═══════════════════════════════════════════════════════════════");

    let doc = "admin' OR '1'='1";
    println!("\n1. SQL INJECTION");
    println!("   Đầu vào tấn công: {:?}", doc);
    println!("   ❌ Ghép chuỗi : {}", dung_cau_sql_dinh_loi(doc));
    let an = dung_cau_sql_an_toan(doc);
    println!("   ✅ Tham số hóa: {} | tham số = {:?}", an.mau, an.tham_so);
    println!("      → Đầu vào chỉ là GIÁ TRỊ, không thể trở thành cú pháp.");

    println!("\n2. XSS");
    let xss = "<script>steal(document.cookie)</script>";
    println!("   Đầu vào: {}", xss);
    println!("   ✅ Sau khi thoát: {}", thoat_html(xss));

    println!("\n3. IDOR");
    let kho = vec![
        HoaDon { id: 100, chu_so_huu: 1, so_tien: 500 },
        HoaDon { id: 101, chu_so_huu: 2, so_tien: 999 },
    ];
    println!("   Người dùng #1 xem hóa đơn #101 (của người #2):");
    println!("   ❌ Bản lỗi cho xem: {:?}", xem_hoa_don_dinh_loi(&kho, 101).map(|h| h.so_tien));
    println!("   ✅ Bản sửa chặn  : {:?}", xem_hoa_don_an_toan(&kho, 101, 1));

    println!("\n4. SSRF");
    let cho_phep = ["api.doitac.vn", "cdn.congty.vn"];
    for u in ["https://api.doitac.vn/data", "http://169.254.169.254/latest/meta-data/", "https://evil.com"] {
        println!("   {:>45} -> {:?}", u, kiem_tra_url_an_toan(u, &cho_phep));
    }

    println!("\n5. XÁC THỰC");
    println!("   So sánh token bất biến: {}", so_sanh_bat_bien(b"secret123", b"secret123"));
    println!("   Mật khẩu 'abc': {:?}", kiem_tra_do_manh("abc").is_err());
    println!("   Mật khẩu 'Rust@2026!Secure': {:?}", kiem_tra_do_manh("Rust@2026!Secure"));

    println!("\n6. PATH TRAVERSAL");
    println!("   {:?}", duong_dan_an_toan("/var/www/uploads", "avatar.png"));
    println!("   {:?}", duong_dan_an_toan("/var/www/uploads", "../../etc/passwd"));

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   ĐỪNG TIN DỮ LIỆU NGƯỜI DÙNG · DÙNG DANH SÁCH TRẮNG · KIỂM QUYỀN ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn sql_tham_so_hoa_khong_tiem_duoc() {
        let doc = "admin' OR '1'='1; DROP TABLE users;--";
        let an = dung_cau_sql_an_toan(doc);
        // Cú pháp cố định, chỉ 1 chỗ ?; toàn bộ đòn tấn công nằm trong THAM SỐ
        assert_eq!(an.tham_so, vec![doc.to_string()]);
        assert!(an.mau.matches('?').count() == 1);
        assert!(!an.mau.contains("OR")); // đầu vào KHÔNG lọt vào cú pháp
    }

    #[test]
    fn xss_thoat_het_ky_tu_nguy_hiem() {
        let out = thoat_html("<script>alert('x')</script>");
        assert!(!out.contains('<'));
        assert!(!out.contains('>'));
        assert!(out.contains("&lt;script&gt;"));
        // Bản sửa KHÔNG chứa thẻ script thực thi được
        assert!(!render_binh_luan_an_toan("<img onerror=hack()>").contains("<img"));
    }

    #[test]
    fn idor_chan_truy_cap_cheo_nguoi_dung() {
        let kho = vec![
            HoaDon { id: 100, chu_so_huu: 1, so_tien: 500 },
            HoaDon { id: 101, chu_so_huu: 2, so_tien: 999 },
        ];
        // Người #1 xem hóa đơn của chính mình -> OK
        assert!(xem_hoa_don_an_toan(&kho, 100, 1).is_ok());
        // Người #1 xem hóa đơn người #2 -> BỊ CHẶN
        assert_eq!(xem_hoa_don_an_toan(&kho, 101, 1), Err(LoiTruyCap::KhongCoQuyen));
        // Hóa đơn không tồn tại
        assert_eq!(xem_hoa_don_an_toan(&kho, 999, 1), Err(LoiTruyCap::KhongTonTai));
    }

    #[test]
    fn ssrf_chan_metadata_dam_may_va_mang_noi_bo() {
        let cp = ["api.tot.vn"];
        assert!(kiem_tra_url_an_toan("https://api.tot.vn/x", &cp).is_ok());
        // Địa chỉ metadata đám mây — mục tiêu SSRF nguy hiểm nhất
        assert_eq!(kiem_tra_url_an_toan("http://169.254.169.254/", &cp), Err(LoiUrl::TroToiMangNoiBo));
        assert_eq!(kiem_tra_url_an_toan("http://127.0.0.1:8080/admin", &cp), Err(LoiUrl::TroToiMangNoiBo));
        assert_eq!(kiem_tra_url_an_toan("http://10.0.0.5/", &cp), Err(LoiUrl::TroToiMangNoiBo));
        assert_eq!(kiem_tra_url_an_toan("http://172.16.0.1/", &cp), Err(LoiUrl::TroToiMangNoiBo));
        assert_eq!(kiem_tra_url_an_toan("http://172.15.0.1/", &["172.15.0.1"]), Ok(())); // 172.15 KHÔNG nội bộ
        // Host lạ không trong danh sách trắng
        assert_eq!(kiem_tra_url_an_toan("https://evil.com/", &cp), Err(LoiUrl::HostKhongDuocPhep));
        // Không phải http(s)
        assert_eq!(kiem_tra_url_an_toan("file:///etc/passwd", &cp), Err(LoiUrl::KhongPhaiHttp));
    }

    #[test]
    fn so_sanh_bat_bien_dung() {
        assert!(so_sanh_bat_bien(b"token-abc", b"token-abc"));
        assert!(!so_sanh_bat_bien(b"token-abc", b"token-xyz"));
        assert!(!so_sanh_bat_bien(b"ngan", b"dai-hon-nhieu")); // độ dài khác
    }

    #[test]
    fn do_manh_mat_khau() {
        assert!(kiem_tra_do_manh("abc").is_err());
        assert!(kiem_tra_do_manh("khongcosohoa!X").is_err()); // thiếu số
        assert!(kiem_tra_do_manh("Rust@2026!Secure").is_ok());
        let loi = kiem_tra_do_manh("short").unwrap_err();
        assert!(loi.contains(&LoiMatKhau::QuaNgan));
    }

    #[test]
    fn path_traversal_bi_chan() {
        assert!(duong_dan_an_toan("/uploads", "anh.png").is_ok());
        assert!(duong_dan_an_toan("/uploads", "../../etc/passwd").is_err());
        assert!(duong_dan_an_toan("/uploads", "/etc/passwd").is_err());
        assert!(duong_dan_an_toan("/uploads", "a/../../secret").is_err());
    }
}
