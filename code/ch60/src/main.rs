//! Chương 60 — Khoa học máy tính: Quy hoạch động, Quay lui, Tham lam, Lý thuyết số.
//! Theo tinh thần TheAlgorithms/Rust và Rusty-CS, giải các bài LeetCode kinh điển.

// ============================================================================
// 1. QUY HOẠCH ĐỘNG (Dynamic Programming) — ghi nhớ để không tính lại
// ============================================================================

/// Fibonacci: minh họa vì sao QHĐ cần thiết.
/// Bản đệ quy ngây thơ là O(2^n) — tính lại cùng một giá trị hàng triệu lần.
pub fn fib_naive(n: u64) -> u64 {
    if n < 2 {
        n
    } else {
        fib_naive(n - 1) + fib_naive(n - 2)
    }
}

/// Bản QHĐ từ dưới lên: O(n) thời gian, O(1) không gian.
/// Lưu ý: fib(94) đã vượt `u64` — với n > 93 phép cộng tràn số (panic ở bản debug).
pub fn fib_dp(n: u64) -> u64 {
    if n < 2 {
        return n;
    }
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
pub fn coin_change(denominations: &[u64], amount: u64) -> Option<u64> {
    let n = amount as usize;
    let mut dp = vec![u64::MAX; n + 1];
    dp[0] = 0; // 0 đồng cần 0 xu
    for total in 1..=n {
        for &coin in denominations {
            let coin = coin as usize;
            if coin <= total && dp[total - coin] != u64::MAX {
                dp[total] = dp[total].min(dp[total - coin] + 1);
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
pub fn knapsack_01(weights: &[u64], values: &[u64], capacity: u64) -> u64 {
    let n = weights.len();
    let w = capacity as usize;
    let mut dp = vec![0u64; w + 1];
    for i in 0..n {
        // duyệt NGƯỢC để mỗi món chỉ dùng 1 lần (0/1)
        for cap in (weights[i] as usize..=w).rev() {
            dp[cap] = dp[cap].max(dp[cap - weights[i] as usize] + values[i]);
        }
    }
    dp[w]
}

// ============================================================================
// 2. QUAY LUI (Backtracking) — thử, sai thì lùi lại
// ============================================================================

/// Sinh mọi hoán vị của một dãy — nền tảng của quay lui.
pub fn permutations<T: Clone>(items: &[T]) -> Vec<Vec<T>> {
    let mut results = Vec::new();
    let mut current = Vec::new();
    let mut used = vec![false; items.len()];
    backtrack_permutations(items, &mut used, &mut current, &mut results);
    results
}
fn backtrack_permutations<T: Clone>(
    items: &[T],
    used: &mut [bool],
    current: &mut Vec<T>,
    results: &mut Vec<Vec<T>>,
) {
    if current.len() == items.len() {
        results.push(current.clone());
        return;
    }
    for i in 0..items.len() {
        if used[i] {
            continue;
        }
        used[i] = true;
        current.push(items[i].clone());
        backtrack_permutations(items, used, current, results);
        current.pop(); // LÙI LẠI
        used[i] = false; // bỏ đánh dấu
    }
}

/// Bài toán N quân hậu (N-Queens): đặt N hậu không quân nào ăn nhau. LeetCode 51.
pub fn n_queens(n: usize) -> usize {
    let mut cols = vec![false; n];
    let mut diag = vec![false; 2 * n];
    let mut anti_diag = vec![false; 2 * n];
    place_queens(0, n, &mut cols, &mut diag, &mut anti_diag)
}
fn place_queens(
    row: usize,
    n: usize,
    cols: &mut [bool],
    diag: &mut [bool],
    anti_diag: &mut [bool],
) -> usize {
    if row == n {
        return 1;
    }
    let mut ways = 0;
    for c in 0..n {
        let d1 = row + c;
        let d2 = row + n - 1 - c;
        if cols[c] || diag[d1] || anti_diag[d2] {
            continue;
        }
        cols[c] = true;
        diag[d1] = true;
        anti_diag[d2] = true;
        ways += place_queens(row + 1, n, cols, diag, anti_diag);
        cols[c] = false;
        diag[d1] = false;
        anti_diag[d2] = false; // LÙI LẠI
    }
    ways
}

// ============================================================================
// 3. THAM LAM (Greedy) — chọn tối ưu cục bộ, hy vọng tối ưu toàn cục
// ============================================================================

/// Bài toán chọn hoạt động (Activity Selection): xếp nhiều cuộc họp nhất
/// vào một phòng không chồng giờ. Tham lam: luôn chọn cuộc KẾT THÚC SỚM NHẤT.
pub fn select_activities(mut intervals: Vec<(u32, u32)>) -> usize {
    intervals.sort_by_key(|&(_, end)| end);
    let mut count = 0;
    let mut last_end = 0;
    for (start, end) in intervals {
        if start >= last_end {
            count += 1;
            last_end = end;
        }
    }
    count
}

/// VÍ DỤ PHẢN CHỨNG: tham lam KHÔNG phải lúc nào cũng đúng.
/// Đổi tiền tham lam (luôn lấy mệnh giá lớn nhất) sai với mệnh giá [1,3,4], tiền=6:
/// tham lam cho 4+1+1=3 xu, nhưng tối ưu là 3+3=2 xu.
pub fn greedy_change(mut denominations: Vec<u64>, mut amount: u64) -> Option<u64> {
    denominations.retain(|&d| d > 0); // mệnh giá 0 sẽ gây chia cho 0
    denominations.sort_by(|a, b| b.cmp(a)); // lớn nhất trước
    let mut count = 0;
    for coin in denominations {
        count += amount / coin;
        amount %= coin;
    }
    // Còn dư mà hết mệnh giá -> tham lam KHÔNG đổi được (dù có thể vẫn tồn tại cách đổi)
    (amount == 0).then_some(count)
}

// ============================================================================
// 4. LÝ THUYẾT SỐ (Number Theory)
// ============================================================================

/// Ước chung lớn nhất — thuật toán Euclid, O(log min(a,b)).
pub fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
/// Bội chung nhỏ nhất.
pub fn lcm(a: u64, b: u64) -> u64 {
    if a == 0 || b == 0 {
        0
    } else {
        a / gcd(a, b) * b
    }
}

/// Sàng Eratosthenes: liệt kê mọi số nguyên tố tới n, O(n log log n).
pub fn sieve_primes(n: usize) -> Vec<usize> {
    if n < 2 {
        return Vec::new();
    }
    let mut is_prime = vec![true; n + 1];
    is_prime[0] = false;
    is_prime[1] = false;
    let mut i = 2;
    while i * i <= n {
        if is_prime[i] {
            let mut j = i * i;
            while j <= n {
                is_prime[j] = false;
                j += i;
            }
        }
        i += 1;
    }
    (2..=n).filter(|&k| is_prime[k]).collect()
}

/// Lũy thừa modulo nhanh (fast modular exponentiation) — nền của mật mã RSA.
/// Tính (base^exp) % modulo trong O(log exp).
pub fn mod_pow(mut base: u64, mut exp: u64, modulo: u64) -> u64 {
    if modulo == 1 {
        return 0;
    }
    let mut results = 1u64;
    base %= modulo;
    while exp > 0 {
        if exp & 1 == 1 {
            results = (results as u128 * base as u128 % modulo as u128) as u64;
        }
        exp >>= 1;
        base = (base as u128 * base as u128 % modulo as u128) as u64;
    }
    results
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   KHOA HỌC MÁY TÍNH: QUY HOẠCH ĐỘNG · QUAY LUI · THAM LAM     ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n1. QUY HOẠCH ĐỘNG");
    println!(
        "   Fibonacci(40): ngây thơ mất O(2^n), QHĐ = {}",
        fib_dp(40)
    );
    println!(
        "   Đổi tiền [1,5,6,9] cho 11: {:?} xu (tối ưu)",
        coin_change(&[1, 5, 6, 9], 11)
    );
    println!(
        "   LCS(\"ABCBDAB\", \"BDCAB\"): {}",
        longest_common_subsequence("ABCBDAB", "BDCAB")
    );
    println!(
        "   Ba lô (tl=[1,3,4,5], gt=[1,4,5,7], sức chứa 7): {}",
        knapsack_01(&[1, 3, 4, 5], &[1, 4, 5, 7], 7)
    );

    println!("\n2. QUAY LUI");
    println!(
        "   Số hoán vị của [1,2,3]: {}",
        permutations(&[1, 2, 3]).len()
    );
    for n in [4, 5, 6, 8] {
        println!("   {} quân hậu: {} cách đặt", n, n_queens(n));
    }

    println!("\n3. THAM LAM");
    let meetings = vec![(1, 3), (2, 5), (4, 7), (1, 8), (5, 9), (8, 10)];
    println!(
        "   Xếp nhiều cuộc họp nhất: {} cuộc (=(1,3),(4,7),(8,10))",
        select_activities(meetings)
    );
    println!(
        "   ⚠ Đổi tiền THAM LAM [1,3,4] cho 6: {:?} xu (SAI!)",
        greedy_change(vec![1, 3, 4], 6)
    );
    println!(
        "     Đổi tiền QHĐ    [1,3,4] cho 6: {:?} xu (ĐÚNG)",
        coin_change(&[1, 3, 4], 6)
    );

    println!("\n4. LÝ THUYẾT SỐ");
    println!("   ƯCLN(48, 36) = {}, BCNN = {}", gcd(48, 36), lcm(48, 36));
    println!("   Số nguyên tố < 30: {:?}", sieve_primes(30));
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
            assert_eq!(fib_naive(n), fib_dp(n), "lệch ở n={}", n);
        }
        assert_eq!(fib_dp(50), 12586269025);
    }

    #[test]
    fn coin_change_dp() {
        assert_eq!(coin_change(&[1, 5, 6, 9], 11), Some(2)); // 5+6
        assert_eq!(coin_change(&[2], 3), None); // không thể
        assert_eq!(coin_change(&[1, 3, 4], 6), Some(2)); // 3+3
        assert_eq!(coin_change(&[1, 2, 5], 0), Some(0)); // 0 tiền = 0 xu
    }

    #[test]
    fn greedy_coin_change_can_be_wrong() {
        // Đây là bằng chứng: tham lam KHÔNG tối ưu với mệnh giá [1,3,4]
        assert_eq!(greedy_change(vec![1, 3, 4], 6), Some(3)); // 4+1+1
        assert_eq!(coin_change(&[1, 3, 4], 6), Some(2)); // 3+3 -> QHĐ đúng
        assert!(greedy_change(vec![1, 3, 4], 6) > coin_change(&[1, 3, 4], 6));
        // Tệ hơn: tham lam có thể bó tay dù cách đổi TỒN TẠI. [4,3] cho 6: lấy 4, dư 2 -> kẹt;
        // trong khi 3+3 = 6. (Bản cũ trả về 1 xu và lặng lẽ bỏ qua phần dư.)
        assert_eq!(greedy_change(vec![4, 3], 6), None);
        assert_eq!(coin_change(&[4, 3], 6), Some(2));
        // Mệnh giá 0 không làm chia cho 0
        assert_eq!(greedy_change(vec![0, 5], 10), Some(2));
    }

    #[test]
    fn lcs_is_correct() {
        assert_eq!(longest_common_subsequence("ABCBDAB", "BDCAB"), 4); // "BCAB" hoặc "BDAB"
        assert_eq!(longest_common_subsequence("abc", "abc"), 3);
        assert_eq!(longest_common_subsequence("abc", "xyz"), 0);
        assert_eq!(longest_common_subsequence("", "abc"), 0);
    }

    #[test]
    fn knapsack_01_is_correct() {
        assert_eq!(knapsack_01(&[1, 3, 4, 5], &[1, 4, 5, 7], 7), 9); // món 3(gt4)+món4(gt5)? kiểm: 3+4=7 -> 4+5=9
        assert_eq!(knapsack_01(&[2, 3], &[10, 20], 1), 0); // không món nào vừa
    }

    #[test]
    fn permutations_have_correct_count() {
        assert_eq!(permutations(&[1, 2, 3]).len(), 6); // 3! = 6
        assert_eq!(permutations(&[1, 2, 3, 4]).len(), 24); // 4! = 24
        assert_eq!(permutations::<i32>(&[]).len(), 1); // hoán vị của rỗng = 1 (dãy rỗng)
    }

    #[test]
    fn n_queens_matches_known_counts() {
        // Dãy số nghiệm N-Queens nổi tiếng: 1,0,0,2,10,4,40,92
        assert_eq!(n_queens(1), 1);
        assert_eq!(n_queens(4), 2);
        assert_eq!(n_queens(5), 10);
        assert_eq!(n_queens(6), 4);
        assert_eq!(n_queens(8), 92);
    }

    #[test]
    fn greedy_activity_selection_is_optimal() {
        // Tham lam theo kết thúc sớm nhất LÀ tối ưu cho bài này (đã chứng minh)
        let meetings = vec![(1, 3), (2, 5), (4, 7), (1, 8), (5, 9), (8, 10)];
        assert_eq!(select_activities(meetings), 3); // (1,3),(4,7),(8,10)
    }

    #[test]
    fn number_theory() {
        assert_eq!(gcd(48, 36), 12);
        assert_eq!(gcd(17, 5), 1); // nguyên tố cùng nhau
        assert_eq!(lcm(4, 6), 12);
        assert_eq!(sieve_primes(20), vec![2, 3, 5, 7, 11, 13, 17, 19]);
        assert_eq!(sieve_primes(1), Vec::<usize>::new());
    }

    #[test]
    fn mod_pow_is_correct() {
        assert_eq!(mod_pow(2, 10, 1000), 24); // 1024 % 1000
        assert_eq!(mod_pow(3, 0, 7), 1); // x^0 = 1
        assert_eq!(mod_pow(7, 256, 13), 9);
        // không tràn số dù số mũ lớn
        assert_eq!(mod_pow(123456789, 987654321, 1_000_000_007), 652541198);
    }
}
