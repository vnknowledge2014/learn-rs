// Tệp: src/main.rs
// Bộ công cụ Iterator đầy đủ: từ filter_map tới FromIterator

// Cố ý viết `fold(0, |a, b| a + b)` thay vì `sum()` để so sánh fold/rfold/reduce,
// nên tắt lint gợi ý rút gọn fold của clippy cho cả tệp.
#![allow(clippy::unnecessary_fold)]

use std::collections::{HashMap, HashSet};

// ============================================================================
// PHẦN 1: TỰ CÀI ĐẶT MỘT ITERATOR
// ============================================================================

/// Bộ đếm ngược: minh họa việc chỉ cần cài `next()` là có ngay hàng chục
/// phương thức miễn phí (map, filter, take, sum...).
pub struct Countdown {
    current: u32,
}

impl Countdown {
    pub fn new(start: u32) -> Self {
        Countdown { current: start }
    }
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.current == 0 {
            None
        } else {
            self.current -= 1;
            Some(self.current + 1)
        }
    }
}

// ============================================================================
// PHẦN 2: TỰ CÀI ĐẶT IntoIterator CHO KIỂU CỦA MÌNH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Cart {
    items: Vec<String>,
}

impl Cart {
    pub fn new(items: Vec<String>) -> Self {
        Cart { items }
    }
}

/// Nhờ trait này, `for x in cart` chạy được — đúng như với Vec.
impl IntoIterator for Cart {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Và nhờ trait này, `for x in &cart` cũng chạy được (chỉ mượn đọc).
impl<'a> IntoIterator for &'a Cart {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

/// Và nhờ FromIterator, `collect()` gom thẳng được vào Cart.
impl FromIterator<String> for Cart {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Cart {
            items: iter.into_iter().collect(),
        }
    }
}

// ============================================================================
// PHẦN 3: MIỀN DỮ LIỆU — NHẬT KÝ BÁN HÀNG THÔ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub id: String,
    pub region: String,
    pub amount: u64,
}

/// Phân tích một dòng thô "MÃ|KHU_VỰC|SỐ_TIỀN". Trả None nếu dòng hỏng.
pub fn parse_trade(line: &str) -> Option<Trade> {
    let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return None;
    }
    let amount = parts[2].parse::<u64>().ok()?;
    if parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some(Trade {
        id: parts[0].to_string(),
        region: parts[1].to_string(),
        amount,
    })
}

fn raw_data() -> Vec<&'static str> {
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

    let raw = raw_data();
    println!("\nDữ liệu thô: {} dòng (có cả dòng hỏng)", raw.len());

    // ------------------------------------------------------------------
    // 1. filter_map — LỌC VÀ BIẾN ĐỔI CÙNG LÚC
    // ------------------------------------------------------------------
    let trades: Vec<Trade> = raw.iter().filter_map(|d| parse_trade(d)).collect();
    println!(
        "\n1. filter_map: {} dòng hợp lệ / {} dòng thô",
        trades.len(),
        raw.len()
    );
    for g in trades.iter().take(3) {
        println!("   {:?}", g);
    }
    println!("   (đã dùng luôn `take(3)` để chỉ in 3 dòng đầu)");

    // ------------------------------------------------------------------
    // 2. any / all / find / position — ĐỀU NGẮN MẠCH
    // ------------------------------------------------------------------
    println!("\n2. any / all / find / position (đều dừng sớm)");
    println!(
        "   Có giao dịch nào > 2 triệu?     : {}",
        trades.iter().any(|g| g.amount > 2_000_000)
    );
    println!(
        "   Mọi giao dịch đều > 100 nghìn?  : {}",
        trades.iter().all(|g| g.amount > 100_000)
    );
    println!(
        "   Giao dịch đầu ở Đà Nẵng         : {:?}",
        trades.iter().find(|g| g.region == "Đà Nẵng").map(|g| &g.id)
    );
    println!(
        "   Vị trí giao dịch đầu ở TP.HCM   : {:?}",
        trades.iter().position(|g| g.region == "TP.HCM")
    );

    // ------------------------------------------------------------------
    // 3. min_by_key / max_by_key
    // ------------------------------------------------------------------
    println!("\n3. min_by_key / max_by_key");
    println!(
        "   Giao dịch nhỏ nhất: {:?}",
        trades
            .iter()
            .min_by_key(|g| g.amount)
            .map(|g| (&g.id, g.amount))
    );
    println!(
        "   Giao dịch lớn nhất: {:?}",
        trades
            .iter()
            .max_by_key(|g| g.amount)
            .map(|g| (&g.id, g.amount))
    );

    // ------------------------------------------------------------------
    // 4. partition — CHIA ĐÔI TRONG MỘT LƯỢT
    // ------------------------------------------------------------------
    let (large, small): (Vec<&Trade>, Vec<&Trade>) =
        trades.iter().partition(|g| g.amount >= 800_000);
    println!(
        "\n4. partition: {} đơn lớn (>=800k), {} đơn nhỏ",
        large.len(),
        small.len()
    );

    // ------------------------------------------------------------------
    // 5. fold / reduce / try_fold — BA KIỂU GỘP
    // ------------------------------------------------------------------
    println!("\n5. fold vs reduce vs try_fold");
    let sum_fold: u64 = trades.iter().map(|g| g.amount).fold(0, |a, b| a + b);
    let sum_reduce: Option<u64> = trades.iter().map(|g| g.amount).reduce(|a, b| a + b);
    println!("   fold  (có giá trị khởi tạo)  : {}", sum_fold);
    println!("   reduce(không có, trả Option) : {:?}", sum_reduce);

    let empty: Vec<u64> = Vec::new();
    println!(
        "   Trên danh sách RỖNG -> fold: {}, reduce: {:?}",
        empty.iter().fold(0u64, |a, b| a + b),
        empty.iter().copied().reduce(|a: u64, b: u64| a + b)
    );

    // try_fold: gộp CÓ THỂ THẤT BẠI, dừng ngay ở lỗi đầu tiên
    let safe: Option<u64> = trades.iter().try_fold(0u64, |a, g| a.checked_add(g.amount));
    println!("   try_fold (chống tràn số)     : {:?}", safe);
    let overflowed: Option<u64> = [u64::MAX, 1]
        .iter()
        .try_fold(0u64, |a, b| a.checked_add(*b));
    println!(
        "   try_fold khi tràn số         : {:?} (dừng ngay, không panic)",
        overflowed
    );

    // ------------------------------------------------------------------
    // 6. scan — GIỐNG fold NHƯNG NHẢ RA TỪNG BƯỚC TRUNG GIAN
    // ------------------------------------------------------------------
    let cumulative: Vec<u64> = trades
        .iter()
        .scan(0u64, |acc, g| {
            *acc += g.amount;
            Some(*acc)
        })
        .collect();
    println!("\n6. scan (tổng lũy kế từng bước): {:?}", cumulative);

    // ------------------------------------------------------------------
    // 7. take_while / skip_while — DỪNG SỚM, KHÁC HẲN filter
    // ------------------------------------------------------------------
    println!("\n7. take_while vs filter");
    let numbers = [1, 3, 5, 4, 7, 9];
    let tw: Vec<i32> = numbers.iter().copied().take_while(|x| x % 2 == 1).collect();
    let ft: Vec<i32> = numbers.iter().copied().filter(|x| x % 2 == 1).collect();
    println!("   dãy gốc              : {:?}", numbers);
    println!(
        "   take_while(lẻ)       : {:?}  ← DỪNG ngay khi gặp số chẵn đầu tiên",
        tw
    );
    println!(
        "   filter(lẻ)           : {:?}  ← duyệt HẾT, giữ mọi số lẻ",
        ft
    );
    let sw: Vec<i32> = numbers.iter().copied().skip_while(|x| x % 2 == 1).collect();
    println!("   skip_while(lẻ)       : {:?}", sw);

    // ------------------------------------------------------------------
    // 8. zip / unzip / chain / rev / step_by
    // ------------------------------------------------------------------
    println!("\n8. zip / unzip / chain / rev / step_by");
    let ids: Vec<&str> = trades.iter().map(|g| g.id.as_str()).collect();
    let amounts: Vec<u64> = trades.iter().map(|g| g.amount).collect();
    let zipped: Vec<(&&str, &u64)> = ids.iter().zip(amounts.iter()).take(3).collect();
    println!("   zip 3 cặp đầu : {:?}", zipped);

    let (ids_back, amounts_back): (Vec<&str>, Vec<u64>) =
        ids.iter().copied().zip(amounts.iter().copied()).unzip();
    println!(
        "   unzip tách lại: {} mã, {} số tiền",
        ids_back.len(),
        amounts_back.len()
    );

    let concat: Vec<i32> = (1..3).chain(10..12).collect();
    println!("   chain         : {:?}", concat);
    // CHÚ Ý: `rev()` đòi hỏi trait `DoubleEndedIterator` — iterator phải biết đi
    // từ CẢ HAI đầu. `Countdown` tự viết chỉ cài `Iterator` (một chiều), nên
    // `Countdown::new(5).rev()` KHÔNG biên dịch được:
    //     error[E0277]: the trait bound `Countdown: DoubleEndedIterator` is not satisfied
    // `Vec` thì có, nên ta gom lại trước rồi mới đảo:
    let inverse: Vec<u32> = Countdown::new(5)
        .collect::<Vec<u32>>()
        .into_iter()
        .rev()
        .collect();
    println!("   rev (cần DoubleEndedIterator): {:?}", inverse);
    let stepped: Vec<i32> = (0..10).step_by(3).collect();
    println!("   step_by(3)    : {:?}", stepped);

    // ------------------------------------------------------------------
    // 9. flat_map / flatten
    // ------------------------------------------------------------------
    println!("\n9. flat_map / flatten");
    let sentences = ["Rust rất nhanh", "và an toàn"];
    let words: Vec<&str> = sentences
        .iter()
        .flat_map(|c| c.split_whitespace())
        .collect();
    println!("   flat_map tách từ: {:?}", words);

    let nested: Vec<Vec<i32>> = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let flat: Vec<i32> = nested.into_iter().flatten().collect();
    println!("   flatten làm phẳng: {:?}", flat);

    let with_none: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
    let without_none: Vec<i32> = with_none.into_iter().flatten().collect();
    println!("   flatten bỏ None  : {:?}", without_none);

    // ------------------------------------------------------------------
    // 10. collect VÀO NHIỀU KIỂU KHÁC NHAU
    // ------------------------------------------------------------------
    println!("\n10. collect() gom vào nhiều kiểu đích");
    let text: String = ids.join(", ");
    println!("   -> String     : {}", text);

    let regions: HashSet<&str> = trades.iter().map(|g| g.region.as_str()).collect();
    let mut sorted_regions: Vec<&&str> = regions.iter().collect();
    sorted_regions.sort();
    println!(
        "   -> HashSet    : {:?} ({} khu vực)",
        sorted_regions,
        regions.len()
    );

    let table: HashMap<&str, u64> = trades.iter().map(|g| (g.id.as_str(), g.amount)).collect();
    println!(
        "   -> HashMap    : tra cứu GD-003 = {:?}",
        table.get("GD-003")
    );

    let all_ok: Result<Vec<i32>, _> = ["1", "2", "3"].iter().map(|s| s.parse::<i32>()).collect();
    let has_bad: Result<Vec<i32>, _> = ["1", "x", "3"].iter().map(|s| s.parse::<i32>()).collect();
    println!("   -> Result (ổn) : {:?}", all_ok);
    println!("   -> Result (hỏng): có lỗi = {}", has_bad.is_err());

    // ------------------------------------------------------------------
    // 11. TỔNG HỢP THEO NHÓM — MẪU DÙNG HẰNG NGÀY
    // ------------------------------------------------------------------
    println!("\n11. Tổng doanh thu theo khu vực (fold + entry API)");
    let by_region: HashMap<&str, u64> = trades.iter().fold(HashMap::new(), |mut table, g| {
        *table.entry(g.region.as_str()).or_insert(0) += g.amount;
        table
    });
    let mut pairs: Vec<(&&str, &u64)> = by_region.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (k, v) in pairs {
        println!("   {:<10} {:>10} đ", k, v);
    }

    // ------------------------------------------------------------------
    // 12. fold vs rfold — KHI THỨ TỰ CÓ Ý NGHĨA
    // ------------------------------------------------------------------
    println!("\n12. fold vs rfold");
    let m = [10i32, 3, 2];
    println!(
        "   Phép CỘNG (giao hoán)      : fold={}, rfold={}  -> GIỐNG nhau",
        m.iter().fold(0, |a, b| a + b),
        m.iter().rfold(0, |a, b| a + b)
    );
    let folded_left: String = m.iter().fold(String::new(), |a, b| a + &b.to_string());
    let folded_right: String = m.iter().rfold(String::new(), |a, b| a + &b.to_string());
    println!(
        "   NỐI CHUỖI (không giao hoán): fold={:?}, rfold={:?}  -> KHÁC nhau",
        folded_left, folded_right
    );
    println!("   → Trước khi song song hóa, phải biết phép gộp của mình có tính gì!");

    // ------------------------------------------------------------------
    // 13. ITERATOR TỰ VIẾT VÀ IntoIterator TỰ VIẾT
    // ------------------------------------------------------------------
    println!("\n13. Iterator và IntoIterator tự cài đặt");
    let count: Vec<u32> = Countdown::new(5).collect();
    println!("   Countdown::new(5)           : {:?}", count);
    println!(
        "   Miễn phí luôn map/filter/sum: {}",
        Countdown::new(100).filter(|x| x % 7 == 0).sum::<u32>()
    );

    let cart = Cart::new(vec!["Bàn phím".into(), "Chuột".into(), "Màn hình".into()]);
    print!("   for x in &cart -> ");
    for m in &cart {
        print!("[{}] ", m);
    }
    println!();

    let long_names: Cart = cart.into_iter().filter(|m| m.chars().count() > 5).collect(); // ← nhờ FromIterator tự cài
    println!("   collect() thẳng vào Cart    : {:?}", long_names);

    // ------------------------------------------------------------------
    // 14. Extend — NỐI THÊM VÀO TẬP HỢP ĐÃ CÓ
    // ------------------------------------------------------------------
    let mut extended: Vec<i32> = vec![1, 2];
    extended.extend(3..6);
    println!("\n14. Extend: {:?}", extended);

    println!("\n============================================================");
    println!("   MỘT `next()` — HÀNG CHỤC CÔNG CỤ MIỄN PHÍ ĐI KÈM         ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_map_skips_bad_lines() {
        let trades: Vec<Trade> = raw_data().iter().filter_map(|d| parse_trade(d)).collect();
        assert_eq!(trades.len(), 6, "9 dòng thô, 3 dòng hỏng -> còn 6");
    }

    #[test]
    fn take_while_differs_from_filter() {
        let numbers = [1, 3, 5, 4, 7, 9];
        let tw: Vec<i32> = numbers.iter().copied().take_while(|x| x % 2 == 1).collect();
        let ft: Vec<i32> = numbers.iter().copied().filter(|x| x % 2 == 1).collect();
        assert_eq!(tw, vec![1, 3, 5]); // dừng ở số 4
        assert_eq!(ft, vec![1, 3, 5, 7, 9]); // duyệt hết
    }

    #[test]
    fn reduce_returns_none_when_empty() {
        let empty: Vec<u64> = Vec::new();
        assert_eq!(empty.iter().copied().reduce(|a, b| a + b), None);
        assert_eq!(empty.iter().fold(0u64, |a, b| a + b), 0); // fold vẫn có câu trả lời
    }

    #[test]
    fn try_fold_stops_on_overflow() {
        let result: Option<u64> = [u64::MAX, 1, 2]
            .iter()
            .try_fold(0u64, |a, b| a.checked_add(*b));
        assert_eq!(result, None);
    }

    #[test]
    fn scan_emits_intermediate_steps() {
        let cumulative: Vec<i32> = [1, 2, 3, 4]
            .iter()
            .scan(0, |t, x| {
                *t += x;
                Some(*t)
            })
            .collect();
        assert_eq!(cumulative, vec![1, 3, 6, 10]);
    }

    #[test]
    fn fold_and_rfold_differ_only_when_non_commutative() {
        let m = [10i32, 3, 2];
        // Phép cộng GIAO HOÁN -> duyệt hai chiều cho cùng kết quả
        assert_eq!(
            m.iter().fold(0, |a, b| a + b),
            m.iter().rfold(0, |a, b| a + b)
        );
        // Nối chuỗi KHÔNG giao hoán -> duyệt hai chiều cho kết quả khác nhau
        let left: String = m.iter().fold(String::new(), |a, b| a + &b.to_string());
        let right: String = m.iter().rfold(String::new(), |a, b| a + &b.to_string());
        assert_eq!(left, "1032");
        assert_eq!(right, "2310");
        assert_ne!(left, right);
    }

    #[test]
    fn collect_targets_many_types() {
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
    fn partition_splits_into_two_groups() {
        let (evens, odds): (Vec<i32>, Vec<i32>) = (1..8).partition(|x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4, 6]);
        assert_eq!(odds, vec![1, 3, 5, 7]);
    }

    #[test]
    fn custom_iterator_works() {
        assert_eq!(Countdown::new(3).collect::<Vec<u32>>(), vec![3, 2, 1]);
        assert_eq!(Countdown::new(10).filter(|x| x % 3 == 0).sum::<u32>(), 18); // 9+6+3
    }

    #[test]
    fn custom_into_and_from_iterator() {
        let cart = Cart::new(vec!["Bàn phím".into(), "Chuột".into()]);
        let names: Vec<&String> = (&cart).into_iter().collect();
        assert_eq!(names.len(), 2);
        let filtered: Cart = cart.into_iter().filter(|m| m.chars().count() > 5).collect();
        assert_eq!(filtered, Cart::new(vec!["Bàn phím".into()]));
    }

    #[test]
    fn totals_by_region_are_correct() {
        let trades: Vec<Trade> = raw_data().iter().filter_map(|d| parse_trade(d)).collect();
        let by_region: HashMap<&str, u64> = trades.iter().fold(HashMap::new(), |mut b, g| {
            *b.entry(g.region.as_str()).or_insert(0) += g.amount;
            b
        });
        assert_eq!(by_region.get("Hà Nội"), Some(&1_570_000)); // 1250000 + 320000
        assert_eq!(by_region.get("Cần Thơ"), Some(&780_000));
    }
}
