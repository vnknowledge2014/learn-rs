# Chương 43: Tư Duy Vibe Coding: Từ Thợ Gõ Cú Pháp Thành Tổng Đạo Diễn Kiến Trúc (The Vibe Coding Paradigm: System Architect vs Syntax Typist)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn đến với Chủ đề 8 — một bước ngoặt mang tính cách mạng trong giáo trình Rust Masterclass: **Lập trình hiện đại cùng AI (Vibe Coding)**.

Trong các thập kỷ trước, một lập trình viên thường được đo lường bằng tốc độ gõ phím, khả năng ghi nhớ từng tên hàm trong thư viện chuẩn, và việc nhớ chính xác vị trí của từng dấu chấm phẩy, dấu ngoặc nhọn. Người ta gọi đó là thời kỳ của những "thợ gõ cú pháp" (syntax typists). Tuy nhiên, sự xuất hiện của các mô hình ngôn ngữ lớn (LLM) và các trợ lý lập trình trí tuệ nhân tạo (AI coding assistants) đã thay đổi hoàn toàn cuộc chơi.

Khái niệm **Vibe Coding** đại diện cho sự dịch chuyển mô hình tư duy: Lập trình viên không còn phải vật lộn với những chi tiết lặp đi lặp lại của cú pháp bề mặt, mà nâng tầm vị thế thành một **Tổng đạo diễn kiến trúc (System Architect)**. Bạn tập trung 90% năng lượng trí tuệ vào việc thiết kế cấu trúc dữ liệu, xác định ranh giới hệ thống, quy định các giao ước hành vi (traits), bảo vệ tính bất biến của nghiệp vụ, và đóng vai trò thẩm định viên chất lượng tối cao.

Điều tuyệt vời nhất là: **Rust chính là ngôn ngữ lập trình hoàn hảo nhất hành tinh để thực hành Vibe Coding**. Trong các ngôn ngữ động như Python hay JavaScript, khi AI sinh mã sai lệch về kiểu dữ liệu hay bỏ quên trường hợp rỗng, chương trình vẫn có thể chạy và chỉ nổ tung lúc nửa đêm khi khách hàng bấm nút thanh toán. Nhưng trong Rust, trình biên dịch `rustc` cực kỳ nghiêm khắc. Trình biên dịch sẽ ngay lập tức "bắt lỗi" bất kỳ ảo giác (hallucination) nào của AI về kiểu dữ liệu, vi phạm quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), hay xung đột đa luồng.

Mục tiêu học tập của chương:
- Thấu hiểu bản chất và triết lý của làn sóng **Vibe Coding** trong kỷ nguyên AI.
- Định vị rõ ràng vai trò: Việc gì giao cho AI thực hiện, việc gì con người bắt buộc phải nắm quyền kiểm soát kiến trúc (system architecture).
- Nắm vững phương pháp thiết kế Hợp đồng giao ước trước (Contract-First Design) bằng Trait và Enum trong Rust để hướng dẫn AI sinh mã chuẩn xác.
- Nhận diện cách hệ thống kiểm tra kiểu tĩnh và quản lý bộ nhớ của Rust trở thành "tấm lá chắn" biến AI thành cộng sự đắc lực thay vì hiểm họa.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy tưởng tượng bạn đang chuẩn bị quay một bộ phim hành động bom tấn chiếu rạp.

### Trường hợp 1: Người thợ gõ cú pháp (The Syntax Typist)
Người này giống như một người làm phim nghiệp dư muốn tự mình làm tất cả:
- Tự tay trèo lên trần nhà mắc từng bóng đèn.
- Tự tay may từng chiếc cúc áo cho diễn viên.
- Tự tay cầm cọ vẽ từng khung hình hoạt họa 24 hình/giây.
- Tự tay vác máy quay chạy vòng quanh sân khấu.

Hậu quả là gì? Anh ta kiệt sức vì những chi tiết vụn vặt. Vì quá mải mê khâu chiếc cúc áo, anh ta quên mất kịch bản tổng thể có logic hay không. Cốt truyện trở nên rời rạc, cảnh quay sau mâu thuẫn với cảnh quay trước, và bộ phim thất bại thảm hại.

### Trường hợp 2: Tổng đạo diễn kiến trúc trong Vibe Coding (The System Architect)
Ngược lại, một Tổng đạo diễn tài hoa làm việc hoàn toàn khác:
- Đạo diễn nắm giữ tầm nhìn: Kịch bản phân cảnh (Storyboard), tính cách từng nhân vật, thông điệp cần truyền tải, và giới hạn ngân sách.
- Đạo diễn không tự diễn cảnh nguy hiểm, mà giao việc đó cho một **đoàn đóng thế chuyên nghiệp siêu tốc (AI)**: *"Tôi cần cảnh một chiếc xe cảnh sát rượt đuổi qua ngã tư lúc trời mưa, tông vào thùng rác nhưng tuyệt đối không được đâm vào cột đèn!"*.
- Đoàn diễn viên đóng thế (AI) có thể thực hiện cảnh quay đó trong tích tắc với 5 phương án khác nhau.
- Sau khi đoàn quay xong, Tổng đạo diễn ngồi trước màn hình giám sát, xem lại từng thước phim và hô: *"Cắt! Cảnh này góc quay bị lệch, làm lại góc nghiêng 45 độ!"*.

Trong lập trình Rust cùng AI:
- Bạn là **Tổng đạo diễn kiến trúc (System Architect)**: Bạn vẽ ra bản vẽ hệ thống, xác định dữ liệu đầu vào, kết quả đầu ra, và các quy tắc nghiệp vụ bất khả xâm phạm.
- AI là **đoàn đóng thế siêu tốc**: Viết các đoạn mã lặp lại, dựng khung mã giả, sinh dữ liệu mẫu, và triển khai các hàm chi tiết theo hợp đồng bạn đã đặt ra.
- Trình biên dịch `rustc` là **Trưởng ban kiểm định an toàn phim trường**: Bất kỳ dây cáp bảo hiểm nào bị lỏng (lỗi vi phạm thời gian sống lifetime, rò rỉ vùng nhớ, hoặc dữ liệu bị mượn borrow sai quy tắc) đều bị đình chỉ quay ngay lập tức!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu

### 1. Sự dịch chuyển từ Gõ mã sang Điều phối kiến trúc
Khi làm việc với các công cụ lập trình hỗ trợ bởi trí tuệ nhân tạo (AI), hiệu suất của bạn không còn bị nghẽn bởi tốc độ gõ bàn phím (WPM - Words Per Minute), mà bị nghẽn bởi **khả năng diễn đạt đặc tả kiến trúc (Specification Expressiveness)**.

Nếu bạn đưa cho AI một yêu cầu mập mờ:
> *"Hãy viết cho tôi một hàm xử lý thanh toán"*

AI sẽ tự suy diễn theo hàng triệu dòng mã trôi nổi trên Internet: Có thể dùng số thực `f64` để lưu tiền tệ (dẫn tới sai số làm tròn tài chính), có thể nuốt lỗi bằng `unwrap()`, hoặc bỏ qua việc ghi nhật ký kiểm toán.

Nhưng khi bạn tiếp cận theo tư duy kiến trúc sư hệ thống:
1. Bạn xác định kiểu dữ liệu bất biến: Tiền tệ phải là số nguyên dương tính theo đơn vị nhỏ nhất (ví dụ: `u64` xu/cents), không dùng số thực.
2. Bạn định nghĩa Enum liệt kê đầy đủ mọi trạng thái lỗi có thể xảy ra (`InsufficientFunds`, `NetworkTimeout`, `InvalidCurrency`).
3. Bạn thiết lập Trait quy định giao ước tương tác giữa các mô-đun.

Khi khung kiến trúc vững như bàn thạch, AI chỉ việc điền phần logic bên trong thân hàm. Khả năng phát sinh lỗi gần như bị triệt tiêu hoàn toàn.

### 2. Vì sao Rust là "Cặp bài trùng" vĩ đại nhất với AI?
Các nhà nghiên cứu công nghệ thường nhận định: *"Ngôn ngữ lập trình càng dễ dãi thì càng nguy hiểm khi kết hợp với AI; ngôn ngữ càng khắt khe thì AI càng phát huy sức mạnh tối thượng"*.

| Tiêu chí | Ngôn ngữ thông dịch/Động (Python, JS) | Rust (Hệ thống kiểu tĩnh & Trình biên dịch khắt khe) |
| :--- | :--- | :--- |
| **Hành vi khi AI suy đoán sai kiểu** | Chương trình vẫn khởi động bình thường. Lỗi kiểu dữ liệu (TypeError) chỉ phát tác khi người dùng chạm vào nhánh code đó. | `rustc` báo lỗi ngay lập tức lúc biên dịch với mã lỗi cụ thể (ví dụ: `E0308`). Mã không thể chạy nếu chưa đúng kiểu 100%. |
| **Quản lý tài nguyên & Bộ nhớ** | Phụ thuộc bộ thu gom rác (Garbage Collector) hoặc giải phóng thủ công. AI dễ tạo ra rò rỉ bộ nhớ (Memory Leak) âm thầm. | Hệ thống quyền sở hữu (ownership), quy tắc mượn (borrow), và thời gian sống (lifetime) đảm bảo an toàn bộ nhớ tuyệt đối mà không cần GC. |
| **Cạnh tranh dữ liệu (Data Race)** | Rất khó phát hiện lỗi đa luồng do AI viết thiếu cơ chế đồng bộ hóa. | Quy tắc Send/Sync của Rust ngăn chặn Data Race ngay tại thời điểm biên dịch. |
| **Phản hồi lỗi để AI tự sửa** | Thông báo lỗi runtime thường chung chung, không kèm giải pháp. | Báo cáo lỗi của Rust cực kỳ chi tiết, kèm vị trí dòng, giải thích lý do, và đề xuất sửa chữa (`help:`). |

### 3. Phương pháp tiếp cận Hợp đồng giao ước trước (Contract-First Architecture)
Để làm chủ Vibe Coding trong Rust, bạn cần thành thạo quy trình 3 bước:
1. **Domain Modeling (Mô hình hóa nghiệp vụ)**: Dùng `struct` và `enum` để mô tả thế giới thực. Biến các trạng thái bất hợp pháp thành những kiểu dữ liệu không thể biểu diễn được trong mã nguồn (Make illegal states unrepresentable).
2. **Behavior Contracts (Giao ước hành vi)**: Dùng `trait` để định nghĩa những gì hệ thống có thể làm, phân tách hoàn toàn giữa "Cái gì cần làm" (Interface) và "Làm như thế nào" (Implementation).
3. **Safe Composition (Lắp ghép an toàn)**: Nhờ AI hiện thực hóa các `impl`, sử dụng con trỏ thông minh (smart pointer) như `Box`, `Rc` hoặc tham chiếu mượn an toàn khi cần thiết, và sử dụng bộ nhớ đệm (buffer) để tối ưu hóa hiệu năng nhập xuất dữ liệu.

---

## Mã nguồn minh họa thực chiến

Dưới đây là một ví dụ hoàn chỉnh, có thể biên dịch và thực thi trực tiếp bằng `rustc --edition=2021`, minh họa cách một Kiến trúc sư Hệ thống định hình hợp đồng thanh toán thương mại điện tử bằng Trait và Enum, sau đó để AI hiện thực hóa các cổng thanh toán giả lập và động cơ xử lý đơn hàng an toàn.

```rust
// ============================================================================
// CHƯƠNG 39: MINH HỌA TƯ DUY VIBE CODING & KIẾN TRÚC HỢP ĐỒNG GIAO ƯỚC (CONTRACT)
// Tác giả: Tổng Đạo Diễn Kiến Trúc Rust (System Architect)
// ============================================================================

// 1. ĐỊNH NGHĨA KIỂU DỮ LIỆU NGHIỆP VỤ (DOMAIN MODELING)
// Sử dụng Enum để quản lý các trạng thái đơn hàng một cách tường minh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderStatus {
    Created,
    Paid { transaction_id: String },
    Failed { reason: String },
}

// Cấu trúc đơn hàng với nguyên tắc: Số tiền luôn dùng số nguyên u64 (cents/xu)
// để loại bỏ triệt để sai số làm tròn số thực của máy tính.
#[derive(Debug, Clone)]
pub struct Order {
    pub id: u64,
    pub customer_name: String,
    pub amount_cents: u64,
    pub status: OrderStatus,
}

impl Order {
    pub fn new(id: u64, customer_name: &str, amount_cents: u64) -> Self {
        Self {
            id,
            customer_name: customer_name.to_string(),
            amount_cents,
            status: OrderStatus::Created,
        }
    }
}

// 2. ĐỊNH NGHĨA TRẠNG THÁI LỖI TƯỜNG MINH (ERROR DOMAIN)
// Không bao giờ dùng chuỗi thô để mô tả lỗi; phân loại rõ ràng giúp hệ thống tự phục hồi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentError {
    InsufficientFunds { available: u64, required: u64 },
    NetworkTimeout(String),
    InvalidCurrency(String),
    CardExpired,
}

// 3. GIAO ƯỚC HÀNH VI (CONTRACT TRAIT)
// Kiến trúc sư định nghĩa bản thiết kế: Bất kỳ cổng thanh toán nào cũng phải tuân thủ trait này.
pub trait PaymentGateway {
    fn process_payment(&self, account_id: &str, amount_cents: u64) -> Result<String, PaymentError>;
}

// 4. HIỆN THỰC HÓA BỞI AI: CỔNG THANH TOÁN THỬ NGHIỆM (MOCK PAYMENT GATEWAY)
// Phần mã chi tiết này do trợ lý AI sinh ra dựa trên bản thiết kế Trait ở trên.
pub struct MockBankingGateway {
    pub mock_balance_cents: u64,
}

impl PaymentGateway for MockBankingGateway {
    fn process_payment(&self, account_id: &str, amount_cents: u64) -> Result<String, PaymentError> {
        // Kiểm tra dữ liệu đầu vào: tài khoản không được để trống
        if account_id.is_empty() {
            return Err(PaymentError::NetworkTimeout("Mã định danh tài khoản không hợp lệ".to_string()));
        }

        // Kiểm tra số dư khả dụng
        if self.mock_balance_cents < amount_cents {
            return Err(PaymentError::InsufficientFunds {
                available: self.mock_balance_cents,
                required: amount_cents,
            });
        }

        // Sinh mã deliver dịch thành công duy nhất
        let tx_id = format!("TXN-{}-OK", amount_cents);
        Ok(tx_id)
    }
}

// 5. BỘ ĐIỀU PHỐI ĐƠN HÀNG (ORDER PROCESSOR)
// Kiến trúc sư thiết kế bộ điều phối nhận vào một tham chiếu mượn (borrow) cổng thanh toán,
// tuân thủ nghiêm ngặt quyền sở hữu (ownership) và không làm sao chép dữ liệu thừa.
pub struct OrderProcessor<'a, G: PaymentGateway> {
    gateway: &'a G,
}

impl<'a, G: PaymentGateway> OrderProcessor<'a, G> {
    pub fn new(gateway: &'a G) -> Self {
        Self { gateway }
    }

    // Xử lý đơn hàng: Mượn khả biến (&mut) đơn hàng để cập nhật trạng thái
    pub fn checkout(&self, order: &mut Order, account_id: &str) -> Result<(), PaymentError> {
        println!("[Hệ thống] Bắt đầu thanh toán đơn hàng #{} cho khách hàng: {}", order.id, order.customer_name);

        match self.gateway.process_payment(account_id, order.amount_cents) {
            Ok(tx_id) => {
                println!("[Hệ thống] Thanh toán thành công! Mã deliver dịch: {}", tx_id);
                order.status = OrderStatus::Paid { transaction_id: tx_id };
                Ok(())
            }
            Err(err) => {
                println!("[Cảnh báo] Thanh toán thất bại: {:?}", err);
                order.status = OrderStatus::Failed {
                    reason: format!("{:?}", err),
                };
                Err(err)
            }
        }
    }
}

// 6. HÀM MAIN KIỂM CHỨNG TOÀN BỘ LUỒNG HOẠT ĐỘNG
fn main() {
    println!("=== DEMO VIBE CODING PARADIGM: KIẾN TRÚC SƯ & HỆ THỐNG GIAO ƯỚC ===");

    // Tạo cổng thanh toán giả lập với số dư 50,000 xu (500 USD)
    let mock_gateway = MockBankingGateway {
        mock_balance_cents: 50_000,
    };

    // Khởi tạo bộ xử lý đơn hàng
    let processor = OrderProcessor::new(&mock_gateway);

    // Kịch bản 1: Đơn hàng hợp lệ (30,000 xu <= 50,000 xu)
    let mut order_1 = Order::new(101, "Nguyễn Văn An", 30_000);
    println!("Trạng thái ban đầu đơn #101: {:?}", order_1.status);
    let result_1 = processor.checkout(&mut order_1, "ACC-USER-888");
    assert!(result_1.is_ok());
    println!("Trạng thái sau thanh toán đơn #101: {:?}\n", order_1.status);

    // Kịch bản 2: Đơn hàng vượt hạn mức (80,000 xu > 50,000 xu)
    let mut order_2 = Order::new(102, "Trần Thị Bình", 80_000);
    println!("Trạng thái ban đầu đơn #102: {:?}", order_2.status);
    let result_2 = processor.checkout(&mut order_2, "ACC-USER-999");
    assert!(result_2.is_err());
    println!("Trạng thái sau thanh toán đơn #102: {:?}", order_2.status);

    println!("\n[Tổng kết] Toàn bộ kịch bản nghiệp vụ hoạt động chính xác 100% theo bản vẽ kiến trúc!");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục

Khi lập trình cùng trợ lý AI, AI có thể vô tình sinh ra mã vi phạm các quy tắc khắt khe của Rust. Dưới đây là bảng tra cứu các lỗi biên dịch điển hình nhất kèm giải pháp xử lý:

| Mã lỗi `rustc` | Tên lỗi & Nguyên nhân điển hình do AI tạo ra | Đoạn mã vi phạm mẫu | Cách khắc phục chuẩn kiến trúc |
| :--- | :--- | :--- | :--- |
| **`E0308`** | **Mismatched types (Không khớp kiểu dữ liệu)**<br>AI thường nhầm lẫn giữa chuỗi mượn `&str` và chuỗi cấp phát `String`, hoặc nhầm giữa số nguyên `u64` và số thực `f64`. | ```rust // compile-fail\nlet s: String = "xin chào";``` | Dùng `.to_string()` hoặc `String::from("...")` để chuyển từ `&str` sang `String`. |
| **`E0382`** | **Use of moved value (Sử dụng giá trị đã bị chuyển quyền sở hữu)**<br>AI quen tư duy Python/JS nên dùng lại biến sau khi đã chuyển quyền sở hữu (ownership) vào hàm khác. | ```rust // compile-fail\nlet s = String::from("Rust");\nlet s2 = s;\nprintln!("{}", s);``` | Truyền tham chiếu mượn (borrow) `&s` thay vì chuyển giao quyền sở hữu, hoặc dùng `.clone()` nếu thực sự cần nhân bản. |
| **`E0599`** | **No method named found for type (Không tìm thấy phương thức)**<br>AI tự "bịa" (hallucinate) ra một phương thức không có thật, hoặc quên chưa `use` Trait chứa phương thức đó vào phạm vi. | ```rust // compile-fail\nlet v = vec![1, 2, 3];\nv.sort_descending();``` | Kiểm tra tài liệu chuẩn của thư viện. Đưa Trait vào phạm vi (`use crate::...`) hoặc tự định nghĩa phương thức trong Trait tương ứng. |
| **`E0061`** | **This function takes X arguments but Y arguments were supplied**<br>AI gọi hàm nhưng cung cấp thiếu hoặc thừa đối số do nhớ sai phiên bản API cũ. | ```rust // compile-fail\nfn add(a: i32, b: i32) -> i32 { a + b }\nadd(10);``` | Kiểm tra chữ ký hàm (function signature) trong mã nguồn và truyền đúng số lượng kiểu tham số theo yêu cầu. |

---

## Tóm tắt chương & Bài tập rèn luyện

### 4 Điểm cốt lõi cần ghi nhớ
1. **Vibe Coding không phải là lập trình cẩu thả**: Đó là sự thăng hoa của tư duy kiến trúc, giải phóng kỹ sư khỏi việc gõ cú pháp để tập trung vào thiết kế hệ thống, xác định ranh giới và mô hình hóa nghiệp vụ.
2. **Rust là đối tác hoàn hảo nhất của AI**: Trình biên dịch `rustc` đóng vai trò người gác cổng an toàn tối cao, tự động phát hiện và chặn đứng mọi ảo giác, lỗi kiểu dữ liệu và vi phạm an toàn bộ nhớ.
3. **Nguyên tắc Hợp đồng trước (Contract-First)**: Luôn phác thảo `struct`, `enum`, và `trait` trước khi yêu cầu AI sinh mã chi tiết. Bản thiết kế càng chặt chẽ thì mã AI sinh ra càng hoàn hảo.
4. **Quyền sở hữu và mượn tham chiếu**: Sử dụng tham chiếu mượn (borrow) hợp lý giúp mã nguồn tinh gọn, hiệu năng cao và tránh cấp phát bộ nhớ lãng phí.

### Bài tập rèn luyện tư duy

**Bài tập 1 (Phân định vai trò Đạo diễn - Diễn viên)**:
Hãy liệt kê 3 nhiệm vụ trong một dự án phần mềm bạn sẽ ủy thác 100% cho trợ lý AI thực hiện, và 3 nhiệm vụ bạn bắt buộc phải tự mình quyết định và kiểm soát chặt chẽ với tư cách là Kiến trúc sư Hệ thống.

**Bài tập 2 (Thiết kế Hợp đồng Kho Hàng)**:
Không cần viết thuật toán phức tạp, hãy sử dụng `struct` và `trait` của Rust để phác thảo hợp đồng cho một hệ thống Quản lý kho hàng (Warehouse Inventory). Hợp đồng cần định nghĩa:
- Một `struct Item` gồm mã sản phẩm, tên, và số lượng còn trong kho.
- Một `enum InventoryError` gồm các lỗi: `OutOfStock`, `ItemNotFound`.
- Một `trait InventoryService` có 2 phương thức: `add_stock` và `deduct_stock`.

**Bài tập 3 (Sửa lỗi quyền sở hữu của AI)**:
Đoạn mã sau do AI sinh ra bị lỗi biên dịch `E0382`. Dựa trên kiến thức về quyền sở hữu (ownership) và mượn (borrow), hãy giải thích nguyên nhân và sửa lại cho đúng:
```rust
fn print_message(msg: String) {
    println!("Tin nhắn: {}", msg);
}

fn main() {
    let greeting = String::from("Chào mừng đến với Rust Vibe Coding!");
    print_message(greeting);
    println!("Độ dài tin nhắn ban đầu: {}", greeting.len());
}
```
*(Gợi ý: Hãy thay đổi chữ ký của hàm `print_message` để mượn lát cắt chuỗi `&str` thay vì chiếm đoạt quyền sở hữu toàn bộ `String`)*.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Ranh giới nằm ở chỗ: việc nào có **đáp án đúng-sai kiểm chứng được ngay** thì ủy thác; việc nào là **đánh đổi không thể hoàn tác hoặc mang hậu quả hệ thống** thì tự quyết.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

**3 việc ủy thác 100% cho AI** — đặc điểm chung: cơ giới, có tiêu chí đúng-sai rõ ràng, sai thì test bắt được ngay:
1. **Viết test đơn vị từ đặc tả có sẵn.** Đặc tả rõ -> test là dịch máy móc; lại còn tự kiểm bằng chính việc chạy.
2. **Chuyển đổi/tái cấu trúc cú pháp cơ giới.** Đổi vòng `for` sang iterator, thêm dẫn xuất trait, sinh mã lặp khuôn (boilerplate). Hành vi phải giữ nguyên, và test hồi quy xác nhận.
3. **Viết tài liệu và chú thích cho mã đã xong.** Rủi ro thấp, dễ soát, không đụng tới logic đang chạy.

**3 việc BẮT BUỘC tự quyết với tư cách Kiến trúc sư** — đặc điểm chung: đánh đổi khó đảo ngược, hậu quả lan ra toàn hệ thống, không có test nào bắt được "quyết định sai":
1. **Ranh giới và hợp đồng giữa các thành phần** (thiết kế API, lược đồ dữ liệu, mô hình sở hữu). Sai ở đây thấm vào mọi nơi và cực đắt để sửa về sau.
2. **Đánh đổi an ninh và quyền riêng tư.** Lưu gì, mã hóa gì, tin ai. AI không gánh trách nhiệm pháp lý khi rò rỉ dữ liệu — bạn gánh.
3. **Lựa chọn kiến trúc lớn** (một khối liền hay vi dịch vụ, đồng bộ hay bất đồng bộ, chọn CSDL nào). Ràng buộc cả dự án trong nhiều năm.

Nguyên tắc gói gọn: **ủy thác việc có đáp án; tự giữ việc có đánh đổi.** AI là diễn viên xuất sắc thực thi cảnh quay; đạo diễn vẫn phải là người chịu trách nhiệm về bộ phim. Và dù ủy thác gì, khâu *rà soát* mã trước khi nhập vào nhánh chính luôn thuộc về bạn — ủy thác việc *viết* không có nghĩa ủy thác việc *chịu trách nhiệm*.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Hợp đồng nghĩa là định nghĩa *hình dạng* (struct, enum lỗi, chữ ký trait) mà không cần cài đặt thuật toán. Trait mô tả 'làm được gì', chưa nói 'làm thế nào'.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
/// Một mặt hàng trong kho.
pub struct Item {
    pub sku: String,      // mã sản phẩm
    pub name: String,
    pub quantity: u32,    // số lượng còn trong kho
}

/// Các lỗi có thể xảy ra khi thao tác kho.
#[derive(Debug, PartialEq)]
pub enum InventoryError {
    OutOfStock,    // trừ nhiều hơn tồn kho
    ItemNotFound,  // không có SKU này
}

/// Hợp đồng cho một dịch vụ quản lý kho — CHỈ nêu hành vi, không nêu cách cài.
pub trait InventoryService {
    /// Nhập thêm hàng cho một SKU.
    fn add_stock(&mut self, sku: &str, amount: u32) -> Result<(), InventoryError>;
    /// Xuất bớt hàng; lỗi nếu không đủ tồn hoặc không tìm thấy SKU.
    fn deduct_stock(&mut self, sku: &str, amount: u32) -> Result<(), InventoryError>;
}

// Một cài đặt tối giản để chứng minh hợp đồng dùng được.
use std::collections::HashMap;
struct MemoryInventory { kho: HashMap<String, u32> }
impl InventoryService for MemoryInventory {
    fn add_stock(&mut self, sku: &str, amount: u32) -> Result<(), InventoryError> {
        *self.kho.entry(sku.to_string()).or_insert(0) += amount;
        Ok(())
    }
    fn deduct_stock(&mut self, sku: &str, amount: u32) -> Result<(), InventoryError> {
        let ton = self.kho.get_mut(sku).ok_or(InventoryError::ItemNotFound)?;
        if *ton < amount { return Err(InventoryError::OutOfStock); }
        *ton -= amount;
        Ok(())
    }
}

#[test]
fn hop_dong_kho_hoat_dong() {
    let mut kho = MemoryInventory { kho: HashMap::new() };
    kho.add_stock("SKU1", 10).unwrap();
    assert_eq!(kho.deduct_stock("SKU1", 4), Ok(()));
    assert_eq!(kho.deduct_stock("SKU1", 100), Err(InventoryError::OutOfStock));
    assert_eq!(kho.deduct_stock("SKU9", 1), Err(InventoryError::ItemNotFound));
}
```

Đây là kỹ năng cốt lõi khi làm việc với AI: **định nghĩa hợp đồng trước, cài đặt sau.** `trait InventoryService` nói *dịch vụ kho phải làm được gì* mà không ràng buộc *làm thế nào* — dữ liệu để trong HashMap, trong CSDL, hay gọi qua mạng đều thỏa cùng hợp đồng. Khi bạn giao cho AI viết phần cài đặt, hợp đồng này là **bản đặc tả** để đối chiếu: `enum InventoryError` liệt kê đủ các ca hỏng, chữ ký `Result` buộc mọi cài đặt phải xử lý chúng. Bạn — kiến trúc sư — vẽ khung; AI tô màu bên trong.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Lỗi E0382 là *dùng-sau-khi-di-chuyển*: `print_message(greeting)` nuốt mất `greeting`, nên dòng sau không dùng lại được. Gợi ý của đề: đổi hàm sang mượn `&str`.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Nguyên nhân E0382 (`borrow of moved value`):** `print_message(msg: String)` nhận `String` **theo sở hữu**. Khi gọi `print_message(greeting)`, quyền sở hữu chuỗi **di chuyển vào hàm** rồi bị hủy khi hàm kết thúc. Dòng `greeting.len()` sau đó dùng một biến đã bị di chuyển — Rust chặn.

```text
print_message(greeting);              // greeting BỊ DI CHUYỂN vào hàm
println!("{}", greeting.len());       // LỖI: greeting không còn nữa
```

**Cách sửa (theo gợi ý — mượn thay vì chiếm đoạt):** đổi hàm nhận `&str`. Hàm chỉ *đọc* chuỗi để in, nó không cần *sở hữu*:

```rust
fn print_message(msg: &str) {          // mượn lát cắt chuỗi, không lấy sở hữu
    println!("Tin nhắn: {msg}");
}

fn main() {
    let greeting = String::from("Chào mừng đến với Rust Vibe Coding!");
    print_message(&greeting);          // CHO MƯỢN, greeting vẫn thuộc về main
    println!("Độ dài tin nhắn ban đầu: {}", greeting.len());  // dùng lại thoải mái
}
```

Vì sao đây là cách sửa *đúng* chứ không phải `greeting.clone()`: nhận `&str` là **nguyên tắc chung của Rust** — *hàm chỉ cần đọc thì mượn, đừng chiếm đoạt*. Nó vừa tránh sao chép thừa (`clone` cấp phát lại cả chuỗi), vừa linh hoạt hơn: `&str` nhận được cả `String` (qua `&`) lẫn chuỗi hằng `"..."`. Đây là lỗi kinh điển AI hay mắc khi sinh mã Rust — nhận `String` ở mọi nơi vì nó "đơn giản", rồi vấp move. Bạn cần nhận ra và sửa nó.
</details>
