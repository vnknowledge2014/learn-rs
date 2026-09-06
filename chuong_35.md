# Chương 35: Giao dịch, Đảm bảo ACID & Kiểm soát đồng thời MVCC (Transactions, ACID Guarantees & MVCC Concurrency Control)

## Giới thiệu & Mục tiêu học tập

Trong một hệ thống ứng dụng tài chính ngân hàng, thương mại điện tử, hoặc mạng xã hội quy mô lớn, có hàng chục ngàn người dùng cùng lúc truy cập vào cơ sở dữ liệu. Hãy tưởng tượng kịch bản sau: Tài khoản của bạn có 1 triệu đồng. Cùng một tích tắc, bạn vừa chuyển 500 nghìn cho bạn bè qua điện thoại, vừa quẹt thẻ mua sắm 700 nghìn tại siêu thị. Nếu hai luồng xử lý cùng đọc số dư 1 triệu và cùng trừ tiền mà không có sự kiểm soát, hệ thống sẽ cho phép bạn tiêu tới 1,2 triệu (vượt quá số dư thực tế), hoặc ngược lại làm thất thoát tiền bạc!

Để giải quyết triệt để các vấn đề xung đột dữ liệu khi nhiều người dùng cùng thao tác đồng thời, các hệ quản trị cơ sở dữ liệu đưa ra khái niệm tối thượng: **Giao dịch (Transaction)** và bộ tiêu chuẩn vàng **ACID (Atomicity, Consistency, Isolation, Durability)**.

Tuy nhiên, làm thế nào để đảm bảo tính cô lập (Isolation) mà không làm tê liệt hệ thống? Nếu mỗi lần một người sửa dữ liệu ta lại khóa cứng toàn bộ bảng lại (Khóa bi quan - Pessimistic Locking), hàng ngàn người dùng khác sẽ phải đứng xếp hàng chờ đợi, gây tắc nghẽn nghiêm trọng. Để đạt được thông lượng xử lý hàng triệu giao dịch mỗi giây, giải pháp hiện đại bậc nhất chính là **Kiểm soát đồng thời đa phiên bản (Multi-Version Concurrency Control - MVCC)**. Đặc biệt, kiến trúc MVCC kết hợp hoàn hảo một cách tự nhiên với động cơ lưu trữ **LSM-Tree** (với các tầng `MemTable` và `SSTable` bất biến) mà chúng ta đã tìm hiểu.

Mục tiêu học tập của chương này:
- Nắm vững bản chất 4 thuộc tính vàng của **ACID**: Tính nguyên tử (Atomicity), Tính nhất quán (Consistency), Tính cô lập (Isolation), và Tính bền vững (Durability).
- Nhận diện các hiện tượng nguy hiểm khi thiếu kiểm soát đồng thời: Đọc rác (Dirty Read), Đọc không thể lặp lại (Non-repeatable Read), và Đọc bóng ma (Phantom Read).
- Phân biệt cơ chế Khóa bi quan (Pessimistic Locking / 2PL) và triết lý tiến bộ của **MVCC**: *"Người đọc không bao giờ chặn người ghi, người ghi không bao giờ chặn người đọc"*.
- Hiểu sâu sắc mối quan hệ cộng sinh giữa MVCC và động cơ **LSM-Tree** (`MemTable`, `SSTable`, và tiến trình `Compaction`).
- Tự tay lập trình một hệ thống lưu trữ đa phiên bản MVCC hoàn chỉnh bằng Rust, kiểm soát tầm nhìn bản ghi (Snapshot Visibility) thông qua mã định danh giao dịch (`tx_id`).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy quan sát hai câu chuyện đời thường vô cùng quen thuộc để thấu hiểu bản chất của Giao dịch ACID và cơ chế MVCC:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG HÓA GIAO DỊCH ACID VÀ CƠ CHẾ MVCC                   │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. TÍNH NGUYÊN TỬ (ATOMICITY): GIAO DỊCH MUA BÁN TẬN TAY]                       │
│                                                                                  │
│         Bạn đưa 50.000đ ══════════════════► Người bán đưa Cốc trà sữa            │
│                                                                                  │
│ - TẤT CẢ HOẶC KHÔNG CÓ GÌ (ALL OR NOTHING):                                      │
│   + Cả 2 việc cùng thành công: Bạn có trà sữa, người bán có tiền (Commit).       │
│   + Nếu bạn rơi tiền hoặc quán hết trà sữa: Tiền trả về túi bạn (Rollback).      │
│   + Tuyệt đối KHÔNG BAO GIỜ có chuyện bạn mất tiền mà không nhận được đồ!        │
│                                                                                  │
│ [2. CƠ CHẾ MVCC: MÁY PHOTOCOPY BẢN SAO HỢP ĐỒNG KHI SỬA ĐỔI]                     │
│                                                                                  │
│ Luật sư đang đọc Hợp đồng v1                 Giám đốc muốn sửa Điều khoản        │
│          │                                                │                      │
│          ▼                                                ▼                      │
│ [Đọc thong thả bản v1]                       [Không giật giấy trên tay luật sư]  │
│ Không bị ai làm phiền!                       Tạo bản copy mới: Hợp đồng v2       │
│                                              Sửa xong ký tên: Đóng dấu v2!       │
│                                                                                  │
│ => NGƯỜI ĐỌC KHÔNG CHẶN NGƯỜI GHI — NGƯỜI GHI KHÔNG CHẶN NGƯỜI ĐỌC!            │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Tính nguyên tử (Atomicity) — Mua bán đổi chác trao tay
- Hãy tưởng tượng bạn đi mua một cốc trà sữa ở góc phố:
  - Bạn rút tờ tiền 50 nghìn trao cho người bán, đồng thời người bán trao cốc trà sữa mát lạnh vào tay bạn.
  - Đây là một **hành động nguyên tử (Atomic)**: Không thể chia cắt nhỏ hơn được nữa.
  - Hoặc là cả hai việc cùng diễn ra trọn vẹn (giao dịch thành công - **Commit**).
  - Hoặc nếu giữa chừng người bán lỡ tay làm rơi cốc trà sữa xuống đất, người bán lập tức trả lại tờ tiền 50 nghìn vào ví bạn (hoàn tác - **Rollback**).
  - Không bao giờ có trạng thái lửng lơ: Tiền của bạn bị trừ mà người bán không đưa hàng!

### 2. MVCC — Máy photocopy hợp đồng trong văn phòng luật
- Hãy tưởng tượng một văn phòng bận rộn:
  - Luật sư đang ngồi tại bàn nghiên cứu một bản hợp đồng kinh tế phiên bản 1 (`Version 1`).
  - Cùng lúc đó, vị Giám đốc bước vào và muốn sửa đổi điều khoản số 5 của hợp đồng.
  - **Cách làm kiểu cũ (Khóa bi quan - Lock)**: Giám đốc giật phắt tờ hợp đồng trên tay Luật sư, bắt Luật sư ngồi im khoanh tay đợi Giám đốc sửa xong thì mới được đọc tiếp. Văn phòng rơi vào tình trạng đóng băng!
  - **Cách làm tân tiến kiểu MVCC (Đa phiên bản)**: Giám đốc không làm phiền Luật sư. Ông ấy chụp scan một bản sao mới, sửa thành phiên bản 2 (`Version 2`) rồi ký tên.
  - Trong suốt thời gian đó, Luật sư vẫn thong thả đọc trọn vẹn phiên bản 1 mà không bị gián đoạn một giây nào. Khi Giám đốc hoàn tất phiên bản 2, các nhân viên mới vào đọc sẽ nhìn thấy phiên bản 2. Người đọc và người ghi làm việc song song tuyệt đối!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Giải mã 4 thuộc tính vàng của ACID

1. **A - Atomicity (Tính nguyên tử)**: Toàn bộ các thao tác trong một giao dịch (Transaction) được đối xử như một đơn vị logic duy nhất. Hoặc tất cả cùng thành công (`COMMIT`), hoặc nếu có một lỗi nhỏ nhất xảy ra, toàn bộ trạng thái sẽ được hoàn tác về như lúc ban đầu (`ROLLBACK`).
2. **C - Consistency (Tính nhất quán)**: Dữ liệu chuyển đổi từ một trạng thái hợp lệ này sang một trạng thái hợp lệ khác, không bao giờ vi phạm các quy tắc nghiệp vụ (ví dụ: Tổng số tiền trong hệ thống không thể tự nhiên sinh ra hay mất đi, số dư không được âm).
3. **I - Isolation (Tính cô lập)**: Xác định mức độ mà các thay đổi trong một giao dịch đang chạy bị ẩn giấu đối với các giao dịch khác.
4. **D - Durability (Tính bền vững)**: Một khi giao dịch đã được xác nhận thành công (`COMMIT`), các thay đổi của nó sẽ được ghi vĩnh viễn xuống đĩa cứng (thông qua nhật ký WAL) và không bao giờ bị mất mát, kể cả khi hệ thống sập nguồn điện ngay sau đó.

### 2. Các hiện tượng xung đột và Các cấp độ cô lập (Isolation Levels)

Khi nhiều giao dịch chạy song song, nếu không cô lập tốt sẽ nảy sinh 3 hiểm họa:
- **Dirty Read (Đọc dữ liệu rác)**: Giao dịch A đọc một giá trị do Giao dịch B vừa sửa, nhưng sau đó Giao dịch B bị hủy (`Rollback`). Giao dịch A đã hành động dựa trên một dữ liệu ma quỷ không có thật!
- **Non-repeatable Read (Đọc không nhất quán)**: Giao dịch A đọc dòng số 1 ra giá trị 100. Giao dịch B vào sửa thành 200 và Commit. Giao dịch A đọc lại dòng số 1 thì thấy giá trị biến thành 200.
- **Phantom Read (Bóng ma xuất hiện)**: Giao dịch A đếm có 5 đơn hàng. Giao dịch B chèn thêm đơn hàng thứ 6. Giao dịch A đếm lại thì thấy xuất hiện thêm dòng mới.

Hội đồng chuẩn SQL định nghĩa 4 cấp độ cô lập từ yếu đến mạnh:
1. `Read Uncommitted`: Cho phép đọc dữ liệu chưa commit (nguy hiểm nhất).
2. `Read Committed`: Chỉ đọc dữ liệu đã commit (chống Dirty Read).
3. `Repeatable Read`: Đảm bảo đọc một dòng nhiều lần luôn ra cùng kết quả (chuẩn mặc định của MySQL).
4. `Serializable`: Các giao dịch chạy như thể tuần tự từng cái một (an toàn nhất nhưng chậm nhất).

### 3. Cơ chế hoạt động của MVCC trong Động cơ lưu trữ

Trong mô hình MVCC, mỗi giao dịch khi bắt đầu được gán một con số nguyên tự tăng đại diện cho dấu mốc thời gian: `tx_id` (Transaction ID).

Mỗi bản ghi trong cơ sở dữ liệu được đính kèm hai trường siêu dữ liệu (metadata):
- `created_by_tx`: Mã của giao dịch đã tạo ra bản ghi này.
- `deleted_by_tx`: Mã của giao dịch đã xóa hoặc ghi đè bản ghi này (nếu chưa bị xóa thì bằng `None`).

```
Khóa: "user:101"
┌──────────────────────┬──────────────────────┬───────────────────────────────┐
│ created_by_tx: 1     │ deleted_by_tx: 5     │ Giá trị: "Alice (Bản gốc v1)" │
├──────────────────────┼──────────────────────┼───────────────────────────────┤
│ created_by_tx: 5     │ deleted_by_tx: None  │ Giá trị: "Alice (Đổi tên v2)" │
└──────────────────────┴──────────────────────┴───────────────────────────────┘
```

**Quy tắc khả kiến (Snapshot Visibility Rule)**:
Khi Giao dịch có mã số `current_tx = 3` thực hiện đọc khóa `"user:101"`:
- Nó kiểm tra phiên bản 1: Được tạo bởi `tx = 1 <= 3` (hợp lệ) và bị xóa bởi `tx = 5 > 3` (tại thời điểm `tx = 3`, hành động xóa của `tx = 5` chưa hề xảy ra!). Do đó, Giao dịch 3 nhìn thấy phiên bản 1!
- Giao dịch 3 hoàn toàn không nhìn thấy phiên bản 2 (vì phiên bản 2 sinh ra ở tương lai `tx = 5`).

### 4. Mối liên hệ tự nhiên giữa MVCC và LSM-Tree

Tại sao các hệ thống cơ sở dữ liệu hiện đại sử dụng **LSM-Tree** lại cực kỳ ưa chuộng **MVCC**?
- Trong LSM-Tree, các tệp **SSTable** trên đĩa cứng là **bất biến (Immutable)**.
- Khi có lệnh cập nhật hay xóa, LSM-Tree không bao giờ sửa đè lên dữ liệu cũ, mà chỉ ghi một phiên bản mới vào `MemTable` kèm theo `tx_id` hoặc cờ Tombstone.
- Điều này trùng khớp 100% với nguyên lý của MVCC! Tiến trình nén gộp (**Compaction**) của LSM-Tree sẽ đóng vai trò như một người thu gom rác (Garbage Collector), chỉ dọn dẹp và tiêu hủy các phiên bản cũ khi chắc chắn rằng không còn bất kỳ giao dịch nào đang hoạt động cần đọc các phiên bản đó nữa.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh và độc lập, cài đặt một hệ thống lưu trữ đa phiên bản **MVCC Store** an toàn luồng dữ liệu, hỗ trợ giao dịch đọc cô lập Snapshot Isolation:

```rust
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

/// Bộ đếm deliver dịch toàn cục tự tăng an toàn luồng
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

    /// Khởi động một deliver dịch mới - Nhận một mã định danh thời gian duy nhất
    pub fn start_trade(&self) -> u64 {
        GLOBAL_TX_COUNTER.fetch_add(1, Ordering::SeqCst)
    }

    /// THAO TÁC GHI TRONG GIAO DỊCH (Write)
    pub fn record(&mut self, key: &str, value: &str, tx_id: u64) {
        let list_session_sell = self.data.entry(key.to_string()).or_default();

        // Nếu đã có phiên bản trước đó chưa bị xóa, đánh dấu bị xóa bởi deliver dịch hiện tại
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
                // Điều kiện 1: Bản ghi phải được tạo trước hoặc cùng thời điểm deliver dịch này
                let hop_le_ve_make = pb.created_by_tx <= current_tx_id;
                // Điều kiện 2: Bản ghi chưa bị xóa, hoặc bị xóa bởi một deliver dịch xảy ra trong tương lai
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

    /// Thao tác dọn rác (Vacuum/Compaction): Xóa bỏ các phiên bản cũ không còn deliver dịch nào cần đến
    pub fn don_dep_rac(&mut self, oldest_active_tx: u64) -> usize {
        let mut num_sell_record_da_remove = 0;
        for list in self.data.values_mut() {
            let first_sell = list.len();
            // Giữ lại các bản ghi: Chưa bị xóa HOẶC bị xóa sau mốc deliver dịch cũ nhất còn sống
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

    let mut mvcc_store = MvccStore::new();

    // 1. Dữ liệu ban đầu được nạp bởi Giao dịch số 1 (Giao dịch khởi tạo hệ thống)
    let tx_block_make = 1;
    mvcc_store.record("tai_khoan:A", "1000", tx_block_make);
    println!("[1] Giao dịch #{}: Khởi tạo số dư tài khoản A = 1000", tx_block_make);

    // 2. Kịch bản chạy đồng thời hai deliver dịch:
    // - Giao dịch Đọc (TX_DOC = 2): Bắt đầu kiểm toán báo cáo tài chính
    // - Giao dịch Ghi  (TX_GHI = 3): Khách hàng nạp thêm tiền vào tài khoản
    let tx_read = mvcc_store.start_trade(); // tx = 2
    let tx_record = mvcc_store.start_trade(); // tx = 3
    println!("\n[2] Hai deliver dịch đồng thời xuất hiện:");
    println!("    - Giao dịch Đọc khởi động tại mốc: tx_id = {}", tx_read);
    println!("    - Giao dịch Ghi khởi động tại mốc : tx_id = {}", tx_record);

    // Giao dịch Ghi cập nhật số dư lên 1500 (Tạo phiên bản mới)
    println!("\n    -> Giao dịch Ghi #{} cập nhật tài khoản A thành 1500...", tx_record);
    mvcc_store.record("tai_khoan:A", "1500", tx_record);

    // 3. Kiểm tra tính cô lập Snapshot Isolation của MVCC:
    // Giao dịch Đọc (tx = 2) đọc lại tài khoản A
    println!("\n[3] Kiểm tra tính cô lập Snapshot Isolation:");
    let balance_read = mvcc_store.doc("tai_khoan:A", tx_read);
    println!("    - Giao dịch Đọc #{} nhìn thấy số dư: {:?}", tx_read, balance_read);

    // Giao dịch tương lai (tx = 4) bước vào hệ thống và đọc
    let future_tx = mvcc_store.start_trade(); // tx = 4
    let new_balance = mvcc_store.doc("tai_khoan:A", future_tx);
    println!("    - Giao dịch mới #{} nhìn thấy số dư : {:?}", future_tx, new_balance);

    // Xác nhận tính chính xác tuyệt đối:
    // Người đọc cũ (tx = 2) nhìn thấy phiên bản cũ "1000" mà không bị chặn bởi người ghi!
    assert_eq!(balance_read, Some("1000"));
    assert_eq!(new_balance, Some("1500"));
    println!("    => KẾT LUẬN: Người đọc không hề bị người ghi chặn, dữ liệu luôn nhất quán!");

    // 4. Kiểm thử tính năng dọn rác Vacuum / Compaction
    println!("\n[4] Kiểm thử dọn rác các phiên bản dữ liệu cũ (Compaction):");
    // Khi deliver dịch cũ tx=2 đã kết thúc, deliver dịch cũ nhất hiện tại là tx=4
    let so_rac_da_don = mvcc_store.don_dep_rac(4);
    println!("    - Đã dọn dẹp thành công {} phiên bản dữ liệu rác cũ!", so_rac_da_don);
    assert_eq!(so_rac_da_don, 1); // Phiên bản v1 đã bị dọn dẹp

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 31               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi lập trình hệ thống giao dịch đồng thời và MVCC trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0502** | `cannot borrow 'kho_mvcc' as mutable because it is also borrowed as immutable` | Bạn đang giữ kết quả tham chiếu mượn của hàm `doc()` (`let val = kho.doc(...)`) nhưng lại gọi phương thức `kho.ghi(...)` làm thay đổi bản đồ bộ nhớ. | Sao chép giá trị chuỗi `.to_string()` hoặc kết thúc phạm vi mượn đọc trước khi thực hiện ghi dữ liệu. |
| **E0382** | `use of moved value: 'list_session_sell'` | Bạn di chuyển quyền sở hữu của vector phiên bản trong vòng lặp bằng cách duyệt qua giá trị thay vì tham chiếu mượn. | Dùng `.iter()` hoặc `.iter_mut()` khi duyệt qua các phiên bản để tránh di chuyển quyền sở hữu (ownership). |
| **E0596** | `cannot borrow field '...' as mutable` | Bạn cố thay đổi trường `deleted_by_tx` trong khi đang duyệt bằng iterator bất biến `.iter()`. | Chuyển sang sử dụng phương thức `.iter_mut()`. |
| **E0277** | `the trait bound 'AtomicU64: Clone' is not satisfied` | Kiểu dữ liệu nguyên tử `AtomicU64` đại diện cho một ô nhớ phần cứng cụ thể, không hỗ trợ sao chép (Clone). | Sử dụng tham chiếu `&AtomicU64` hoặc chia sẻ qua con trỏ đếm tham chiếu đa luồng `Arc<AtomicU64>`. |

### Ví dụ phân tích lỗi `E0502` khi vừa đọc vừa ghi trong MVCC:

```rust
// Đoạn mã lỗi minh họa E0502: Xung đột mượn đọc và mượn ghi
fn broken_mvcc(store: &mut MvccStore) {
    // let ket_qua = store.doc("key", 2); // Mượn bất biến store
    // store.ghi("key", "val_moi", 3);    // LỖI E0502: Mượn khả biến store khi đang bị mượn đọc!
    // println!("Đã đọc: {:?}", ket_qua);
}

// Cách sửa chữa đúng chuẩn: Chuyển dữ liệu mượn thành kiểu sở hữu độc lập
fn correct_mvcc(store: &mut MvccStore) {
    // Bước 1: Sao chép kết quả ra biến String độc lập
    let ket_qua = store.doc("key", 2).map(|s| s.to_string());
    
    // Bước 2: Tự do thực hiện thao tác ghi mà không vi phạm quy tắc mượn
    store.record("key", "val_moi", 3);
    
    println!("Dữ liệu đọc trước đó: {:?}", ket_qua);
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Tiêu chuẩn ACID**: Là nền móng bảo đảm tính toàn vẹn và độ tin cậy của mọi hệ thống dữ liệu; đảm bảo các giao dịch diễn ra nguyên tử, nhất quán, cô lập và bền vững vĩnh viễn.
2. **Triết lý MVCC đỉnh cao**: Bằng cách lưu trữ nhiều phiên bản kèm dấu mốc thời gian giao dịch (`tx_id`), MVCC triệt tiêu việc khóa bảng, giúp người đọc và người ghi không bao giờ cản trở lẫn nhau.
3. **Quy tắc khả kiến (Visibility)**: Một giao dịch chỉ được phép nhìn thấy các bản ghi được tạo ra trước thời điểm nó bắt đầu và chưa bị xóa trước thời điểm đó.
4. **Cộng sinh hoàn hảo với LSM-Tree**: Tính chất bất biến (Immutable) của các tệp `SSTable` trong LSM-Tree biến nó thành động cơ tự nhiên tối ưu nhất để triển khai MVCC.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phân tích kịch bản chuyển tiền ACID)**:  
   Giao dịch $T_1$ chuyển 200 nghìn từ tài khoản A sang tài khoản B gồm hai bước: `A = A - 200` và `B = B + 200`. Nếu máy tính sập nguồn ngay sau khi bước 1 hoàn thành, thuộc tính ACID nào sẽ đảm bảo tài khoản A không bị mất oan 200 nghìn? Quy trình khôi phục diễn ra như thế nào?
2. **Bài tập 2 (Xử lý Rollback trong MVCC)**:  
   Hãy thiết kế thêm phương thức `fn rollback_giao_dich(&mut self, tx_id: u64)` cho `MvccStore`: Tìm tất cả các bản ghi có `created_by_tx == tx_id` và xóa chúng khỏi hệ thống, đồng thời khôi phục lại các bản ghi cũ bị đánh dấu `deleted_by_tx == Some(tx_id)` về trạng thái `None`.
3. **Bài tập 3 (Tư duy mở rộng)**:  
   Trong các hệ quản trị cơ sở dữ liệu lớn như PostgreSQL, hiện tượng gì sẽ xảy ra nếu một giao dịch đọc kéo dài hàng tuần lễ mà không chịu đóng lại (`commit`/`abort`)? Giao dịch này sẽ gây ảnh hưởng tiêu cực như thế nào đến tiến trình dọn rác (Vacuum / Compaction) của MVCC?
