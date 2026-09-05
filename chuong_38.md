# Chương 38: Tư duy tấn công thực chiến OSCP, Mô hình hóa mối đe dọa & Gia cố bảo mật hệ thống (OSCP Offensive Mindset, Threat Modeling & Hardening)

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
│ Hệ điều hành thấy chim bị đổi số ──► Lập tức cắt điện tắt máy bảo vệ hệ thống!   │
│                                                                                  │
│ [LỚP 4: CỬA HẦM CHỐNG BOM & QUYỀN TỐI THIỂU (LEAST PRIVILEGE)]                   │
│ Ngay cả khi tên trộm lẻn được vào quầy giao dịch, cửa hầm chứa tiền vẫn khóa chặt.│
│ Nhân viên kế toán chỉ có chìa khóa mở ngăn kéo đựng bút, không ai có quyền Root! │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Chu trình tấn công OSCP giống như kế hoạch đột nhập dinh thự
- **Giai đoạn 1 (Reconnaissance - Trinh sát)**: Kẻ trộm đi vòng quanh dinh thự, ghi chép giờ giấc sinh hoạt của chủ nhà, xem tường rào cao bao nhiêu mét.
- **Giai đoạn 2 (Scanning & Enumeration - Dò xét cửa mở)**: Tên trộm đến từng cánh cửa sổ, lay thử then cài xem có then nào bị lỏng (giống như chạy Port Scanner ở Chương 36 để tìm cổng mạng mở).
- **Giai đoạn 3 (Initial Foothold - Đột nhập ban đầu)**: Phát hiện cửa sổ phòng bếp hé mở, tên trộm trèo vào được bên trong phòng bếp (chiếm được một tài khoản người dùng bình thường không có quyền admin).
- **Giai đoạn 4 (Privilege Escalation - Leo thang đặc quyền)**: Từ phòng bếp, tên trộm tìm kiếm chìa khóa vạn năng của quản gia để mở cửa phòng điều khiển trung tâm (chiếm quyền Quản trị viên tối cao `root` hoặc `SYSTEM`).

### 2. Chim hoàng yến trong hầm than (Stack Canary)
- Khi đào than dưới lòng đất, hiểm họa vô hình lớn nhất là khí độc methane không mùi không màu. Thợ mỏ luôn treo một chiếc lồng có chú chim hoàng yến bên cạnh. Cơ thể chim rất nhạy cảm; nếu có khí độc, chim sẽ lảo đảo ngất xỉu trước khi con người kịp nhận ra nguy hiểm.
- Trong ngăn xếp máy tính, **Stack Canary** là một giá trị số ngẫu nhiên được trình biên dịch tự động đặt vào ngay phía trước con trỏ địa chỉ trả về `Saved RIP`.
- Kẻ tấn công muốn tràn bộ đệm đè lên `RIP` thì bắt buộc phải đè qua giá trị Canary này. Trước khi hàm kết thúc, CPU liếc nhìn lại giá trị con chim: Nếu thấy giá trị bị biến dạng, CPU lập tức kích hoạt lệnh hủy khẩn cấp (`__stack_chk_fail`), dập tắt hoàn toàn âm mưu của kẻ tấn công!

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
strip = true             # Gọt bỏ toàn bộ bảng biểu tượng Symbol Table để chống dịch ngược
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
- **Giải pháp**: Phải sử dụng **So sánh thời gian bất biến (Constant-Time Comparison)**: Luôn luôn so sánh đủ 100% các byte bất kể đúng hay sai, khiến thời gian phản hồi luôn luôn bằng nhau, triệt tiêu hoàn toàn khả năng do thám của kẻ tấn công.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là chương trình Rust hoàn chỉnh hiện thực hóa một động cơ kiểm tra bảo mật cấp doanh nghiệp: Tích hợp cơ chế so sánh thời gian bất biến (Constant-time token validation) chống tấn công Timing Attack, cùng bộ lọc làm sạch đầu vào theo chuẩn mô hình STRIDE:

```rust
use std::hint::black_box;

/// Hàm so sánh mảng byte với thời gian bất biến (Constant-Time Comparison)
/// Tuyệt đối không kết thúc sớm khi gặp byte sai, ngăn chặn Timing Attack 100%!
pub fn constant_time_compare(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut difference_accumulator: u8 = 0;

    // Duyệt qua toàn bộ các phần tử mà không dùng lệnh 'break' hay 'return' sớm
    for (byte_a, byte_b) in a.iter().zip(b.iter()) {
        // Phép XOR: Nếu hai byte giống nhau thì kết quả bằng 0, khác nhau thì khác 0
        difference_accumulator |= byte_a ^ byte_b;
    }

    // Đảm bảo trình biên dịch không tối ưu hóa làm biến mất vòng lặp
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
            return Err("Dau vao trong: Tu choi xu ly!");
        }

        if raw_input.len() > 64 {
            return Err("Dau vao qua dai: Nguy co tran bo dem hoac DoS bi chan dung!");
        }

        // Nguyên tắc Whitelist: Chỉ cho phép chữ cái, chữ số, gạch dưới và khoảng trắng
        let is_safe = raw_input
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == ' ');

        if !is_safe {
            return Err("Phat hien ky tu nguy hiem (SQL/Shell Injection blocked)!");
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
            Err("Tu choi truy cap: Khong du dac quyen (Elevation of Privilege blocked)!")
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   GIA CO HE THONG RUST & MO HINH HOA MOI DE DOA STRIDE / OSCP    ");
    println!("==================================================================");

    // Khởi tạo động cơ an ninh với Master Token bí mật 16 bytes
    let master_token = b"OSCP_RUST_KEY_99";
    let security_gate = SecurityGateEngine::new(master_token);

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM CHỐNG TẤN CÔNG TIMING ATTACK QUA CONSTANT-TIME
    // -------------------------------------------------------------
    println!("\n[1] Kiem chung so sanh thoi gian bat bien (Constant-Time):");
    let valid_attempt = b"OSCP_RUST_KEY_99";
    let wrong_first_byte = b"XSCP_RUST_KEY_99";
    let wrong_last_byte = b"OSCP_RUST_KEY_00";

    println!(
        "    - Thu token hop le      : {}",
        security_gate.authenticate_token(valid_attempt)
    );
    println!(
        "    - Thu token sai byte dau : {}",
        security_gate.authenticate_token(wrong_first_byte)
    );
    println!(
        "    - Thu token sai byte cuoi: {}",
        security_gate.authenticate_token(wrong_last_byte)
    );
    println!("    => Moi phep so sanh deu duyet 100% mang byte voi thoi gian dong nhat!");

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM LÀM SẠCH ĐẦU VÀO CHỐNG INJECTION & BUFFER FLOOD
    // -------------------------------------------------------------
    println!("\n[2] Kiem thu lam sach du lieu dau vao (Input Sanitization):");

    let safe_input = "get_system_status";
    match security_gate.sanitize_command_input(safe_input) {
        Ok(clean) => println!("    - Lenh an toan duoc chap nhan: '{}'", clean),
        Err(err) => println!("    [!] Tu choi: {}", err),
    }

    let malicious_injection = "get_status; rm -rf /; --";
    println!("    - Thu gui payload doc hai: '{}'", malicious_injection);
    match security_gate.sanitize_command_input(malicious_injection) {
        Ok(_) => println!("    [!] [CANH BAO] Lenh doc hai da lot qua!"),
        Err(err) => println!("    [+] [CHAN DUNG AN TOAN] {}", err),
    }

    let overflow_dos_attempt = "A".repeat(128);
    println!("    - Thu gui chuoi tan cong DoS dai {} bytes...", overflow_dos_attempt.len());
    match security_gate.sanitize_command_input(&overflow_dos_attempt) {
        Ok(_) => println!("    [!] [CANH BAO] Payload DoS da duoc chap nhan!"),
        Err(err) => println!("    [+] [CHAN DUNG AN TOAN] {}", err),
    }

    // -------------------------------------------------------------
    // 3. THỬ NGHIỆM KIỂM SOÁT PHÂN QUYỀN TỐI THIỂU (LEAST PRIVILEGE)
    // -------------------------------------------------------------
    println!("\n[3] Kiem tra kiem soat phan quyen truy cap (RBAC):");
    let user_role = UserRole::Member;
    println!("    - Nguoi dung dang co vai tro: {:?}", user_role);

    let audit_access = security_gate.verify_permission(user_role, UserRole::Auditor);
    println!("    - Yeu cau truy cap vung Auditor: {:?}", audit_access);
    assert!(audit_access.is_err());

    let member_access = security_gate.verify_permission(user_role, UserRole::Member);
    println!("    - Yeu cau truy cap vung Member : {:?}", member_access);
    assert!(member_access.is_ok());
    println!("    => Ngăn chan triet de nguy co Leo thang dac quyen (Elevation of Privilege)!");

    println!("\n==================================================================");
    println!("   XAC NHAN: HE THONG PHONG THU CHIEU SAU SAN SANG HOAT DONG!    ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi triển khai các cơ chế an ninh, mã hóa và bảo vệ hệ thống trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0308** | `mismatched types: expected '&[u8]', found '&str'` | Nhầm lẫn giữa chuỗi ký tự UTF-8 văn bản (`&str`) và lát cắt mảng byte thô (`&[u8]`) khi so sánh mã hóa. | Gọi phương thức `.as_bytes()` trên chuỗi ký tự, hoặc sử dụng tiền tố byte literal `b"..."`. |
| **E0596** | `cannot borrow 'security_gate' as mutable` | Cố gắng gọi một phương thức thay đổi trạng thái nội bộ mà đối tượng không được khai báo với từ khóa `mut`. | Thêm từ khóa `mut` vào biến khi khởi tạo: `let mut security_gate = ...`. |
| **E0425** | `cannot find value 'SECRET_KEY' in this scope` | Truy cập một biến toàn cục hoặc cấu hình bí mật chưa được định nghĩa hoặc nằm ngoài tầm vực module. | Đảm bảo biến được khai báo với `const` hoặc `static`, và đưa vào tầm vực thông qua `use`. |
| **E0277** | `the trait 'Ord' is not implemented for 'UserRole'` | Cố gắng so sánh thứ tự lớn hơn nhỏ hơn (`current_role >= required_role`) trên một `enum` chưa triển khai trait `PartialOrd` và `Ord`. | Thêm macro derive tự động: `#[derive(PartialEq, Eq, PartialOrd, Ord)]` lên trên định nghĩa `enum`. |

### Ví dụ phân tích lỗi `E0277` khi so sánh phân quyền Enum:

```rust
// Đoạn mã lỗi minh họa E0277:
#[derive(Debug, PartialEq)] // Quên thêm PartialOrd
enum CapBacLoi {
    NhanVien,
    GiamDoc,
}

fn kiem_tra_quyen_loi(cap: CapBacLoi) {
    // if cap >= CapBacLoi::GiamDoc { ... } // LỖI E0277: Không thể dùng toán tử >= trên CapBacLoi!
}

// Cách sửa chữa đúng chuẩn: Triển khai đầy đủ PartialOrd và Ord
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum CapBacDung {
    NhanVien = 1,
    GiamDoc = 2,
}

fn kiem_tra_quyen_dung(cap: CapBacDung) {
    if cap >= CapBacDung::GiamDoc {
        println!("Chào mừng Giám đốc điều hành!");
    }
}
```

---

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
   Viết hàm sinh một chuỗi khóa bí mật 32 bytes ngẫu nhiên chuẩn an toàn mật mã (Cryptographically Secure Pseudo-Random Number) mà không sử dụng thuật toán giả ngẫu nhiên yếu như `rand::random()`. Giải thích vì sao việc dùng hàm ngẫu nhiên yếu lại là lỗ hổng nghiêm trọng trong các bài thi OSCP.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Tại sao `panic = "abort"` lại tăng tính bảo mật?)**:  
   Khi một chương trình Rust gặp lỗi `panic!`, mặc định nó sẽ thực hiện quy trình "Cuộn ngược ngăn xếp (Stack Unwinding)" để dọn dẹp các biến. Tại sao việc chuyển sang `panic = "abort"` (tắt tiến trình ngay lập tức) lại giúp thu nhỏ kích thước nhị phân và loại bỏ các đoạn mã máy thừa thãi (gadgets) mà kẻ tấn công có thể lợi dụng để xây dựng chuỗi ROP (Return-Oriented Programming)?
