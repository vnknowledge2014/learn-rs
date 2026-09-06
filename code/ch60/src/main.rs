#![allow(dead_code, unused_variables)]
//! Chương 60 — Khoa học máy tính: Quy hoạch động, Quay lui, Tham lam, Lý thuyết số.
//! Theo tinh thần TheAlgorithms/Rust và Rusty-CS, giải các bài LeetCode kinh điển.


// ============================================================================
// 1. QUY HOẠCH ĐỘNG (Dynamic Programming) — ghi nhớ để không tính lại
// ============================================================================

/// Fibonacci: minh họa vì sao QHĐ cần thiết.
/// Bản đệ quy ngây thơ là O(2^n) — tính lại cùng một giá trị hàng triệu lần.
pub fn fib_naive(n: u64) -> u64 {
    if n < 2 { n } else { fib_naive(n - 1) + fib_naive(n - 2) }
}

/// Bản QHĐ từ dưới lên: O(n) thời gian, O(1) không gian.
pub fn fib_qhd(n: u64) -> u64 {
    if n < 2 { return n; }
    let (mut a, mut b) = (0u64, 1u64);
    for _ in 2..=n {
        let c = a + b;
        a = b;
        b = c;
    }
    b
}

/// Bài toán "đổi tiền" (Coin Change): số đồng xu ÍT NHẤT để đủ số tiền.
/// QHĐ kinh điển — LeetCode 322.
pub fn swap_tien(cac_menh_gia: &[u64], so_tien: u64) -> Option<u64> {
    let n = so_tien as usize;
    let mut dp = vec![u64::MAX; n + 1];
    dp[0] = 0; // 0 đồng cần 0 xu
    for tien in 1..=n {
        for &xu in cac_menh_gia {
            let xu = xu as usize;
            if xu <= tien && dp[tien - xu] != u64::MAX {
                dp[tien] = dp[tien].min(dp[tien - xu] + 1);
            }
        }
    }
    if dp[n] == u64::MAX { None } else { Some(dp[n]) }
}

/// Dãy con chung dài nhất (Longest Common Subsequence) — LeetCode 1143.
/// Nền tảng của công cụ `diff` và tin sinh học (so sánh chuỗi DNA).
pub fn longest_common_subsequence(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let (m, n) = (a.len(), b.len());
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            dp[i][j] = if a[i - 1] == b[j - 1] {
                dp[i - 1][j - 1] + 1
            } else {
                dp[i - 1][j].max(dp[i][j - 1])
            };
        }
    }
    dp[m][n]
}

/// Ba lô 0/1 (0/1 Knapsack): giá trị lớn nhất trong giới hạn trọng lượng.
pub fn ba_lo(trong_luong: &[u64], value: &[u64], capacity: u64) -> u64 {
    let n = trong_luong.len();
    let w = capacity as usize;
    let mut dp = vec![0u64; w + 1];
    for i in 0..n {
        // duyệt NGƯỢC để mỗi món chỉ dùng 1 lần (0/1)
        for cap in (trong_luong[i] as usize..=w).rev() {
            dp[cap] = dp[cap].max(dp[cap - trong_luong[i] as usize] + value[i]);
        }
    }
    dp[w]
}

// ============================================================================
// 2. QUAY LUI (Backtracking) — thử, sai thì lùi lại
// ============================================================================

/// Sinh mọi hoán vị của một dãy — nền tảng của quay lui.
pub fn swap_pos<T: Clone>(cac_phan_tu: &[T]) -> Vec<Vec<T>> {
    let mut ket_qua = Vec::new();
    let mut current = Vec::new();
    let mut da_dung = vec![false; cac_phan_tu.len()];
    backtrack_permutations(cac_phan_tu, &mut da_dung, &mut current, &mut ket_qua);
    ket_qua
}
fn backtrack_permutations<T: Clone>(
    pt: &[T], da_dung: &mut [bool], current: &mut Vec<T>, kq: &mut Vec<Vec<T>>,
) {
    if current.len() == pt.len() {
        kq.push(current.clone());
        return;
    }
    for i in 0..pt.len() {
        if da_dung[i] { continue; }
        da_dung[i] = true;
        current.push(pt[i].clone());
        backtrack_permutations(pt, da_dung, current, kq);
        current.pop();        // LÙI LẠI
        da_dung[i] = false;    // bỏ đánh dấu
    }
}

/// Bài toán N quân hậu (N-Queens): đặt N hậu không quân nào ăn nhau. LeetCode 51.
pub fn n_hau(n: usize) -> usize {
    let mut cot = vec![false; n];
    let mut cheo_xuoi = vec![false; 2 * n];
    let mut cheo_nguoc = vec![false; 2 * n];
    set_suffix(0, n, &mut cot, &mut cheo_xuoi, &mut cheo_nguoc)
}
fn set_suffix(queue: usize, n: usize, cot: &mut [bool], cx: &mut [bool], cn: &mut [bool]) -> usize {
    if queue == n { return 1; }
    let mut num_way = 0;
    for c in 0..n {
        let d1 = queue + c;
        let d2 = queue + n - 1 - c;
        if cot[c] || cx[d1] || cn[d2] { continue; }
        cot[c] = true; cx[d1] = true; cn[d2] = true;
        num_way += set_suffix(queue + 1, n, cot, cx, cn);
        cot[c] = false; cx[d1] = false; cn[d2] = false; // LÙI LẠI
    }
    num_way
}

// ============================================================================
// 3. THAM LAM (Greedy) — chọn tối ưu cục bộ, hy vọng tối ưu toàn cục
// ============================================================================

/// Bài toán chọn hoạt động (Activity Selection): xếp nhiều cuộc họp nhất
/// vào một phòng không chồng giờ. Tham lam: luôn chọn cuộc KẾT THÚC SỚM NHẤT.
pub fn select_active(mut khoang: Vec<(u32, u32)>) -> usize {
    khoang.sort_by_key(|&(_, end)| end);
    let mut count = 0;
    let mut het_gio = 0;
    for (start, end) in khoang {
        if start >= het_gio {
            count += 1;
            het_gio = end;
        }
    }
    count
}

/// VÍ DỤ PHẢN CHỨNG: tham lam KHÔNG phải lúc nào cũng đúng.
/// Đổi tiền tham lam (luôn lấy mệnh giá lớn nhất) sai với mệnh giá [1,3,4], tiền=6:
/// tham lam cho 4+1+1=3 xu, nhưng tối ưu là 3+3=2 xu.
pub fn greedy_change(mut menh_gia: Vec<u64>, mut so_tien: u64) -> u64 {
    menh_gia.sort_by(|a, b| b.cmp(a)); // lớn nhất trước
    let mut count = 0;
    for xu in menh_gia {
        count += so_tien / xu;
        so_tien %= xu;
    }
    count
}

// ============================================================================
// 4. LÝ THUYẾT SỐ (Number Theory)
// ============================================================================

/// Ước chung lớn nhất — thuật toán Euclid, O(log min(a,b)).
pub fn ucln(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
/// Bội chung nhỏ nhất.
pub fn bcnn(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 { 0 } else { a / ucln(a, b) * b }
}

/// Sàng Eratosthenes: liệt kê mọi số nguyên tố tới n, O(n log log n).
pub fn sang_nguyen_to(n: usize) -> Vec<usize> {
    if n < 2 { return Vec::new(); }
    let mut la_nt = vec![true; n + 1];
    la_nt[0] = false;
    la_nt[1] = false;
    let mut i = 2;
    while i * i <= n {
        if la_nt[i] {
            let mut j = i * i;
            while j <= n {
                la_nt[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    (2..=n).filter(|&k| la_nt[k]).collect()
}

/// Lũy thừa modulo nhanh (fast modular exponentiation) — nền của mật mã RSA.
/// Tính (has_num^so_mu) % modulo trong O(log so_mu).
pub fn mod_pow(mut has_num: u64, mut so_mu: u64, modulo: u64) -> u64 {
    if modulo == 1 { return 0; }
    let mut kq = 1u64;
    has_num %= modulo;
    while so_mu > 0 {
        if so_mu & 1 == 1 {
            kq = (kq as u128 * has_num as u128 % modulo as u128) as u64;
        }
        so_mu >>= 1;
        has_num = (has_num as u128 * has_num as u128 % modulo as u128) as u64;
    }
    kq
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   KHOA HỌC MÁY TÍNH: QUY HOẠCH ĐỘNG · QUAY LUI · THAM LAM     ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n1. QUY HOẠCH ĐỘNG");
    println!("   Fibonacci(40): ngây thơ mất O(2^n), QHĐ = {}", fib_qhd(40));
    println!("   Đổi tiền [1,5,6,9] cho 11: {:?} xu (tối ưu)", swap_tien(&[1, 5, 6, 9], 11));
    println!("   LCS(\"ABCBDAB\", \"BDCAB\"): {}", longest_common_subsequence("ABCBDAB", "BDCAB"));
    println!("   Ba lô (tl=[1,3,4,5], gt=[1,4,5,7], sức chứa 7): {}",
             ba_lo(&[1, 3, 4, 5], &[1, 4, 5, 7], 7));

    println!("\n2. QUAY LUI");
    println!("   Số hoán vị của [1,2,3]: {}", swap_pos(&[1, 2, 3]).len());
    for n in [4, 5, 6, 8] {
        println!("   {} quân hậu: {} cách đặt", n, n_hau(n));
    }

    println!("\n3. THAM LAM");
    let hop = vec![(1, 3), (2, 5), (4, 7), (1, 8), (5, 9), (8, 10)];
    println!("   Xếp nhiều cuộc họp nhất: {} cuộc (=(1,3),(4,7),(8,10))", select_active(hop));
    println!("   ⚠ Đổi tiền THAM LAM [1,3,4] cho 6: {} xu (SAI!)", greedy_change(vec![1, 3, 4], 6));
    println!("     Đổi tiền QHĐ    [1,3,4] cho 6: {:?} xu (ĐÚNG)", swap_tien(&[1, 3, 4], 6));

    println!("\n4. LÝ THUYẾT SỐ");
    println!("   ƯCLN(48, 36) = {}, BCNN = {}", ucln(48, 36), bcnn(48, 36));
    println!("   Số nguyên tố < 30: {:?}", sang_nguyen_to(30));
    println!("   (7^256) mod 13 = {}", mod_pow(7, 256, 13));

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   NHẬN RA CẤU TRÚC BÀI TOÁN → CHỌN ĐÚNG KỸ THUẬT GIẢI          ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fib_both_methods_agree() {
        for n in 0..=20 {
            assert_eq!(fib_naive(n), fib_qhd(n), "lệch ở n={}", n);
        }
        assert_eq!(fib_qhd(50), 12586269025);
    }

    #[test]
    fn coin_change_dp() {
        assert_eq!(swap_tien(&[1, 5, 6, 9], 11), Some(2)); // 5+6
        assert_eq!(swap_tien(&[2], 3), None);              // không thể
        assert_eq!(swap_tien(&[1, 3, 4], 6), Some(2));     // 3+3
        assert_eq!(swap_tien(&[1, 2, 5], 0), Some(0));     // 0 tiền = 0 xu
    }

    #[test]
    fn greedy_coin_change_can_be_wrong() {
        // Đây là bằng chứng: tham lam KHÔNG tối ưu với mệnh giá [1,3,4]
        assert_eq!(greedy_change(vec![1, 3, 4], 6), 3); // 4+1+1
        assert_eq!(swap_tien(&[1, 3, 4], 6), Some(2));        // 3+3 -> QHĐ đúng
        assert!(greedy_change(vec![1, 3, 4], 6) as u64 > swap_tien(&[1, 3, 4], 6).unwrap());
    }

    #[test]
    fn lcs_is_correct() {
        assert_eq!(longest_common_subsequence("ABCBDAB", "BDCAB"), 4); // "BCAB" hoặc "BDAB"
        assert_eq!(longest_common_subsequence("abc", "abc"), 3);
        assert_eq!(longest_common_subsequence("abc", "xyz"), 0);
        assert_eq!(longest_common_subsequence("", "abc"), 0);
    }

    #[test]
    fn ba_lo_01() {
        assert_eq!(ba_lo(&[1, 3, 4, 5], &[1, 4, 5, 7], 7), 9); // món 3(gt4)+món4(gt5)? kiểm: 3+4=7 -> 4+5=9
        assert_eq!(ba_lo(&[2, 3], &[10, 20], 1), 0); // không món nào vừa
    }

    #[test]
    fn permutations_have_correct_count() {
        assert_eq!(swap_pos(&[1, 2, 3]).len(), 6);   // 3! = 6
        assert_eq!(swap_pos(&[1, 2, 3, 4]).len(), 24); // 4! = 24
        assert_eq!(swap_pos::<i32>(&[]).len(), 1);   // hoán vị của rỗng = 1 (dãy rỗng)
    }

    #[test]
    fn n_queens_matches_known_counts() {
        // Dãy số nghiệm N-Queens nổi tiếng: 1,0,0,2,10,4,40,92
        assert_eq!(n_hau(1), 1);
        assert_eq!(n_hau(4), 2);
        assert_eq!(n_hau(5), 10);
        assert_eq!(n_hau(6), 4);
        assert_eq!(n_hau(8), 92);
    }

    #[test]
    fn greedy_activity_selection_is_optimal() {
        // Tham lam theo kết thúc sớm nhất LÀ tối ưu cho bài này (đã chứng minh)
        let hop = vec![(1, 3), (2, 5), (4, 7), (1, 8), (5, 9), (8, 10)];
        assert_eq!(select_active(hop), 3); // (1,3),(4,7),(8,10)
    }

    #[test]
    fn number_theory() {
        assert_eq!(ucln(48, 36), 12);
        assert_eq!(ucln(17, 5), 1); // nguyên tố cùng nhau
        assert_eq!(bcnn(4, 6), 12);
        assert_eq!(sang_nguyen_to(20), vec![2, 3, 5, 7, 11, 13, 17, 19]);
        assert_eq!(sang_nguyen_to(1), Vec::<usize>::new());
    }

    #[test]
    fn mod_pow_is_correct() {
        assert_eq!(mod_pow(2, 10, 1000), 24);   // 1024 % 1000
        assert_eq!(mod_pow(3, 0, 7), 1);        // x^0 = 1
        assert_eq!(mod_pow(7, 256, 13), 9);
        // không tràn số dù số mũ lớn
        assert_eq!(mod_pow(123456789, 987654321, 1_000_000_007), 652541198);
    }
}
