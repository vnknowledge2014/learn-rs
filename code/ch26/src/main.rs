#![allow(dead_code, unused_variables, unused_imports)]
/// Hàm tính tổng các phần tử sử dụng lát cắt mượn &[i32]
/// Hàm này có tính tổng quát cực cao: Nó chấp nhận cả mảng tĩnh [i32; N],
/// một phần mảng, hoặc toàn bộ Vector động Vec<i32> mà không cần sao chép dữ liệu!
pub fn tinh_tong_lat_cat(du_lieu: &[i32]) -> i64 {
    let mut tong: i64 = 0;
    for &gia_tri in du_lieu {
        tong += gia_tri as i64;
    }
    tong
}

/// Hàm đảo ngược các phần tử tại chỗ trên một lát cắt khả biến &mut [i32]
pub fn dao_nguoc_tai_cho(du_lieu: &mut [i32]) {
    if du_lieu.is_empty() {
        return;
    }
    let mut trai = 0;
    let mut phai = du_lieu.len() - 1;
    while trai < phai {
        du_lieu.swap(trai, phai);
        trai += 1;
        phai -= 1;
    }
}

fn main() {
    println!("============================================================");
    println!("     KHẢO SÁT VÙNG NHỚ LIỀN KỀ: ARRAY, VECTOR VÀ SLICE      ");
    println!("============================================================");

    // 1. Khảo sát Mảng tĩnh [T; N] cố định trên Stack
    let mang_tinh: [i32; 5] = [10, 20, 30, 40, 50];
    println!("[1] Mảng tĩnh trên Stack:");
    println!("    - Kích thước vật lý : {} bytes", std::mem::size_of_val(&mang_tinh));
    println!("    - Số lượng phần tử  : {}", mang_tinh.len());
    
    // Kiểm chứng tính chất liền kề của các địa chỉ ô nhớ
    print!("    - Địa chỉ ô nhớ từng phần tử: ");
    for i in 0..mang_tinh.len() {
        let dia_chi = &mang_tinh[i] as *const i32 as usize;
        print!("[Phần tử {}: đuôi ...{:x}] ", i, dia_chi % 0x1000);
    }
    println!("\n    => Mỗi ô nhớ cách nhau đúng 4 bytes (kích thước i32)!");

    // 2. Khảo sát Vector động Vec<T> và chu kỳ co giãn dung lượng
    println!("\n[2] Vòng đời co giãn của Vector động (Heap Allocation):");
    let mut vec_dong: Vec<i32> = Vec::new();
    println!("    Ban đầu khi mới tạo: len = {}, cap = {}", vec_dong.len(), vec_dong.capacity());

    let mut dia_chi_truoc: usize = 0;
    for i in 1..=9 {
        vec_dong.push(i * 10);
        let dia_chi_hien_tai = vec_dong.as_ptr() as usize;
        
        // Phát hiện thời điểm vector đổi nhà sang vùng nhớ mới
        let thong_bao_doi_nha = if dia_chi_hien_tai != dia_chi_truoc && dia_chi_truoc != 0 {
            dia_chi_truoc = dia_chi_hien_tai;
            " -> [ĐỔI NHÀ MỚI TRÊN HEAP!]"
        } else {
            dia_chi_truoc = dia_chi_hien_tai;
            ""
        };

        println!(
            "    - Thêm {:2}: len = {}, cap = {:2}, ptr = {:x}{}",
            i * 10,
            vec_dong.len(),
            vec_dong.capacity(),
            dia_chi_hien_tai % 0x10000,
            thong_bao_doi_nha
        );
    }

    // 3. Tối ưu hóa trước với with_capacity
    println!("\n[3] Tối ưu hóa Vector với with_capacity(100):");
    let mut vec_toi_uu: Vec<i32> = Vec::with_capacity(100);
    let ptr_goc = vec_toi_uu.as_ptr() as usize;
    for i in 0..100 {
        vec_toi_uu.push(i);
    }
    let ptr_sau = vec_toi_uu.as_ptr() as usize;
    println!("    - Sau khi nạp 100 phần tử: len = {}, cap = {}", vec_toi_uu.len(), vec_toi_uu.capacity());
    println!("    - Địa chỉ vùng nhớ có đổi không? {}", if ptr_goc == ptr_sau { "KHÔNG ĐỔI (Cực kỳ tối ưu!)" } else { "CÓ ĐỔI" });
    assert_eq!(ptr_goc, ptr_sau);

    // 4. Khảo sát Lát cắt (Slice) - Cửa sổ góc nhìn không tốn phí sao chép
    println!("\n[4] Ứng dụng Lát cắt (Slice) linh hoạt:");
    // Lấy lát cắt từ mảng tĩnh
    let lat_cat_mang = &mang_tinh[1..4]; // Lấy phần tử chỉ số 1, 2, 3 -> [20, 30, 40]
    println!("    - Lát cắt từ mảng tĩnh [1..4]: {:?}", lat_cat_mang);
    let tong_mang = tinh_tong_lat_cat(lat_cat_mang);
    println!("    - Tổng tính từ lát cắt mảng  : {}", tong_mang);
    assert_eq!(tong_mang, 90);

    // Lấy lát cắt từ vector động
    let lat_cat_vec = &vec_dong[0..5]; // Lấy 5 phần tử đầu tiên
    println!("    - Lát cắt từ vector [0..5]   : {:?}", lat_cat_vec);
    let tong_vec = tinh_tong_lat_cat(lat_cat_vec);
    println!("    - Tổng tính từ lát cắt vector: {}", tong_vec);
    assert_eq!(tong_vec, 150);

    // 5. Thao tác trên lát cắt khả biến &mut [T]
    let mut mang_can_dao = [1, 2, 3, 4, 5, 6];
    println!("\n[5] Đảo ngược tại chỗ trên lát cắt khả biến:");
    println!("    - Mảng ban đầu : {:?}", mang_can_dao);
    // Đảo ngược chỉ một đoạn ở giữa: từ chỉ số 1 đến 4 (các số 2, 3, 4, 5)
    dao_nguoc_tai_cho(&mut mang_can_dao[1..5]);
    println!("    - Sau khi đảo đoạn [1..5]: {:?}", mang_can_dao);
    assert_eq!(mang_can_dao, [1, 5, 4, 3, 2, 6]);

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 22               ");
    println!("============================================================");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn tong_lat_cat() {
        assert_eq!(tinh_tong_lat_cat(&[10, 20, 30]), 60);
        assert_eq!(tinh_tong_lat_cat(&[]), 0);
    }

    #[test]
    fn dao_nguoc_tai_cho_khong_cap_phat_moi() {
        let mut v = vec![1, 2, 3, 4, 5];
        dao_nguoc_tai_cho(&mut v);
        assert_eq!(v, vec![5, 4, 3, 2, 1]);
    }

    #[test]
    fn dao_nguoc_hai_lan_ve_ban_dau() {
        let goc = vec![7, 3, 9, 1];
        let mut v = goc.clone();
        dao_nguoc_tai_cho(&mut v);
        dao_nguoc_tai_cho(&mut v);
        assert_eq!(v, goc); // đảo hai lần = phép đồng nhất
    }

    #[test]
    fn dao_nguoc_do_dai_le_giu_nguyen_giua() {
        let mut v = vec![1, 2, 3];
        dao_nguoc_tai_cho(&mut v);
        assert_eq!(v, vec![3, 2, 1]);
        let mut r = vec![42];
        dao_nguoc_tai_cho(&mut r);
        assert_eq!(r, vec![42]);
    }
}
