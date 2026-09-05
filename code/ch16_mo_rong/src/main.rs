#![allow(dead_code, unused_variables, unused_imports)]
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Iterator: map, filter, fold, collect trong Rust

#[derive(Debug, Clone, PartialEq)]
pub struct BanGhiCamBien {
    pub ma_cam_bien: String,
    pub nhiet_do_c: f64,
    pub ap_suat_bar: f64,
    pub hop_le: bool,
}

#[derive(Debug, PartialEq)]
pub struct ThongBaoNguyHiem {
    pub thu_tu_ghi_nhan: usize,
    pub noi_dung: String,
    pub muc_do: String,
}

fn main() {
    println!("============================================================");
    println!("   HỆ THỐNG XỬ LÝ DÒNG DỮ LIỆU CẢM BIẾN NHÀ MÁY (IOT FP)   ");
    println!("============================================================");

    // 1. Khởi tạo danh sách dữ liệu cảm biến thô ban đầu
    let mut du_lieu_tho: Vec<BanGhiCamBien> = vec![
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-01"),
            nhiet_do_c: 85.5,
            ap_suat_bar: 3.2,
            hop_le: true,
        },
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-02"),
            nhiet_do_c: -999.0, // Dữ liệu lỗi do đứt dây cáp
            ap_suat_bar: 0.0,
            hop_le: false,
        },
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-03"),
            nhiet_do_c: 125.0, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            ap_suat_bar: 4.8,
            hop_le: true,
        },
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-04"),
            nhiet_do_c: 72.0,
            ap_suat_bar: 2.9,
            hop_le: true,
        },
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-05"),
            nhiet_do_c: 110.5, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            ap_suat_bar: 5.1,
            hop_le: true,
        },
    ];

    println!("Số lượng bản ghi thu thập được: {}", du_lieu_tho.len());

    // ------------------------------------------------------------------------
    // KỸ THUẬT 1: Dùng .iter_mut() để hiệu chỉnh dữ liệu trực tiếp tại chỗ
    // Giả sử cảm biến có sai số cố định +0.5°C cần được bù trừ
    // ------------------------------------------------------------------------
    println!("\n1. Tiến hành bù trừ sai số thiết bị qua .iter_mut():");
    du_lieu_tho
        .iter_mut()
        .filter(|ban_ghi| ban_ghi.hop_le)
        .for_each(|ban_ghi| {
            ban_ghi.nhiet_do_c -= 0.5; // Trừ trực tiếp trên ô nhớ RAM
        });
    println!("-> Đã hiệu chỉnh sai số cho tất cả cảm biến hợp lệ thành công.");

    // ------------------------------------------------------------------------
    // KỸ THUẬT 2: Dùng .iter(), .filter(), .map() xây dựng đường ống lọc & trích xuất
    // Lấy danh sách nhiệt độ của các cảm biến an toàn (nhiệt độ <= 100°C)
    // ------------------------------------------------------------------------
    println!("\n2. Trích xuất danh sách nhiệt độ hoạt động an toàn (<= 100°C):");
    let nhiet_do_an_toan: Vec<f64> = du_lieu_tho
        .iter()
        .filter(|bg| bg.hop_le)                  // Lọc bỏ cảm biến hỏng
        .filter(|bg| bg.nhiet_do_c <= 100.0)     // Lọc cảm biến trong ngưỡng an toàn
        .map(|bg| bg.nhiet_do_c)                 // Chỉ trích xuất lấy số đo nhiệt độ
        .collect();                              // Gom tụ thành Vector mới

    println!("-> Các mức nhiệt độ an toàn: {:?}", nhiet_do_an_toan);

    // ------------------------------------------------------------------------
    // KỸ THUẬT 3: Dùng .fold() để tổng hợp thống kê phức tạp trong một lượt duyệt duy nhất
    // Tính tổng nhiệt độ và đếm số lượng cảm biến an toàn để tính trung bình
    // ------------------------------------------------------------------------
    println!("\n3. Tính nhiệt độ trung bình của phân xưởng qua .fold():");
    let (tong_nhiet, so_luong) = du_lieu_tho
        .iter()
        .filter(|bg| bg.hop_le)
        .fold((0.0, 0usize), |(tong, dem), bg| {
            (tong + bg.nhiet_do_c, dem + 1)
        });

    if so_luong > 0 {
        let trung_binh = tong_nhiet / (so_luong as f64);
        println!("-> Tổng nhiệt độ: {:.2}°C trên {} cảm biến.", tong_nhiet, so_luong);
        println!("-> Nhiệt độ trung bình toàn xưởng: {:.2}°C", trung_binh);
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 4: Kết hợp .enumerate(), .filter(), và .collect()
    // Tạo danh sách cảnh báo khẩn cấp cho các cảm biến vượt ngưỡng (> 100°C)
    // ------------------------------------------------------------------------
    println!("\n4. Phát hiện nguy cơ và tổng hợp danh sách cảnh báo khẩn cấp:");
    let danh_sach_canh_bao: Vec<ThongBaoNguyHiem> = du_lieu_tho
        .iter()
        .enumerate() // Cung cấp chỉ số thứ tự (0, 1, 2...) đi kèm với phần tử
        .filter(|(_, bg)| bg.hop_le && bg.nhiet_do_c > 100.0)
        .map(|(chi_so, bg)| ThongBaoNguyHiem {
            thu_tu_ghi_nhan: chi_so + 1,
            noi_dung: format!("Cảm biến [{}] vượt ngưỡng nhiệt độ: {:.2}°C", bg.ma_cam_bien, bg.nhiet_do_c),
            muc_do: String::from("KHẨN CẤP"),
        })
        .collect();

    for cb in &danh_sach_canh_bao {
        println!("  [!] Vị trí #{}: {} (Mức độ: {})", 
                 cb.thu_tu_ghi_nhan, cb.noi_dung, cb.muc_do);
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 5: Dùng .into_iter() để tiêu thụ toàn bộ dữ liệu và giải phóng bộ nhớ
    // ------------------------------------------------------------------------
    println!("\n5. Di chuyển quyền sở hữu toàn bộ qua .into_iter():");
    let ma_tat_ca_cam_bien: Vec<String> = du_lieu_tho
        .into_iter()
        .map(|bg| bg.ma_cam_bien) // Đoạt quyền sở hữu trường String mà không cần clone!
        .collect();

    println!("-> Danh sách mã thiết bị sau khi thu hồi: {:?}", ma_tat_ca_cam_bien);
    // du_lieu_tho đã bị tiêu thụ tại đây, giải phóng bộ nhớ sạch sẽ!

    println!("\n============================================================");
    println!("     XỬ LÝ TOÀN BỘ ĐƯỜNG ỐNG ITERATOR THÀNH CÔNG RỰC RỠ     ");
    println!("============================================================");
}
