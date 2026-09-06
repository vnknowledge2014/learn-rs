# Chương 60: Khoa học máy tính — Quy hoạch động, Quay lui, Tham lam & Lý thuyết số (Algorithm Design Paradigms)

## Giới thiệu & Mục tiêu học tập

Chủ đề 5 (Chương 25–30) đã dạy *cấu trúc dữ liệu* và các thuật toán cơ bản. Nhưng phỏng vấn kỹ thuật ở các công ty lớn, các kỳ thi lập trình, và rất nhiều bài toán thực tế đòi hỏi tầng cao hơn: **các mô thức thiết kế thuật toán** (algorithm design paradigms). Đây là chương lấp đầy khoảng trống đó, theo tinh thần các kho [TheAlgorithms/Rust](https://github.com/TheAlgorithms/Rust), [LeetCode-in-Rust](https://github.com/LeetCode-in-Rust/LeetCode-in-Rust) và giáo trình [Rusty-CS](https://github.com/AbdesamedBendjeddou/Rusty-CS).

Điểm quan trọng nhất của chương này **không phải là thuộc lòng thuật toán**, mà là **nhận ra cấu trúc bài toán** để chọn đúng mô thức:

| Nếu bài toán... | Hãy nghĩ tới |
|---|---|
| có bài toán con **chồng lặp** và **cấu trúc con tối ưu** | **Quy hoạch động** |
| cần **thử mọi khả năng**, sai thì lùi | **Quay lui** |
| chọn **tối ưu cục bộ** dẫn tới tối ưu toàn cục | **Tham lam** |
| liên quan số nguyên tố, chia hết, modulo | **Lý thuyết số** |

Mục tiêu học tập:
- Hiểu **Quy hoạch động**: từ đệ quy O(2ⁿ) tới O(n) bằng cách *ghi nhớ*; giải đổi tiền, LCS, ba lô.
- Làm chủ **Quay lui**: mẫu "thử → đệ quy → lùi lại"; giải hoán vị và N quân hậu.
- Nắm **Tham lam** — và **biết khi nào nó SAI** (một bài học quan trọng hơn cả khi nó đúng).
- Cài các thuật toán **lý thuyết số** nền tảng: Euclid, sàng nguyên tố, lũy thừa modulo (nền của RSA).
- Rèn kỹ năng cốt lõi: **nhận diện mô thức** từ đặc điểm bài toán.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: BỐN CÁCH GIẢI QUYẾT VẤN ĐỀ TRONG ĐỜI                          │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│  QUY HOẠCH ĐỘNG = "GHI SẴN ĐÁP ÁN VÀO SỔ, KHỎI TÍNH LẠI"                        │
│     Tính tiền công cho từng ngày dựa trên ngày trước. Thay vì tính lại từ        │
│     đầu mỗi lần, bạn ghi kết quả từng ngày vào sổ và tra khi cần.                │
│                                                                                  │
│  QUAY LUI = "ĐI MÊ CUNG: THỬ MỘT LỐI, CỤT THÌ QUAY LẠI NGÃ RẼ"                  │
│     Rẽ trái. Cụt. Quay lại ngã ba, rẽ phải. Cụt. Quay lại nữa...                 │
│     Vét cạn mọi lối đi, nhưng bỏ ngay nhánh vừa biết là sai.                     │
│                                                                                  │
│  THAM LAM = "LUÔN CHỌN MIẾNG NGON NHẤT TRƯỚC MẮT"                                │
│     Trả tiền thừa: luôn đưa tờ mệnh giá lớn nhất trước. NHANH — nhưng            │
│     KHÔNG PHẢI LÚC NÀO CŨNG cho ít tờ nhất! (xem phản ví dụ trong chương)        │
│                                                                                  │
│  LÝ THUYẾT SỐ = "TÍNH CHẤT ẨN CỦA CON SỐ"                                        │
│     Vì sao đồng hồ 12 giờ + 5 giờ = 5 giờ chứ không phải 17? Đó là modulo.       │
│     Toàn bộ mật mã hiện đại đứng trên các tính chất này.                         │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Quy hoạch động — hai điều kiện cần

Một bài toán giải được bằng QHĐ khi có **cả hai** đặc điểm:
1. **Bài toán con chồng lặp (overlapping subproblems)**: cùng một bài toán con được tính lại nhiều lần. Fibonacci ngây thơ tính `fib(30)` hàng triệu lần.
2. **Cấu trúc con tối ưu (optimal substructure)**: lời giải tối ưu của bài lớn dựng được từ lời giải tối ưu của bài con.

Hai cách cài:
- **Ghi nhớ (memoization, top-down)**: đệ quy như thường, nhưng lưu kết quả vào `HashMap`/mảng để không tính lại.
- **Từ dưới lên (tabulation, bottom-up)**: điền bảng `dp` từ bài con nhỏ nhất lên. Thường tiết kiệm bộ nhớ hơn.

`fib_naive` (O(2ⁿ)) so với `fib_qhd` (O(n), O(1) bộ nhớ) trong mã dưới là minh họa rõ nhất: cùng công thức, khác cách tổ chức tính toán, chênh nhau *hàng tỷ lần* ở n=50.

### 2. Quay lui — "thử, đệ quy, LÙI LẠI"

Mọi thuật toán quay lui đều theo đúng một khuôn:

```
fn quay_lui(trạng_thái) {
    nếu (đủ điều kiện dừng) { ghi nhận lời giải; return; }
    cho mỗi lựa chọn hợp lệ {
        áp dụng lựa chọn;        // THỬ
        quay_lui(trạng thái mới); // ĐỆ QUY
        hoàn tác lựa chọn;        // LÙI LẠI  ← đây là "backtrack"
    }
}
```

Bước **lùi lại** (undo) là linh hồn của kỹ thuật: sau khi khám phá xong một nhánh, ta khôi phục trạng thái để thử nhánh khác. N quân hậu trong mã dưới đánh dấu cột và hai đường chéo bị chiếm, đệ quy, rồi *bỏ đánh dấu* — nhờ vậy một mảng trạng thái duy nhất phục vụ cả cây tìm kiếm.

Quay lui thường có độ phức tạp hàm mũ, nhưng **cắt tỉa (pruning)** — bỏ sớm nhánh không thể dẫn tới lời giải — làm nó khả thi trong thực tế.

### 3. Tham lam — và bài học khi nó SAI

Tham lam chọn tối ưu cục bộ ở mỗi bước. Nó nhanh và đơn giản, nhưng **chỉ đúng khi bài toán có "tính chất tham lam"** (greedy choice property). Nhiều bài toán *trông giống* nhưng tham lam lại sai.

Mã dưới đây chứng minh điều đó bằng test: với mệnh giá `[1, 3, 4]` và số tiền `6`:
- **Tham lam** (luôn lấy tờ lớn nhất): `4 + 1 + 1 = 3 tờ`.
- **Quy hoạch động** (tối ưu thật): `3 + 3 = 2 tờ`.

Đây là lý do vì sao *biết khi nào tham lam sai* quan trọng hơn biết khi nào nó đúng. Ngược lại, bài **chọn hoạt động** (xếp nhiều cuộc họp nhất) thì tham lam theo "kết thúc sớm nhất" *chứng minh được* là tối ưu — đó mới là chỗ dùng tham lam an toàn.

### 4. Lý thuyết số — nền của mật mã

- **Euclid (ƯCLN)**: `ucln(a,b) = ucln(b, a%b)`. Một trong những thuật toán cổ nhất còn dùng, O(log n).
- **Sàng Eratosthenes**: gạch bội của từng số nguyên tố; liệt kê nguyên tố tới n trong O(n log log n).
- **Lũy thừa modulo nhanh**: tính `aᵇ mod m` trong O(log b) bằng cách bình phương liên tiếp. Đây là **trái tim của RSA** — nơi b có thể lớn hàng nghìn bit. Chú ý mã dùng `u128` ở bước trung gian để **chống tràn số** (Chương 13: hàm toàn phần).

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

```bash
cd code
cargo run  -p ch60
cargo test -p ch60
```

```rust
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
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Nhận diện mô thức là kỹ năng quan trọng nhất.** Bài toán con chồng lặp → QHĐ; thử-và-lùi → quay lui; tối ưu cục bộ → tham lam.
2. **QHĐ = đệ quy + ghi nhớ.** Cùng công thức, khác cách tổ chức, chênh nhau hàng tỷ lần.
3. **Tham lam nhanh nhưng dễ sai.** Luôn kiểm chứng "tính chất tham lam" trước khi tin nó; khi nghi ngờ, dùng QHĐ.
4. **Lý thuyết số là nền của mật mã.** Lũy thừa modulo nhanh + số nguyên tố = RSA. Nhớ chống tràn số bằng `u128`.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (QHĐ — Dãy con tăng dài nhất)**
Viết `day_tang_dai_nhat(so: &[i32]) -> usize` trả về độ dài dãy con tăng dài nhất (LeetCode 300). Ví dụ `[10,9,2,5,3,7,101,18]` → `4` (là `2,3,7,101`).

<details>
<summary><b>Gợi ý</b></summary>

`dp[i]` = độ dài dãy tăng dài nhất KẾT THÚC tại `i`. `dp[i] = 1 + max(dp[j])` với mọi `j < i` mà `so[j] < so[i]`. Đáp án là `max(dp)`. Độ phức tạp O(n²) (có bản O(n log n) dùng tìm nhị phân).
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn day_tang_dai_nhat(so: &[i32]) -> usize {
    if so.is_empty() { return 0; }
    let n = so.len();
    let mut dp = vec![1usize; n];
    for i in 1..n {
        for j in 0..i {
            if so[j] < so[i] {
                dp[i] = dp[i].max(dp[j] + 1);
            }
        }
    }
    *dp.iter().max().unwrap()
}

#[cfg(test)]
mod bt1 {
    use super::*;
    #[test]
    fn lis() {
        assert_eq!(day_tang_dai_nhat(&[10, 9, 2, 5, 3, 7, 101, 18]), 4);
        assert_eq!(day_tang_dai_nhat(&[0, 1, 0, 3, 2, 3]), 4);
        assert_eq!(day_tang_dai_nhat(&[]), 0);
    }
}
```
</details>

**Bài tập 2 (Quay lui — Tổ hợp)**
Viết `to_hop(n: u32, k: u32) -> Vec<Vec<u32>>` sinh mọi tổ hợp `k` số chọn từ `1..=n` (LeetCode 77). Với `n=4, k=2` phải cho 6 tổ hợp.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn to_hop(n: u32, k: u32) -> Vec<Vec<u32>> {
    let mut kq = Vec::new();
    let mut current = Vec::new();
    fn di(start: u32, n: u32, k: u32, ht: &mut Vec<u32>, kq: &mut Vec<Vec<u32>>) {
        if ht.len() as u32 == k { kq.push(ht.clone()); return; }
        for x in start..=n {
            ht.push(x);
            di(x + 1, n, k, ht, kq);
            ht.pop(); // LÙI LẠI
        }
    }
    di(1, n, k, &mut current, &mut kq);
    kq
}

#[cfg(test)]
mod bt2 {
    use super::*;
    #[test]
    fn to_hop_dung() {
        assert_eq!(to_hop(4, 2).len(), 6); // C(4,2) = 6
        assert_eq!(to_hop(5, 5).len(), 1);
        assert_eq!(to_hop(5, 0).len(), 1); // tổ hợp rỗng
    }
}
```
</details>

**Bài tập 3 (Tư duy: chọn mô thức)**
Với mỗi bài toán, chọn mô thức (QHĐ / quay lui / tham lam / lý thuyết số) và giải thích ngắn:
1. Tìm số cách leo n bậc thang khi mỗi bước leo 1 hoặc 2 bậc.
2. Giải một câu đố Sudoku.
3. Lên lịch phát sóng để chiếu được nhiều chương trình nhất trong ngày.
4. Kiểm tra một số 200 chữ số có phải nguyên tố không.

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **Quy hoạch động.** `cach(n) = cach(n-1) + cach(n-2)` — chính là Fibonacci! Bài toán con chồng lặp.
2. **Quay lui.** Thử điền từng số 1–9 vào ô trống, mâu thuẫn thì lùi. Cắt tỉa bằng ràng buộc hàng/cột/ô 3×3.
3. **Tham lam.** Chọn chương trình kết thúc sớm nhất — đúng bài "chọn hoạt động", tham lam tối ưu.
4. **Lý thuyết số.** Kiểm tra tính nguyên tố xác suất (Miller-Rabin) dùng lũy thừa modulo nhanh — không thể thử chia hết tới √n với số 200 chữ số.

Kỹ năng cốt lõi: đọc đề, tìm *dấu hiệu* (chồng lặp? thử mọi khả năng? chọn cục bộ? tính chất số?) rồi mới chọn công cụ.
</details>
