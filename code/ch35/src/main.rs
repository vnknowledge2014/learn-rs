#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bộ đếm giao dịch toàn cục tự tăng an toàn luồng
static GLOBAL_TX_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Cấu trúc một bản ghi dữ liệu có gắn phiên bản thời gian (Versioned Record)
#[derive(Clone, Debug, PartialEq)]
pub struct BanGhiPhienBan {
    pub created_by_tx: u64,         // Giao dịch tạo ra bản ghi
    pub deleted_by_tx: Option<u64>, // Giao dịch xóa bản ghi (None nếu còn hiệu lực)
    pub gia_tri: String,            // Dữ liệu thực tế
}

/// Hệ thống lưu trữ dữ liệu đa phiên bản MVCC Store
pub struct MvccStore {
    du_lieu: HashMap<String, Vec<BanGhiPhienBan>>,
}

impl MvccStore {
    pub fn new() -> Self {
        Self {
            du_lieu: HashMap::new(),
        }
    }

    /// Khởi động một giao dịch mới - Nhận một mã định danh thời gian duy nhất
    pub fn bat_dau_giao_dich(&self) -> u64 {
        GLOBAL_TX_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// THAO TÁC GHI TRONG GIAO DỊCH (Write)
    pub fn ghi(&mut self, khoa: &str, gia_tri: &str, tx_id: u64) {
        let danh_sach_phien_ban = self.du_lieu.entry(khoa.to_string()).or_default();

        // Nếu đã có phiên bản trước đó chưa bị xóa, đánh dấu bị xóa bởi giao dịch hiện tại
        for pb in danh_sach_phien_ban.iter_mut().rev() {
            if pb.deleted_by_tx.is_none() {
                pb.deleted_by_tx = Some(tx_id);
                break;
            }
        }

        // Thêm phiên bản mới vào danh sách
        danh_sach_phien_ban.push(BanGhiPhienBan {
            created_by_tx: tx_id,
            deleted_by_tx: None,
            gia_tri: gia_tri.to_string(),
        });
    }

    /// THAO TÁC ĐỌC CÔ LẬP THEO PHIÊN BẢN (Snapshot Read)
    /// Áp dụng quy tắc khả kiến: Chỉ đọc bản ghi được tạo TRƯỚC tx_id và CHƯA BỊ XÓA trước tx_id
    pub fn doc(&self, khoa: &str, current_tx_id: u64) -> Option<&str> {
        if let Some(danh_sach_phien_ban) = self.du_lieu.get(khoa) {
            // Duyệt từ phiên bản mới nhất lùi về phiên bản cũ nhất
            for pb in danh_sach_phien_ban.iter().rev() {
                // Điều kiện 1: Bản ghi phải được tạo trước hoặc cùng thời điểm giao dịch này
                let hop_le_ve_tao = pb.created_by_tx <= current_tx_id;
                // Điều kiện 2: Bản ghi chưa bị xóa, hoặc bị xóa bởi một giao dịch xảy ra trong tương lai
                let hop_le_ve_xoa = match pb.deleted_by_tx {
                    None => true,
                    Some(del_tx) => del_tx > current_tx_id,
                };

                if hop_le_ve_tao && hop_le_ve_xoa {
                    return Some(&pb.gia_tri);
                }
            }
        }
        None
    }

    /// Thao tác dọn rác (Vacuum/Compaction): Xóa bỏ các phiên bản cũ không còn giao dịch nào cần đến
    pub fn don_dep_rac(&mut self, oldest_active_tx: u64) -> usize {
        let mut so_ban_ghi_da_xoa = 0;
        for danh_sach in self.du_lieu.values_mut() {
            let ban_dau = danh_sach.len();
            // Giữ lại các bản ghi: Chưa bị xóa HOẶC bị xóa sau mốc giao dịch cũ nhất còn sống
            danh_sach.retain(|pb| {
                match pb.deleted_by_tx {
                    None => true,
                    Some(del_tx) => del_tx >= oldest_active_tx,
                }
            });
            so_ban_ghi_da_xoa += ban_dau - danh_sach.len();
        }
        so_ban_ghi_da_xoa
    }
}

impl Default for MvccStore {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("  GIAO DỊCH, ĐẢM BẢO ACID & KIỂM SOÁT ĐỒNG THỜI MVCC TRONG RUST");
    println!("============================================================");

    let mut kho_mvcc = MvccStore::new();

    // 1. Dữ liệu ban đầu được nạp bởi Giao dịch số 1 (Giao dịch khởi tạo hệ thống)
    let tx_khoi_tao = 1;
    kho_mvcc.ghi("tai_khoan:A", "1000", tx_khoi_tao);
    println!("[1] Giao dịch #{}: Khởi tạo số dư tài khoản A = 1000", tx_khoi_tao);

    // 2. Kịch bản chạy đồng thời hai giao dịch:
    // - Giao dịch Đọc (TX_DOC = 2): Bắt đầu kiểm toán báo cáo tài chính
    // - Giao dịch Ghi  (TX_GHI = 3): Khách hàng nạp thêm tiền vào tài khoản
    let tx_doc = kho_mvcc.bat_dau_giao_dich(); // tx = 2
    let tx_ghi = kho_mvcc.bat_dau_giao_dich(); // tx = 3
    println!("\n[2] Hai giao dịch đồng thời xuất hiện:");
    println!("    - Giao dịch Đọc khởi động tại mốc: tx_id = {}", tx_doc);
    println!("    - Giao dịch Ghi khởi động tại mốc : tx_id = {}", tx_ghi);

    // Giao dịch Ghi cập nhật số dư lên 1500 (Tạo phiên bản mới)
    println!("\n    -> Giao dịch Ghi #{} cập nhật tài khoản A thành 1500...", tx_ghi);
    kho_mvcc.ghi("tai_khoan:A", "1500", tx_ghi);

    // 3. Kiểm tra tính cô lập Snapshot Isolation của MVCC:
    // Giao dịch Đọc (tx = 2) đọc lại tài khoản A
    println!("\n[3] Kiểm tra tính cô lập Snapshot Isolation:");
    let so_du_doc = kho_mvcc.doc("tai_khoan:A", tx_doc);
    println!("    - Giao dịch Đọc #{} nhìn thấy số dư: {:?}", tx_doc, so_du_doc);

    // Giao dịch tương lai (tx = 4) bước vào hệ thống và đọc
    let tx_tuong_lai = kho_mvcc.bat_dau_giao_dich(); // tx = 4
    let so_du_moi = kho_mvcc.doc("tai_khoan:A", tx_tuong_lai);
    println!("    - Giao dịch mới #{} nhìn thấy số dư : {:?}", tx_tuong_lai, so_du_moi);

    // Xác nhận tính chính xác tuyệt đối:
    // Người đọc cũ (tx = 2) nhìn thấy phiên bản cũ "1000" mà không bị chặn bởi người ghi!
    assert_eq!(so_du_doc, Some("1000"));
    assert_eq!(so_du_moi, Some("1500"));
    println!("    => KẾT LUẬN: Người đọc không hề bị người ghi chặn, dữ liệu luôn nhất quán!");

    // 4. Kiểm thử tính năng dọn rác Vacuum / Compaction
    println!("\n[4] Kiểm thử dọn rác các phiên bản dữ liệu cũ (Compaction):");
    // Khi giao dịch cũ tx=2 đã kết thúc, giao dịch cũ nhất hiện tại là tx=4
    let so_rac_da_don = kho_mvcc.don_dep_rac(4);
    println!("    - Đã dọn dẹp thành công {} phiên bản dữ liệu rác cũ!", so_rac_da_don);
    assert_eq!(so_rac_da_don, 1); // Phiên bản v1 đã bị dọn dẹp

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 31               ");
    println!("============================================================");
}
