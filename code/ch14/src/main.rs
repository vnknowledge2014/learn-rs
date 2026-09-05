#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến: Ghép hàm, Curry hóa và Áp dụng từng phần trong Rust

use std::collections::HashMap;

// ============================================================================
// PHẦN 1: BỘ CÔNG CỤ GHÉP HÀM (COMPOSITION TOOLKIT)
// ============================================================================

/// Ghép 2 hàm: (A -> B) và (B -> C) thành (A -> C).
/// Đây chính là phép toán `g ∘ f` viết bằng cú pháp Rust.
pub fn ghep<A, B, C>(f: impl Fn(A) -> B, g: impl Fn(B) -> C) -> impl Fn(A) -> C {
    move |x| g(f(x))
}

/// Ghép 3 hàm liên tiếp cho tiện dùng.
pub fn ghep3<A, B, C, D>(
    f: impl Fn(A) -> B,
    g: impl Fn(B) -> C,
    h: impl Fn(C) -> D,
) -> impl Fn(A) -> D {
    move |x| h(g(f(x)))
}

/// Bộ kết hợp `identity`: phần tử đơn vị của phép ghép hàm.
pub fn dong_nhat<T>(x: T) -> T {
    x
}

/// Bộ kết hợp `const`: nuốt tham số, luôn trả về giá trị đã khóa sẵn.
pub fn hang_so<A: Clone, B>(gia_tri: A) -> impl Fn(B) -> A {
    move |_bo_qua| gia_tri.clone()
}

/// Bộ kết hợp `flip`: đảo thứ tự hai tham số của một hàm.
pub fn dao_tham_so<A, B, C>(f: impl Fn(A, B) -> C) -> impl Fn(B, A) -> C {
    move |b, a| f(a, b)
}

// ============================================================================
// PHẦN 2: CÁC HÀM NHỎ THUẦN TÚY — TỪNG "ĐOẠN ỐNG" RIÊNG LẺ
// ============================================================================

/// Cắt bỏ khoảng trắng thừa ở hai đầu.
pub fn cat_khoang_trang(s: &str) -> String {
    s.trim().to_string()
}

/// Thu gọn nhiều khoảng trắng liên tiếp thành một khoảng trắng duy nhất.
pub fn thu_gon_khoang_trang(s: String) -> String {
    s.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Viết hoa chữ cái đầu tiên của câu (an toàn với tiếng Việt có dấu).
pub fn viet_hoa_chu_dau(s: String) -> String {
    let mut cac_ky_tu = s.chars();
    match cac_ky_tu.next() {
        None => String::new(),
        Some(dau) => dau.to_uppercase().collect::<String>() + cac_ky_tu.as_str(),
    }
}

// ============================================================================
// PHẦN 3: CURRY HÓA & ÁP DỤNG TỪNG PHẦN — CÁC "NHÀ MÁY" SINH HÀM
// ============================================================================

/// Dạng thông thường: nhận đủ 2 tham số cùng lúc.
pub fn cat_bot(gioi_han: usize, s: &str) -> String {
    if s.chars().count() <= gioi_han {
        s.to_string()
    } else {
        let phan_dau: String = s.chars().take(gioi_han).collect();
        format!("{}…", phan_dau)
    }
}

/// Dạng đã curry hóa: khóa trước `gioi_han`, sinh ra một hàm chuyên dụng.
pub fn cat_bot_curry(gioi_han: usize) -> impl Fn(&str) -> String {
    move |s: &str| cat_bot(gioi_han, s)
}

/// Nhà máy sinh bộ lọc từ cấm: khóa sẵn danh sách từ, trả về một vị từ (predicate).
pub fn tao_bo_loc_tu_cam(tu_cam: Vec<String>) -> impl Fn(&str) -> bool {
    move |van_ban: &str| {
        let chu_thuong = van_ban.to_lowercase();
        !tu_cam.iter().any(|tu| chu_thuong.contains(tu.as_str()))
    }
}

/// Nhà máy sinh bộ che từ cấm bằng dấu sao.
pub fn tao_bo_che_tu_cam(tu_cam: Vec<String>) -> impl Fn(String) -> String {
    move |van_ban: String| {
        tu_cam.iter().fold(van_ban, |ket_qua, tu| {
            let che = "*".repeat(tu.chars().count());
            ket_qua.replace(tu.as_str(), che.as_str())
        })
    }
}

// ============================================================================
// PHẦN 4: TIÊM PHỤ THUỘC BẰNG ÁP DỤNG TỪNG PHẦN
// ============================================================================

/// Bản ghi nhật ký kiểm duyệt (thay cho việc ghi ra tệp thật).
#[derive(Debug, Clone, PartialEq)]
pub struct BanGhiNhatKy {
    pub ma_binh_luan: u32,
    pub ket_luan: String,
}

/// "Phụ thuộc" ở đây là hàm ghi nhật ký. Ta KHÓA nó vào trong bộ kiểm duyệt
/// bằng áp dụng từng phần, thay vì để bộ kiểm duyệt tự đi tìm.
/// `ghi_nhat_ky` phải là `FnMut` vì nó ghi thêm vào sổ sau mỗi lần gọi.
pub fn tao_bo_kiem_duyet<L>(
    kiem_tra_sach: impl Fn(&str) -> bool,
    lam_sach: impl Fn(String) -> String,
    mut ghi_nhat_ky: L,
) -> impl FnMut(u32, &str) -> String
where
    L: FnMut(BanGhiNhatKy),
{
    move |ma: u32, tho: &str| {
        let chuan = cat_khoang_trang(tho);
        // Kiểm tra TRƯỚC khi che — nếu che trước thì từ cấm biến mất
        // và bộ kiểm tra sẽ luôn báo "hợp lệ". Thứ tự các bước rất quan trọng!
        let ket_luan = if kiem_tra_sach(&chuan) {
            "HỢP LỆ"
        } else {
            "CHỨA TỪ CẤM — ĐÃ CHE"
        };
        let da_lam_sach = lam_sach(chuan);
        ghi_nhat_ky(BanGhiNhatKy {
            ma_binh_luan: ma,
            ket_luan: ket_luan.to_string(),
        });
        da_lam_sach
    }
}

// ============================================================================
// CHƯƠNG TRÌNH ĐIỀU HÀNH CHÍNH
// ============================================================================

fn main() {
    println!("============================================================");
    println!("   GHÉP HÀM, CURRY HÓA & ÁP DỤNG TỪNG PHẦN TRONG RUST      ");
    println!("============================================================");

    // ------------------------------------------------------------------
    // 1. LẮP REN ỐNG NƯỚC: ghép 3 hàm nhỏ thành 1 đường ống chuẩn hóa
    // ------------------------------------------------------------------
    let chuan_hoa = ghep3(cat_khoang_trang, thu_gon_khoang_trang, viet_hoa_chu_dau);

    let tho = "   xin    chào     các bạn  ";
    println!("\n1. GHÉP HÀM (Composition)");
    println!("   Đầu vào thô  : {:?}", tho);
    println!("   Sau đường ống: {:?}", chuan_hoa(tho));

    // ------------------------------------------------------------------
    // 2. KIỂM CHỨNG LUẬT KẾT HỢP: h ∘ (g ∘ f) == (h ∘ g) ∘ f
    // ------------------------------------------------------------------
    let cach_a = ghep(ghep(cat_khoang_trang, thu_gon_khoang_trang), viet_hoa_chu_dau);
    let cach_b = ghep(cat_khoang_trang, ghep(thu_gon_khoang_trang, viet_hoa_chu_dau));
    assert_eq!(cach_a(tho), cach_b(tho));
    println!("\n2. LUẬT KẾT HỢP");
    println!("   h∘(g∘f) và (h∘g)∘f cho cùng kết quả: {:?} ✓", cach_a(tho));

    // ------------------------------------------------------------------
    // 3. LUẬT ĐƠN VỊ: ghép với `identity` không làm thay đổi gì
    // ------------------------------------------------------------------
    let voi_don_vi = ghep(dong_nhat::<&str>, &chuan_hoa);
    assert_eq!(voi_don_vi(tho), chuan_hoa(tho));
    println!("\n3. LUẬT ĐƠN VỊ");
    println!("   identity ∘ f == f  ✓ (kết quả không đổi)");

    // ------------------------------------------------------------------
    // 4. CURRY HÓA: một hàm gốc sinh ra nhiều hàm chuyên dụng
    // ------------------------------------------------------------------
    println!("\n4. CURRY HÓA & ÁP DỤNG TỪNG PHẦN");
    let cat_ngan = cat_bot_curry(10); // Máy đã khóa núm "10 ký tự"
    let cat_dai = cat_bot_curry(25);  // Máy đã khóa núm "25 ký tự"

    let cau = "Rust là ngôn ngữ lập trình hệ thống hiện đại";
    println!("   Bản gốc   : {}", cau);
    println!("   Cắt còn 10: {}", cat_ngan(cau));
    println!("   Cắt còn 25: {}", cat_dai(cau));

    // ------------------------------------------------------------------
    // 5. NHÀ MÁY SINH HÀM: cùng một danh sách từ cấm, hai công cụ khác nhau
    // ------------------------------------------------------------------
    let tu_cam: Vec<String> = vec!["lừa đảo".to_string(), "spam".to_string()];
    let la_sach = tao_bo_loc_tu_cam(tu_cam.clone());
    let che_di = tao_bo_che_tu_cam(tu_cam.clone());

    println!("\n5. NHÀ MÁY SINH HÀM (Closure Factory)");
    let binh_luan_ban = "Đây là tin spam lừa đảo";
    println!("   {:?} có sạch không? {}", binh_luan_ban, la_sach(binh_luan_ban));
    println!("   Sau khi che: {}", che_di(binh_luan_ban.to_string()));

    // ------------------------------------------------------------------
    // 6. TIÊM PHỤ THUỘC: khóa "bộ ghi nhật ký" vào bộ kiểm duyệt
    // ------------------------------------------------------------------
    println!("\n6. TIÊM PHỤ THUỘC BẰNG ÁP DỤNG TỪNG PHẦN");
    let mut so_nhat_ky: Vec<BanGhiNhatKy> = Vec::new();

    {
        // Phụ thuộc thật: ghi vào sổ nhật ký trong bộ nhớ.
        let ghi_vao_so = |ban_ghi: BanGhiNhatKy| so_nhat_ky.push(ban_ghi);
        let mut kiem_duyet = tao_bo_kiem_duyet(&la_sach, &che_di, ghi_vao_so);

        println!("   #101 -> {}", kiem_duyet(101, "  Bài viết rất hay!  "));
        println!("   #102 -> {}", kiem_duyet(102, "  Cẩn thận kẻo bị lừa đảo  "));
    }

    println!("   Nhật ký thu được ({} dòng):", so_nhat_ky.len());
    for ban_ghi in &so_nhat_ky {
        println!("     - Bình luận #{}: {}", ban_ghi.ma_binh_luan, ban_ghi.ket_luan);
    }

    // ------------------------------------------------------------------
    // 7. BỘ KẾT HỢP `flip` VÀ `const`
    // ------------------------------------------------------------------
    println!("\n7. BỘ KẾT HỢP flip & const");
    let chia = |a: f64, b: f64| a / b;
    let chia_nguoc = dao_tham_so(chia);
    println!("   chia(10, 2)       = {}", chia(10.0, 2.0));
    println!("   flip(chia)(10, 2) = {}", chia_nguoc(10.0, 2.0)); // = chia(2, 10)

    let luon_tra_ve_0 = hang_so::<i32, &str>(0);
    println!("   const(0)(\"bất kỳ\") = {}", luon_tra_ve_0("bất kỳ"));

    // ------------------------------------------------------------------
    // 8. `identity` GIÚP LỌC BỎ None — ỨNG DỤNG THỰC TẾ
    // ------------------------------------------------------------------
    let du_lieu_tho: Vec<Option<i32>> = vec![Some(1), None, Some(3), None, Some(5)];
    let sach: Vec<i32> = du_lieu_tho.into_iter().flat_map(dong_nhat).collect();
    println!("\n8. identity LỌC BỎ None: {:?}", sach);
    assert_eq!(sach, vec![1, 3, 5]);

    // ------------------------------------------------------------------
    // 9. GHÉP HÀM QUY MÔ LỚN: xử lý cả một danh sách bình luận
    // ------------------------------------------------------------------
    println!("\n9. ÁP DỤNG ĐƯỜNG ỐNG LÊN TOÀN BỘ DỮ LIỆU");
    let binh_luan_tho = vec![
        "   rust rất   thú vị  ",
        " cẩn thận trò spam này ",
        "   giáo trình  hay quá   ",
    ];

    let thong_ke: HashMap<bool, usize> = binh_luan_tho
        .iter()
        .map(|b| chuan_hoa(b))
        .fold(HashMap::new(), |mut bang, cau| {
            *bang.entry(la_sach(&cau)).or_insert(0) += 1;
            bang
        });

    for b in binh_luan_tho.iter() {
        println!("   {:?} -> {:?}", b, chuan_hoa(b));
    }
    println!("   Thống kê [sạch = true/false]: {:?}", thong_ke);

    println!("\n============================================================");
    println!("      HOÀN TẤT: TỪ HÀM NHỎ LẮP THÀNH HỆ THỐNG LỚN          ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ: BIẾN "LUẬT" THÀNH BÀI TEST CHẠY ĐƯỢC
// ============================================================================

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn luat_ket_hop_cua_phep_ghep() {
        let mau = ["  a   b ", "Xin   chào", "   rust  "];
        for s in mau {
            let a = ghep(ghep(cat_khoang_trang, thu_gon_khoang_trang), viet_hoa_chu_dau);
            let b = ghep(cat_khoang_trang, ghep(thu_gon_khoang_trang, viet_hoa_chu_dau));
            assert_eq!(a(s), b(s), "Luật kết hợp bị vi phạm với đầu vào {:?}", s);
        }
    }

    #[test]
    fn luat_don_vi_cua_phep_ghep() {
        let f = ghep(cat_khoang_trang, viet_hoa_chu_dau);
        let trai = ghep(dong_nhat::<&str>, &f);
        for s in ["  xin chào ", "rust"] {
            assert_eq!(trai(s), f(s));
        }
    }

    #[test]
    fn curry_hoa_tuong_duong_ham_goc() {
        let cat_15 = cat_bot_curry(15);
        let cau = "Rust là ngôn ngữ tuyệt vời";
        assert_eq!(cat_15(cau), cat_bot(15, cau));
    }

    #[test]
    fn flip_dao_dung_thu_tu_tham_so() {
        let tru = |a: i32, b: i32| a - b;
        let tru_nguoc = dao_tham_so(tru);
        assert_eq!(tru(10, 3), 7);
        assert_eq!(tru_nguoc(10, 3), -7); // = tru(3, 10)
    }

    #[test]
    fn nha_may_sinh_ham_hoat_dong_doc_lap() {
        let loc = tao_bo_loc_tu_cam(vec!["spam".to_string()]);
        assert!(loc("bài viết hay"));
        assert!(!loc("đây là SPAM"));
    }
}
