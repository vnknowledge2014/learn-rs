# Chương 41: Quy Trình Phát Triển Dựa Trên Đặc Tả & TDD Cùng AI (Spec-Driven Development SDD & AI-Assisted TDD)

## Giới thiệu & Mục tiêu học tập

Trong lập trình truyền thống, một trong những cạm bẫy lớn nhất khiến các dự án phần mềm thất bại là tình trạng "vừa viết vừa nghĩ" — lập trình viên mở trình soạn thảo, gõ mã ào ạt, sau đó chạy thử thấy lỗi thì chắp vá tạm bợ. Khi có thêm sự xuất hiện của trợ lý AI, cái bẫy này càng trở nên nguy hiểm gấp bội: AI có thể sinh ra 500 dòng code trong vòng 5 giây, nhưng nếu 500 dòng code đó được xây dựng trên một nền móng không có định hướng rõ ràng, bạn sẽ nhận về một mớ "hỗn độn kỹ thuật" (spaghetti code) không thể bảo trì và tiềm ẩn hàng tá lỗi bảo mật.

Để khắc phục triệt để vấn đề này, các kỹ sư hệ thống hàng đầu thế giới đã đúc kết nên một phương pháp luận tối ưu: **Quy trình phát triển dựa trên đặc tả (Spec-Driven Development - SDD)** kết hợp cùng **Phát triển hướng kiểm thử cùng AI (AI-Assisted Test-Driven Development - TDD)**.

Thay vì yêu cầu AI viết code ngay lập tức, bạn sẽ yêu cầu AI cùng bạn làm rõ bản đặc tả kỹ thuật (`SPEC.md`), sau đó tạo ra một bộ bài thi kiểm tra nghiêm ngặt (Unit Tests) trước khi viết dù chỉ một dòng mã thực thi. Quy trình này biến AI thành một cỗ máy giải đố cực kỳ chuẩn xác, đảm bảo mọi ngóc ngách của hệ thống đều tuân thủ các quy tắc an toàn về quyền sở hữu (ownership), mượn (borrow), và thời gian sống (lifetime).

Mục tiêu học tập của chương:
- Thấu hiểu triết lý và chu trình làm việc khép kín của **Spec-Driven Development (SDD)**.
- Làm chủ chu trình 3 bước kinh điển của **AI-Assisted TDD**: Red (Viết test thất bại) -> Green (Viết mã tối thiểu để vượt qua) -> Refactor (Tái cấu trúc tối ưu).
- Xây dựng tư duy phát hiện trường hợp biên (Edge Cases): Dữ liệu rỗng, độ dài bất thường, ký tự dị biệt, và lỗi logic nghiệp vụ.
- Thiết lập bộ kiểm thử đơn vị tự động trong Rust bằng `#[cfg(test)]` và các macro kiểm tra khẳng định (`assert!`, `assert_eq!`).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

### Thanh tra an toàn xây dựng và Chiếc xe thử nghiệm va chạm

Hãy tưởng tượng bạn chuẩn bị sản xuất hàng loạt một dòng xe hơi gia đình đời mới.

#### Cách làm sai lầm (Code First - Viết mã trước):
- Đội ngũ kỹ sư bắt tay vào lắp ráp toàn bộ khung gầm, động cơ, ghế ngồi, sơn màu thật đẹp.
- Sau khi chiếc xe hoàn thiện và đem bán cho khách hàng, họ mới bắt đầu cầu nguyện cho chiếc xe không bị lật khi phanh gấp ở tốc độ cao.
- Nếu xe gặp tai nạn trên đường phố, hãng xe phải thu hồi hàng triệu chiếc, tốn kém hàng tỷ đô la và đánh mất uy tín hoàn toàn.

#### Cách làm đúng đắn trong SDD & TDD (Spec & Test First):
1. **Bản đặc tả kỹ thuật (Specification - SDD)**:
   - Trước khi mua một thanh thép nào, Kỹ sư trưởng ban hành tài liệu quy chuẩn: *"Xe phải có 4 cửa; phanh xe phải dừng được trong 30 mét ở vận tốc 80km/h; túi khí phải bung trong 0.03 giây khi xảy ra va chạm; tuyệt đối không rò rỉ nhiên liệu khi lật nghiêng"*.
2. **Thiết kế bài kiểm tra trước (Red Phase)**:
   - Kỹ sư dựng sẵn một phòng thí nghiệm va chạm với hình nhân cảm biến (Crash Test Dummies) và rào chắn thép (bộ Unit Tests).
   - Khi chưa có chiếc xe nào được đưa vào thử, bài kiểm tra đương nhiên ghi nhận trạng thái **Đỏ (Red)** vì chưa có sản phẩm.
3. **Chế tạo để vượt qua bài kiểm tra (Green Phase)**:
   - Xưởng sản xuất (đóng vai trò là trợ lý AI) bắt đầu lắp ráp khung xe với mục tiêu duy nhất: Vượt qua bài kiểm tra va chạm của phòng thí nghiệm.
   - Khi chiếc xe chạy đâm vào tường và các túi khí bung hoàn hảo, hệ thống thông báo trạng thái chuyển sang **Xanh (Green)**!
4. **Tối ưu hóa và tinh chỉnh (Refactor Phase)**:
   - Sau khi các tiêu chuẩn an toàn đã vượt qua, kỹ sư yêu cầu làm nhẵn bề mặt sơn, thay ghế nỉ bằng ghế da cao cấp, nhưng giữ nguyên khung gầm an toàn đã kiểm định.

Trong lập trình Rust:
- Bạn viết **Bản đặc tả (Spec)** quy định rõ các ràng buộc nghiệp vụ.
- Bạn yêu cầu AI viết **Bộ kiểm thử (Tests)** dựa trên đặc tả đó.
- Sau đó, bạn để AI viết **Mã thực thi (Implementation)** cho đến khi lệnh `cargo test` hiện lên toàn màu xanh lá rực rỡ!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Vòng đời của Spec-Driven Development (SDD)
Một quy trình SDD chuẩn mực gồm 4 giai đoạn tuần tự:

```
[1. Viết SPEC.md] ──► [2. Định nghĩa Types/Traits] ──► [3. AI viết Tests (RED)] ──► [4. AI viết Logic (GREEN)]
        ▲                                                                                   │
        │                                                                                   ▼
        └────────────────────────── [5. Tối ưu hóa (REFACTOR)] ◄────────────────────────────┘
```

1. **Giai đoạn 1 - Đặc tả yêu cầu (`SPEC.md`)**:
   - Xác định mục tiêu của mô-đun.
   - Liệt kê các quy tắc nghiệp vụ bất biến (Business Invariants).
   - Xác định rõ danh sách các trường hợp ngoại lệ (Edge Cases): Chuỗi rỗng, số vượt giới hạn, ký tự đặc biệt, hoặc ngắt kết nối đột ngột.
2. **Giai đoạn 2 - Mô hình hóa hệ thống kiểu (Type Modeling)**:
   - Dùng hệ thống kiểu dữ liệu tĩnh của Rust để khóa chặt các trạng thái bất hợp pháp.
   - Định nghĩa `struct`, `enum`, và `trait`.
3. **Giai đoạn 3 - Tạo bài thi TDD (Red Phase)**:
   - Yêu cầu AI: *"Dựa trên file SPEC.md và các kiểu dữ liệu trên, hãy viết một bộ kiểm thử đơn vị bao phủ toàn bộ các trường hợp thành công lẫn thất bại"*.
   - Chạy `cargo test`: Các bài test chắc chắn sẽ báo lỗi biên dịch hoặc thất bại vì chưa viết thân hàm.
4. **Giai đoạn 4 - Hiện thực hóa mã nguồn (Green Phase)**:
   - Yêu cầu AI: *"Bây giờ hãy viết thân hàm thực thi tối thiểu sao cho toàn bộ bài test trên đều vượt qua (PASS)"*.
5. **Giai đoạn 5 - Tái cấu trúc an toàn (Refactor Phase)**:
   - Làm sạch mã nguồn: Chuyển đổi các vòng lặp thủ công thành các hàm chuyển đổi dòng chảy (iterators), loại bỏ cấp phát bộ nhớ thừa, dùng bộ nhớ đệm (buffer) để tăng tốc độ xử lý dữ liệu, và áp dụng con trỏ thông minh (smart pointer) khi cần chia sẻ quyền sở hữu dữ liệu.

### 2. Sức mạnh của Kiểm thử trong Rust (`#[cfg(test)]`)
Rust tích hợp sẵn khung kiểm thử mạnh mẽ ngay trong ngôn ngữ chuẩn mà không cần cài đặt thêm bất kỳ thư viện bên thứ ba nào:
- Thuộc tính `#[cfg(test)]` báo cho trình biên dịch chỉ biên dịch mã kiểm thử khi chạy lệnh `cargo test`, hoàn toàn không làm phình to kích thước tệp nhị phân cuối cùng khi triển khai thương mại (Zero Binary Overhead).
- Các macro kiểm tra cơ bản:
  - `assert!(condition)`: Kiểm tra điều kiện phải đúng (`true`).
  - `assert_eq!(left, right)`: Kiểm tra hai giá trị bằng nhau.
  - `assert_ne!(left, right)`: Kiểm tra hai giá trị khác nhau.
  - `#[should_panic]`: Kiểm tra đoạn mã bắt buộc phải kích hoạt hoảng loạn (panic) khi gặp lỗi nghiêm trọng.

---

## Mã nguồn minh họa thực chiến

Dưới đây là một mô-đun Rust hoàn chỉnh, minh họa trọn vẹn quy trình SDD & TDD: Xây dựng một **Động cơ xác thực tài khoản ngân hàng và giao dịch chuyển tiền (BankTransactionValidator)**. Toàn bộ mã nguồn có thể biên dịch và thực thi bằng `rustc --edition=2021`.

```rust
// ============================================================================
// CHƯƠNG 41: MINH HỌA QUY TRÌNH SPEC-DRIVEN DEVELOPMENT & TDD CÙNG AI
// Tác giả: Kỹ Sư Hệ Thống Rust
// ============================================================================

// ----------------------------------------------------------------------------
// PHẦN 1: BẢN ĐẶC TẢ NGHIỆP VỤ & HỆ THỐNG KIỂU (SPEC & DOMAIN TYPES)
// Ràng buộc đặc tả (SPEC):
// 1. Số tài khoản phải có tiền tố "VN", độ dài đúng 10 ký tự, phần sau là chữ số.
// 2. Số tiền chuyển khoản phải lớn hơn 0 và không vượt quá hạn mức 100,000,000 xu.
// 3. Tài khoản nguồn và tài khoản đích tuyệt đối không được trùng nhau.
// ----------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    InvalidAccountLength { expected: usize, actual: usize },
    InvalidAccountPrefix(String),
    InvalidAccountDigits,
    SameSourceAndDestination,
    ZeroOrNegativeAmount,
    AmountExceedsLimit { limit: u64, requested: u64 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransferRequest {
    pub from_account: String,
    pub to_account: String,
    pub amount_cents: u64,
}

pub struct BankTransactionValidator {
    pub max_limit_cents: u64,
}

impl BankTransactionValidator {
    pub fn new(max_limit_cents: u64) -> Self {
        Self { max_limit_cents }
    }

    // Xác thực định dạng của một số tài khoản theo quy chuẩn
    // Mượn (borrow) tham chiếu lát cắt chuỗi &str để tối ưu hóa hiệu năng, zero-copy
    pub fn validate_account_format(&self, account: &str) -> Result<(), ValidationError> {
        if account.len() != 10 {
            return Err(ValidationError::InvalidAccountLength {
                expected: 10,
                actual: account.len(),
            });
        }

        if !account.starts_with("VN") {
            return Err(ValidationError::InvalidAccountPrefix(account[0..2].to_string()));
        }

        // Kiểm tra 8 ký tự phía sau phải là chữ số hợp lệ
        if !account[2..].chars().all(|c| c.is_ascii_digit()) {
            return Err(ValidationError::InvalidAccountDigits);
        }

        Ok(())
    }

    // Xác thực toàn bộ yêu cầu giao dịch chuyển khoản
    pub fn validate_transfer(&self, req: &TransferRequest) -> Result<(), ValidationError> {
        // 1. Kiểm tra tài khoản nguồn
        self.validate_account_format(&req.from_account)?;

        // 2. Kiểm tra tài khoản đích
        self.validate_account_format(&req.to_account)?;

        // 3. Kiểm tra trùng lặp
        if req.from_account == req.to_account {
            return Err(ValidationError::SameSourceAndDestination);
        }

        // 4. Kiểm tra số tiền
        if req.amount_cents == 0 {
            return Err(ValidationError::ZeroOrNegativeAmount);
        }

        if req.amount_cents > self.max_limit_cents {
            return Err(ValidationError::AmountExceedsLimit {
                limit: self.max_limit_cents,
                requested: req.amount_cents,
            });
        }

        Ok(())
    }
}

// ----------------------------------------------------------------------------
// PHẦN 2: BỘ KIỂM THỬ ĐƠN VỊ TDD DO AI SINH RA TỪ FILE SPEC (RED -> GREEN)
// ----------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_account_format() {
        let validator = BankTransactionValidator::new(50_000_000);
        assert!(validator.validate_account_format("VN12345678").is_ok());
    }

    #[test]
    fn test_account_invalid_length() {
        let validator = BankTransactionValidator::new(50_000_000);
        // Quá ngắn
        let err_short = validator.validate_account_format("VN123").unwrap_err();
        assert_eq!(err_short, ValidationError::InvalidAccountLength { expected: 10, actual: 5 });

        // Quá dài
        let err_long = validator.validate_account_format("VN12345678999").unwrap_err();
        assert_eq!(err_long, ValidationError::InvalidAccountLength { expected: 10, actual: 13 });
    }

    #[test]
    fn test_account_invalid_prefix() {
        let validator = BankTransactionValidator::new(50_000_000);
        let err = validator.validate_account_format("US12345678").unwrap_err();
        assert_eq!(err, ValidationError::InvalidAccountPrefix("US".to_string()));
    }

    #[test]
    fn test_account_non_digit_characters() {
        let validator = BankTransactionValidator::new(50_000_000);
        let err = validator.validate_account_format("VN1234ABCD").unwrap_err();
        assert_eq!(err, ValidationError::InvalidAccountDigits);
    }

    #[test]
    fn test_transfer_same_account_fails() {
        let validator = BankTransactionValidator::new(50_000_000);
        let req = TransferRequest {
            from_account: "VN12345678".to_string(),
            to_account: "VN12345678".to_string(),
            amount_cents: 1_000_000,
        };
        assert_eq!(validator.validate_transfer(&req), Err(ValidationError::SameSourceAndDestination));
    }

    #[test]
    fn test_transfer_amount_exceeds_limit() {
        let validator = BankTransactionValidator::new(10_000_000);
        let req = TransferRequest {
            from_account: "VN12345678".to_string(),
            to_account: "VN87654321".to_string(),
            amount_cents: 20_000_000, // Vượt quá hạn mức 10 triệu
        };
        assert_eq!(
            validator.validate_transfer(&req),
            Err(ValidationError::AmountExceedsLimit { limit: 10_000_000, requested: 20_000_000 })
        );
    }

    #[test]
    fn test_transfer_success() {
        let validator = BankTransactionValidator::new(50_000_000);
        let req = TransferRequest {
            from_account: "VN12345678".to_string(),
            to_account: "VN87654321".to_string(),
            amount_cents: 5_000_000,
        };
        assert!(validator.validate_transfer(&req).is_ok());
    }
}

// ----------------------------------------------------------------------------
// PHẦN 3: HÀM MAIN THỰC THI TRỰC TIẾP ĐỂ KIỂM CHỨNG TÍNH NĂNG
// ----------------------------------------------------------------------------
fn main() {
    println!("=== CHƯƠNG 41: MINH HỌA QUY TRÌNH SPEC-DRIVEN DEVELOPMENT (SDD) ===");

    // Khởi tạo bộ kiểm định giao dịch với hạn mức 50 triệu xu
    let validator = BankTransactionValidator::new(50_000_000);

    // Kịch bản kiểm thử trực tiếp 1: Giao dịch thành công
    let req_ok = TransferRequest {
        from_account: "VN11112222".to_string(),
        to_account: "VN33334444".to_string(),
        amount_cents: 15_000_000,
    };
    match validator.validate_transfer(&req_ok) {
        Ok(()) => println!("[Xác nhận] Giao dịch 15,000,000 xu từ {} sang {} HỢP LỆ!", req_ok.from_account, req_ok.to_account),
        Err(e) => println!("[Từ chối] Lỗi: {:?}", e),
    }

    // Kịch bản kiểm thử trực tiếp 2: Chuyển khoản trùng tài khoản
    let req_duplicate = TransferRequest {
        from_account: "VN11112222".to_string(),
        to_account: "VN11112222".to_string(),
        amount_cents: 500_000,
    };
    match validator.validate_transfer(&req_duplicate) {
        Ok(()) => println!("[Xác nhận] Giao dịch hợp lệ!"),
        Err(e) => println!("[Đặc tả chặn thành công] Phát hiện lỗi nghiệp vụ mong đợi: {:?}", e),
    }

    println!("\n[Tổng kết] Tất cả các điều kiện ràng buộc trong file SPEC đều được kiểm chứng chặt chẽ!");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục

Dưới đây là các lỗi biên dịch thường phát sinh trong chu trình viết test và hoàn thiện mã cùng trợ lý AI:

| Mã lỗi `rustc` | Nguyên nhân gốc rễ trong quá trình TDD | Đoạn mã vi phạm mẫu | Giải pháp sửa chữa chuẩn quy trình |
| :--- | :--- | :--- | :--- |
| **`E0277`** | **Trait bound `PartialEq` is not satisfied**<br>AI sử dụng `assert_eq!(a, b)` trong bài test nhưng kiểu dữ liệu tùy biến chưa được dẫn xuất trait so sánh. | ```rust // compile-fail\nstruct Point { x: i32 }\nassert_eq!(Point { x: 1 }, Point { x: 1 });``` | Bổ sung macro dẫn xuất `#[derive(Debug, PartialEq, Eq)]` phía trên định nghĩa cấu trúc dữ liệu. |
| **`E0308`** | **Mismatched types in assertions**<br>Trong bài test, AI so sánh một giá trị kiểu `Result<(), ValidationError>` với một kiểu lỗi chưa bọc trong `Err(...)`. | ```rust // compile-fail\nlet res: Result<(), i32> = Err(404);\nassert_eq!(res, 404);``` | Sửa lại biểu thức so sánh cho khớp kiểu: `assert_eq!(res, Err(404));`. |
| **`E0433`** | **Failed to resolve: use of undeclared module/crate**<br>AI tự tiện gọi các thư viện kiểm thử nâng cao (như `mockall` hoặc `proptest`) khi dự án chưa khai báo trong `Cargo.toml`. | ```rust // compile-fail\nuse proptest::prelude::*;``` | Yêu cầu AI chỉ sử dụng khung kiểm thử tích hợp chuẩn của Rust (`#[cfg(test)]`, `assert!`) trừ khi bạn cho phép nạp thêm dependency. |
| **`E0603`** | **Struct/Field is private**<br>AI viết module kiểm thử tách rời nhưng các trường của struct cần kiểm tra không được gắn từ khóa `pub`. | ```rust // compile-fail\nmod inner { pub struct Item { count: u32 } }\nlet it = inner::Item { count: 5 };``` | Thêm từ khóa `pub` trước các trường hoặc cung cấp phương thức khởi tạo công khai `pub fn new(...)`. |

---

## Tóm tắt chương & Bài tập rèn luyện

### 4 Điểm cốt lõi cần ghi nhớ
1. **Spec-Driven Development (SDD) là kim chỉ nam**: Không bao giờ viết code khi chưa có bản đặc tả kỹ thuật mô tả rõ ràng các trường hợp biên và điều kiện bất biến.
2. **Quy trình TDD 3 bước cùng AI**:
   - **Red**: Yêu cầu AI sinh bài test kiểm chứng đặc tả (Test thất bại trước).
   - **Green**: Yêu cầu AI viết logic tối thiểu để vượt qua toàn bộ bài test.
   - **Refactor**: Yêu cầu AI dọn dẹp và tối ưu hóa mã nguồn mà không làm gãy test.
3. **Rust biến bài test thành công cụ bảo vệ tuyệt đối**: Kết hợp giữa hệ thống kiểm tra kiểu tĩnh của trình biên dịch và bộ unit tests tự động giúp loại bỏ triệt để mọi lỗi hồi quy (Regression Bugs).
4. **Tối ưu hóa hiệu năng bằng tham chiếu mượn (borrow)**: Luôn ưu tiên truyền tham chiếu lát cắt `&str` hoặc `&[u8]` trong các hàm kiểm định để đạt hiệu năng đỉnh cao, hạn chế việc nhân bản bộ nhớ (`.clone()`).

### Bài tập rèn luyện tư duy

**Bài tập 1 (Tập viết Bản đặc tả SPEC.md)**:
Hãy viết một bản đặc tả ngắn cho tính năng: "Xác thực mật khẩu người dùng (Password Strength Validator)".
Bản đặc tả cần nêu rõ:
- Độ dài tối thiểu và tối đa cho phép.
- Các loại ký tự bắt buộc phải có (chữ hoa, chữ thường, chữ số, ký tự đặc biệt).
- Danh sách các mã lỗi tương ứng trong `enum PasswordError`.

**Bài tập 2 (Thiết kế bài kiểm tra TDD trước khi viết code)**:
Dựa trên bản đặc tả mật khẩu ở Bài tập 1, hãy viết một bộ kiểm thử đơn vị bằng Rust gồm ít nhất 4 hàm kiểm thử:
- Mật khẩu quá ngắn.
- Mật khẩu thiếu chữ viết hoa.
- Mật khẩu thiếu chữ số.
- Mật khẩu hoàn hảo hợp lệ.

**Bài tập 3 (Sửa lỗi thiếu Trait so sánh của AI)**:
Đoạn mã sau do AI tạo ra bị lỗi biên dịch `E0277` khi chạy lệnh kiểm thử:
```rust
struct OrderId(u64);

#[test]
fn test_order_id_equality() {
    let id1 = OrderId(100);
    let id2 = OrderId(100);
    // Lỗi: binary operation `==` cannot be applied to type `OrderId`
    assert_eq!(id1, id2);
}
```
Hãy giải thích vì sao macro `assert_eq!` đòi hỏi những trait nào, và bổ sung dòng lệnh chính xác để đoạn mã trên vượt qua kỳ kiểm thử.
*(Gợi ý: Dẫn xuất `#[derive(Debug, PartialEq)]` cho cấu trúc tuple struct `OrderId`)*.
