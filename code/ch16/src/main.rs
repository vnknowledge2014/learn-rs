#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Bộ công cụ Iterator đầy đủ: từ filter_map tới FromIterator

use std::collections::{HashMap, HashSet};

// ============================================================================
// PHẦN 1: TỰ CÀI ĐẶT MỘT ITERATOR
// ============================================================================

/// Bộ đếm ngược: minh họa việc chỉ cần cài `next()` là có ngay hàng chục
/// phương thức miễn phí (map, filter, take, sum...).
pub struct DemNguoc {
    hien_tai: u32,
}

impl DemNguoc {
    pub fn moi(bat_dau: u32) -> Self {
        DemNguoc { hien_tai: bat_dau }
    }
}

impl Iterator for DemNguoc {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.hien_tai == 0 {
            None
        } else {
            self.hien_tai -= 1;
            Some(self.hien_tai + 1)
        }
    }
}

// ============================================================================
// PHẦN 2: TỰ CÀI ĐẶT IntoIterator CHO KIỂU CỦA MÌNH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GioHang {
    mat_hang: Vec<String>,
}

impl GioHang {
    pub fn moi(mat_hang: Vec<String>) -> Self {
        GioHang { mat_hang }
    }
}

/// Nhờ trait này, `for x in gio_hang` chạy được — đúng như với Vec.
impl IntoIterator for GioHang {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;
    fn into_iter(self) -> Self::IntoIter {
        self.mat_hang.into_iter()
    }
}

/// Và nhờ trait này, `for x in &gio_hang` cũng chạy được (chỉ mượn đọc).
impl<'a> IntoIterator for &'a GioHang {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.mat_hang.iter()
    }
}

/// Và nhờ FromIterator, `collect()` gom thẳng được vào GioHang.
impl FromIterator<String> for GioHang {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        GioHang { mat_hang: iter.into_iter().collect() }
    }
}

// ============================================================================
// PHẦN 3: MIỀN DỮ LIỆU — NHẬT KÝ BÁN HÀNG THÔ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GiaoDich {
    pub ma: String,
    pub khu_vuc: String,
    pub so_tien: u64,
}

/// Phân tích một dòng thô "MA|KHU_VUC|SO_TIEN". Trả None nếu dòng hỏng.
pub fn phan_tich_dong(dong: &str) -> Option<GiaoDich> {
    let phan: Vec<&str> = dong.split('|').map(|s| s.trim()).collect();
    if phan.len() != 3 {
        return None;
    }
    let so_tien = phan[2].parse::<u64>().ok()?;
    if phan[0].is_empty() || phan[1].is_empty() {
        return None;
    }
    Some(GiaoDich {
        ma: phan[0].to_string(),
        khu_vuc: phan[1].to_string(),
        so_tien,
    })
}

fn du_lieu_tho() -> Vec<&'static str> {
    vec![
        "GD-001 | Hà Nội       | 1250000",
        "GD-002 | TP.HCM       | 890000",
        "dòng hỏng không có dấu gạch",
        "GD-003 | Đà Nẵng      | 450000",
        "GD-004 | Hà Nội       | không phải số",
        "GD-005 | TP.HCM       | 2100000",
        "GD-006 | Hà Nội       | 320000",
        "       |              | 999",
        "GD-007 | Cần Thơ      | 780000",
    ]
}

fn main() {
    println!("============================================================");
    println!("        BỘ CÔNG CỤ ITERATOR ĐẦY ĐỦ CỦA RUST                ");
    println!("============================================================");

    let tho = du_lieu_tho();
    println!("\nDữ liệu thô: {} dòng (có cả dòng hỏng)", tho.len());

    // ------------------------------------------------------------------
    // 1. filter_map — LỌC VÀ BIẾN ĐỔI CÙNG LÚC
    // ------------------------------------------------------------------
    let gd: Vec<GiaoDich> = tho.iter().filter_map(|d| phan_tich_dong(d)).collect();
    println!("\n1. filter_map: {} dòng hợp lệ / {} dòng thô", gd.len(), tho.len());
    for g in gd.iter().take(3) {
        println!("   {:?}", g);
    }
    println!("   (đã dùng luôn `take(3)` để chỉ in 3 dòng đầu)");

    // ------------------------------------------------------------------
    // 2. any / all / find / position — ĐỀU NGẮN MẠCH
    // ------------------------------------------------------------------
    println!("\n2. any / all / find / position (đều dừng sớm)");
    println!("   Có giao dịch nào > 2 triệu?     : {}", gd.iter().any(|g| g.so_tien > 2_000_000));
    println!("   Mọi giao dịch đều > 100 nghìn?  : {}", gd.iter().all(|g| g.so_tien > 100_000));
    println!("   Giao dịch đầu ở Đà Nẵng         : {:?}", gd.iter().find(|g| g.khu_vuc == "Đà Nẵng").map(|g| &g.ma));
    println!("   Vị trí giao dịch đầu ở TP.HCM   : {:?}", gd.iter().position(|g| g.khu_vuc == "TP.HCM"));

    // ------------------------------------------------------------------
    // 3. min_by_key / max_by_key
    // ------------------------------------------------------------------
    println!("\n3. min_by_key / max_by_key");
    println!("   Giao dịch nhỏ nhất: {:?}", gd.iter().min_by_key(|g| g.so_tien).map(|g| (&g.ma, g.so_tien)));
    println!("   Giao dịch lớn nhất: {:?}", gd.iter().max_by_key(|g| g.so_tien).map(|g| (&g.ma, g.so_tien)));

    // ------------------------------------------------------------------
    // 4. partition — CHIA ĐÔI TRONG MỘT LƯỢT
    // ------------------------------------------------------------------
    let (lon, nho): (Vec<&GiaoDich>, Vec<&GiaoDich>) =
        gd.iter().partition(|g| g.so_tien >= 800_000);
    println!("\n4. partition: {} đơn lớn (>=800k), {} đơn nhỏ", lon.len(), nho.len());

    // ------------------------------------------------------------------
    // 5. fold / reduce / try_fold — BA KIỂU GỘP
    // ------------------------------------------------------------------
    println!("\n5. fold vs reduce vs try_fold");
    let tong_fold: u64 = gd.iter().map(|g| g.so_tien).fold(0, |a, b| a + b);
    let tong_reduce: Option<u64> = gd.iter().map(|g| g.so_tien).reduce(|a, b| a + b);
    println!("   fold  (có giá trị khởi tạo)  : {}", tong_fold);
    println!("   reduce(không có, trả Option) : {:?}", tong_reduce);

    let rong: Vec<u64> = Vec::new();
    println!("   Trên danh sách RỖNG -> fold: {}, reduce: {:?}",
             rong.iter().fold(0u64, |a, b| a + b),
             rong.iter().copied().reduce(|a: u64, b: u64| a + b));

    // try_fold: gộp CÓ THỂ THẤT BẠI, dừng ngay ở lỗi đầu tiên
    let an_toan: Option<u64> = gd.iter().try_fold(0u64, |a, g| a.checked_add(g.so_tien));
    println!("   try_fold (chống tràn số)     : {:?}", an_toan);
    let se_tran: Option<u64> = [u64::MAX, 1].iter().try_fold(0u64, |a, b| a.checked_add(*b));
    println!("   try_fold khi tràn số         : {:?} (dừng ngay, không panic)", se_tran);

    // ------------------------------------------------------------------
    // 6. scan — GIỐNG fold NHƯNG NHẢ RA TỪNG BƯỚC TRUNG GIAN
    // ------------------------------------------------------------------
    let luy_ke: Vec<u64> = gd
        .iter()
        .scan(0u64, |tong, g| {
            *tong += g.so_tien;
            Some(*tong)
        })
        .collect();
    println!("\n6. scan (tổng lũy kế từng bước): {:?}", luy_ke);

    // ------------------------------------------------------------------
    // 7. take_while / skip_while — DỪNG SỚM, KHÁC HẲN filter
    // ------------------------------------------------------------------
    println!("\n7. take_while vs filter");
    let so = [1, 3, 5, 4, 7, 9];
    let tw: Vec<i32> = so.iter().copied().take_while(|x| x % 2 == 1).collect();
    let ft: Vec<i32> = so.iter().copied().filter(|x| x % 2 == 1).collect();
    println!("   dãy gốc              : {:?}", so);
    println!("   take_while(lẻ)       : {:?}  ← DỪNG ngay khi gặp số chẵn đầu tiên", tw);
    println!("   filter(lẻ)           : {:?}  ← duyệt HẾT, giữ mọi số lẻ", ft);
    let sw: Vec<i32> = so.iter().copied().skip_while(|x| x % 2 == 1).collect();
    println!("   skip_while(lẻ)       : {:?}", sw);

    // ------------------------------------------------------------------
    // 8. zip / unzip / chain / rev / step_by
    // ------------------------------------------------------------------
    println!("\n8. zip / unzip / chain / rev / step_by");
    let ma: Vec<&str> = gd.iter().map(|g| g.ma.as_str()).collect();
    let tien: Vec<u64> = gd.iter().map(|g| g.so_tien).collect();
    let ghep: Vec<(&&str, &u64)> = ma.iter().zip(tien.iter()).take(3).collect();
    println!("   zip 3 cặp đầu : {:?}", ghep);

    let (lai_ma, lai_tien): (Vec<&str>, Vec<u64>) =
        ma.iter().copied().zip(tien.iter().copied()).unzip();
    println!("   unzip tách lại: {} mã, {} số tiền", lai_ma.len(), lai_tien.len());

    let noi: Vec<i32> = (1..3).chain(10..12).collect();
    println!("   chain         : {:?}", noi);
    // CHÚ Ý: `rev()` đòi hỏi trait `DoubleEndedIterator` — iterator phải biết đi
    // từ CẢ HAI đầu. `DemNguoc` tự viết chỉ cài `Iterator` (một chiều), nên
    // `DemNguoc::moi(5).rev()` KHÔNG biên dịch được:
    //     error[E0277]: the trait bound `DemNguoc: DoubleEndedIterator` is not satisfied
    // `Vec` thì có, nên ta gom lại trước rồi mới đảo:
    let nguoc: Vec<u32> = DemNguoc::moi(5).collect::<Vec<u32>>().into_iter().rev().collect();
    println!("   rev (cần DoubleEndedIterator): {:?}", nguoc);
    let cach_quang: Vec<i32> = (0..10).step_by(3).collect();
    println!("   step_by(3)    : {:?}", cach_quang);

    // ------------------------------------------------------------------
    // 9. flat_map / flatten
    // ------------------------------------------------------------------
    println!("\n9. flat_map / flatten");
    let cau = ["Rust rất nhanh", "và an toàn"];
    let tu: Vec<&str> = cau.iter().flat_map(|c| c.split_whitespace()).collect();
    println!("   flat_map tách từ: {:?}", tu);

    let long: Vec<Vec<i32>> = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let phang: Vec<i32> = long.into_iter().flatten().collect();
    println!("   flatten làm phẳng: {:?}", phang);

    let co_none: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
    let bo_none: Vec<i32> = co_none.into_iter().flatten().collect();
    println!("   flatten bỏ None  : {:?}", bo_none);

    // ------------------------------------------------------------------
    // 10. collect VÀO NHIỀU KIỂU KHÁC NHAU
    // ------------------------------------------------------------------
    println!("\n10. collect() gom vào nhiều kiểu đích");
    let chuoi: String = ma.iter().copied().collect::<Vec<&str>>().join(", ");
    println!("   -> String     : {}", chuoi);

    let khu_vuc: HashSet<&str> = gd.iter().map(|g| g.khu_vuc.as_str()).collect();
    let mut kv: Vec<&&str> = khu_vuc.iter().collect();
    kv.sort();
    println!("   -> HashSet    : {:?} ({} khu vực)", kv, khu_vuc.len());

    let bang: HashMap<&str, u64> = gd.iter().map(|g| (g.ma.as_str(), g.so_tien)).collect();
    println!("   -> HashMap    : tra cứu GD-003 = {:?}", bang.get("GD-003"));

    let tot: Result<Vec<i32>, _> = ["1", "2", "3"].iter().map(|s| s.parse::<i32>()).collect();
    let xau: Result<Vec<i32>, _> = ["1", "x", "3"].iter().map(|s| s.parse::<i32>()).collect();
    println!("   -> Result (ổn) : {:?}", tot);
    println!("   -> Result (hỏng): có lỗi = {}", xau.is_err());

    // ------------------------------------------------------------------
    // 11. TỔNG HỢP THEO NHÓM — MẪU DÙNG HẰNG NGÀY
    // ------------------------------------------------------------------
    println!("\n11. Tổng doanh thu theo khu vực (fold + entry API)");
    let theo_kv: HashMap<&str, u64> =
        gd.iter().fold(HashMap::new(), |mut bang, g| {
            *bang.entry(g.khu_vuc.as_str()).or_insert(0) += g.so_tien;
            bang
        });
    let mut cac_kv: Vec<(&&str, &u64)> = theo_kv.iter().collect();
    cac_kv.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (k, v) in cac_kv {
        println!("   {:<10} {:>10} đ", k, v);
    }

    // ------------------------------------------------------------------
    // 12. fold vs rfold — KHI THỨ TỰ CÓ Ý NGHĨA
    // ------------------------------------------------------------------
    println!("\n12. fold vs rfold");
    let m = [10i32, 3, 2];
    println!("   Phép CỘNG (giao hoán)      : fold={}, rfold={}  -> GIỐNG nhau",
             m.iter().fold(0, |a, b| a + b), m.iter().rfold(0, |a, b| a + b));
    let noi_trai: String = m.iter().fold(String::new(), |a, b| a + &b.to_string());
    let noi_phai: String = m.iter().rfold(String::new(), |a, b| a + &b.to_string());
    println!("   NỐI CHUỖI (không giao hoán): fold={:?}, rfold={:?}  -> KHÁC nhau",
             noi_trai, noi_phai);
    println!("   → Trước khi song song hóa, phải biết phép gộp của mình có tính gì!");

    // ------------------------------------------------------------------
    // 13. ITERATOR TỰ VIẾT VÀ IntoIterator TỰ VIẾT
    // ------------------------------------------------------------------
    println!("\n13. Iterator và IntoIterator tự cài đặt");
    let dem: Vec<u32> = DemNguoc::moi(5).collect();
    println!("   DemNguoc(5)                 : {:?}", dem);
    println!("   Miễn phí luôn map/filter/sum: {}", DemNguoc::moi(100).filter(|x| x % 7 == 0).sum::<u32>());

    let gio = GioHang::moi(vec!["Bàn phím".into(), "Chuột".into(), "Màn hình".into()]);
    print!("   for x in &gio_hang -> ");
    for m in &gio {
        print!("[{}] ", m);
    }
    println!();

    let gio_moi: GioHang = gio
        .into_iter()
        .filter(|m| m.chars().count() > 5)
        .collect(); // ← nhờ FromIterator tự cài
    println!("   collect() thẳng vào GioHang : {:?}", gio_moi);

    // ------------------------------------------------------------------
    // 14. Extend — NỐI THÊM VÀO TẬP HỢP ĐÃ CÓ
    // ------------------------------------------------------------------
    let mut kho: Vec<i32> = vec![1, 2];
    kho.extend(3..6);
    println!("\n14. Extend: {:?}", kho);

    println!("\n============================================================");
    println!("   MỘT `next()` — HÀNG CHỤC CÔNG CỤ MIỄN PHÍ ĐI KÈM         ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn filter_map_bo_qua_dong_hong() {
        let gd: Vec<GiaoDich> = du_lieu_tho().iter().filter_map(|d| phan_tich_dong(d)).collect();
        assert_eq!(gd.len(), 6, "9 dòng thô, 3 dòng hỏng -> còn 6");
    }

    #[test]
    fn take_while_khac_filter() {
        let so = [1, 3, 5, 4, 7, 9];
        let tw: Vec<i32> = so.iter().copied().take_while(|x| x % 2 == 1).collect();
        let ft: Vec<i32> = so.iter().copied().filter(|x| x % 2 == 1).collect();
        assert_eq!(tw, vec![1, 3, 5]); // dừng ở số 4
        assert_eq!(ft, vec![1, 3, 5, 7, 9]); // duyệt hết
    }

    #[test]
    fn reduce_tra_none_khi_rong() {
        let rong: Vec<u64> = Vec::new();
        assert_eq!(rong.iter().copied().reduce(|a, b| a + b), None);
        assert_eq!(rong.iter().fold(0u64, |a, b| a + b), 0); // fold vẫn có câu trả lời
    }

    #[test]
    fn try_fold_dung_ngay_khi_tran_so() {
        let kq: Option<u64> = [u64::MAX, 1, 2].iter().try_fold(0u64, |a, b| a.checked_add(*b));
        assert_eq!(kq, None);
    }

    #[test]
    fn scan_nha_ra_tung_buoc_trung_gian() {
        let luy_ke: Vec<i32> = [1, 2, 3, 4]
            .iter()
            .scan(0, |t, x| { *t += x; Some(*t) })
            .collect();
        assert_eq!(luy_ke, vec![1, 3, 6, 10]);
    }

    #[test]
    fn fold_va_rfold_chi_khac_nhau_voi_phep_khong_giao_hoan() {
        let m = [10i32, 3, 2];
        // Phép cộng GIAO HOÁN -> duyệt hai chiều cho cùng kết quả
        assert_eq!(m.iter().fold(0, |a, b| a + b), m.iter().rfold(0, |a, b| a + b));
        // Nối chuỗi KHÔNG giao hoán -> duyệt hai chiều cho kết quả khác nhau
        let trai: String = m.iter().fold(String::new(), |a, b| a + &b.to_string());
        let phai: String = m.iter().rfold(String::new(), |a, b| a + &b.to_string());
        assert_eq!(trai, "1032");
        assert_eq!(phai, "2310");
        assert_ne!(trai, phai);
    }

    #[test]
    fn collect_gom_duoc_nhieu_kieu_dich() {
        let v: Vec<i32> = (1..4).collect();
        assert_eq!(v, vec![1, 2, 3]);
        let s: String = ['R', 'u', 's', 't'].into_iter().collect();
        assert_eq!(s, "Rust");
        let t: HashSet<i32> = [1, 2, 2, 3].into_iter().collect();
        assert_eq!(t.len(), 3);
        let b: HashMap<&str, i32> = [("a", 1), ("b", 2)].into_iter().collect();
        assert_eq!(b.get("b"), Some(&2));
        let r: Result<Vec<i32>, _> = ["1", "2"].iter().map(|s| s.parse::<i32>()).collect();
        assert_eq!(r, Ok(vec![1, 2]));
    }

    #[test]
    fn partition_chia_dung_hai_nhom() {
        let (chan, le): (Vec<i32>, Vec<i32>) = (1..8).partition(|x| x % 2 == 0);
        assert_eq!(chan, vec![2, 4, 6]);
        assert_eq!(le, vec![1, 3, 5, 7]);
    }

    #[test]
    fn iterator_tu_viet_hoat_dong() {
        assert_eq!(DemNguoc::moi(3).collect::<Vec<u32>>(), vec![3, 2, 1]);
        assert_eq!(DemNguoc::moi(10).filter(|x| x % 3 == 0).sum::<u32>(), 18); // 9+6+3
    }

    #[test]
    fn into_iterator_va_from_iterator_tu_viet() {
        let gio = GioHang::moi(vec!["Bàn phím".into(), "Chuột".into()]);
        let ten: Vec<&String> = (&gio).into_iter().collect();
        assert_eq!(ten.len(), 2);
        let loc: GioHang = gio.into_iter().filter(|m| m.chars().count() > 5).collect();
        assert_eq!(loc, GioHang::moi(vec!["Bàn phím".into()]));
    }

    #[test]
    fn tong_hop_theo_khu_vuc_dung() {
        let gd: Vec<GiaoDich> = du_lieu_tho().iter().filter_map(|d| phan_tich_dong(d)).collect();
        let theo_kv: HashMap<&str, u64> = gd.iter().fold(HashMap::new(), |mut b, g| {
            *b.entry(g.khu_vuc.as_str()).or_insert(0) += g.so_tien;
            b
        });
        assert_eq!(theo_kv.get("Hà Nội"), Some(&1_570_000)); // 1250000 + 320000
        assert_eq!(theo_kv.get("Cần Thơ"), Some(&780_000));
    }
}
