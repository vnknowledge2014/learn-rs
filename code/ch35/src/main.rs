#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bộ đếm giao dịch toàn cục tự tăng an toàn luồng
static GLOBAL_TX_COUNTER: AtomicU64 = AtomicU64::new(1);

/// Cấu trúc một bản ghi dữ liệu có gắn phiên bản thời gian (Versioned Record)
#[derive(Clone, Debug, PartialEq)]
pub struct SellRecordSessionSell {
    pub created_by_tx: u64,         // Giao dịch tạo ra bản ghi
    pub deleted_by_tx: Option<u64>, // Giao dịch xóa bản ghi (None nếu còn hiệu lực)
    pub value: String,            // Dữ liệu thực tế
}

/// Hệ thống lưu trữ dữ liệu đa phiên bản MVCC Store
pub struct MvccStore {
    data: HashMap<String, Vec<SellRecordSessionSell>>,
}

impl MvccStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }

    /// Khởi động một giao dịch mới - Nhận một mã định danh thời gian duy nhất
    pub fn start_trade(&self) -> u64 {
        GLOBAL_TX_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// THAO TÁC GHI TRONG GIAO DỊCH (Write)
    pub fn record(&mut self, key: &str, value: &str, tx_id: u64) {
        let list_session_sell = self.data.entry(key.to_string()).or_default();

        // Nếu đã có phiên bản trước đó chưa bị xóa, đánh dấu bị xóa bởi giao dịch hiện tại
        for pb in list_session_sell.iter_mut().rev() {
            if pb.deleted_by_tx.is_none() {
                pb.deleted_by_tx = Some(tx_id);
                break;
            }
        }

        // Thêm phiên bản mới vào danh sách
        list_session_sell.push(SellRecordSessionSell {
            created_by_tx: tx_id,
            deleted_by_tx: None,
            value: value.to_string(),
        });
    }

    /// THAO TÁC ĐỌC CÔ LẬP THEO PHIÊN BẢN (Snapshot Read)
    /// Áp dụng quy tắc khả kiến: Chỉ đọc bản ghi được tạo TRƯỚC tx_id và CHƯA BỊ XÓA trước tx_id
    pub fn doc(&self, key: &str, current_tx_id: u64) -> Option<&str> {
        if let Some(list_session_sell) = self.data.get(key) {
            // Duyệt từ phiên bản mới nhất lùi về phiên bản cũ nhất
            for pb in list_session_sell.iter().rev() {
                // Điều kiện 1: Bản ghi phải được tạo trước hoặc cùng thời điểm giao dịch này
                let hop_le_ve_make = pb.created_by_tx <= current_tx_id;
                // Điều kiện 2: Bản ghi chưa bị xóa, hoặc bị xóa bởi một giao dịch xảy ra trong tương lai
                let hop_le_ve_remove = match pb.deleted_by_tx {
                    None => true,
                    Some(del_tx) => del_tx > current_tx_id,
                };

                if hop_le_ve_make && hop_le_ve_remove {
                    return Some(&pb.value);
                }
            }
        }
        None
    }

    /// Thao tác dọn rác (Vacuum/Compaction): Xóa bỏ các phiên bản cũ không còn giao dịch nào cần đến
    pub fn don_dep_rac(&mut self, oldest_active_tx: u64) -> usize {
        let mut num_sell_record_da_remove = 0;
        for list in self.data.values_mut() {
            let first_sell = list.len();
            // Giữ lại các bản ghi: Chưa bị xóa HOẶC bị xóa sau mốc giao dịch cũ nhất còn sống
            list.retain(|pb| {
                match pb.deleted_by_tx {
                    None => true,
                    Some(del_tx) => del_tx >= oldest_active_tx,
                }
            });
            num_sell_record_da_remove += first_sell - list.len();
        }
        num_sell_record_da_remove
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
    let tx_block_make = 1;
    kho_mvcc.record("tai_khoan:A", "1000", tx_block_make);
    println!("[1] Giao dịch #{}: Khởi tạo số dư tài khoản A = 1000", tx_block_make);

    // 2. Kịch bản chạy đồng thời hai giao dịch:
    // - Giao dịch Đọc (TX_DOC = 2): Bắt đầu kiểm toán báo cáo tài chính
    // - Giao dịch Ghi  (TX_GHI = 3): Khách hàng nạp thêm tiền vào tài khoản
    let tx_read = kho_mvcc.start_trade(); // tx = 2
    let tx_record = kho_mvcc.start_trade(); // tx = 3
    println!("\n[2] Hai giao dịch đồng thời xuất hiện:");
    println!("    - Giao dịch Đọc khởi động tại mốc: tx_id = {}", tx_read);
    println!("    - Giao dịch Ghi khởi động tại mốc : tx_id = {}", tx_record);

    // Giao dịch Ghi cập nhật số dư lên 1500 (Tạo phiên bản mới)
    println!("\n    -> Giao dịch Ghi #{} cập nhật tài khoản A thành 1500...", tx_record);
    kho_mvcc.record("tai_khoan:A", "1500", tx_record);

    // 3. Kiểm tra tính cô lập Snapshot Isolation của MVCC:
    // Giao dịch Đọc (tx = 2) đọc lại tài khoản A
    println!("\n[3] Kiểm tra tính cô lập Snapshot Isolation:");
    let balance_read = kho_mvcc.doc("tai_khoan:A", tx_read);
    println!("    - Giao dịch Đọc #{} nhìn thấy số dư: {:?}", tx_read, balance_read);

    // Giao dịch tương lai (tx = 4) bước vào hệ thống và đọc
    let tx_tuong_lai = kho_mvcc.start_trade(); // tx = 4
    let new_balance = kho_mvcc.doc("tai_khoan:A", tx_tuong_lai);
    println!("    - Giao dịch mới #{} nhìn thấy số dư : {:?}", tx_tuong_lai, new_balance);

    // Xác nhận tính chính xác tuyệt đối:
    // Người đọc cũ (tx = 2) nhìn thấy phiên bản cũ "1000" mà không bị chặn bởi người ghi!
    assert_eq!(balance_read, Some("1000"));
    assert_eq!(new_balance, Some("1500"));
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
