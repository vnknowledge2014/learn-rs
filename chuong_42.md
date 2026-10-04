# Chương 42: Tư duy tấn công thực chiến OSCP, Mô hình hóa mối đe dọa & Gia cố bảo mật hệ thống (OSCP Offensive Mindset, Threat Modeling & Hardening)

## Giới thiệu & Mục tiêu học tập

Trong thế giới an ninh mạng chuyên nghiệp, có một câu châm ngôn kinh điển của Tôn Tử: *"Biết người biết ta, trăm trận trăm thắng"*. Một kỹ sư phần mềm hệ thống không thể xây dựng nên một pháo đài vững chắc nếu không hiểu rõ cách thức kẻ tấn công (hacker / pentester) tư duy và hành động.

Chứng chỉ **OSCP (Offensive Security Certified Professional)** được coi là tiêu chuẩn vàng toàn cầu về kỹ năng tấn công thực chiến: Học viên bị ném vào một mạng lưới máy chủ thực tế và phải tự mình tìm ra lỗ hổng, khai thác ban đầu, và leo thang đặc quyền tối cao trong vòng 24 giờ liên tục. Khi bạn nhìn nhận hệ thống qua lăng kính của một chiến binh OSCP, bạn sẽ không còn nhìn mã nguồn như những dòng chữ đơn thuần, mà nhìn thấy các bề mặt tấn công (attack surfaces) tiềm tàng.

Trong chương cuối cùng của Chủ đề 7, chúng ta sẽ trang bị:
- **Tư duy tấn công thực chiến OSCP**: Chu trình 5 giai đoạn từ thu thập thông tin trinh sát, dò quét dịch vụ, khai thác ban đầu, đến leo thang đặc quyền (Privilege Escalation).
- **Mô hình hóa mối đe dọa (Threat Modeling)** theo tiêu chuẩn công nghiệp **STRIDE** của Microsoft: Nhận diện và đo lường rủi ro có hệ thống.
- Các cơ chế phòng vệ phần cứng và hệ điều hành hiện đại: **ASLR** (Trộn ngẫu nhiên địa chỉ), **DEP/NX** (Cấm thực thi vùng dữ liệu), và **Stack Canaries** (Chim hoàng yến ngăn xếp).
- **Kỹ thuật Gia cố nhị phân (Binary Hardening)** cho các ứng dụng Rust thông qua cờ biên dịch trong `Cargo.toml`: `panic = "abort"`, `overflow-checks = true`, `lto = true`.
- Kỹ thuật lập trình an toàn cấp cao: Chống lại các cuộc tấn công kênh kề (Side-Channel Timing Attacks) bằng thuật toán so sánh thời gian bất biến (Constant-time comparison).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu rõ triết lý Phòng thủ chiều sâu (Defense-in-Depth) và Tư duy OSCP, hãy quan sát hệ thống bảo vệ một kho vàng ngân hàng quốc gia:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG HÓA: HỆ THỐNG PHÒNG THỦ CHIỀU SÂU KHO VÀNG QUỐC GIA       │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [LỚP 1: HÀO NƯỚC & HÀNG RÀO THÉP GAI (INPUT SANITIZATION - LÀM SẠCH ĐẦU VÀO)]   │
│ Khách vào ngân hàng phải bước qua cổng dò kim loại. Bất kỳ ai mang súng          │
│ hay dao găm (ký tự độc hại, chuỗi tràn) đều bị chặn đứng ngay từ cổng vào!       │
│                                                                                  │
│ [LỚP 2: ĐỔI SỐ PHÒNG RANDOM MỖI NGÀY (ASLR - XÁO TRỘN ĐỊA CHỈ BỘ NHỚ)]          │
│ Tên trộm biết két sắt số 99 chứa vàng. Nhưng mỗi sáng, ngân hàng xáo trộn        │
│ biển số phòng ngẫu nhiên: Két sắt chứa vàng hôm nay biến thành phòng 412,        │
│ ngày mai thành phòng 785. Tên trộm không biết đường nào mà lần!                  │
│                                                                                  │
│ [LỚP 3: CHIM HOÀNG YẾN BÁO ĐỘNG (STACK CANARIES - BẢO VỆ NGĂN XẾP)]              │
│ Ngày xưa thợ mỏ mang chim hoàng yến xuống hầm than. Khi có khí độc rò rỉ,        │
│ chim ngất trước để báo động. Stack Canary là con số bí mật đặt trước RIP:       │
│ Nếu kẻ tấn công cố tình tràn bộ nhớ, nó buộc phải đè chết con chim này trước!    │
│ Mã kiểm tra do trình biên dịch chèn thấy chim bị đổi số ──► Tắt máy ngay!       │
│                                                                                  │
│ [LỚP 4: CỬA HẦM CHỐNG BOM & QUYỀN TỐI THIỂU (LEAST PRIVILEGE)]                   │
│ Ngay cả khi tên trộm lẻn được vào quầy giao dịch, cửa hầm chứa tiền vẫn khóa chặt.│
│ Nhân viên kế toán chỉ có chìa khóa mở ngăn kéo đựng bút, không ai có quyền Root! │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Chu trình tấn công OSCP giống như kế hoạch đột nhập dinh thự
- **Giai đoạn 1 (Reconnaissance - Trinh sát)**: Kẻ trộm đi vòng quanh dinh thự, ghi chép giờ giấc sinh hoạt của chủ nhà, xem tường rào cao bao nhiêu mét.
- **Giai đoạn 2 (Scanning & Enumeration - Dò xét cửa mở)**: Tên trộm đến từng cánh cửa sổ, lay thử then cài xem có then nào bị lỏng (giống như chạy Port Scanner ở Chương 40 để tìm cổng mạng mở).
- **Giai đoạn 3 (Initial Foothold - Đột nhập ban đầu)**: Phát hiện cửa sổ phòng bếp hé mở, tên trộm trèo vào được bên trong phòng bếp (chiếm được một tài khoản người dùng bình thường không có quyền admin).
- **Giai đoạn 4 (Privilege Escalation - Leo thang đặc quyền)**: Từ phòng bếp, tên trộm tìm kiếm chìa khóa vạn năng của quản gia để mở cửa phòng điều khiển trung tâm (chiếm quyền Quản trị viên tối cao `root` hoặc `SYSTEM`).

### 2. Chim hoàng yến trong hầm than (Stack Canary)
- Khi đào than dưới lòng đất, hiểm họa vô hình lớn nhất là khí độc methane không mùi không màu. Thợ mỏ luôn treo một chiếc lồng có chú chim hoàng yến bên cạnh. Cơ thể chim rất nhạy cảm; nếu có khí độc, chim sẽ lảo đảo ngất xỉu trước khi con người kịp nhận ra nguy hiểm.
- Trong ngăn xếp máy tính, **Stack Canary** là một giá trị số ngẫu nhiên được trình biên dịch tự động đặt vào ngay phía trước con trỏ địa chỉ trả về `Saved RIP`.
- Kẻ tấn công muốn tràn bộ đệm đè lên `RIP` thì bắt buộc phải đè qua giá trị Canary này. Trước lệnh `ret`, **đoạn mã kiểm tra mà trình biên dịch chèn vào cuối hàm** (không phải CPU tự làm) so giá trị con chim với bản gốc: Nếu thấy bị biến dạng, nó gọi hàm hủy khẩn cấp `__stack_chk_fail`, tiến trình bị kết thúc trước khi kịp nhảy tới địa chỉ do kẻ tấn công sắp đặt!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Mô hình Hóa Mối Đe Dọa STRIDE (Microsoft STRIDE Threat Model)

STRIDE là phương pháp luận chuẩn quốc tế giúp kỹ sư phân tích rủi ro hệ thống trước khi bắt tay vào viết mã:

| Chữ cái | Tên mối đe dọa (Threat) | Ý nghĩa bảo mật | Thuộc tính bị xâm phạm | Giải pháp khắc phục trong Rust |
|---|---|---|---|---|
| **S** | **Spoofing** (Giả mạo) | Mạo danh người dùng hoặc hệ thống khác. | Tính Xác thực (Authenticity) | Xác thực chữ ký mã hóa ed25519, token JWT có thời hạn. |
| **T** | **Tampering** (Làm sai lệch) | Thay đổi trái phép dữ liệu trên đường truyền hoặc trong bộ nhớ. | Tính Toàn vẹn (Integrity) | Sử dụng mã kiểm tra HMAC, mã hóa TLS 1.3, kiểu dữ liệu bất biến. |
| **R** | **Repudiation** (Chối bỏ) | Người dùng thực hiện hành vi rồi chối cãi. | Tính Bất khả chối bỏ (Non-repudiation) | Ghi nhật ký kiểm toán bất biến (Audit Logging), lưu chữ ký số. |
| **I** | **Information Disclosure** (Tiết lộ tin) | Để lộ dữ liệu mật cho người không phận sự. | Tính Bảo mật (Confidentiality) | Chống rò rỉ bộ nhớ, so sánh thời gian bất biến (Constant-time). |
| **D** | **Denial of Service** (Từ chối dịch vụ) | Làm kiệt quệ tài nguyên khiến hệ thống tê liệt. | Tính Sẵn sàng (Availability) | Giới hạn dung lượng bộ đệm (buffer), đặt Timeout kết nối mạng. |
| **E** | **Elevation of Privilege** (Leo thang quyền) | Người dùng quyền thấp tự nâng thành Admin. | Tính Phân quyền (Authorization) | Nguyên tắc quyền tối thiểu, không dùng `setuid root`, đóng gói an toàn. |

### 2. Các Cơ chế Bảo vệ Hệ điều hành cốt lõi (OS Mitigations)

Hệ điều hành hiện đại phối hợp cùng CPU để dựng nên các rào cản nhị phân:
1. **ASLR (Address Space Layout Randomization)**:
   - Mỗi lần tiến trình khởi động, hệ điều hành đặt phân đoạn Stack, Heap, và các thư viện chia sẻ vào các địa chỉ ngẫu nhiên trong không gian địa chỉ ảo 64-bit. Kẻ tấn công không thể đoán trước vị trí con trỏ hàm để bẻ lái CPU.
2. **DEP / NX (Data Execution Prevention / No-Execute - Chính sách $W \oplus X$)**:
   - Một trang bộ nhớ chỉ được phép có quyền Ghi ($W$) HOẶC quyền Thực thi ($X$), không bao giờ được phép có cả hai ($Write \oplus Execute$).
   - Vùng Stack và Heap chỉ có quyền Đọc/Ghi (`RW-`). Nếu kẻ tấn công bơm mã độc nhị phân (shellcode) vào một mảng trên Stack rồi hướng CPU nhảy vào đó, CPU sẽ kích hoạt ngoại lệ phần cứng chặn đứng ngay lập tức!
3. **Stack Canaries**:
   - Trình biên dịch chèn một giá trị bí mật (Canary) vào đầu hàm và kiểm tra lại ở cuối hàm để phát hiện tràn bộ đệm.

### 3. Cấu hình Gia cố Nhị phân trong Rust (`Cargo.toml`)

Để tối ưu hóa bảo mật và triệt tiêu diện tích tấn công (Attack Surface) trong các sản phẩm thực chiến, chúng ta cấu hình hồ sơ phát hành (`[profile.release]`):

```toml
[profile.release]
opt-level = 3            # Tối ưu hóa hiệu năng tối đa
lto = true               # Link-Time Optimization: Loại bỏ toàn bộ mã chết (Dead code)
codegen-units = 1        # Gom mã thành 1 đơn vị duy nhất để tối ưu LTO toàn diện
panic = "abort"          # Khi gặp lỗi nghiêm trọng, lập tức tắt ngay (không để lại Landing Pad)
overflow-checks = true   # Bắt buộc kiểm tra tràn số nguyên ngay cả trong bản Release!
strip = true             # Gọt bỏ bảng biểu tượng (Symbol Table): gây khó cho dịch ngược, KHÔNG ngăn được nó
```

### 4. Tấn công Kênh Kề Dựa Trên Thời Gian (Timing Attack) & Giải Pháp

Khi kiểm tra mật khẩu hay mã xác thực API Token, lập trình viên thường viết:
```rust
// NGUY HIỂM: So sánh chuỗi thông thường kết thúc sớm khi gặp ký tự sai!
if user_token == SECRET_TOKEN { ... }
```
- Toán tử `==` so sánh từng byte từ trái qua phải. Nếu byte đầu tiên sai, nó dừng lại ngay lập tức và trả về `false` trong 1 nano-giây.
- Nếu người dùng đoán đúng 5 byte đầu, máy tính mất 5 nano-giây mới trả về `false`.
- Kẻ tấn công OSCP sử dụng đồng hồ đo thời gian siêu chính xác để đoán từng ký tự một!
- **Giải pháp**: Phải sử dụng **So sánh thời gian bất biến (Constant-Time Comparison)**: Luôn luôn so sánh đủ 100% các byte bất kể đúng hay sai, để thời gian phản hồi không phụ thuộc vào vị trí byte sai.
- **Cảnh báo quan trọng**: viết vòng lặp "không thoát sớm" bằng Rust thường là chưa đủ. Rust và LLVM **không hứa** giữ nguyên tính hằng thời gian của mã nguồn trong mã máy — trình tối ưu có quyền biến vòng lặp XOR thành phép so sánh thoát sớm. `std::hint::black_box` chỉ là gợi ý "nỗ lực tốt nhất", không phải bảo đảm. Mã thật phải dùng crate đã được kiểm định như **`subtle`** (`ConstantTimeEq`, `Choice`) hoặc hàm so sánh của thư viện mật mã (`ring::constant_time`), và tốt nhất kiểm tra bằng công cụ đo rò rỉ thời gian như `dudect`.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là chương trình Rust hoàn chỉnh hiện thực hóa một động cơ kiểm tra bảo mật cấp doanh nghiệp: Tích hợp cơ chế so sánh thời gian bất biến (Constant-time token validation) chống tấn công Timing Attack — bản minh hoạ nguyên lý, mã thật hãy dùng crate `subtle` — cùng bộ lọc làm sạch đầu vào theo chuẩn mô hình STRIDE:

```rust
use std::hint::black_box;

/// Hàm so sánh mảng byte theo kiểu thời gian bất biến (Constant-Time Comparison)
/// Không kết thúc sớm khi gặp byte sai, nên thời gian chạy không phụ thuộc VỊ TRÍ byte sai.
///
/// GIỚI HẠN QUAN TRỌNG: đây là mã minh hoạ. Rust/LLVM KHÔNG hứa sinh mã máy hằng thời gian
/// (`black_box` chỉ là gợi ý "đừng tối ưu", không phải bảo đảm). Mã thật hãy dùng crate
/// chuyên dụng như `subtle` (`ConstantTimeEq`), được viết và kiểm định cho đúng mục đích này.
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    // Độ dài khác nhau thì trả về ngay: điều này tiết lộ ĐỘ DÀI bí mật (thường chấp nhận
    // được vì độ dài token là công khai), nhưng không tiết lộ nội dung.
    if a.len() != b.len() {
        return false;
    }

    let mut difference_accumulator: u8 = 0;

    // Duyệt qua toàn bộ các phần tử mà không dùng lệnh 'break' hay 'return' sớm
    for (byte_a, byte_b) in a.iter().zip(b.iter()) {
        // Phép XOR: Nếu hai byte giống nhau thì kết quả bằng 0, khác nhau thì khác 0
        difference_accumulator |= byte_a ^ byte_b;
    }

    // Gợi ý trình biên dịch đừng "thông minh" biến vòng lặp thành so sánh thoát sớm
    // (chỉ là nỗ lực tốt nhất — xem giới hạn ở chú thích đầu hàm)
    black_box(difference_accumulator) == 0
}

/// Các mức phân quyền người dùng trong mô hình bảo mật
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub enum UserRole {
    Guest = 0,
    Member = 1,
    Auditor = 2,
    Administrator = 3,
}

/// Động cơ xác thực và lọc mối đe dọa an ninh theo mô hình STRIDE
pub struct SecurityGateEngine {
    secret_master_token: Vec<u8>,
}

impl SecurityGateEngine {
    pub fn new(master_token: &[u8]) -> Self {
        Self {
            secret_master_token: master_token.to_vec(),
        }
    }

    /// Làm sạch dữ liệu đầu vào (Input Sanitization) theo nguyên tắc Whitelist
    /// Ngăn chặn Tampering và Injection
    pub fn sanitize_command_input(&self, raw_input: &str) -> Result<String, &'static str> {
        if raw_input.is_empty() {
            return Err("Đầu vào trống: Từ chối xử lý!");
        }

        if raw_input.len() > 64 {
            return Err("Đầu vào quá dài: Nguy cơ DoS bị chặn đứng!");
        }

        // Nguyên tắc Whitelist: Chỉ cho phép chữ cái/chữ số ASCII, gạch dưới, gạch ngang
        // và khoảng trắng. Dùng is_ascii_alphanumeric (KHÔNG phải is_alphanumeric):
        // danh sách trắng phải hẹp và rõ ràng — is_alphanumeric chấp nhận hàng chục
        // nghìn ký tự Unicode, gồm cả các ký tự trông giống hệt chữ Latinh.
        let is_safe = raw_input
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ' ');

        if !is_safe {
            return Err("Phát hiện ký tự nguy hiểm (đã chặn SQL/Shell Injection)!");
        }

        Ok(raw_input.trim().to_string())
    }

    /// Xác thực khóa bí mật với cơ chế chống Timing Attack
    pub fn authenticate_token(&self, provided_token: &[u8]) -> bool {
        constant_time_compare(&self.secret_master_token, provided_token)
    }

    /// Kiểm tra phân quyền truy cập theo nguyên tắc quyền tối thiểu (Least Privilege)
    pub fn verify_permission(
        &self,
        current_role: UserRole,
        required_role: UserRole,
    ) -> Result<(), &'static str> {
        if current_role >= required_role {
            Ok(())
        } else {
            Err("Từ chối truy cập: Không đủ đặc quyền (đã chặn Elevation of Privilege)!")
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   GIA CỐ HỆ THỐNG RUST & MÔ HÌNH HÓA MỐI ĐE DỌA STRIDE / OSCP    ");
    println!("==================================================================");

    // Khởi tạo động cơ an ninh với Master Token bí mật 16 bytes
    let master_token = b"OSCP_RUST_KEY_99";
    let security_gate = SecurityGateEngine::new(master_token);

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM CHỐNG TẤN CÔNG TIMING ATTACK QUA CONSTANT-TIME
    // -------------------------------------------------------------
    println!("\n[1] Kiểm chứng so sánh thời gian bất biến (Constant-Time):");
    let valid_attempt = b"OSCP_RUST_KEY_99";
    let wrong_first_byte = b"XSCP_RUST_KEY_99";
    let wrong_last_byte = b"OSCP_RUST_KEY_00";

    println!(
        "    - Thử token hợp lệ      : {}",
        security_gate.authenticate_token(valid_attempt)
    );
    println!(
        "    - Thử token sai byte đầu : {}",
        security_gate.authenticate_token(wrong_first_byte)
    );
    println!(
        "    - Thử token sai byte cuối: {}",
        security_gate.authenticate_token(wrong_last_byte)
    );
    println!("    => Mọi phép so sánh đều duyệt hết mảng byte, bất kể byte sai nằm ở đâu!");

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM LÀM SẠCH ĐẦU VÀO CHỐNG INJECTION & BUFFER FLOOD
    // -------------------------------------------------------------
    println!("\n[2] Kiểm thử làm sạch dữ liệu đầu vào (Input Sanitization):");

    let safe_input = "get_system_status";
    match security_gate.sanitize_command_input(safe_input) {
        Ok(clean) => println!("    - Lệnh an toàn được chấp nhận: '{}'", clean),
        Err(err) => println!("    [!] Từ chối: {}", err),
    }

    let malicious_injection = "get_status; rm -rf /; --";
    println!("    - Thử gửi payload độc hại: '{}'", malicious_injection);
    match security_gate.sanitize_command_input(malicious_injection) {
        Ok(_) => println!("    [!] [CẢNH BÁO] Lệnh độc hại đã lọt qua!"),
        Err(err) => println!("    [+] [CHẶN ĐỨNG AN TOÀN] {}", err),
    }

    let overflow_dos_attempt = "A".repeat(128);
    println!(
        "    - Thử gửi chuỗi tấn công DoS dài {} bytes...",
        overflow_dos_attempt.len()
    );
    match security_gate.sanitize_command_input(&overflow_dos_attempt) {
        Ok(_) => println!("    [!] [CẢNH BÁO] Payload DoS đã được chấp nhận!"),
        Err(err) => println!("    [+] [CHẶN ĐỨNG AN TOÀN] {}", err),
    }

    // -------------------------------------------------------------
    // 3. THỬ NGHIỆM KIỂM SOÁT PHÂN QUYỀN TỐI THIỂU (LEAST PRIVILEGE)
    // -------------------------------------------------------------
    println!("\n[3] Kiểm tra kiểm soát phân quyền truy cập (RBAC):");
    let user_role = UserRole::Member;
    println!("    - Người dùng đang có vai trò: {:?}", user_role);

    let audit_access = security_gate.verify_permission(user_role, UserRole::Auditor);
    println!("    - Yêu cầu truy cập vùng Auditor: {:?}", audit_access);
    assert!(audit_access.is_err());

    let member_access = security_gate.verify_permission(user_role, UserRole::Member);
    println!("    - Yêu cầu truy cập vùng Member : {:?}", member_access);
    assert!(member_access.is_ok());
    println!("    => Ngăn chặn triệt để nguy cơ Leo thang đặc quyền (Elevation of Privilege)!");

    println!("\n==================================================================");
    println!("   XÁC NHẬN: HỆ THỐNG PHÒNG THỦ CHIỀU SÂU SẴN SÀNG HOẠT ĐỘNG!    ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi triển khai các cơ chế an ninh, mã hóa và bảo vệ hệ thống trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0308** | `mismatched types: expected '&[u8]', found '&str'` | Nhầm lẫn giữa chuỗi ký tự UTF-8 văn bản (`&str`) và lát cắt mảng byte thô (`&[u8]`) khi so sánh mã hóa. | Gọi phương thức `.as_bytes()` trên chuỗi ký tự, hoặc sử dụng tiền tố byte literal `b"..."`. |
| **E0596** | `cannot borrow 'security_gate' as mutable, as it is not declared as mutable` | Cố gắng gọi một phương thức thay đổi trạng thái nội bộ mà đối tượng không được khai báo với từ khóa `mut`. | Thêm từ khóa `mut` vào biến khi khởi tạo: `let mut security_gate = ...`. |
| **E0425** | `cannot find value 'SECRET_KEY' in this scope` | Truy cập một biến toàn cục hoặc cấu hình bí mật chưa được định nghĩa hoặc nằm ngoài tầm vực module. | Đảm bảo biến được khai báo với `const` hoặc `static`, và đưa vào tầm vực thông qua `use`. |
| **E0369** | `binary operation '>=' cannot be applied to type 'UserRole'` | Cố gắng so sánh thứ tự lớn hơn nhỏ hơn (`current_role >= required_role`) trên một `enum` chưa triển khai trait `PartialOrd`. (Nếu kiểu được dùng ở chỗ *đòi* trait, ví dụ `BTreeMap<UserRole, _>` hay `.max()`, lỗi sẽ là **E0277** `the trait bound 'UserRole: Ord' is not satisfied`.) | Thêm macro derive tự động: `#[derive(PartialEq, Eq, PartialOrd, Ord)]` lên trên định nghĩa `enum`. |

### Ví dụ phân tích lỗi `E0369` khi so sánh phân quyền Enum:

```rust
// Đoạn mã lỗi minh họa E0369:
#[derive(Debug, PartialEq)] // Quên thêm PartialOrd
enum RankBroken {
    Staff,
    Director,
}

fn authz_broken(cap: RankBroken) {
    // if cap >= RankBroken::Director { ... } // LỖI E0369: Không thể dùng toán tử >= trên RankBroken!
}

// Cách sửa chữa đúng chuẩn: Triển khai đầy đủ PartialOrd và Ord
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Rank {
    Staff = 1,
    Director = 2,
}

fn authz_correct(cap: Rank) {
    if cap >= Rank::Director {
        println!("Chào mừng Giám đốc điều hành!");
    }
}
```

---

## Kiểm thử tự động (Automated Tests)

Mã bảo mật cần test cho cả hai chiều: thứ *phải* được chấp nhận và thứ *phải* bị từ chối. Test thứ hai dưới đây đáng chú ý: một chữ `а` Kirin trông y hệt chữ `a` Latinh — nếu danh sách trắng dùng `is_alphanumeric` (chấp nhận mọi chữ cái Unicode) thì tên lệnh giả mạo sẽ lọt qua. Lưu ý test **không** đo thời gian: tính hằng thời gian không kiểm được bằng unit test, chỉ bằng phân tích mã máy hoặc công cụ chuyên dụng (như `dudect`).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_time_compare_is_correct() {
        assert!(constant_time_compare(b"abc", b"abc"));
        assert!(!constant_time_compare(b"abc", b"abd"));
        assert!(!constant_time_compare(b"abc", b"xbc"));
        assert!(!constant_time_compare(b"abc", b"abcd"));
        assert!(constant_time_compare(b"", b""));
    }

    #[test]
    fn whitelist_is_ascii_only() {
        let gate = SecurityGateEngine::new(b"k");
        assert!(gate.sanitize_command_input("get_status-2 now").is_ok());
        assert!(gate.sanitize_command_input("rm -rf /").is_err());
        // Chữ 'а' Kirin (U+0430) trông giống hệt 'a' Latinh: is_alphanumeric chấp nhận nó,
        // danh sách trắng ASCII thì không.
        assert!(gate.sanitize_command_input("\u{0430}dmin").is_err());
        assert!(gate.sanitize_command_input("").is_err());
        assert!(gate.sanitize_command_input(&"A".repeat(65)).is_err());
    }

    #[test]
    fn least_privilege_ordering() {
        let gate = SecurityGateEngine::new(b"k");
        assert!(
            gate.verify_permission(UserRole::Administrator, UserRole::Auditor)
                .is_ok()
        );
        assert!(
            gate.verify_permission(UserRole::Guest, UserRole::Member)
                .is_err()
        );
    }
}
```

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Tư duy OSCP thực chiến**: Hiểu rõ từng bước đi của kẻ tấn công từ trinh sát, dò quét cổng mạng, xâm nhập ban đầu cho đến leo thang đặc quyền để thiết kế hệ thống miễn nhiễm từ gốc.
2. **Mô hình hóa STRIDE**: Hệ thống hóa 6 hiểm họa cốt lõi (Spoofing, Tampering, Repudiation, Information Disclosure, Denial of Service, Elevation of Privilege) để chủ động xây dựng phương án khắc phục.
3. **Phòng thủ chiều sâu (Defense-in-Depth)**: Tận dụng tối đa các lớp giáp của hệ điều hành (ASLR, DEP/NX, Stack Canaries) kết hợp cùng cấu hình gia cố `Cargo.toml` (`panic = "abort"`, `overflow-checks = true`, `lto = true`).
4. **Ngăn chặn Tấn công Kênh Kề (Timing Attacks)**: Luôn sử dụng so sánh thời gian bất biến (Constant-Time) cho các dữ liệu bí mật và áp dụng nguyên tắc quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để bảo vệ toàn vẹn tài nguyên hệ thống.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bộ hạn chế tần suất thử mật khẩu - Rate Limiter)**:  
   Viết một cấu trúc `LoginRateLimiter` theo dõi số lần đăng nhập thất bại của một địa chỉ IP. Nếu một IP thử sai mật khẩu quá 5 lần trong vòng 60 giây, khóa tạm thời IP đó trong 5 phút để triệt tiêu các cuộc tấn công Brute-Force mật mã.
2. **Bài tập 2 (Bộ tạo Token ngẫu nhiên an toàn mật mã)**:  
   Viết hàm sinh một chuỗi khóa bí mật 32 bytes ngẫu nhiên chuẩn an toàn mật mã (Cryptographically Secure Pseudo-Random Number) mà không sử dụng bộ sinh giả ngẫu nhiên không-mật-mã (như `rand()` của C, `Math.random()` của JavaScript, hay một PRNG nhanh được seed bằng thời gian). Giải thích vì sao việc dùng hàm ngẫu nhiên yếu lại là lỗ hổng nghiêm trọng trong các bài thi OSCP.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Tại sao `panic = "abort"` lại tăng tính bảo mật?)**:  
   Khi một chương trình Rust gặp lỗi `panic!`, mặc định nó sẽ thực hiện quy trình "Cuộn ngược ngăn xếp (Stack Unwinding)" để dọn dẹp các biến. Tại sao việc chuyển sang `panic = "abort"` (tắt tiến trình ngay lập tức) lại giúp thu nhỏ kích thước nhị phân và loại bỏ các đoạn mã máy thừa thãi (gadgets) mà kẻ tấn công có thể lợi dụng để xây dựng chuỗi ROP (Return-Oriented Programming)?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Bộ hạn chế tần suất theo dõi mỗi IP: số lần sai + mốc thời gian bắt đầu cửa sổ. Quá 5 lần trong 60 giây thì đặt mốc khóa 5 phút. Dùng `Instant` cho thời gian đơn điệu.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

pub struct LoginRateLimiter {
    // Mỗi IP: (số lần sai trong cửa sổ, thời điểm bắt đầu cửa sổ, thời điểm hết khóa nếu có)
    tracked: HashMap<String, (u32, Instant, Option<Instant>)>,
}

impl LoginRateLimiter {
    pub fn new() -> Self {
        Self { tracked: HashMap::new() }
    }

    /// Gọi khi có lần thử SAI. Trả về true nếu IP hiện đang bị khóa.
    pub fn record_failure(&mut self, ip: &str, now: Instant) -> bool {
        let e = self.tracked.entry(ip.to_string())
            .or_insert((0, now, None));

        // Đang trong thời gian khóa? -> vẫn khóa.
        if let Some(locked_until) = e.2 {
            if now < locked_until { return true; }
            *e = (0, now, None); // hết hạn khóa -> làm mới
        }

        // Cửa sổ 60s trôi qua -> đếm lại từ đầu.
        if now.duration_since(e.1) > Duration::from_secs(60) {
            *e = (1, now, None);
            return false;
        }

        e.0 += 1;
        if e.0 > 5 {
            // Quá 5 lần sai trong 60s -> khóa 5 phút.
            e.2 = Some(now + Duration::from_secs(5 * 60));
            return true;
        }
        false
    }

    pub fn is_locked(&self, ip: &str, now: Instant) -> bool {
        matches!(self.tracked.get(ip), Some((_, _, Some(until))) if now < *until)
    }
}

#[test]
fn locks_after_more_than_5_failures() {
    let mut rl = LoginRateLimiter::new();
    let t0 = Instant::now();
    // 5 lần sai đầu: chưa khóa.
    for _ in 0..5 { assert!(!rl.record_failure("1.2.3.4", t0)); }
    // Lần thứ 6 trong cửa sổ 60s -> khóa.
    assert!(rl.record_failure("1.2.3.4", t0));
    assert!(rl.is_locked("1.2.3.4", t0));
    // IP khác không bị ảnh hưởng.
    assert!(!rl.is_locked("9.9.9.9", t0));
}
```

Vì sao thiết kế này chặn brute-force: tấn công dò mật khẩu dựa vào **thử thật nhiều lần thật nhanh**. Giới hạn 5 lần/60 giây rồi khóa 5 phút biến một cuộc dò hàng triệu mật khẩu/giây thành vài lần mỗi 5 phút — chậm tới mức vô dụng. Chi tiết đúng đắn: dùng `Instant` (đồng hồ *đơn điệu*, không bao giờ chạy lùi) chứ không dùng `SystemTime` (giờ hệ thống có thể bị chỉnh, kẻ tấn công lợi dụng để lách khóa). Truyền `now` vào làm tham số giúp *kiểm thử được* — không phụ thuộc đồng hồ thật.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Ngẫu nhiên an toàn mật mã KHÔNG lấy từ thuật toán giả ngẫu nhiên, mà lấy từ **nguồn entropy của hệ điều hành**. Trên Unix đó là `/dev/urandom`; std đọc được trực tiếp.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
use std::fs::File;
use std::io::Read;

/// Sinh 32 byte ngẫu nhiên AN TOÀN MẬT MÃ, lấy từ nguồn entropy của HĐH.
/// KHÔNG dùng PRNG không-mật-mã seed bằng thời gian — xem giải thích bên dưới.
fn generate_secret_token() -> std::io::Result<[u8; 32]> {
    let mut token = [0u8; 32];
    // /dev/urandom là nguồn ngẫu nhiên mật mã do nhân HĐH nuôi bằng entropy
    // phần cứng (nhiễu nhiệt, thời điểm ngắt...). Không thể đoán trước.
    let mut f = File::open("/dev/urandom")?;
    f.read_exact(&mut token)?; // đọc đúng 32 byte
    Ok(token)
}

#[test]
fn token_is_32_bytes_and_unique() {
    let a = generate_secret_token().unwrap();
    let b = generate_secret_token().unwrap();
    assert_eq!(a.len(), 32);
    // Hai lần sinh gần như chắc chắn khác nhau (xác suất trùng ~ 1/2^256).
    assert_ne!(a, b);
}
```

*(Trong dự án thật, dùng crate `getrandom` để lấy cùng nguồn này một cách đa nền tảng — nó gọi `getrandom(2)` trên Linux, `BCryptGenRandom` trên Windows. Ở đây ta đọc thẳng `/dev/urandom` để thấy rõ bản chất.)*

**Vì sao dùng ngẫu nhiên yếu là lỗ hổng nghiêm trọng (bối cảnh OSCP):**

Một bộ sinh giả ngẫu nhiên thường (như `rand()` của C, `java.util.Random`, hay `SmallRng`/`StdRng::seed_from_u64(...)` của crate `rand` khi bạn tự seed bằng thời gian) là **thuật toán tất định**: từ một "hạt giống" (seed), nó sinh ra một chuỗi số *hoàn toàn xác định*. Nếu kẻ tấn công đoán được seed, chúng tái tạo được **toàn bộ** chuỗi token.

Điều chết người là seed thường lấy từ những nguồn **đoán được**:
- Thời gian hệ thống (giây kể từ 1970) — không gian tìm kiếm rất nhỏ.
- ID tiến trình — vài chục nghìn khả năng.

Trong một bài OSCP, nếu máy chủ sinh token phiên (session token) hay token đặt-lại-mật-khẩu bằng bộ giả ngẫu nhiên seed theo thời gian, kẻ tấn công chỉ cần **dò seed quanh thời điểm đăng nhập** để tái tạo token phiên của quản trị viên — chiếm phiên mà *không cần biết mật khẩu*. Đây là một lớp lỗ hổng có thật, được xếp hạng CWE-338 (dùng PRNG yếu về mật mã).

(Công bằng mà nói: `rand::random()` của crate `rand` hiện nay dùng `ThreadRng` — một CSPRNG được seed từ hệ điều hành — nên không thuộc nhóm yếu này; cái bẫy nằm ở PRNG "nhanh" và ở việc tự seed bằng giá trị đoán được.)

Nguồn mật mã (`/dev/urandom`) khác về bản chất: nhân HĐH nuôi nó bằng **entropy phần cứng** (nhiễu nhiệt, thời điểm các ngắt) mà không ai đoán hay tái tạo được — kể cả khi biết mọi token đã sinh trước đó, cũng không suy ra được token tiếp theo. Quy tắc sắt: **bất cứ thứ gì làm bí mật — khóa, token, salt, IV — phải sinh từ nguồn mật mã, không bao giờ từ PRNG thường.**
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Cuộn ngược ngăn xếp (stack unwinding) đòi trình biên dịch chèn mã dọn dẹp tại mỗi khung hàm. `panic = "abort"` bỏ hết phần đó — nhỏ hơn và ít 'nguyên liệu' cho kẻ tấn công.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

`panic = "abort"` cải thiện bảo mật (và kích thước) bằng cách **loại bỏ toàn bộ cơ chế cuộn ngược ngăn xếp**.

**Mặc định — cuộn ngược (unwinding):** khi `panic!` xảy ra, Rust đi ngược ngăn xếp lời gọi, chạy `Drop` để dọn dẹp từng biến (đóng tệp, giải phóng bộ nhớ...) cho tới khi tới đỉnh luồng. Để làm được, trình biên dịch phải **chèn mã và bảng dọn dẹp (landing pads) tại mỗi khung hàm** có gì đó cần dọn.

**`panic = "abort"`:** khi `panic!` xảy ra, tiến trình **dừng ngay lập tức** (gọi `abort`), không dọn dẹp gì cả — giao lại toàn bộ cho hệ điều hành thu hồi.

**Lợi ích 1 — thu nhỏ nhị phân:** bỏ cuộn ngược nghĩa là bỏ hết mã dọn dẹp và bảng unwind rải khắp mọi hàm. Với nhị phân nhỏ, phần này có thể chiếm 10% trở lên. Đây là lý do nhị phân nhúng và WebAssembly gần như luôn bật `panic = "abort"`.

**Lợi ích 2 — bớt 'nguyên liệu' cho kẻ tấn công (giảm bề mặt tấn công):**

Đây là ý sâu hơn. Trong kỹ thuật khai thác **ROP (Return-Oriented Programming)**, kẻ tấn công không tiêm mã mới (bộ nhớ thường đánh dấu không-thực-thi), mà **xâu chuỗi lại những mẩu mã máy có sẵn** trong nhị phân — gọi là *gadget*, thường là các đoạn kết thúc bằng lệnh `ret`. Càng nhiều mã trong nhị phân, càng nhiều gadget để ghép thành chuỗi tấn công.

Mã cuộn ngược chính là **một kho gadget dồi dào**: nó đầy những đoạn dọn dẹp, gọi hủy, thao tác con trỏ khung — rải rác khắp nơi và kết thúc bằng `ret`. Bỏ nó đi (`panic = "abort"`):
- **Nhị phân nhỏ hơn -> ít gadget hơn** để kẻ tấn công lựa chọn.
- **Xóa hẳn một lớp đường thực thi phức tạp** (bộ máy unwinding) mà bản thân nó từng là nguồn của lỗi bảo mật.

Nói gọn: cuộn ngược cho ta dọn dẹp *duyên dáng* khi panic, nhưng đổi lại phình nhị phân và tặng kẻ tấn công thêm nguyên liệu. Với dịch vụ mà chiến lược xử lý panic là "chết ngay và để bộ giám sát khởi động lại" (triết lý phổ biến cho máy chủ), thì `panic = "abort"` vừa nhẹ hơn vừa an toàn hơn — một đánh đổi hời.
</details>
