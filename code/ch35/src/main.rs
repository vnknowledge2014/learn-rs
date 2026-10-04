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
    /// Ta từ chối NGAY lúc ghi (first-updater-wins, không chờ). PostgreSQL ở mức REPEATABLE READ
    /// thì CHỜ giao dịch kia kết thúc và chỉ báo lỗi nếu nó commit. Cả hai đều bảo đảm không bao giờ
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
