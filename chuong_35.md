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
- Tự tay lập trình một hệ thống lưu trữ đa phiên bản MVCC hoàn chỉnh bằng Rust, kiểm soát tầm nhìn bản ghi (Snapshot Visibility) thông qua mã định danh giao dịch (`tx_id`) kết hợp trạng thái commit, và phát hiện xung đột ghi–ghi.

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

Ngoài ba hiện tượng đọc, còn một hiểm hoạ về **ghi**:
- **Lost Update (Mất cập nhật)**: Giao dịch A và B cùng đọc số dư 1000. A ghi 1100 (nạp 100) và commit; B ghi 1200 (nạp 200) và commit. Kết quả 1200 — khoản nạp của A biến mất không dấu vết.

Hội đồng chuẩn SQL định nghĩa 4 cấp độ cô lập từ yếu đến mạnh:
1. `Read Uncommitted`: Cho phép đọc dữ liệu chưa commit (nguy hiểm nhất).
2. `Read Committed`: Chỉ đọc dữ liệu đã commit (chống Dirty Read).
3. `Repeatable Read`: Đảm bảo đọc một dòng nhiều lần luôn ra cùng kết quả (chuẩn mặc định của MySQL).
4. `Serializable`: Các giao dịch chạy như thể tuần tự từng cái một (an toàn nhất nhưng chậm nhất).

MVCC thường hiện thực một mức nằm giữa 3 và 4 gọi là **Snapshot Isolation (SI)**: mỗi giao dịch đọc từ một *ảnh chụp* các giao dịch **đã commit** tại thời điểm nó bắt đầu, và hai giao dịch đồng thời không được cùng commit thay đổi trên một khoá (**first-committer-wins**). SI chặn được Dirty Read, Non-repeatable Read, Phantom (trong ảnh chụp) và Lost Update — nhưng vẫn để lọt một dị thường tinh vi gọi là *write skew* (hai giao dịch đọc chung dữ liệu rồi ghi vào hai khoá *khác nhau*), nên SI chưa phải Serializable.

### 3. Cơ chế hoạt động của MVCC trong Động cơ lưu trữ

Trong mô hình MVCC, mỗi giao dịch khi bắt đầu được gán một con số nguyên tự tăng đại diện cho dấu mốc thời gian: `tx_id` (Transaction ID). Hệ thống còn nhớ **trạng thái** của từng giao dịch: đang chạy (`Active`), đã commit (`Committed`) hay đã huỷ (`Aborted`).

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

**Ảnh chụp (Snapshot)**: lúc bắt đầu, giao dịch `T` chụp lại hai thứ: `xmax` = mã của chính nó (mọi giao dịch có mã `>= xmax` bắt đầu sau `T`), và tập `active` = các giao dịch **còn đang chạy** ngay lúc đó.

**Quy tắc khả kiến (Snapshot Visibility Rule)**: `T` *nhìn thấy* tác động của giao dịch `X` khi và chỉ khi `X` là chính `T`, **hoặc** `X` đã commit **và** `X < xmax` **và** `X` không nằm trong `active`. Một phiên bản hiện ra với `T` khi `T` nhìn thấy giao dịch đã tạo ra nó, và **không** nhìn thấy giao dịch đã xoá nó.

Khi Giao dịch có mã số `3` (bắt đầu lúc giao dịch 1 đã commit) đọc khóa `"user:101"`:
- Phiên bản 1: tạo bởi `tx = 1` (đã commit trước khi 3 bắt đầu → thấy), bị xóa bởi `tx = 5` (`5 >= xmax = 3`, bắt đầu sau → không thấy lệnh xoá). Do đó, Giao dịch 3 nhìn thấy phiên bản 1!
- Phiên bản 2: tạo bởi `tx = 5` → vô hình với giao dịch 3.

> **Cái bẫy kinh điển:** chỉ so sánh `created_by_tx <= current_tx` là **chưa đủ**. Giả sử giao dịch 2 ghi một phiên bản rồi *chưa commit* (hoặc sẽ bị huỷ). Giao dịch 3 có `2 <= 3` nên sẽ đọc được phiên bản đó — đó chính là **Dirty Read**. Ngược lại, giao dịch 2 commit *sau khi* 3 bắt đầu cũng không được hiện ra với 3, nếu không ảnh chụp sẽ "trôi". Vì vậy ảnh chụp phải ghi nhớ tập giao dịch đang chạy, và hệ thống phải biết trạng thái commit của từng giao dịch.

**Chống Lost Update**: trước khi ghi đè một khoá, giao dịch kiểm tra phiên bản mới nhất của khoá đó. Nếu nó do một giao dịch mà ta **không nhìn thấy** tạo ra (đang chạy, hoặc commit sau khi ta bắt đầu) thì ta đang định ghi đè lên một thay đổi mình chưa từng đọc — giao dịch bị từ chối với lỗi xung đột ghi–ghi và phải thử lại với ảnh chụp mới. PostgreSQL ở mức `REPEATABLE READ` làm đúng như vậy (*"could not serialize access due to concurrent update"*).

### 4. Mối liên hệ tự nhiên giữa MVCC và LSM-Tree

Tại sao các hệ thống cơ sở dữ liệu hiện đại sử dụng **LSM-Tree** lại cực kỳ ưa chuộng **MVCC**?
- Trong LSM-Tree, các tệp **SSTable** trên đĩa cứng là **bất biến (Immutable)**.
- Khi có lệnh cập nhật hay xóa, LSM-Tree không bao giờ sửa đè lên dữ liệu cũ, mà chỉ ghi một phiên bản mới vào `MemTable` kèm theo `tx_id` hoặc cờ Tombstone.
- Điều này trùng khớp 100% với nguyên lý của MVCC! Tiến trình nén gộp (**Compaction**) của LSM-Tree sẽ đóng vai trò như một người thu gom rác (Garbage Collector), chỉ dọn dẹp và tiêu hủy các phiên bản cũ khi chắc chắn rằng không còn bất kỳ giao dịch nào đang hoạt động cần đọc các phiên bản đó nữa.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh và độc lập, cài đặt một hệ thống lưu trữ đa phiên bản **MVCC Store** với Snapshot Isolation đúng nghĩa: ảnh chụp các giao dịch đã commit, trạng thái giao dịch (`Active`/`Committed`/`Aborted`), phát hiện xung đột ghi–ghi, và dọn rác theo "đường chân trời" của giao dịch cũ nhất còn chạy. Để tập trung vào thuật toán, đây là mô hình **đơn luồng**: nhiều giao dịch chạy *xen kẽ* nhau, mỗi lời gọi là nguyên tử. Lưu ý cách thiết kế dùng hệ thống kiểu của Rust: `commit`/`abort` nhận `Transaction` *theo giá trị*, nên trình biên dịch cấm dùng lại một giao dịch đã kết thúc:

```rust
use std::collections::{HashMap, HashSet};

/// Mã giao dịch: số nguyên tự tăng, cấp theo thứ tự BẮT ĐẦU giao dịch
pub type TxId = u64;

/// Trạng thái của một giao dịch
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TxStatus {
    Active,
    Committed,
    Aborted,
}

/// Ảnh chụp (snapshot) chụp lúc giao dịch bắt đầu:
/// - `xmax`: mọi giao dịch có mã >= xmax bắt đầu SAU ta -> vô hình
/// - `active`: các giao dịch còn đang chạy lúc ta bắt đầu -> vô hình,
///   kể cả khi sau này chúng commit
#[derive(Clone, Debug)]
pub struct Snapshot {
    xmax: TxId,
    active: HashSet<TxId>,
}

/// Một giao dịch đang mở. Không `Clone`, và `commit`/`abort` nhận nó THEO GIÁ TRỊ:
/// trình biên dịch cấm dùng lại một giao dịch đã kết thúc.
#[derive(Debug)]
pub struct Transaction {
    pub id: TxId,
    snapshot: Snapshot,
}

/// Cấu trúc một bản ghi dữ liệu có gắn phiên bản (Versioned Record)
#[derive(Clone, Debug, PartialEq)]
pub struct VersionedRecord {
    pub created_by_tx: TxId,         // Giao dịch tạo ra phiên bản này
    pub deleted_by_tx: Option<TxId>, // Giao dịch xoá/ghi đè phiên bản này (None nếu chưa)
    pub value: String,               // Dữ liệu thực tế
}

/// Lỗi xung đột ghi–ghi: giao dịch khác đã (hoặc đang) sửa cùng khoá
#[derive(Debug, PartialEq, Eq)]
pub enum MvccError {
    WriteConflict { key: String, other_tx: TxId },
}

/// Hệ thống lưu trữ dữ liệu đa phiên bản MVCC Store với Snapshot Isolation.
/// Mô hình đơn luồng: nhiều giao dịch chạy XEN KẼ nhau, mỗi thao tác là nguyên tử.
pub struct MvccStore {
    data: HashMap<String, Vec<VersionedRecord>>,
    statuses: HashMap<TxId, TxStatus>,
    /// Với mỗi giao dịch đang chạy: mã nhỏ nhất mà nó có thể KHÔNG nhìn thấy (xmin)
    active_xmins: HashMap<TxId, TxId>,
    next_tx_id: TxId,
}

impl MvccStore {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            statuses: HashMap::new(),
            active_xmins: HashMap::new(),
            next_tx_id: 1,
        }
    }

    fn status(&self, tx_id: TxId) -> TxStatus {
        // Mã không có trong bảng = chưa từng tồn tại -> coi như bị huỷ
        self.statuses
            .get(&tx_id)
            .copied()
            .unwrap_or(TxStatus::Aborted)
    }

    /// Khởi động một giao dịch mới: cấp mã và chụp ảnh các giao dịch đang chạy
    pub fn begin(&mut self) -> Transaction {
        let id = self.next_tx_id;
        self.next_tx_id += 1;
        let active: HashSet<TxId> = self.active_xmins.keys().copied().collect();
        let xmin = active.iter().copied().min().unwrap_or(id);
        self.statuses.insert(id, TxStatus::Active);
        self.active_xmins.insert(id, xmin);
        Transaction {
            id,
            snapshot: Snapshot { xmax: id, active },
        }
    }

    /// Giao dịch `tx` có nhìn thấy tác động của giao dịch `other` không?
    fn sees(&self, tx: &Transaction, other: TxId) -> bool {
        other == tx.id
            || (self.status(other) == TxStatus::Committed
                && other < tx.snapshot.xmax
                && !tx.snapshot.active.contains(&other))
    }

    /// QUY TẮC KHẢ KIẾN: phiên bản hiện ra với `tx` khi người tạo được `tx` nhìn thấy
    /// và lệnh xoá (nếu có) thì KHÔNG được `tx` nhìn thấy.
    fn is_visible(&self, tx: &Transaction, version: &VersionedRecord) -> bool {
        self.sees(tx, version.created_by_tx)
            && !version.deleted_by_tx.is_some_and(|d| self.sees(tx, d))
    }

    /// THAO TÁC ĐỌC CÔ LẬP THEO ẢNH CHỤP (Snapshot Read)
    pub fn read(&self, tx: &Transaction, key: &str) -> Option<&str> {
        self.data
            .get(key)?
            .iter()
            .rev()
            .find(|v| self.is_visible(tx, v))
            .map(|v| v.value.as_str())
    }

    /// Chỉ số của phiên bản "đầu" của khoá: phiên bản mới nhất không do giao dịch bị huỷ tạo ra
    fn head_index(&self, key: &str) -> Option<usize> {
        self.data
            .get(key)?
            .iter()
            .rposition(|v| self.status(v.created_by_tx) != TxStatus::Aborted)
    }

    /// Kiểm xung đột ghi–ghi trên phiên bản đầu của khoá. Ghi đè lên một thay đổi mà ta
    /// KHÔNG nhìn thấy (chưa commit, hoặc commit sau khi ta bắt đầu) chính là "lost update".
    /// Ta từ chối ngay lúc ghi (giống PostgreSQL ở mức REPEATABLE READ): nhờ vậy không bao giờ
    /// có hai giao dịch đồng thời cùng commit thay đổi trên một khoá (first-committer-wins).
    fn check_write_conflict(&self, tx: &Transaction, key: &str) -> Result<(), MvccError> {
        let Some(i) = self.head_index(key) else {
            return Ok(());
        };
        let head = &self.data[key][i];
        let conflict = |other: TxId| MvccError::WriteConflict {
            key: key.to_string(),
            other_tx: other,
        };
        if !self.sees(tx, head.created_by_tx) {
            return Err(conflict(head.created_by_tx));
        }
        if let Some(d) = head.deleted_by_tx
            && self.status(d) != TxStatus::Aborted
            && !self.sees(tx, d)
        {
            return Err(conflict(d));
        }
        Ok(())
    }

    /// THAO TÁC GHI TRONG GIAO DỊCH (Write): tạo phiên bản mới, đánh dấu phiên bản cũ
    pub fn write(&mut self, tx: &Transaction, key: &str, value: &str) -> Result<(), MvccError> {
        self.check_write_conflict(tx, key)?;
        let head = self.head_index(key);
        let head_dead = head.is_some_and(|i| {
            self.data[key][i]
                .deleted_by_tx
                .is_some_and(|d| self.status(d) != TxStatus::Aborted)
        });
        let versions = self.data.entry(key.to_string()).or_default();
        if let Some(i) = head {
            let h = &mut versions[i];
            if h.created_by_tx == tx.id && h.deleted_by_tx.is_none() {
                // Ghi lần hai trong cùng giao dịch: sửa luôn phiên bản riêng của mình
                h.value = value.to_string();
                return Ok(());
            }
            if !head_dead {
                // Phiên bản cũ bị "xoá" bởi giao dịch hiện tại (ghi đè dấu của tx đã huỷ nếu có)
                h.deleted_by_tx = Some(tx.id);
            }
        }
        versions.push(VersionedRecord {
            created_by_tx: tx.id,
            deleted_by_tx: None,
            value: value.to_string(),
        });
        Ok(())
    }

    /// THAO TÁC XOÁ: đánh dấu phiên bản đang thấy là bị xoá bởi giao dịch hiện tại
    pub fn delete(&mut self, tx: &Transaction, key: &str) -> Result<bool, MvccError> {
        self.check_write_conflict(tx, key)?;
        let Some(i) = self.head_index(key) else {
            return Ok(false);
        };
        if !self.is_visible(tx, &self.data[key][i]) {
            return Ok(false); // khoá đã bị xoá trong ảnh chụp của ta
        }
        if let Some(versions) = self.data.get_mut(key) {
            versions[i].deleted_by_tx = Some(tx.id);
        }
        Ok(true)
    }

    /// COMMIT: từ giờ mọi giao dịch BẮT ĐẦU SAU sẽ nhìn thấy thay đổi của `tx`
    pub fn commit(&mut self, tx: Transaction) {
        self.statuses.insert(tx.id, TxStatus::Committed);
        self.active_xmins.remove(&tx.id);
    }

    /// ABORT: chỉ đánh dấu trạng thái (giống PostgreSQL). Quy tắc khả kiến tự bỏ qua
    /// mọi phiên bản/dấu xoá của giao dịch bị huỷ; VACUUM sẽ dọn chúng sau.
    pub fn abort(&mut self, tx: Transaction) {
        self.statuses.insert(tx.id, TxStatus::Aborted);
        self.active_xmins.remove(&tx.id);
    }

    /// Dọn rác (Vacuum/Compaction): bỏ các phiên bản mà KHÔNG giao dịch nào —
    /// đang chạy hay sẽ bắt đầu — còn có thể nhìn thấy. Trả về số phiên bản đã dọn.
    pub fn vacuum(&mut self) -> usize {
        // Mọi giao dịch đã kết thúc với mã < horizon đều được MỌI giao dịch đang chạy nhìn thấy
        let horizon = self
            .active_xmins
            .values()
            .copied()
            .min()
            .unwrap_or(self.next_tx_id);
        let statuses = &self.statuses;
        let status = |id: TxId| statuses.get(&id).copied().unwrap_or(TxStatus::Aborted);
        let mut removed = 0;
        for versions in self.data.values_mut() {
            let before = versions.len();
            versions.retain(|v| {
                let creator_aborted = status(v.created_by_tx) == TxStatus::Aborted;
                let dead_for_everyone = v
                    .deleted_by_tx
                    .is_some_and(|d| status(d) == TxStatus::Committed && d < horizon);
                !creator_aborted && !dead_for_everyone
            });
            removed += before - versions.len();
            // Dấu xoá của giao dịch đã huỷ là vô nghĩa -> gỡ bỏ
            for v in versions.iter_mut() {
                if v.deleted_by_tx
                    .is_some_and(|d| status(d) == TxStatus::Aborted)
                {
                    v.deleted_by_tx = None;
                }
            }
        }
        self.data.retain(|_, v| !v.is_empty());
        removed
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

    let mut store = MvccStore::new();

    // 1. Dữ liệu ban đầu được nạp bởi một giao dịch khởi tạo (#1) và commit ngay
    let init = store.begin();
    assert_eq!(init.id, 1);
    store.write(&init, "account:A", "1000").unwrap();
    println!(
        "[1] Giao dịch #{}: Khởi tạo số dư tài khoản A = 1000 rồi COMMIT",
        init.id
    );
    store.commit(init);

    // 2. Kịch bản chạy đồng thời hai giao dịch:
    // - Giao dịch Đọc (#2): Bắt đầu kiểm toán báo cáo tài chính
    // - Giao dịch Ghi (#3): Khách hàng nạp thêm tiền vào tài khoản
    let reader = store.begin();
    let writer = store.begin();
    assert_eq!((reader.id, writer.id), (2, 3));
    println!("\n[2] Hai giao dịch đồng thời xuất hiện:");
    println!("    - Giao dịch Đọc khởi động: tx_id = {}", reader.id);
    println!("    - Giao dịch Ghi khởi động: tx_id = {}", writer.id);

    println!(
        "\n    -> Giao dịch Ghi #{} cập nhật tài khoản A thành 1500 (CHƯA commit)...",
        writer.id
    );
    store.write(&writer, "account:A", "1500").unwrap();

    // 3. Không có Dirty Read: chưa commit thì không ai khác thấy, kể cả giao dịch mới hơn
    println!("\n[3] Kiểm tra tính cô lập Snapshot Isolation:");
    let early = store.begin(); // #4 — bắt đầu khi #3 còn đang chạy
    println!(
        "    - Giao dịch Đọc #{} thấy: {:?}",
        reader.id,
        store.read(&reader, "account:A")
    );
    println!(
        "    - Giao dịch #{} (bắt đầu khi #{} chưa commit) thấy: {:?}",
        early.id,
        writer.id,
        store.read(&early, "account:A")
    );
    println!(
        "    - Chính Giao dịch Ghi #{} thấy thay đổi của mình: {:?}",
        writer.id,
        store.read(&writer, "account:A")
    );
    assert_eq!(store.read(&reader, "account:A"), Some("1000"));
    assert_eq!(store.read(&early, "account:A"), Some("1000"));
    assert_eq!(store.read(&writer, "account:A"), Some("1500"));

    store.commit(writer);
    // Repeatable Read: dù #3 đã commit, ảnh chụp của #2 và #4 không đổi
    assert_eq!(store.read(&reader, "account:A"), Some("1000"));
    assert_eq!(store.read(&early, "account:A"), Some("1000"));
    let late = store.begin(); // #5 — bắt đầu sau khi #3 commit
    println!(
        "    - Sau khi #3 commit: #{} vẫn thấy {:?}, giao dịch mới #{} thấy {:?}",
        reader.id,
        store.read(&reader, "account:A"),
        late.id,
        store.read(&late, "account:A")
    );
    assert_eq!(store.read(&late, "account:A"), Some("1500"));
    println!(
        "    => Người đọc không bị người ghi chặn, và không bao giờ thấy dữ liệu chưa commit!"
    );
    store.commit(reader);
    store.commit(early);
    store.commit(late);

    // 4. Chống Lost Update: hai giao dịch cùng đọc 1500 rồi cùng cộng thêm tiền
    println!("\n[4] Chống mất cập nhật (Lost Update) — first-committer-wins:");
    let t_a = store.begin();
    let t_b = store.begin();
    let seen_a: i64 = store.read(&t_a, "account:A").unwrap().parse().unwrap();
    let seen_b: i64 = store.read(&t_b, "account:A").unwrap().parse().unwrap();
    store
        .write(&t_a, "account:A", &(seen_a + 100).to_string())
        .unwrap();
    let result_b = store.write(&t_b, "account:A", &(seen_b + 200).to_string());
    println!("    - #{} ghi 1600: thành công", t_a.id);
    println!("    - #{} ghi 1700: {:?}", t_b.id, result_b);
    assert!(matches!(result_b, Err(MvccError::WriteConflict { .. })));
    store.commit(t_a);
    store.abort(t_b);
    // Giao dịch bị từ chối thử lại với ảnh chụp MỚI
    let retry = store.begin();
    let seen: i64 = store.read(&retry, "account:A").unwrap().parse().unwrap();
    store
        .write(&retry, "account:A", &(seen + 200).to_string())
        .unwrap();
    store.commit(retry);
    let check = store.begin();
    println!(
        "    - Sau khi thử lại: số dư = {:?} (không mất khoản nạp nào)",
        store.read(&check, "account:A")
    );
    assert_eq!(store.read(&check, "account:A"), Some("1800"));
    store.commit(check);

    // 5. Dọn rác: không còn giao dịch nào đang chạy -> mọi phiên bản cũ đều là rác
    println!("\n[5] Kiểm thử dọn rác các phiên bản dữ liệu cũ (Vacuum):");
    let removed = store.vacuum();
    println!("    - Đã dọn dẹp {} phiên bản cũ/bị huỷ!", removed);
    // 1000, 1500, 1600 đã bị ghi đè = 3 phiên bản rác; còn lại 1800.
    // (Bản 1700 của giao dịch bị xung đột bị từ chối ngay lúc ghi nên chưa từng tồn tại.)
    assert_eq!(removed, 3);

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 35               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi lập trình hệ thống giao dịch đồng thời và MVCC trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0502** | `cannot borrow '*store' as mutable because it is also borrowed as immutable` | Bạn đang giữ kết quả tham chiếu mượn của hàm `read()` (`let val = store.read(&tx, ...)`, kiểu `Option<&str>` mượn từ `store`) nhưng lại gọi `store.write(...)` làm thay đổi kho. | Sao chép giá trị chuỗi `.to_string()` hoặc kết thúc phạm vi mượn đọc trước khi thực hiện ghi dữ liệu. |
| **E0382** | `borrow of moved value: 'versions'` | Bạn di chuyển quyền sở hữu của vector phiên bản trong vòng lặp (`for v in versions`) thay vì duyệt qua tham chiếu mượn, rồi lại dùng `versions` sau vòng lặp. | Dùng `.iter()` hoặc `.iter_mut()` khi duyệt qua các phiên bản để tránh di chuyển quyền sở hữu (ownership). |
| **E0594** | `cannot assign to 'v.deleted_by_tx', which is behind a '&' reference` | Bạn cố thay đổi trường `deleted_by_tx` trong khi đang duyệt bằng iterator bất biến `.iter()`. | Chuyển sang sử dụng phương thức `.iter_mut()`. |
| **E0382** | `borrow of moved value: 'tx'` | Bạn gọi `store.commit(tx)` rồi lại `store.read(&tx, ...)`. `commit` nhận `Transaction` theo giá trị nên `tx` đã bị di chuyển — đây là lỗi **có chủ đích**: giao dịch đã kết thúc thì không được đọc/ghi nữa. | Bắt đầu giao dịch mới bằng `store.begin()`. |

### Ví dụ phân tích lỗi `E0502` khi vừa đọc vừa ghi trong MVCC:

```rust
// Đoạn mã lỗi minh họa E0502: Xung đột mượn đọc và mượn ghi
fn broken_mvcc(store: &mut MvccStore, tx: &Transaction) {
    // let result = store.read(tx, "key");       // Mượn bất biến store
    // store.write(tx, "key", "new_value");      // LỖI E0502: Mượn khả biến store khi đang bị mượn đọc!
    // println!("Đã đọc: {:?}", result);
    let _ = (store, tx);
}

// Cách sửa chữa đúng chuẩn: Chuyển dữ liệu mượn thành kiểu sở hữu độc lập
fn correct_mvcc(store: &mut MvccStore, tx: &Transaction) -> Result<(), MvccError> {
    // Bước 1: Sao chép kết quả ra biến String độc lập
    let result = store.read(tx, "key").map(|s| s.to_string());

    // Bước 2: Tự do thực hiện thao tác ghi mà không vi phạm quy tắc mượn
    store.write(tx, "key", "new_value")?;

    println!("Dữ liệu đọc trước đó: {:?}", result);
    Ok(())
}
```

---

## Kiểm thử tự động (Automated Tests)

Kiểm soát đồng thời là nơi lỗi **không** lộ ra khi chạy thử một luồng đơn giản: phiên bản trước của chương này chỉ so `created_by_tx <= current`, và chương trình mẫu vẫn chạy "đúng" — trong khi nó cho phép Dirty Read và Lost Update. Mỗi test dưới đây dựng đúng một kịch bản xen kẽ giữa các giao dịch và khẳng định điều Snapshot Isolation hứa hẹn.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn store_with(key: &str, value: &str) -> MvccStore {
        let mut store = MvccStore::new();
        let t = store.begin();
        store.write(&t, key, value).unwrap();
        store.commit(t);
        store
    }

    #[test]
    fn no_dirty_read_even_for_younger_transactions() {
        // Lỗi cũ: chỉ so created_by_tx <= current nên giao dịch có mã LỚN HƠN
        // đọc được phiên bản CHƯA commit của giao dịch có mã nhỏ hơn.
        let mut store = store_with("k", "old");
        let writer = store.begin();
        store.write(&writer, "k", "uncommitted").unwrap();
        let younger = store.begin();
        assert_eq!(store.read(&younger, "k"), Some("old"));
        store.abort(writer);
        assert_eq!(store.read(&younger, "k"), Some("old"));
    }

    #[test]
    fn repeatable_read_within_snapshot() {
        let mut store = store_with("k", "v1");
        let reader = store.begin();
        assert_eq!(store.read(&reader, "k"), Some("v1"));
        let writer = store.begin();
        store.write(&writer, "k", "v2").unwrap();
        store.commit(writer);
        assert_eq!(
            store.read(&reader, "k"),
            Some("v1"),
            "ảnh chụp không được đổi"
        );
        let after = store.begin();
        assert_eq!(store.read(&after, "k"), Some("v2"));
    }

    #[test]
    fn transaction_active_at_begin_stays_invisible_after_commit() {
        // #2 bắt đầu trước #3, nhưng commit SAU khi #3 bắt đầu -> #3 không được thấy
        let mut store = MvccStore::new();
        let t2 = store.begin();
        let t3 = store.begin();
        store.write(&t2, "k", "from_t2").unwrap();
        store.commit(t2);
        assert_eq!(store.read(&t3, "k"), None);
    }

    #[test]
    fn lost_update_is_prevented() {
        let mut store = store_with("counter", "0");
        let a = store.begin();
        let b = store.begin();
        store.write(&a, "counter", "1").unwrap();
        // b ghi đè thay đổi chưa commit của a -> xung đột
        assert_eq!(
            store.write(&b, "counter", "1"),
            Err(MvccError::WriteConflict {
                key: "counter".into(),
                other_tx: a.id
            })
        );
        store.commit(a);
        // kể cả sau khi a đã commit, b vẫn không được ghi đè (a commit sau ảnh chụp của b)
        assert!(store.write(&b, "counter", "1").is_err());
        store.abort(b);
        let c = store.begin();
        store.write(&c, "counter", "2").unwrap();
        store.commit(c);
        let d = store.begin();
        assert_eq!(store.read(&d, "counter"), Some("2"));
    }

    #[test]
    fn aborted_writes_and_deletes_are_invisible() {
        let mut store = store_with("k", "keep");
        let t = store.begin();
        store.write(&t, "k", "discard").unwrap();
        store.write(&t, "k", "discard again").unwrap(); // ghi hai lần trong cùng giao dịch
        assert_eq!(store.read(&t, "k"), Some("discard again"));
        store.abort(t);
        let t = store.begin();
        assert!(store.delete(&t, "k").unwrap());
        assert_eq!(store.read(&t, "k"), None);
        store.abort(t);
        let after = store.begin();
        assert_eq!(store.read(&after, "k"), Some("keep"));
        // và giao dịch sau vẫn ghi được (không bị dấu vết của tx đã huỷ chặn)
        store.write(&after, "k", "new").unwrap();
        store.commit(after);
    }

    #[test]
    fn committed_delete_hides_key_for_later_snapshots_only() {
        let mut store = store_with("k", "v");
        let old_reader = store.begin();
        let deleter = store.begin();
        assert!(store.delete(&deleter, "k").unwrap());
        store.commit(deleter);
        assert_eq!(store.read(&old_reader, "k"), Some("v"));
        let new_reader = store.begin();
        assert_eq!(store.read(&new_reader, "k"), None);
    }

    #[test]
    fn vacuum_keeps_versions_needed_by_active_snapshots() {
        let mut store = store_with("k", "v1");
        let old_reader = store.begin();
        let w = store.begin();
        store.write(&w, "k", "v2").unwrap();
        store.commit(w);
        assert_eq!(store.vacuum(), 0, "old_reader vẫn cần v1");
        assert_eq!(store.read(&old_reader, "k"), Some("v1"));
        store.commit(old_reader);
        assert_eq!(store.vacuum(), 1);
        let t = store.begin();
        assert_eq!(store.read(&t, "k"), Some("v2"));
    }

    #[test]
    fn transaction_ids_are_unique_and_increasing() {
        let mut store = MvccStore::new();
        let ids: Vec<TxId> = (0..5)
            .map(|_| {
                let t = store.begin();
                let id = t.id;
                store.commit(t);
                id
            })
            .collect();
        assert_eq!(ids, vec![1, 2, 3, 4, 5]);
    }
}
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Tiêu chuẩn ACID**: Là nền móng bảo đảm tính toàn vẹn và độ tin cậy của mọi hệ thống dữ liệu; đảm bảo các giao dịch diễn ra nguyên tử, nhất quán, cô lập và bền vững vĩnh viễn.
2. **Triết lý MVCC đỉnh cao**: Bằng cách lưu trữ nhiều phiên bản kèm dấu mốc thời gian giao dịch (`tx_id`), MVCC triệt tiêu việc khóa bảng, giúp người đọc và người ghi không bao giờ cản trở lẫn nhau.
3. **Quy tắc khả kiến (Visibility)**: Một giao dịch chỉ nhìn thấy các phiên bản do giao dịch **đã commit trước khi nó bắt đầu** (hoặc do chính nó) tạo ra, và chưa bị một giao dịch như thế xoá. So sánh mã giao dịch thôi là chưa đủ — phải biết trạng thái commit và tập giao dịch đang chạy lúc chụp ảnh. Ghi đè lên thay đổi mình không nhìn thấy phải bị từ chối (chống Lost Update).
4. **Cộng sinh hoàn hảo với LSM-Tree**: Tính chất bất biến (Immutable) của các tệp `SSTable` trong LSM-Tree biến nó thành động cơ tự nhiên tối ưu nhất để triển khai MVCC.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phân tích kịch bản chuyển tiền ACID)**:  
   Giao dịch $T_1$ chuyển 200 nghìn từ tài khoản A sang tài khoản B gồm hai bước: `A = A - 200` và `B = B + 200`. Nếu máy tính sập nguồn ngay sau khi bước 1 hoàn thành, thuộc tính ACID nào sẽ đảm bảo tài khoản A không bị mất oan 200 nghìn? Quy trình khôi phục diễn ra như thế nào?
2. **Bài tập 2 (Hoàn tác vật lý — Rollback trong MVCC)**:  
   Phương thức `abort` của `MvccStore` chỉ đánh dấu giao dịch là `Aborted` (giống PostgreSQL): các phiên bản nó tạo ra vẫn nằm đó, chỉ bị quy tắc khả kiến bỏ qua, chờ `vacuum` dọn. Hãy thiết kế thêm phương thức `fn rollback(&mut self, tx: Transaction)` hoàn tác **ngay lập tức**: đánh dấu giao dịch bị huỷ, xóa mọi phiên bản có `created_by_tx == tx.id`, đồng thời khôi phục các phiên bản cũ bị đánh dấu `deleted_by_tx == Some(tx.id)` về trạng thái `None`.
3. **Bài tập 3 (Tư duy mở rộng)**:  
   Trong các hệ quản trị cơ sở dữ liệu lớn như PostgreSQL, hiện tượng gì sẽ xảy ra nếu một giao dịch đọc kéo dài hàng tuần lễ mà không chịu đóng lại (`commit`/`abort`)? Giao dịch này sẽ gây ảnh hưởng tiêu cực như thế nào đến tiến trình dọn rác (Vacuum / Compaction) của MVCC?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Bốn chữ cái ACID: Atomicity, Consistency, Isolation, Durability. Hỏi: chữ nào hứa rằng *một nửa giao dịch* không bao giờ được coi là đã xong?
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

**Thuộc tính bảo vệ A là Nguyên tử (Atomicity)** — chữ *A* trong ACID.

Nguyên tử hứa: giao dịch **hoặc xong hẳn, hoặc như chưa từng bắt đầu**. Không có trạng thái giữa chừng. Sập nguồn sau bước 1 tạo ra đúng cái trạng thái giữa chừng đó, và Nguyên tử là thứ bảo đảm nó không được phép tồn tại.

**Quy trình khôi phục, theo đúng thứ tự:**

```
1. Khởi động lại -> động cơ đọc WAL từ điểm kiểm tra gần nhất
2. Với mỗi giao dịch trong WAL, hỏi: có bản ghi COMMIT không?
       CÓ    -> REDO: áp dụng lại mọi thay đổi (bảo đảm chữ D — Bền vững)
       KHÔNG -> UNDO: hoàn tác mọi thay đổi đã ghi (bảo đảm chữ A)
3. T1 có "A = A - 200" nhưng KHÔNG có COMMIT
       -> UNDO: A được trả về giá trị cũ
4. Cơ sở dữ liệu mở cửa trở lại, A nguyên vẹn
```

**Vì sao WAL biết đường hoàn tác:** vì nó ghi cả **ảnh cũ** lẫn **ảnh mới** của mỗi thay đổi:

```
LSN 101  T1  BEGIN
LSN 102  T1  UPDATE A: cũ=1000, mới=800
         <-- SẬP NGUỒN TẠI ĐÂY. Không có COMMIT.
```

Khi khôi phục, thấy T1 không commit, động cơ ghi lại `A = 1000` từ trường "cũ". Đây là **hoàn tác vật lý**, và nó cần WAL đã nằm chắc trên đĩa **trước khi** trang dữ liệu bị sửa — nguyên tắc *write-ahead* mà cái tên WAL nói tới.

Ba chữ còn lại đóng vai khác: **C** (Nhất quán) bảo đảm ràng buộc như "số dư không âm" vẫn đúng sau giao dịch; **I** (Cô lập) bảo đảm giao dịch khác không nhìn thấy trạng thái nửa vời của T1 — đó chính là việc MVCC làm; **D** (Bền vững) bảo đảm giao dịch **đã commit** thì không mất dù sập nguồn.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Hoàn tác cần làm hai việc ngược nhau: **bỏ** những bản ghi giao dịch này tạo ra, và **hồi sinh** những bản ghi nó đã đánh dấu là xoá.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
impl MvccStore {
    /// Hoàn tác NGAY một giao dịch chưa commit.
    /// HAI việc ngược nhau, đều bắt buộc:
    ///   1. bỏ những phiên bản do tx này TẠO RA
    ///   2. hồi sinh những phiên bản do tx này ĐÁNH DẤU XOÁ
    pub fn rollback(&mut self, tx: Transaction) {
        let tx_id = tx.id;
        self.abort(tx); // trạng thái Aborted + rời danh sách đang chạy
        for versions in self.data.values_mut() {
            // 1. Bỏ phiên bản do chính tx này tạo — chúng chưa từng hiện ra
            //    với ai khác, nên bỏ đi là an toàn tuyệt đối.
            versions.retain(|v| v.created_by_tx != tx_id);

            // 2. Gỡ dấu xoá: bản ghi cũ phải sống lại nguyên trạng.
            for v in versions.iter_mut() {
                if v.deleted_by_tx == Some(tx_id) {
                    v.deleted_by_tx = None;
                }
            }
        }
        // Khoá nào không còn phiên bản nào thì bỏ luôn, tránh rác.
        self.data.retain(|_, v| !v.is_empty());
    }
}

#[test]
fn rollback_restores_state_before_tx() {
    let mut store = MvccStore::new();
    let t1 = store.begin();
    store.write(&t1, "a", "original").unwrap();
    store.commit(t1);

    let t2 = store.begin();
    store.write(&t2, "a", "changed_by_t2").unwrap(); // tạo phiên bản mới
    assert_eq!(store.read(&t2, "a"), Some("changed_by_t2"));
    let t2_id = t2.id;
    store.rollback(t2);

    let t3 = store.begin();
    assert_eq!(store.read(&t3, "a"), Some("original"),
               "hoàn tác phải trả về đúng giá trị trước giao dịch");
    // Không còn dấu vết vật lý nào của t2
    assert!(store.data["a"].iter()
        .all(|v| v.created_by_tx != t2_id && v.deleted_by_tx != Some(t2_id)));

    // Hoàn tác một tx chưa từng chạm gì -> không được làm hỏng gì.
    let t4 = store.begin();
    store.rollback(t4);
    assert_eq!(store.read(&t3, "a"), Some("original"));
}
```

**Vì sao bước 2 dễ quên nhưng chí mạng:** nếu chỉ làm bước 1, bản ghi cũ vẫn mang dấu `deleted_by_tx = Some(T2)`. Trong store này quy tắc khả kiến tạm thời cứu bạn (T2 có trạng thái `Aborted` nên dấu xoá bị bỏ qua) — nhưng chỉ cần một tối ưu "vô hại" như xoá trạng thái của các giao dịch cũ khỏi bảng `statuses` cho đỡ tốn RAM, hay mã giao dịch bị tái sử dụng sau khi quấn vòng (PostgreSQL dùng mã 32 bit), là bản ghi đó **vĩnh viễn không đọc được nữa**. Dữ liệu không mất khỏi đĩa, nhưng mất khỏi tầm nhìn — dạng mất dữ liệu tệ nhất vì nó im lặng.

**Vì sao hoàn tác trong MVCC rẻ:** nó không đụng gì tới dữ liệu *cũ*. Giao dịch chưa commit chỉ ghi vào phiên bản riêng của nó, nên hoàn tác chỉ là vứt phiên bản đó đi. So với hoàn tác kiểu ghi-đè-tại-chỗ — nơi bạn phải đọc ảnh cũ từ WAL rồi ghi ngược lại — thì đây gần như miễn phí.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Dọn rác MVCC chỉ được xoá phiên bản mà **không giao dịch nào còn có thể nhìn thấy**. Một giao dịch mở từ tuần trước vẫn có quyền nhìn thấy trạng thái của tuần trước.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Hiện tượng: cơ sở dữ liệu phình to không kiểm soát, và mọi truy vấn chậm dần.**

MVCC giữ nhiều phiên bản của cùng một dòng. Dọn rác (`VACUUM` trong Postgres) chỉ được bỏ một phiên bản khi **chắc chắn không giao dịch đang chạy nào còn có thể nhìn thấy nó** — nghĩa là khi nó cũ hơn giao dịch đang mở lâu đời nhất.

```
Giao dịch T_cu mở lúc 09:00 thứ Hai, KHÔNG BAO GIỜ ĐÓNG.

Suốt tuần, hàng triệu dòng được cập nhật.
Mỗi cập nhật để lại một phiên bản cũ.

VACUUM chạy, nhìn thấy T_cu vẫn mở, và kết luận:
    "T_cu có thể vẫn cần đọc trạng thái lúc 09:00 thứ Hai
     -> KHÔNG được xoá bất cứ phiên bản nào tạo sau thời điểm đó"

=> VACUUM chạy đều đặn mà không thu hồi được một byte nào.
```

**Hậu quả dây chuyền:**

| | Hệ quả |
|---|---|
| Bảng phình xác | Một bảng 10 GB có thể thành 100 GB toàn phiên bản chết |
| Quét bảng chậm dần | Đọc 100 GB để lấy 10 GB dữ liệu sống |
| Chỉ mục cũng phình | Mỗi phiên bản cần một mục chỉ mục riêng |
| Bộ đệm mất tác dụng | Buffer pool chứa đầy phiên bản chết |
| Nguy cơ quấn vòng ID giao dịch | Postgres dùng ID 32 bit; không dọn được thì tới ngưỡng nó **dừng nhận ghi** để tự bảo vệ |

Cái cuối là kịch bản tệ nhất và có thật: một giao dịch bị quên có thể khiến cả cụm cơ sở dữ liệu ngừng nhận ghi.

**Vì vậy trong thực tế:** đặt `idle_in_transaction_session_timeout` để tự giết phiên bỏ quên; theo dõi `pg_stat_activity` tìm giao dịch mở lâu; và **không bao giờ mở giao dịch rồi đi làm việc khác** — mở muộn nhất có thể, đóng sớm nhất có thể. Một giao dịch chỉ đọc mà mở suốt phiên làm việc của người dùng là mẫu thiết kế sai kinh điển.
</details>
