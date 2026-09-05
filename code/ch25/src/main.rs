#![allow(dead_code, unused_variables, unused_imports)]
use std::time::Instant;

/// Minh họa giải thuật O(1) - Truy cập phần tử qua chỉ số mảng
/// Bất kể danh sách có 10 phần tử hay 10 triệu phần tử,
/// CPU chỉ cần 1 phép tính cộng địa chỉ bộ nhớ là lấy được giá trị ngay!
pub fn truy_cap_chi_so_o1(danh_sach: &[i32], chi_so: usize) -> Option<i32> {
    // Thao tác kiểm tra biên giới và đọc ô nhớ diễn ra trong thời gian hằng số O(1)
    danh_sach.get(chi_so).copied()
}

/// Minh họa giải thuật O(N) - Tìm kiếm tuyến tính (Linear Search)
/// Trong trường hợp xấu nhất (Worst-case), phần tử cần tìm nằm ở cuối danh sách
/// hoặc không tồn tại, hàm bắt buộc phải duyệt qua toàn bộ N phần tử.
pub fn tim_kiem_tuyen_tinh_on(danh_sach: &[i32], muc_tieu: i32) -> Option<usize> {
    for (vi_tri, &gia_tri) in danh_sach.iter().enumerate() {
        if gia_tri == muc_tieu {
            return Some(vi_tri); // Tìm thấy tại vị trí vi_tri
        }
    }
    None // Không tìm thấy sau khi duyệt hết N phần tử
}

/// Minh họa giải thuật O(log N) - Tìm kiếm nhị phân (Binary Search)
/// Điều kiện tiên quyết: Mảng đầu vào PHẢI được sắp xếp tăng dần từ trước.
/// Tại mỗi bước, ta so sánh mục tiêu với phần tử ở giữa và loại bỏ 50% phạm vi tìm kiếm.
pub fn tim_kiem_nhi_phan_ologn(danh_sach: &[i32], muc_tieu: i32) -> Option<usize> {
    if danh_sach.is_empty() {
        return None;
    }

    let mut trai: usize = 0;
    let mut phai: usize = danh_sach.len() - 1;

    while trai <= phai {
        // Tính vị trí ở giữa an toàn để tránh nguy cơ tràn số (integer overflow)
        let giua = trai + (phai - trai) / 2;
        let gia_tri_giua = danh_sach[giua];

        if gia_tri_giua == muc_tieu {
            return Some(giua);
        } else if gia_tri_giua < muc_tieu {
            // Mục tiêu nằm ở nửa bên phải, dời biên trái lên
            trai = giua + 1;
        } else {
            // Mục tiêu nằm ở nửa bên trái, dời biên phải xuống
            if giua == 0 {
                break; // Ngăn chặn tràn số usize khi trừ về dưới 0
            }
            phai = giua - 1;
        }
    }

    None
}

/// Minh họa độ phức tạp không gian O(1) vs O(N)
/// Hàm 1: Tính tổng tích lũy tại chỗ - Tiêu tốn O(1) bộ nhớ phụ
pub fn tinh_tong_tai_cho_o1_space(danh_sach: &[i32]) -> i64 {
    let mut tong: i64 = 0; // Biến duy nhất trên Stack, không tốn thêm Heap
    for &so in danh_sach {
        tong += so as i64;
    }
    tong
}

/// Hàm 2: Tạo mảng nhân đôi - Tiêu tốn O(N) bộ nhớ phụ trên Heap
pub fn tao_mang_nhan_doi_on_space(danh_sach: &[i32]) -> Vec<i32> {
    let mut ket_qua = Vec::with_capacity(danh_sach.len());
    for &so in danh_sach {
        ket_qua.push(so * 2);
    }
    ket_qua
}

fn main() {
    println!("============================================================");
    println!("   THỰC NGHIỆM ĐO ĐẠC ĐỘ PHỨC TẠP TÍNH TOÁN VỚI BIG-O       ");
    println!("============================================================");

    // Chuẩn bị tập dữ liệu lớn gồm 1.000.000 (1 triệu) số nguyên đã sắp xếp
    let quy_mo: usize = 1_000_000;
    println!("Khởi tạo danh sách gồm {} phần tử...", quy_mo);
    let danh_sach: Vec<i32> = (0..quy_mo as i32).collect();

    let muc_tieu: i32 = 999_999; // Phần tử nằm ở cuối cùng (trường hợp xấu nhất)

    // 1. Thực nghiệm O(1) - Truy cập trực tiếp qua chỉ số
    let bat_dau_o1 = Instant::now();
    let ket_qua_o1 = truy_cap_chi_so_o1(&danh_sach, quy_mo - 1);
    let thoi_gian_o1 = bat_dau_o1.elapsed();
    println!("\n[1] Thao tác O(1) - Truy cập chỉ số:");
    println!("    - Giá trị tìm được: {:?}", ket_qua_o1);
    println!("    - Thời gian thực thi: {:?}", thoi_gian_o1);

    // 2. Thực nghiệm O(N) - Tìm kiếm tuyến tính duyệt từ đầu đến cuối
    let bat_dau_on = Instant::now();
    let ket_qua_on = tim_kiem_tuyen_tinh_on(&danh_sach, muc_tieu);
    let thoi_gian_on = bat_dau_on.elapsed();
    println!("\n[2] Thao tác O(N) - Tìm kiếm tuyến tính (Duyệt 1 triệu phần tử):");
    println!("    - Vị trí tìm được: {:?}", ket_qua_on);
    println!("    - Thời gian thực thi: {:?}", thoi_gian_on);

    // 3. Thực nghiệm O(log N) - Tìm kiếm nhị phân (Chặt đôi chia để trị)
    let bat_dau_ologn = Instant::now();
    let ket_qua_ologn = tim_kiem_nhi_phan_ologn(&danh_sach, muc_tieu);
    let thoi_gian_ologn = bat_dau_ologn.elapsed();
    println!("\n[3] Thao tác O(log N) - Tìm kiếm nhị phân (Chỉ tốn ~20 phép chia):");
    println!("    - Vị trí tìm được: {:?}", ket_qua_ologn);
    println!("    - Thời gian thực thi: {:?}", thoi_gian_ologn);

    // Xác nhận tính nhất quán của kết quả
    assert_eq!(ket_qua_on, Some(quy_mo - 1));
    assert_eq!(ket_qua_ologn, Some(quy_mo - 1));

    // 4. So sánh tỷ lệ chênh lệch thời gian giữa O(log N) và O(N)
    if thoi_gian_ologn.as_nanos() > 0 {
        let ti_le = thoi_gian_on.as_nanos() as f64 / thoi_gian_ologn.as_nanos() as f64;
        println!("\n=> ĐÁNH GIÁ: O(log N) chạy nhanh gấp xấp xỉ {:.1} lần so với O(N)!", ti_le);
    }

    // 5. Kiểm tra tính năng tiêu thụ bộ nhớ không gian
    let tong_o1 = tinh_tong_tai_cho_o1_space(&danh_sach[0..100]);
    let mang_on = tao_mang_nhan_doi_on_space(&danh_sach[0..100]);
    println!("\n[4] Không gian bộ nhớ:");
    println!("    - Tổng O(1) Space: {}", tong_o1);
    println!("    - Kích thước mảng phụ O(N) Space: {} phần tử", mang_on.len());
    println!("============================================================");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn truy_cap_o1_dung_va_ngoai_bien() {
        let ds = [10, 20, 30];
        assert_eq!(truy_cap_chi_so_o1(&ds, 1), Some(20));
        assert_eq!(truy_cap_chi_so_o1(&ds, 5), None); // an toàn, không panic
    }

    #[test]
    fn tim_kiem_tuyen_tinh() {
        let ds = [4, 8, 15, 16, 23, 42];
        assert_eq!(tim_kiem_tuyen_tinh_on(&ds, 15), Some(2));
        assert_eq!(tim_kiem_tuyen_tinh_on(&ds, 99), None);
    }

    #[test]
    fn tim_kiem_nhi_phan_khop_voi_tuyen_tinh() {
        let ds: Vec<i32> = (0..1000).map(|x| x * 3).collect();
        for &muc_tieu in &[0, 297, 1500, 2997, 1, 2998] {
            // hai thuật toán phải cho CÙNG kết luận có/không
            assert_eq!(
                tim_kiem_nhi_phan_ologn(&ds, muc_tieu).is_some(),
                tim_kiem_tuyen_tinh_on(&ds, muc_tieu).is_some(),
                "bất đồng ở {}", muc_tieu
            );
        }
        assert_eq!(tim_kiem_nhi_phan_ologn(&ds, 297), Some(99));
    }

    #[test]
    fn tim_kiem_nhi_phan_mang_rong_va_mot_phan_tu() {
        assert_eq!(tim_kiem_nhi_phan_ologn(&[], 5), None);
        assert_eq!(tim_kiem_nhi_phan_ologn(&[5], 5), Some(0));
        assert_eq!(tim_kiem_nhi_phan_ologn(&[5], 3), None);
    }

    #[test]
    fn tong_o1_khong_gian() {
        assert_eq!(tinh_tong_tai_cho_o1_space(&[1, 2, 3, 4]), 10);
        assert_eq!(tinh_tong_tai_cho_o1_space(&[]), 0);
    }

    #[test]
    fn nhan_doi_on_khong_gian() {
        assert_eq!(tao_mang_nhan_doi_on_space(&[1, 2, 3]), vec![2, 4, 6]);
    }
}
