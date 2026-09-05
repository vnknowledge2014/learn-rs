# Chương 55: Kim tự tháp Kiểm thử — Unit, Integration, E2E, TDD, BDD, Property & Doctest (The Testing Pyramid)

## Giới thiệu & Mục tiêu học tập

Xuyên suốt giáo trình, mỗi chương đều kèm một module `#[cfg(test)]`. Nhưng chúng ta chưa bao giờ dừng lại để trả lời câu hỏi nền tảng: **kiểm thử là gì, có mấy loại, và khi nào dùng loại nào?**

Đây không phải câu hỏi phụ. Một dự án không có test là một dự án mà **mỗi lần sửa một dòng, bạn phải cầu nguyện**. Còn một dự án test sai cách — ví dụ 500 test tích hợp chậm chạp thay vì 5000 unit test nhanh — thì tệ theo kiểu khác: không ai dám chạy test, nên test trở nên vô dụng.

Chương này trình bày **Kim tự tháp Kiểm thử (Testing Pyramid)** và toàn bộ các phương pháp mà bạn nghe tên nhưng có thể chưa phân biệt được: TDD, BDD, unit, integration, E2E, property-based, doctest, mocking, fuzzing. Điều đặc biệt: Rust có **hệ thống kiểm thử tích hợp sẵn trong ngôn ngữ** mạnh bậc nhất trong các ngôn ngữ hệ thống — kể cả doctest (test nằm trong tài liệu) mà rất ít ngôn ngữ có.

Mục tiêu học tập của chương này:
- Hiểu **Kim tự tháp Kiểm thử**: vì sao nhiều unit test, ít E2E test — không phải ngược lại.
- Phân biệt **Unit / Integration / E2E test** và cách Rust tổ chức từng loại (`#[cfg(test)]` trong `src/` vs thư mục `tests/`).
- Nắm quy trình **TDD (Red → Green → Refactor)** và cấu trúc **BDD (Given → When → Then)**.
- Làm chủ **Test Double**: mock, stub, spy, fake — thay phụ thuộc bằng bản giả, không cần thư viện ngoài.
- Viết **kiểm thử theo tính chất (property-based)** — kiểm một đẳng thức đúng với *mọi* đầu vào.
- Dùng **doctest** — biến ví dụ trong tài liệu thành test chạy được.
- Biết khi nào cần **fuzzing** và mối liên hệ của nó với kiểm thử thâm nhập (pen-test) ở Chương 42.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│           HÌNH TƯỢNG: KIỂM ĐỊNH MỘT CHIẾC Ô TÔ TRƯỚC KHI XUẤT XƯỞNG               │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│                              ▲  ÍT, CHẬM, ĐẮT                                     │
│                             ╱ ╲                                                   │
│                            ╱E2E╲     E2E = LÁI THỬ CẢ CHIẾC XE trên đường thật    │
│                           ╱─────╲    "khách bấm nút mua → tiền bị trừ → hàng giao"│
│                          ╱       ╲                                                │
│                         ╱ TÍCH HỢP╲   INTEGRATION = ghép động cơ + hộp số, nổ máy │
│                        ╱───────────╲  "giỏ hàng + cổng thanh toán làm việc cùng"  │
│                       ╱             ╲                                             │
│                      ╱   UNIT TEST   ╲  UNIT = kiểm TỪNG con ốc, từng bugi riêng  │
│                     ╱─────────────────╲ "tổng tiền tính đúng không?"              │
│                    ╱___________________╲                                          │
│                              ▼  NHIỀU, NHANH, RẺ                                  │
│                                                                                  │
│   Vì sao hình KIM TỰ THÁP chứ không phải hình thoi hay tháp ngược?               │
│   · Kiểm một con ốc: mất 1 giây, làm được 5000 lần/ngày.                         │
│   · Lái thử cả xe : mất 30 phút, làm được vài lần/ngày.                          │
│   → Đặt phần lớn niềm tin vào tầng đáy (nhanh, rẻ), chỉ dùng tầng đỉnh để        │
│     xác nhận "mọi bộ phận đã ghép lại thì cả chiếc xe có chạy không".            │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### Vì sao là kim tự tháp, không phải tháp ngược?

Một sai lầm kinh điển là viết quá nhiều test ở tầng cao (E2E) và quá ít ở tầng đáy (unit) — tạo thành **kim tự tháp ngược** (ice-cream cone). Hậu quả:
- Bộ test chạy hàng chục phút, nên lập trình viên né không chạy.
- Khi một E2E test đỏ, bạn không biết *bộ phận nào* hỏng — phải dò cả chiếc xe.
- Test giòn: đổi một nút bấm giao diện là chục E2E test đổ theo.

**Nguyên tắc vàng**: mỗi hành vi nên được kiểm ở **tầng thấp nhất có thể**. Logic tính tiền → unit test. Việc giỏ hàng gọi đúng cổng thanh toán → integration test. Toàn bộ hành trình mua hàng → một vài E2E test là đủ.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Ba tầng và cách Rust tổ chức chúng

Rust có một quy ước thư mục rất rõ ràng cho từng tầng:

| Tầng | Đặt ở đâu | Nhìn thấy gì | Tốc độ |
|---|---|---|---|
| **Unit** | `#[cfg(test)] mod ...` ngay trong `src/lib.rs` | Cả hàm **riêng tư** (`use super::*`) | Mili-giây |
| **Integration** | Tệp riêng trong thư mục **`tests/`** | Chỉ API **công khai** (như người dùng thật) | Nhanh |
| **E2E** | Thường ở `tests/` nhưng dựng cả hệ thống, hoặc dùng công cụ ngoài | Toàn bộ hệ thống qua giao diện thật | Chậm |
| **Doc-test** | Trong khối ` ```  ` của chú thích `///` | API công khai (kiêm luôn tài liệu) | Chậm (biên dịch riêng) |

> **Điểm mấu chốt ít người biết**: tệp trong thư mục `tests/` được biên dịch thành **một crate độc lập**. Nó chỉ `use` được những gì bạn khai báo `pub`. Đây chính là lý do integration test bắt được lỗi mà unit test bỏ sót: nếu bạn quên `pub` một hàm, unit test vẫn xanh nhưng integration test không biên dịch được — đúng như người dùng thật sẽ gặp.

### 2. TDD — Phát triển hướng kiểm thử (Test-Driven Development)

TDD đảo ngược thứ tự quen thuộc: **viết test TRƯỚC, code SAU**. Vòng lặp ba nhịp:

```
  ┌─────────────────────────────────────────────────────────┐
  │  1. RED   — Viết một test cho hành vi CHƯA tồn tại.      │
  │             Chạy `cargo test` → nó ĐỎ (thất bại). Tốt!  │
  │             (Nếu nó xanh ngay, test của bạn vô nghĩa.)   │
  │                          │                              │
  │                          ▼                              │
  │  2. GREEN — Viết ĐÚNG lượng code TỐI THIỂU để test xanh.│
  │             Không làm gì hơn. Chống "mạ vàng".          │
  │                          │                              │
  │                          ▼                              │
  │  3. REFACTOR — Dọn dẹp code cho sạch, test vẫn phải     │
  │                xanh. Test chính là lưới an toàn.        │
  └────────────────────────────┬────────────────────────────┘
                               │ lặp lại cho hành vi tiếp theo
                               ▼
```

Lợi ích không nằm ở "có test" — mà ở chỗ TDD **buộc bạn thiết kế API từ góc nhìn người dùng trước khi cài đặt**. Bạn viết `gio.them("A", 10_000, 3)` trong test, và ngay lập tức nhận ra chữ ký hàm nên trông thế nào. Chương 45 đã dùng TDD cùng AI; chương này trình bày nó một cách hệ thống.

### 3. BDD — Phát triển hướng hành vi (Behaviour-Driven Development)

BDD là TDD nhìn từ góc **ngôn ngữ nghiệp vụ**. Thay vì test tên `test_discount_calculation`, bạn viết test kể một **kịch bản** mà cả người không lập trình cũng đọc hiểu, theo cấu trúc **Given–When–Then**:

```
GIVEN  (cho trước)  một giỏ hàng trị giá 1.000.000đ
WHEN   (khi)        khách VIP thanh toán với mức giảm 15%
THEN   (thì)        số tiền bị trừ đúng 850.000đ
```

Trong hệ sinh thái Rust, thư viện `cucumber` cho phép viết kịch bản bằng tệp `.feature` ngôn ngữ Gherkin. Nhưng bạn **không cần thư viện** để làm BDD — chỉ cần đặt tên test theo hành vi và bố cục thân test theo ba khối Given/When/Then, như mã minh họa bên dưới. Bản chất BDD là một *kỷ luật viết test*, không phải một công cụ.

### 4. Test Double — bốn loại "diễn viên đóng thế"

Khi test một hàm phụ thuộc cổng thanh toán, cơ sở dữ liệu, hay đồng hồ, bạn **không** muốn gọi thứ thật (chậm, tốn tiền, không tất định). Bạn thay nó bằng một "diễn viên đóng thế". Có bốn loại, phân biệt theo *mục đích*:

| Loại | Mục đích | Ví dụ trong mã dưới |
|---|---|---|
| **Stub** | Trả về câu trả lời cố định | `AlwaysFailGateway` luôn báo lỗi |
| **Spy** | Ghi lại nó ĐƯỢC GỌI thế nào | `SpyGateway` lưu số tiền đã nhận |
| **Mock** | Spy + kiểm chứng kỳ vọng | kiểm `called_with == vec![80_000]` |
| **Fake** | Bản cài đặt thật nhưng đơn giản hóa | `HashMap` thay cho cơ sở dữ liệu |

Chìa khóa để thay được: hàm nghiệp vụ phải nhận phụ thuộc **qua một `trait`** (`&dyn PaymentGateway`), chứ không tự tạo ra nó bên trong. Đây chính là *tiêm phụ thuộc* ở Chương 14 và *đảo ngược phụ thuộc* — và là lý do sâu xa khiến kiến trúc "lõi thuần túy, vỏ mệnh lệnh" (Chương 20) dễ kiểm thử đến vậy.

### 5. Kiểm thử theo tính chất (Property-Based) và Fuzzing

Ở Chương 18 bạn đã gặp kỹ thuật này để kiểm chứng luật đại số. Nó tổng quát hơn thế: thay vì kiểm *một* ví dụ, ta khẳng định một **tính chất đúng với mọi đầu vào** rồi cho máy sinh hàng nghìn đầu vào ngẫu nhiên để tìm phản ví dụ.

```
Test ví dụ   :  discount(100_000, 10%)  == 90_000          (một điểm)
Test tính chất:  ∀ tổng, ∀ %:  discount(tổng, %)  ≤  tổng    (cả một miền)
```

Crate `proptest` và `quickcheck` làm việc này chuyên nghiệp, kèm khả năng **tự thu nhỏ (shrink)** phản ví dụ về dạng đơn giản nhất. **Fuzzing** (`cargo-fuzz`) là họ hàng gần: nó ném dữ liệu ngẫu nhiên/độc hại vào chương trình để tìm điểm **panic hoặc treo** — chính là công cụ mà kẻ tấn công OSCP/OSWE ở Chương 42 dùng để tìm lỗ hổng. Viết fuzz test cho bộ phân tích dữ liệu của bạn nghĩa là bạn tự tấn công mình trước khi kẻ xấu kịp làm.

### 6. Doctest — tài liệu không bao giờ lỗi thời

Đây là tính năng Rust đặc biệt tự hào. Mọi khối ` ``` ` trong chú thích `///` **được biên dịch và chạy như một test** khi bạn gõ `cargo test`. Hệ quả tuyệt vời: **ví dụ trong tài liệu không thể lỗi thời** — nếu bạn đổi API mà quên cập nhật ví dụ, `cargo test` sẽ đỏ. Tài liệu và mã nguồn không bao giờ nói dối nhau.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Toàn bộ mã dưới đây là một crate **thư viện** (`src/lib.rs`) — vì chương này nói về kiểm thử, và kiểm thử tổ chức tự nhiên nhất quanh một thư viện. Nó gồm miền nghiệp vụ "Giỏ hàng" cùng bốn module test minh họa bốn phong cách: unit, test-double, property, và BDD.

```bash
cd code
cargo test -p ch55            # chạy CẢ unit + integration + doctest
cargo test -p ch55 --doc      # chỉ doctest
```

`src/lib.rs`:

```rust
#![allow(dead_code, unused_variables)]
//! Chương 55 — Kim tự tháp Kiểm thử: Unit, Integration, E2E, TDD, BDD, Property, Doctest.

// ============================================================================
// PHẦN 1: MIỀN NGHIỆP VỤ ĐƯỢC PHÁT TRIỂN THEO TDD (Red → Green → Refactor)
// ============================================================================

/// Giỏ hàng — ta sẽ "viết test trước, code sau" cho từng hành vi.
#[derive(Debug, Clone, PartialEq)]
pub struct Cart {
    mat_queue: Vec<(String, u64, u32)>, // (tên, đơn giá, số lượng)
}

#[derive(Debug, PartialEq, Eq)]
pub enum CartError {
    SoLuongBangKhong,
    KhongTonTai,
}

impl Cart {
    pub fn new() -> Self {
        Cart { mat_queue: Vec::new() }
    }

    /// Thêm mặt hàng. Số lượng 0 là lỗi nghiệp vụ (không phải panic).
    pub fn them(&mut self, name: &str, don_price: u64, quantity: u32) -> Result<(), CartError> {
        if quantity == 0 {
            return Err(CartError::SoLuongBangKhong);
        }
        // Nếu đã có, cộng dồn số lượng thay vì tạo dòng mới
        if let Some(dong) = self.mat_queue.iter_mut().find(|(t, _, _)| t == name) {
            dong.2 += quantity;
        } else {
            self.mat_queue.push((name.to_string(), don_price, quantity));
        }
        Ok(())
    }

    /// Tổng tiền, tính bằng đơn vị nhỏ nhất (đồng) — KHÔNG dùng f64.
    ///
    /// # Ví dụ (đây cũng là một DOCTEST — chạy khi `cargo test`)
    /// ```
    /// # use ch55::Cart;
    /// let mut gio = Cart::moi();
    /// gio.them("Sách", 45_000, 2).unwrap();
    /// gio.them("Bút", 5_000, 3).unwrap();
    /// assert_eq!(gio.tong_tien(), 105_000);
    /// ```
    pub fn tong_tien(&self) -> u64 {
        self.mat_queue.iter().map(|(_, gia, sl)| gia * *sl as u64).sum()
    }

    pub fn so_dong(&self) -> usize {
        self.mat_queue.len()
    }

    /// Áp mã giảm giá phần trăm (0..=100).
    pub fn after_discount(&self, percent: u32) -> u64 {
        let tong = self.tong_tien();
        let pt = percent.min(100) as u64;
        tong - tong * pt / 100
    }
}

// ============================================================================
// PHẦN 2: TEST DOUBLE (MOCK/FAKE) BẰNG TRAIT — không cần thư viện ngoài
// ============================================================================

/// Cổng thanh toán là một PHỤ THUỘC. Trong test ta thay nó bằng bản giả.
pub trait PaymentGateway {
    fn debit(&self, so_tien: u64) -> Result<String, String>;
}

/// Bản thật (chỉ mô phỏng, không gọi mạng thật ở đây).
pub struct RealGateway;
impl PaymentGateway for RealGateway {
    fn debit(&self, so_tien: u64) -> Result<String, String> {
        Ok(format!("TXN-THAT-{}", so_tien))
    }
}

/// Hàm nghiệp vụ nhận phụ thuộc qua trait (tiêm phụ thuộc, Chương 14).
pub fn checkout(
    gio: &Cart,
    cong: &dyn PaymentGateway,
    discount: u32,
) -> Result<String, String> {
    let so_tien = gio.after_discount(discount);
    if so_tien == 0 {
        return Err("Giỏ rỗng hoặc miễn phí, không cần thanh toán".to_string());
    }
    cong.debit(so_tien)
}

// ============================================================================
// PHẦN 3: MÁY TRẠNG THÁI ĐỂ DEMO KIỂM THỬ THEO TÍNH CHẤT (PROPERTY-BASED)
// ============================================================================

/// Bộ sinh giả ngẫu nhiên tất định (LCG) — giống Chương 18, không cần crate.
pub struct Generator(u64);
impl Generator {
    pub fn moi(hat: u64) -> Self { Generator(hat) }
    pub fn so(&mut self, tran: u32) -> u32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as u32) % tran
    }
}


// ============================================================================
// TẦNG 1 — UNIT TESTS: nhanh, nhiều, kiểm một đơn vị biệt lập
// ============================================================================

#[cfg(test)]
mod unit {
    use super::*;

    // --- Phong cách TDD: mỗi test mô tả MỘT hành vi mong muốn ---

    #[test]
    fn new_cart_is_empty() {
        let gio = Cart::moi();
        assert_eq!(gio.so_dong(), 0);
        assert_eq!(gio.tong_tien(), 0);
    }

    #[test]
    fn add_item_totals_correctly() {
        let mut gio = Cart::moi();
        gio.them("A", 10_000, 3).unwrap();
        assert_eq!(gio.tong_tien(), 30_000);
    }

    #[test]
    fn same_name_merges_quantity() {
        let mut gio = Cart::moi();
        gio.them("A", 10_000, 1).unwrap();
        gio.them("A", 10_000, 2).unwrap();
        assert_eq!(gio.so_dong(), 1, "phải gộp thành 1 dòng");
        assert_eq!(gio.tong_tien(), 30_000);
    }

    #[test]
    fn zero_quantity_is_error_not_panic() {
        let mut gio = Cart::moi();
        assert_eq!(gio.them("A", 10_000, 0), Err(CartError::SoLuongBangKhong));
        assert_eq!(gio.so_dong(), 0); // không thêm gì
    }

    #[test]
    fn discount_clamped_at_100() {
        let mut gio = Cart::moi();
        gio.them("A", 100_000, 1).unwrap();
        assert_eq!(gio.after_discount(200), 0); // ghim ở 100%, không âm
    }
}

// ============================================================================
// TẦNG 2 — TEST VỚI TEST DOUBLE (MOCK): thay phụ thuộc bằng bản giả
// ============================================================================

#[cfg(test)]
mod test_double {
    use super::*;
    use std::cell::RefCell;

    /// SPY: cổng giả ghi lại nó được gọi với số tiền bao nhiêu.
    struct SpyGateway {
        called_with: RefCell<Vec<u64>>,
    }
    impl PaymentGateway for SpyGateway {
        fn debit(&self, so_tien: u64) -> Result<String, String> {
            self.called_with.borrow_mut().push(so_tien);
            Ok("TXN-GIA".to_string())
        }
    }

    /// STUB: cổng giả luôn báo lỗi, để test nhánh thất bại.
    struct AlwaysFailGateway;
    impl PaymentGateway for AlwaysFailGateway {
        fn debit(&self, _: u64) -> Result<String, String> {
            Err("Thẻ bị từ chối".to_string())
        }
    }

    #[test]
    fn checkout_charges_discounted_total() {
        let mut gio = Cart::moi();
        gio.them("A", 100_000, 1).unwrap();
        let spy = SpyGateway { called_with: RefCell::new(vec![]) };

        checkout(&gio, &spy, 20).unwrap(); // giảm 20% -> 80.000

        assert_eq!(*spy.called_with.borrow(), vec![80_000], "phải trừ đúng số sau giảm giá");
    }

    #[test]
    fn checkout_propagates_gateway_error() {
        let mut gio = Cart::moi();
        gio.them("A", 100_000, 1).unwrap();
        assert_eq!(checkout(&gio, &AlwaysFailGateway, 0), Err("Thẻ bị từ chối".to_string()));
    }

    #[test]
    fn empty_cart_skips_gateway() {
        let gio = Cart::moi();
        let spy = SpyGateway { called_with: RefCell::new(vec![]) };
        let kq = checkout(&gio, &spy, 0);
        assert!(kq.is_err());
        assert!(spy.called_with.borrow().is_empty(), "cổng KHÔNG được gọi khi giỏ rỗng");
    }
}

// ============================================================================
// TẦNG 3 — KIỂM THỬ THEO TÍNH CHẤT (PROPERTY-BASED)
// Không kiểm một ví dụ, mà kiểm một ĐẲNG THỨC đúng với mọi đầu vào.
// ============================================================================

#[cfg(test)]
mod property {
    use super::*;

    #[test]
    fn discount_within_bounds() {
        let mut sinh = Generator::moi(2026);
        for _ in 0..2000 {
            let mut gio = Cart::moi();
            let so_mat_hang = sinh.so(5) + 1;
            for i in 0..so_mat_hang {
                let _ = gio.them(&format!("SP{}", i), (sinh.so(100_000) + 1) as u64, sinh.so(5) + 1);
            }
            let pt = sinh.so(150); // cố tình cho vượt 100
            let sau = gio.after_discount(pt);
            // TÍNH CHẤT: giá sau giảm luôn trong [0, tổng]
            assert!(sau <= gio.tong_tien(), "giảm giá không được làm TĂNG tiền");
        }
    }

    #[test]
    fn zero_discount_keeps_total() {
        let mut sinh = Generator::moi(7);
        for _ in 0..1000 {
            let mut gio = Cart::moi();
            gio.them("X", (sinh.so(50_000) + 1) as u64, sinh.so(9) + 1).unwrap();
            // TÍNH CHẤT: giảm 0% là phép đồng nhất
            assert_eq!(gio.after_discount(0), gio.tong_tien());
        }
    }

    #[test]
    fn sum_equals_parts() {
        let mut sinh = Generator::moi(99);
        for _ in 0..1000 {
            let (g1, sl1) = ((sinh.so(1000) + 1) as u64, sinh.so(9) + 1);
            let (g2, sl2) = ((sinh.so(1000) + 1) as u64, sinh.so(9) + 1);
            let mut gio = Cart::moi();
            gio.them("A", g1, sl1).unwrap();
            gio.them("B", g2, sl2).unwrap();
            // TÍNH CHẤT: tổng = tổng thành tiền từng dòng
            assert_eq!(gio.tong_tien(), g1 * sl1 as u64 + g2 * sl2 as u64);
        }
    }
}

// ============================================================================
// TẦNG 4 — BDD: cấu trúc GIVEN / WHEN / THEN (Behaviour-Driven Development)
// Không cần cucumber-rs: chỉ cần đặt tên và bố cục test theo ngôn ngữ nghiệp vụ.
// ============================================================================

#[cfg(test)]
mod bdd {
    use super::*;
    use std::cell::RefCell;

    struct OkGateway(RefCell<Vec<u64>>);
    impl PaymentGateway for OkGateway {
        fn debit(&self, s: u64) -> Result<String, String> { self.0.borrow_mut().push(s); Ok("OK".into()) }
    }

    /// Kịch bản: "Khách VIP mua hàng và được giảm 15%".
    #[test]
    fn vip_gets_15_percent_off() {
        // GIVEN — một giỏ hàng trị giá 1.000.000đ và một cổng thanh toán
        let mut gio = Cart::moi();
        gio.them("Tai nghe", 1_000_000, 1).unwrap();
        let cong = OkGateway(RefCell::new(vec![]));

        // WHEN — khách VIP (giảm 15%) thanh toán
        let ket_qua = checkout(&gio, &cong, 15);

        // THEN — thanh toán thành công và số tiền bị trừ đúng 850.000đ
        assert!(ket_qua.is_ok());
        assert_eq!(*cong.0.borrow(), vec![850_000]);
    }

    /// Kịch bản: "Không thể thanh toán một giỏ hàng rỗng".
    #[test]
    fn cannot_checkout_empty_cart() {
        // GIVEN — một giỏ hàng rỗng
        let gio = Cart::moi();
        let cong = OkGateway(RefCell::new(vec![]));

        // WHEN — cố gắng thanh toán
        let ket_qua = checkout(&gio, &cong, 0);

        // THEN — hệ thống từ chối và không gọi cổng thanh toán
        assert!(ket_qua.is_err());
        assert!(cong.0.borrow().is_empty());
    }
}
```

Và đây là **tầng kiểm thử tích hợp**, đặt ở `tests/integration.rs` — một crate riêng chỉ thấy API công khai:

```rust
//! TẦNG 3 — KIỂM THỬ TÍCH HỢP (Integration Test).
//! Tệp trong thư mục `tests/` được biên dịch thành MỘT CRATE RIÊNG, chỉ nhìn thấy
//! API CÔNG KHAI của `ch55` — đúng như một người dùng thật. Đây là điểm khác biệt
//! cốt lõi so với unit test (nằm trong lib, thấy được cả hàm riêng tư).

use ch55::{checkout, PaymentGateway, Cart};

/// Cổng giả cấp module test tích hợp (không truy cập được nội bộ crate).
struct CongGia;
impl PaymentGateway for CongGia {
    fn debit(&self, so_tien: u64) -> Result<String, String> {
        Ok(format!("TICH-HOP-{}", so_tien))
    }
}

#[test]
fn luong_mua_hang_hoan_chinh_tu_ben_ngoai() {
    // Dựng giỏ, cộng dồn, giảm giá, thanh toán — toàn bộ qua API công khai
    let mut gio = Cart::new();
    gio.them("Màn hình", 5_000_000, 1).unwrap();
    gio.them("Cáp", 150_000, 2).unwrap();
    gio.them("Màn hình", 5_000_000, 1).unwrap(); // gộp dòng

    assert_eq!(gio.so_dong(), 2);
    assert_eq!(gio.tong_tien(), 10_300_000);

    let id = checkout(&gio, &CongGia, 10).unwrap();
    assert_eq!(id, "TICH-HOP-9270000"); // 10.300.000 - 10%
}

#[test]
fn giu_bat_bien_qua_nhieu_thao_tac() {
    let mut gio = Cart::new();
    for i in 0..20 {
        gio.them(&format!("SP{}", i % 5), 1000, 1).unwrap(); // 5 tên, mỗi tên 4 lần
    }
    assert_eq!(gio.so_dong(), 5, "20 lần thêm 5 tên -> đúng 5 dòng");
    assert_eq!(gio.tong_tien(), 20_000);
}
```

---

## Bảng tra cứu lỗi & Công cụ (Testing Toolbox)

| Bạn muốn | Công cụ Rust | Ghi chú |
|---|---|---|
| Chạy toàn bộ test | `cargo test` | Bao gồm unit + integration + doctest |
| Chỉ một test | `cargo test ten_test` | Lọc theo tên |
| Xem `println!` trong test | `cargo test -- --nocapture` | Mặc định Rust nuốt stdout của test xanh |
| Test phải panic | `#[should_panic(expected = "...")]` | Kiểm nhánh `panic!` có kiểm soát |
| Test có thể bỏ qua | `#[ignore]` rồi `cargo test -- --ignored` | Cho test chậm |
| Đo độ phủ | `cargo llvm-cov` | Phần trăm dòng được test chạm tới |
| Property-based | crate `proptest` / `quickcheck` | Sinh đầu vào + thu nhỏ phản ví dụ |
| Fuzzing | `cargo fuzz` | Tìm panic/treo — nối với Chương 42 |
| Snapshot | crate `insta` | So kết quả với ảnh chụp đã duyệt |
| BDD Gherkin | crate `cucumber` | Kịch bản `.feature` cho người không code |

### Ba sai lầm kiểm thử phổ biến

1. **Test kiểm cài đặt thay vì hành vi.** Nếu đổi cách viết bên trong (không đổi kết quả) mà test đỏ, test của bạn quá dính vào chi tiết. Hãy test *cái gì*, đừng test *thế nào*.
2. **Test không tất định (flaky).** Test đọc đồng hồ thật, số ngẫu nhiên thật, hay mạng thật sẽ lúc xanh lúc đỏ. Dùng test double và bộ sinh có hạt giống cố định (như `Generator` ở trên).
3. **Kim tự tháp ngược.** Quá nhiều E2E, quá ít unit. Bộ test chạy 20 phút thì chẳng ai chạy.

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Kim tự tháp**: nhiều unit test (nhanh, rẻ), ít E2E test (chậm, đắt). Kiểm mỗi hành vi ở *tầng thấp nhất có thể*.
2. **Rust tổ chức tầng bằng thư mục**: `#[cfg(test)]` trong `src/` thấy hàm riêng tư (unit); thư mục `tests/` là crate riêng chỉ thấy API công khai (integration).
3. **TDD** (Red-Green-Refactor) thiết kế API từ góc người dùng; **BDD** (Given-When-Then) viết test bằng ngôn ngữ nghiệp vụ. Cả hai không cần thư viện — chúng là kỷ luật.
4. **Test double qua trait** là chìa khóa để test biệt lập; **property-based** kiểm cả một miền đầu vào; **doctest** giữ tài liệu không bao giờ lỗi thời.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (TDD một hành vi mới)**
Theo đúng vòng Red-Green-Refactor, thêm phương thức `xoa(&mut self, ten: &str) -> Result<(), CartError>` cho `Cart`: xóa một dòng theo tên, trả `Err(CartError::KhongTonTai)` nếu không có. Viết test ĐỎ trước, rồi mới cài đặt.

<details>
<summary><b>Gợi ý</b></summary>

Test đỏ trước: `assert_eq!(gio.xoa("KhongCo"), Err(CartError::KhongTonTai));`. Cài đặt dùng `Vec::iter().position(...)` rồi `Vec::remove`. Đừng viết code trước khi có test đỏ.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
impl Cart {
    pub fn remove(&mut self, name: &str) -> Result<(), CartError> {
        match self.mat_queue.iter().position(|(t, _, _)| t == name) {
            Some(i) => { self.mat_queue.remove(i); Ok(()) }
            None => Err(CartError::KhongTonTai),
        }
    }
}

#[cfg(test)]
mod bai_tap_1 {
    use super::*;
    #[test]
    fn xoa_dong_ton_tai() {
        let mut gio = Cart::new();
        gio.them("A", 100, 1).unwrap();
        gio.them("B", 200, 1).unwrap();
        assert_eq!(gio.remove("A"), Ok(()));
        assert_eq!(gio.so_dong(), 1);
        assert_eq!(gio.tong_tien(), 200);
    }
    #[test]
    fn xoa_dong_khong_ton_tai_bao_loi() {
        let mut gio = Cart::new();
        assert_eq!(gio.remove("KhongCo"), Err(CartError::KhongTonTai));
    }
}
```
</details>

**Bài tập 2 (Test double cho đồng hồ)**
Nhiều hàm cần "thời gian hiện tại" — nhưng gọi `Instant::now()` khiến test không tất định. Thiết kế một `trait DongHo { fn now(&self) -> u64; }`, một bản thật và một bản giả trả về thời gian cố định. Viết một hàm `ma_don_hang(clock: &dyn DongHo) -> String` sinh mã theo thời gian, và test nó **một cách tất định**.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub trait DongHo { fn now(&self) -> u64; }

pub struct DongHoGia(pub u64);
impl DongHo for DongHoGia { fn now(&self) -> u64 { self.0 } }

pub fn ma_don_hang(clock: &dyn DongHo) -> String {
    format!("ORD-{}", clock.now())
}

#[cfg(test)]
mod bai_tap_2 {
    use super::*;
    #[test]
    fn ma_don_hang_tat_dinh_nho_dong_ho_gia() {
        let dh = DongHoGia(1_700_000_000);
        assert_eq!(ma_don_hang(&dh), "ORD-1700000000"); // luôn giống nhau!
    }
}
```

Bài học: mọi phụ thuộc "không tất định" (đồng hồ, số ngẫu nhiên, mạng, tệp) đều nên đi qua một trait để test thay được. Đây là cách biến một hàm *không thuần túy* thành *kiểm thử được*.
</details>

**Bài tập 3 (Tư duy: xếp test vào đúng tầng)**
Với mỗi tình huống, hãy chọn tầng phù hợp nhất (unit / integration / E2E) và giải thích:
1. Hàm `after_discount` tính đúng với mức giảm 37%.
2. Giỏ hàng gọi đúng cổng thanh toán với số tiền đã giảm giá.
3. Khách bấm "Thanh toán" trên web → nhận email xác nhận.
4. Bộ phân tích JSON không panic với dữ liệu rác bất kỳ.

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **Unit** — logic thuần túy, một hàm, không phụ thuộc gì. Nhanh nhất, viết nhiều nhất.
2. **Integration** — hai bộ phận (giỏ + cổng) làm việc cùng nhau, dùng test double cho cổng.
3. **E2E** — cả hệ thống qua giao diện thật; chậm và đắt, chỉ viết cho vài hành trình quan trọng nhất.
4. **Property-based / Fuzzing** — không phải một ví dụ, mà là "với *mọi* đầu vào rác, không được panic". Đây cũng chính là tư duy phòng thủ của Chương 42.

Nguyên tắc rút ra: câu 1 có thể phủ bằng 20 unit test rẻ tiền; câu 3 chỉ cần 1–2 E2E test. Đảo ngược tỷ lệ đó là dấu hiệu của một bộ test ốm yếu.
</details>
