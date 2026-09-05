# Chương 38: Tam đại hiểm họa tham nhũng bộ nhớ: Buffer Overflow, Use-After-Free & Format Strings (Memory Corruption: Buffer Overflow, UAF & Format Strings)

## Giới thiệu & Mục tiêu học tập

Trong lịch sử hơn 50 năm của ngành khoa học máy tính, có một sự thật gây kinh ngạc cho bất kỳ ai mới bước chân vào lĩnh vực an toàn thông tin: **Khoảng 70% toàn bộ các lỗ hổng bảo mật nghiêm trọng (CVE) được phát hiện hàng năm trong các phần mềm lớn của Microsoft (Windows, Office) và Google (Chromium, Android) đều xuất phát từ cùng một thủ phạm duy nhất: Các lỗi tham nhũng bộ nhớ (Memory Corruption Bugs).**

Những lỗi này không bắt nguồn từ thuật toán nghiệp vụ sai hay thiếu sót tính năng, mà phát sinh từ sự lỏng lẻo trong việc quản lý bộ đệm và con trỏ của các ngôn ngữ lập trình truyền thống như C và C++. Trong chương này, chúng ta sẽ mổ xẻ "Tam đại hiểm họa" kinh điển nhất trong thế giới nhị phân:
1. **Tràn bộ đệm (Buffer Overflow)**: Kẻ tấn công ghi đè dữ liệu vượt ngoài biên vùng nhớ được cấp phát để cướp quyền điều khiển thanh ghi con trỏ lệnh `RIP`.
2. **Sử dụng vùng nhớ sau giải phóng (Use-After-Free - UAF)**: Đọc hoặc ghi vào ô nhớ trên Heap sau khi đã bị thu hồi, dẫn tới nguy cơ thực thi mã từ xa (RCE).
3. **Lỗ hổng chuỗi định dạng (Format String)**: Lợi dụng hàm in ấn dữ liệu thiếu kiểm tra kiểu để đọc trộm hoặc ghi đè tùy ý lên ngăn xếp.

Mục tiêu học tập của bạn:
- Nắm vững cơ chế giải phẫu của từng loại lỗ hổng ở cấp độ thanh ghi và ô nhớ mà không cần tính toán số học phức tạp.
- Hiểu được kỹ thuật khai thác cơ bản: Làm thế nào một mảng byte tràn có thể bẻ lái luồng chạy của CPU.
- Khám phá cách hệ thống kiểu dữ liệu, cơ chế kiểm tra biên tự động, và trình kiểm tra mượn (borrow checker) của Rust giúp tiêu diệt hoàn toàn cả 3 hiểm họa này ngay từ khâu biên dịch và thực thi.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng hình tượng hóa 3 lỗ hổng nguy hiểm này qua những tình huống đời thực sinh động:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA TAM ĐẠI HIỂM HỌA THAM NHŨNG BỘ NHỚ                  │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. BUFFER OVERFLOW: RÓT NƯỚC LÀM CHÁY LAPTOP KẾ BÊN]                           │
│ ┌───────────────────────┐ ┌──────────────────────────────────────────┐          │
│ │ Chiếc cốc 200ml       │ │ Chiếc Laptop chứa tài liệu mật           │          │
│ │ (Bộ đệm mảng 200B)    │ │ (Saved Return Address - Con trỏ RIP)     │          │
│ ├───────────────────────┤ ├──────────────────────────────────────────┤          │
│ │ Rót cố tình 500ml...  │ │ Nước tràn ra bàn làm chập cháy bo mạch!  │          │
│ │ ~~~~~~~~~~~~~~~~~~~~~ │─┼─────────────────────────────────────────►│          │
│ └───────────────────────┘ └──────────────────────────────────────────┘          │
│                                                                                  │
│ [2. USE-AFTER-FREE: GIỮ LÉN CHÌA KHÓA PHÒNG TRỌ ĐÃ TRẢ]                          │
│ Bạn thuê phòng trọ số 5 ──► Trả phòng (Free) nhưng giữ lại chìa khóa cũ (UAF)    │
│ Hôm sau khách VIP mới vào ở ──► Bạn dùng chìa cũ mở cửa vào quậy phá!            │
│                                                                                  │
│ [3. FORMAT STRING: TỜ PHIẾU ĐẶT HÀNG GHI MÃ BÍ MẬT]                              │
│ Khách hàng điền vào ô Tên món: "%s %x %x (Đọc két sắt cho tôi)"                  │
│ Nhân viên thu ngân ngây thơ đọc to toàn bộ sổ cái kế toán trước mặt mọi người!   │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Rót nước làm cháy laptop (Buffer Overflow)
- Hãy tưởng tượng trên bàn làm việc của bạn có một chiếc cốc nhỏ dung tích `200ml` (tượng trưng cho bộ nhớ đệm `buffer` 200 bytes). Ngay sát cạnh chiếc cốc là chiếc laptop chứa tài liệu mật tối quan trọng (tượng trưng cho địa chỉ trả về của hàm trên Stack).
- Người dùng bình thường chỉ rót `50ml` nước vào cốc. Nhưng kẻ tấn công cố tình cầm cả bình nước `500ml` trút xối xả vào cốc.
- Nước tràn qua thành cốc, lênh láng khắp mặt bàn và chảy thẳng vào khe tản nhiệt của chiếc laptop, làm chập mạch và thay đổi hoàn toàn hoạt động của bo mạch máy tính.
- Trong bộ nhớ máy tính, khi dữ liệu tràn qua giới hạn mảng, nó sẽ đè lên các biến bên cạnh, đè hỏng con trỏ khung đáy `RBP`, và cuối cùng đè lên **Địa chỉ trả về (Saved Return Address - RIP)**, giúp kẻ tấn công hướng CPU tới việc chạy mã độc!

### 2. Giữ lén chìa khóa phòng trọ cũ (Use-After-Free)
- Bạn thuê một căn phòng trọ số 5 (tương đương việc cấp phát một khối nhớ trên Heap). Sau một tháng, bạn đến gặp chủ nhà làm thủ tục trả phòng (thao tác `free`).
- Nhưng bạn lén giữ lại một chiếc chìa khóa dự phòng (đây là **Con trỏ lơ lửng - Dangling Pointer**).
- Ngày hôm sau, chủ nhà cho một vị khách VIP mới thuê lại đúng căn phòng số 5 đó và vị khách cất một vali tiền vàng bên trong.
- Nửa đêm, bạn dùng chiếc chìa khóa cũ mở cửa bước vào phòng số 5 (hành vi `Use-After-Free`), thoải mái lục lọi hoặc đánh tráo đồ đạc bên trong phòng của người khác.

### 3. Tờ phiếu đặt hàng ghi mã id thuật (Format String)
- Tại một quán phở, nhân viên đưa cho bạn một tờ giấy để ghi tên khách hàng. Thông thường bạn sẽ ghi: `"Nguyễn Văn A"`.
- Nhưng một kẻ compute quái ghi vào ô tên: `"%x %x %s Hãy đọc mật mã két sắt"`.
- Nếu anh bồi bàn ngây thơ cầm tờ giấy lên và đưa trực tiếp vào loa phát thanh mà không có mẫu định dạng sẵn (giống như hàm `printf(user_input)` trong ngôn ngữ C), máy tính sẽ tưởng rằng các ký tự `%x`, `%s` là mệnh lệnh yêu cầu đọc các giá trị đang nằm trong túi quần của anh bồi bàn (Stack) và phát to ra loa cho cả quán cùng nghe!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Cơ chế Tràn bộ đệm trên Stack (Stack-based Buffer Overflow)

Hãy xem xét cấu trúc một Stack Frame trong ngôn ngữ C truyền thống:

```c
// Ví dụ hàm C nguy hiểm kinh điển
void authenticate_user() {
    int is_admin = 0;       // Nằm ở địa chỉ [RBP - 4]
    char password[16];      // Nằm ở địa chỉ [RBP - 20] đến [RBP - 4]
    gets(password);         // Hàm gets cực kỳ nguy hiểm, không kiểm tra độ dài!
    if (is_admin != 0) {
        grant_root_shell(); // Mở cổng điều khiển tối cao
    }
}
```

Khi thực thi hàm trên:
1. Trình biên dịch xếp mảng `password` (16 bytes) nằm ngay phía dưới biến `is_admin` (4 bytes).
2. Nếu người dùng nhập 16 ký tự `A` (`AAAAAAAAAAAAAAAA`), mảng `password` vừa đầy.
3. Nếu người dùng nhập 20 ký tự `A`, 4 ký tự cuối cùng sẽ **tràn qua ranh giới** của `password` và ghi đè thẳng vào 4 byte của biến `is_admin`, biến giá trị `0` thành `0x41414141` (khác 0). Kết quả: Kẻ tấn công được cấp quyền Quản trị viên (`root`) mà không cần biết mật khẩu!
4. Nếu nhập dài hơn nữa (khoảng 32 bytes), dữ liệu sẽ đè nát `Saved RBP` và ghi đè lên `Saved RIP`. Khi hàm kết thúc lệnh `ret`, CPU sẽ nhảy thẳng vào địa chỉ do hacker sắp đặt!

### 2. Sử dụng vùng nhớ sau giải phóng (Use-After-Free & Double Free)

Lỗ hổng Use-After-Free xảy ra chủ yếu trên vùng nhớ động `Heap`:
- **Bước 1 (Allocate)**: Chương trình gọi `malloc()` xin cấp phát một khối nhớ chứa cấu trúc người dùng, ví dụ `UserSession` (trong đó có con trỏ hàm chỉ tới logic phân quyền).
- **Bước 2 (Free)**: Người dùng đăng xuất, chương trình gọi `free(session_ptr)` để trả lại ô nhớ cho hệ điều hành. Tuy nhiên, lập trình viên quên gán `session_ptr = NULL`. Con trỏ này trở thành **Dangling Pointer**.
- **Bước 3 (Reallocate / Heap Spraying)**: Kẻ tấn công tạo ra một đối tượng dữ liệu giả mạo (ví dụ gửi một ảnh hoặc văn bản tải lên) có cùng kích thước byte. Trình quản lý Heap sẽ tái sử dụng lại chính khối ô nhớ vừa bị thu hồi đó để chứa dữ liệu độc hại của kẻ tấn công.
- **Bước 4 (Trigger)**: Chương trình vô tình gọi lại `session_ptr->authenticate()`. Thay vì gọi mã gốc, CPU nhảy vào con trỏ độc hại mà kẻ tấn công vừa bơm vào khối nhớ!

### 3. Nguy cơ Lỗ hổng Chuỗi định dạng (Format String)

Trong ngôn ngữ C, hàm `printf` hoạt động dựa trên danh sách tham số biến thiên (`va_list`):
```c
printf("Xin chao %s, ban co %d thong bao", name, count);
```
- Mỗi khi gặp một ký tự định dạng `%`, `printf` sẽ lấy giá trị tiếp theo từ thanh ghi hoặc từ Stack để hiển thị:
  - `%x`: In giá trị 4 byte tiếp theo trên Stack dưới dạng mã Hexadecimal (giúp kẻ tấn công dò tìm địa chỉ bộ nhớ để vượt qua lớp bảo vệ ASLR).
  - `%s`: Đọc chuỗi ký tự tại địa chỉ nằm trên Stack (giúp đọc trộm mật khẩu, khóa bí mật).
  - `%n`: **Ghi số lượng ký tự đã in vào địa chỉ trỏ tới trên Stack** — cho phép kẻ tấn công ghi đè tùy ý lên bộ nhớ!

### 4. Cách Rust triệt tiêu Tam đại hiểm họa từ gốc

Rust được thiết kế với triết lý an toàn bộ nhớ tuyệt đối (Memory Safety by Default):
1. **Chống Buffer Overflow**:
   - Mọi thao tác truy cập mảng qua chỉ số `arr[i]` đều được chèn mã kiểm tra biên tự động (`bounds check`). Nếu chỉ số vượt quá kích thước mảng, Rust lập tức kích hoạt `panic!` có kiểm soát, ngăn chặn hoàn toàn việc đọc/ghi lấn sang ô nhớ bên cạnh.
   - Thao tác lấy phần tử an toàn thông qua phương thức `.get(i)` trả về `Option<&T>` buộc lập trình viên phải xử lý trường hợp ngoài biên.
2. **Chống Use-After-Free**:
   - Hệ thống **quyền sở hữu (ownership)** và **thời gian sống (lifetime)**: Trình kiểm tra **mượn (borrow)** của Rust đảm bảo rằng không bao giờ tồn tại một tham chiếu sống lâu hơn dữ liệu mà nó trỏ tới.
   - Khi một vùng nhớ bị hủy (thông qua trait `Drop`), mọi tham chiếu tới nó đều đã hết hiệu lực từ trước đó ở cấp độ biên dịch. Lỗi Double Free và Use-After-Free hoàn toàn bị triệt tiêu!
3. **Chống Format String**:
   - Trong Rust, các macro định dạng như `println!`, `format!`, `eprintln!` phân tích chuỗi định dạng ngay ở thời điểm biên dịch (Compile-time).
   - Tham số đầu tiên bắt buộc phải là một chuỗi hằng số (String Literal), không thể là một biến động do người dùng nhập vào. Trình biên dịch kiểm tra tính tương thích giữa số lượng `{}` và số lượng đối số truyền vào, loại bỏ hoàn toàn khả năng khai thác chuỗi định dạng.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn Rust chứng minh cách ngôn ngữ ngăn chặn triệt để Tam đại hiểm họa thông qua kiểm tra biên an toàn, kiểm soát vòng đời con trỏ thông minh (smart pointer), và định dạng an toàn:

```rust
/// Cấu trúc mô phỏng một phiên đăng nhập người dùng an toàn
#[derive(Debug, Clone)]
pub struct SafeUserSession {
    pub username: String,
    pub is_admin: bool,
}

impl SafeUserSession {
    pub fn new(username: &str, is_admin: bool) -> Self {
        Self {
            username: username.to_string(),
            is_admin,
        }
    }
}

/// Trình xử lý bộ đệm an toàn tuyệt đối chống Buffer Overflow
pub struct SafeBufferManager {
    buffer: [u8; 16], // Bộ đệm cố định 16 bytes
}

impl SafeBufferManager {
    pub fn new() -> Self {
        Self { buffer: [0u8; 16] }
    }

    /// Ghi dữ liệu vào bộ đệm với cơ chế kiểm tra biên chặt chẽ
    pub fn safe_write(&mut self, input_data: &[u8]) -> Result<usize, &'static str> {
        if input_data.len() > self.buffer.len() {
            // Ngăn chặn tràn bộ đệm: Từ chối ghi đè khi dữ liệu quá lớn
            return Err("Kich thuoc du lieu vuot qua gioi han bo dem (Buffer Overflow prevented)!");
        }

        // Sao chép an toàn đúng số lượng byte hợp lệ
        for (idx, &byte) in input_data.iter().enumerate() {
            self.buffer[idx] = byte;
        }

        Ok(input_data.len())
    }

    /// Đọc một byte tại chỉ số xác định mà không gây panic sập chương trình
    pub fn safe_read(&self, index: usize) -> Option<u8> {
        self.buffer.get(index).copied()
    }
}

fn main() {
    println!("==================================================================");
    println!("   KIEM CHUNG AN TOAN BO NHO RUST: TRIET TIEU MEMORY CORRUPTION   ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. KIỂM THỬ PHÒNG CHỐNG TRÀN BỘ ĐỆM (BUFFER OVERFLOW)
    // -------------------------------------------------------------
    println!("\n[1] Thu nghiem phong chong Tran bo dem (Buffer Overflow):");
    let mut manager = SafeBufferManager::new();

    let safe_payload = b"MatKhauAnToan"; // 13 bytes (< 16 bytes)
    match manager.safe_write(safe_payload) {
        Ok(bytes_written) => println!("    - Ghi payload hop le thanh cong: {} bytes", bytes_written),
        Err(err) => println!("    - Loi: {}", err),
    }

    let exploit_payload = b"ChuoiPayloadRatDaiCoTinhLamTranBoNhoDeChiChiemThanhGhiRIP"; // 55 bytes
    println!("    - Thu gui payload tan cong co do dai {} bytes...", exploit_payload.len());
    match manager.safe_write(exploit_payload) {
        Ok(_) => println!("    - [NGUY HIEM] Payload da ghi de thanh cong!"),
        Err(err) => println!("    - [CHẶN ĐỨNG AN TOÀN] Trinh quan ly tu choi: '{}'", err),
    }

    // Đọc ngoài biên an toàn qua Option
    println!("    - Thu doc ky tu tai chi so index = 99:");
    match manager.safe_read(99) {
        Some(val) => println!("    - Gia tri: {}", val),
        None => println!("    - [SAFE BOUNDS] Tra ve None: Chi so ngoai bien duoc xu ly an toan!"),
    }

    // -------------------------------------------------------------
    // 2. KIỂM THỬ PHÒNG CHỐNG USE-AFTER-FREE (UAF)
    // -------------------------------------------------------------
    println!("\n[2] Thu nghiem phong chong Use-After-Free (UAF):");
    {
        let session = Box::new(SafeUserSession::new("ChuyenGiaBaoMat", false));
        println!("    - Khoi tao phien lam viec tai Heap: {:p}", session.as_ref());
        println!("    - Nguoi dung: {}, Admin: {}", session.username, session.is_admin);

        // Trong Rust, khi session ra khoi khoi lenh nay, trait Drop se tu dong
        // giai phong vung nho mot cach sach se. Trinh bien dich Rust tuyet doi
        // CAM moi hanh vi giu lai con tro tham chieu den session sau khi no da chet!
    }
    println!("    - [UAF ELIMINATED] Vung nho da duoc thu hoi tu dong.");
    println!("    - Trinh bien dich dam bao 100% khong con con tro lo lung ton tai!");

    // -------------------------------------------------------------
    // 3. KIỂM THỬ PHÒNG CHỐNG LỖ HỔNG FORMAT STRING
    // -------------------------------------------------------------
    println!("\n[3] Thu nghiem phong chong Lo hong Chuoi dinh dang (Format String):");
    // Giả sử kẻ tấn công cố tình nhập vào chuỗi chứa các mã id thuật độc hại của C
    let malicious_user_input = "%x %x %s %p %n ChiemDoatBoNho";
    println!("    - Chuoi dau vao tu nguoi dung: '{}'", malicious_user_input);

    // Trong C: printf(malicious_user_input) se lam ro ri toan bo Stack.
    // Trong Rust: Chuoi nguoi dung chi la du lieu (data) truyen qua placeholder `{}`
    println!("    - Ket qua in qua Rust format: \"{}\"", malicious_user_input);
    println!("    - [FORMAT STRING SECURE] Rust coi chuoi nguoi dung la chuoi thuan túy,");
    println!("      khong bao gio phan tich cac ky tu '%' thanh lenh thuc thi!");

    println!("\n==================================================================");
    println!("   KET LUAN: RUST LOAI BO HOAN TOAN 70% NGUON GOC LO HONG CVE!   ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch điển hình mà bạn sẽ gặp khi trình biên dịch Rust ngăn chặn các hành vi tiềm ẩn nguy cơ tham nhũng bộ nhớ:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0382** | `use of moved value: 'session'` | Bạn cố gắng sử dụng lại một biến sau khi quyền sở hữu của nó đã bị di chuyển sang hàm hoặc biến khác (chống Use-After-Free). | Sử dụng phương thức `.clone()` nếu muốn tạo bản sao độc lập, hoặc chỉ truyền tham chiếu mượn `&session`. |
| **E0506** | `cannot assign to 'val' because it is borrowed` | Cố gắng thay đổi dữ liệu trong khi một biến khác đang mượn tham chiếu đọc dữ liệu đó (chống Data Race & Iterator Invalidation). | Đảm bảo rằng tham chiếu mượn kết thúc phạm vi sử dụng trước khi thực hiện phép gán thay đổi. |
| **E0499** | `cannot borrow 'buffer' as mutable more than once at a time` | Tạo ra hai tham chiếu mượn khả biến `&mut` cùng một lúc tới cùng một vùng nhớ. | Giới hạn mỗi thời điểm chỉ có duy nhất một tham chiếu `&mut`, hoặc đưa các thao tác vào các khối ngoặc nhọn `{}` riêng biệt. |
| **E0597** | `'local_val' does not live long enough` | Một con trỏ hoặc tham chiếu mượn cố tình sống lâu hơn giá trị thực tế của nó (ngăn chặn Dangling Pointer). | Kéo dài thời gian sống của biến gốc, hoặc lưu trữ dữ liệu trực tiếp thay vì lưu tham chiếu. |

### Ví dụ phân tích lỗi `E0382` giúp ngăn chặn lỗ hổng Use-After-Free:

```rust
// Đoạn mã lỗi minh họa E0382:
fn vi_du_ngan_chan_uaf() {
    let data = Box::new(String::from("BiMatDoanhNghiep"));
    
    // Ham drop() giai phong vung nho tren Heap
    std::mem::drop(data); 

    // LỖI E0382: Trình biên dịch Rust NGĂN CHẶN bạn đọc ô nhớ đã bị giải phóng!
    // println!("Dữ liệu sau khi drop: {}", data);
}

// Cách viết an toàn: Không truy cập biến sau khi đã từ bỏ quyền sở hữu
fn vi_du_an_toan() {
    let data = Box::new(String::from("BiMatDoanhNghiep"));
    println!("Dữ liệu an toàn: {}", data);
    // Vùng nhớ sẽ tự động được dọn dẹp sạch sẽ khi hết phạm vi hàm
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **70% Lỗ hổng an ninh**: Bắt nguồn từ các lỗi thao tác bộ nhớ trực tiếp trong C/C++ như Buffer Overflow, Use-After-Free và Format Strings.
2. **Nguyên lý Buffer Overflow**: Ghi vượt quá dung lượng mảng làm biến dạng dữ liệu kế bên và đè lên Saved Return Address (`RIP`) để chuyển hướng CPU.
3. **Bản chất của Use-After-Free**: Giữ lại con trỏ cũ (Dangling Pointer) sau khi ô nhớ Heap đã giải phóng và tái sử dụng, cho phép kẻ tấn công tráo đổi nội dung đối tượng.
4. **Rust là lá chắn tối thượng**: Cơ chế quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) có kiểm tra biên tự động triệt tiêu hoàn toàn các mối nguy hiểm này từ trong trứng nước.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Xây dựng Bộ đệm vòng an toàn - Safe Ring Buffer)**:  
   Tạo một cấu trúc `SafeRingBuffer` có dung lượng cố định 8 bytes. Cài đặt hai phương thức `push(&mut self, byte: u8)` và `pop(&mut self) -> Option<u8>`. Đảm bảo rằng khi người dùng ghi liên tục 100 bytes, bộ đệm sẽ tự động quay vòng ghi đè các vị trí cũ bên trong giới hạn 8 bytes mà không bao giờ ghi lấn ra ngoài vùng nhớ cấp phát.
2. **Bài tập 2 (Phân tích chỉ số mảng không Panic)**:  
   Viết một hàm nhận vào một lát cắt chuỗi `&str` và một chỉ số `index: usize`. Thay vì truy cập trực tiếp bằng toán tử chỉ mục `&text[index..index+4]` (có thể gây panic làm sập máy chủ), hãy sử dụng các phương thức an toàn của Rust để trích xuất 4 bytes con, trả về `Result<&str, &'static str>`.
3. **Bài tập 3 (Mô hình tư duy: Double Free)**:  
   Hãy giải thích bằng ngôn ngữ đời sống: Hiện tượng "Giải phóng hai lần (Double Free)" là gì? Tại sao trong Rust, cơ chế tự động gọi hàm hủy `Drop` khi một biến hết phạm vi (scope) lại đảm bảo mỗi ô nhớ chỉ được giải phóng đúng 1 lần duy nhất?
