# Chương 03: Biến, Bất biến và Kiểu dữ liệu nguyên bản (Variables, Mutability & Primitive Types)

## Giới thiệu & Mục tiêu học tập

Trong lập trình, để lưu trữ và thao tác với thông tin — ví dụ như điểm số người chơi, số dư tài khoản ngân hàng, hay nhiệt độ ngoài trời — chúng ta cần một cơ chế để ghi lại các giá trị đó vào bộ nhớ RAM của máy tính. Cơ chế đó được gọi là **Biến (Variable)**.

Tuy nhiên, cách tiếp cận của Rust đối với biến số rất khác biệt so với phần lớn các ngôn ngữ bạn từng nghe tên (như Python, JavaScript hay C++). Thay vì cho phép bạn thay đổi giá trị một cách tùy tiện, Rust đặt ra một triết lý thiết kế mang tính cách mạng: **Bất biến mặc định (Immutability by Default)**.

Mục tiêu học tập của chương này:
- Hiểu bản chất biến là gì trong bộ nhớ vật lý: một chiếc nhãn dán định danh cho một ô nhớ trên RAM.
- Nắm vững triết lý "Bất biến mặc định" và lý do tại sao nó giúp loại bỏ hàng loạt lỗi nghiêm trọng trong phần mềm.
- Biết cách sử dụng từ khóa `mut` một cách an toàn khi thực sự cần thay đổi dữ liệu.
- Phân biệt rạch ròi giữa việc sửa giá trị của biến (`mut`) và kỹ thuật "che khuất biến" (**Variable Shadowing**).
- Làm chủ toàn bộ hệ thống kiểu dữ liệu vô hướng nguyên bản của Rust: Số nguyên có dấu và không dấu (`i8`..`i128`, `u8`..`u128`), số thực (`f32`, `f64`), kiểu logic (`bool`), và ký tự Unicode chuẩn 4-byte (`char`).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để những khái niệm trên trở nên sống động và dễ hiểu, hãy hình dung các đồ vật quen thuộc sau trong đời sống hàng ngày:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                           HÌNH TƯỢNG VỀ BIẾN TRONG RUST                          │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│    HỘP NIÊM PHONG ĐỎ    │       BẢNG PHẤN TREO TƯỜNG    │     DÁN ĐÈ TỜ GIẤY MỚI │
│     (Biến bất biến)     │       (Biến khả biến mut)     │      (Shadowing)       │
│                         │                               │                        │
│ - Bỏ đồ vào và dán kín  │ - Viết điểm số bằng phấn      │ - Lấy một tờ giấy mới  │
│ - Không ai được mở đổi  │ - Điểm đổi -> Lấy giẻ lau xóa │ - Dán đè lên bảng cũ   │
│ - An tâm 100% không sợ  │   và viết số mới vào ô đó     │ - Thay đổi hoàn toàn cả│
│   ai lén sửa sau lưng   │ - Kiểu dáng bảng không đổi    │   kiểu dữ liệu lẫn tên │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Chiếc hộp carton dán băng keo niêm phong (Biến bất biến mặc định)
Khi bạn viết `let nam_sinh = 1995;`, bạn đang đặt con số `1995` vào một chiếc hộp carton nhỏ, sau đó dán băng keo niêm phong màu đỏ lên miệng hộp.
- Không ai — kể cả chính bạn — được phép bóc băng keo ra để nhét con số khác vào chiếc hộp đó nữa.
- Điều này mang lại sự an tâm tuyệt đối: Nếu bạn đưa chiếc hộp này cho 10 người khác xem, bạn biết chắc chắn 100% rằng khi nhận lại hộp, con số bên trong vẫn mãi mãi là `1995`.

### 2. Chiếc bảng phấn đen treo tường (Biến khả biến với từ khóa `mut`)
Nếu bạn muốn một đại lượng có thể thay đổi liên tục theo thời gian (ví dụ: điểm số trong một trận bóng đá), bạn phải báo trước cho Rust bằng từ khóa `mut` (viết tắt của *mutable* - có thể biến đổi):
```rust
let mut diem_so = 0;
```
Điều này giống như bạn dựng một chiếc bảng phấn đen lên tường. Bạn viết số `0`. Khi đội nhà ghi bàn, bạn cầm giẻ lau xóa số `0` đi và viết số `1` vào chính vị trí đó. Chiếc bảng vẫn là chiếc bảng đó, vị trí vẫn ở đó, chỉ có nội dung bên trong được cập nhật.

### 3. Dán một tờ giấy mới đè lên vị trí cũ (Hiện tượng che khuất - Shadowing)
Rust cho phép bạn khai báo lại một biến mới toanh có **cùng tên** với một biến cũ đã tồn tại bằng từ khóa `let`:
```rust
let tien_luong = "5000000"; // Chuỗi văn bản
let tien_luong = 5000000;   // Số nguyên thực tế
```
Hiện tượng này giống như bạn có một chiếc bảng cũ, nhưng thay vì lau phấn, bạn lấy một tờ giấy dán tường mới tinh dán đè kín mít lên chiếc bảng đó. Từ nay về sau, khi ai đó nhắc đến "tờ giấy trên tường", họ chỉ nhìn thấy nội dung mới. Điều kỳ diệu là: tờ giấy mới có thể mang kiểu dáng, kích thước và màu sắc hoàn toàn khác biệt so với chiếc bảng cũ ban đầu!

### 4. Khay chia tiền xu nhiều kích cỡ (Hệ thống kiểu dữ liệu nguyên thủy)
Hãy tưởng tượng trong ngăn kéo của bạn có các khay nhựa đựng tiền xu với các kích cỡ khác nhau:
- **Khay siêu nhỏ (`u8`)**: Chỉ đựng vừa các đồng xu có mệnh giá từ `0` đến `255`. Nếu bạn cố nhét một đồng tiền mệnh giá `300` vào chiếc khay này, chiếc khay nhựa sẽ bị nứt vỡ ngay lập tức! Hiện tượng này trong khoa học máy tính được gọi là **Lỗi tràn số (Integer Overflow)**.
- **Khay vừa (`i32`)**: Đựng được các con số từ âm hơn 2 tỷ đến dương hơn 2 tỷ — đủ rộng rãi cho hầu hết các nhu cầu tính toán thông thường.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

Bây giờ chúng ta sẽ đi sâu vào cấu trúc phần cứng và cách bộ nhớ RAM quản lý các kiểu dữ liệu này.

### 1. Tại sao Rust lại chọn "Bất biến mặc định"?

Trong các ngôn ngữ truyền thống, mọi biến đều có thể bị sửa đổi tự do. Khi một chương trình trở nên đồ sộ với hàng trăm nghìn dòng mã và hàng chục kỹ sư cùng làm việc:
- Hàm A sửa biến `X`.
- Hàm B ở tệp khác cũng âm thầm sửa biến `X`.
- Chương trình bị sập và không ai biết ai đã thay đổi giá trị của `X` tại thời điểm nào! Lỗi này cực kỳ khó tìm và tiêu tốn hàng tuần lễ của lập trình viên.

Khi Rust đặt biến ở trạng thái bất biến mặc định:
- Bạn chỉ được phép đọc dữ liệu, không thể vô tình sửa đổi.
- CPU có thể tối ưu hóa cực đại: nạp giá trị này vào thẳng các thanh ghi siêu tốc mà không cần lo lắng ô nhớ bị thay đổi lén lút.
- **Chia sẻ an toàn tuyệt đối giữa nhiều nơi**: Giống như một bảng thông báo tin tức niêm yết cố định ở sảnh chung cư, hàng trăm người có thể an tâm cùng nhau xem thông tin mà không bao giờ lo sợ có ai đó vừa đọc thì người khác lại chạy đến lén xóa sửa hay viết đè nội dung khác lên!

### 2. Hệ thống kiểu dữ liệu Số nguyên (Integers)

Số nguyên là các số không có phần thập phân (như `-5`, `0`, `42`). Trong Rust, số nguyên được chia thành hai họ chính:
1. **Số nguyên có dấu (Signed Integers)**: Có thể mang dấu âm (`-`) hoặc dấu dương (`+`). Bắt đầu bằng chữ cái `i` (viết tắt của *integer*).
2. **Số nguyên không dấu (Unsigned Integers)**: Chỉ có thể mang giá trị từ `0` trở lên (không bao giờ âm). Bắt đầu bằng chữ cái `u` (viết tắt của *unsigned*).

Bảng phân loại chi tiết theo số lượng bit chiếm dụng trên RAM:

| Kích thước trên RAM | Kiểu có dấu (`i`) | Kiểu không dấu (`u`) | Phạm vi giá trị có thể lưu trữ |
|---|---|---|---|
| **8-bit (1 Byte)** | `i8` | `u8` | `i8`: từ $-128$ đến $127$<br>`u8`: từ $0$ đến $255$ |
| **16-bit (2 Bytes)**| `i16` | `u16` | `i16`: từ $-32{,}768$ đến $32{,}767$<br>`u16`: từ $0$ đến $65{,}535$ |
| **32-bit (4 Bytes)**| `i32` *(Mặc định)*| `u32` | `i32`: từ $-2{,}147{,}483{,}648$ đến $2{,}147{,}483{,}647$<br>`u32`: từ $0$ đến $4{,}294{,}967{,}295$ |
| **64-bit (8 Bytes)**| `i64` | `u64` | Phù hợp cho tính toán khoa học, dấu thời gian, dữ liệu cực lớn |
| **128-bit (16 Bytes)**| `i128` | `u128` | Dành cho các con số thiên văn, mật mã học |
| **Theo kiến trúc chip**| `isize` | `usize` | 4 bytes trên chip 32-bit, 8 bytes trên chip 64-bit (dùng làm chỉ mục mảng) |

> **Nguyên tắc chọn kiểu số**: Nếu không có lý do đặc biệt, hãy luôn chọn `i32` cho số nguyên vì đây là kiểu số có tốc độ xử lý nhanh nhất trên hầu hết các dòng CPU hiện đại. Nếu biểu diễn các đại lượng không thể âm (như số lượng người, tuổi tác, kích thước tệp), hãy chọn `u32` hoặc `usize`.

### 3. Số thực dấu phẩy động (Floating-Point Numbers)

Khi cần biểu diễn các con số có phần thập phân (như số pi `3.14159` hay nhiệt độ `36.5`), Rust cung cấp hai kiểu dữ liệu tuân theo tiêu chuẩn quốc tế IEEE 754:
- **`f32` (Độ chính xác đơn - Single precision)**: Chiếm 4 bytes trên RAM.
- **`f64` (Độ chính xác kép - Double precision)**: Chiếm 8 bytes trên RAM (*Đây là lựa chọn mặc định của Rust khi bạn viết số thực mà không ghi rõ kiểu*).

> **Cảnh báo sống còn**: Tuyệt đối không bao giờ dùng `f32` hay `f64` để tính toán tiền tệ trong các hệ thống ngân hàng! Do cách lưu trữ nhị phân, số thực luôn có một sai số làm tròn cực nhỏ (ví dụ `0.1 + 0.2` có thể ra kết quả `0.30000000000000004`). Với tiền bạc, các kỹ sư luôn dùng số nguyên đơn vị xu/đồng hoặc các thư viện số học thập phân chuyên dụng.

### 4. Kiểu logic nhị phân (`bool`) và Kiểu ký tự (`char`)

- **Kiểu logic (`bool`)**: Chỉ có hai giá trị duy nhất: `true` (Đúng) hoặc `false` (Sai). Chiếm 1 byte trong bộ nhớ.
- **Kiểu ký tự (`char`)**: Trong Rust, một ký tự được bao bọc bởi cặp **dấu nháy đơn** (ví dụ `'A'`, `'ế'`, `'🦀'`).
  - Điểm đặc biệt: Trong khi ngôn ngữ C chỉ dành 1 byte cho ký tự (chỉ lưu được bảng chữ cái tiếng Anh ASCII), thì Rust dành trọn vẹn **4 bytes (32-bit)** cho mỗi ký tự `char`.
  - Điều này đồng nghĩa với việc Rust hỗ trợ Unicode tự nhiên ngay từ gốc rễ: bạn có thể lưu chữ cái tiếng Việt có dấu, chữ tượng hình tiếng Nhật, tiếng Hàn hay thậm chí cả các biểu tượng cảm xúc Emoji một cách hoàn hảo!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình minh họa toàn diện tất cả các khái niệm về biến, tính bất biến, shadowing và các kiểu dữ liệu nguyên thủy:

```rust
// File: src/main.rs
// Chương trình thực hành làm chủ Biến và Kiểu dữ liệu nguyên bản

fn main() {
    println!("=== 1. KHÁM PHÁ TÍNH BẤT BIẾN (IMMUTABILITY) ===");
    let founding_year = 2006; // Biến bất biến: không thể sửa
    println!("Năm ngôn ngữ Rust bắt đầu được thai nghén: {}", founding_year);
    // Nếu bạn bỏ chú thích dòng dưới, compiler sẽ lập tức báo lỗi E0384:
    // founding_year = 2010;

    println!("\n=== 2. KHÁM PHÁ BIẾN KHẢ BIẾN VỚI TỪ KHÓA 'mut' ===");
    let mut rust_version = 1.0; // Chiếc bảng phấn: cho phép xóa đi viết lại
    println!("Phiên bản Rust ban đầu: {}", rust_version);
    
    rust_version = 1.85; // Cập nhật giá trị mới hợp lệ
    println!("Phiên bản Rust hiện đại : {}", rust_version);

    println!("\n=== 3. KỸ THUẬT CHE KHUẤT BIẾN (SHADOWING) ===");
    // Giả sử nhận được dữ liệu dạng chuỗi văn bản từ người dùng nhập
    let quantity_ve = "5"; 
    println!("Dữ liệu người dùng nhập (chuỗi): {}", quantity_ve);

    // Dán đè một biến mới cùng tên nhưng đổi kiểu dữ liệu sang số nguyên:
    let quantity_ve: u32 = quantity_ve.parse().expect("Không phải con số hợp lệ!");
    let tong_tien = quantity_ve * 100_000; // Rust cho phép dùng dấu gạch dưới _ để số dễ đọc hơn
    println!("Số vé sau khi chuyển đổi: {} vé", quantity_ve);
    println!("Tổng tiền cần thanh toán : {} VND", tong_tien);

    println!("\n=== 4. CÁC KIỂU DỮ LIỆU SỐ HỌC NGUYÊN BẢN ===");
    let age: u8 = 25;                       // Số nguyên không dấu 8-bit (0..255)
    let nhiet_do: i16 = -15;                  // Số nguyên có dấu 16-bit
    let derive_num_write_nam: u32 = 100_000_000;   // Số nguyên không dấu 32-bit
    let pos_value_distance: f64 = 384_400.5; // Khoảng cách tới Mặt Trăng (km)
    
    println!("Tuổi học viên   : {} tuổi (chiếm {} byte)", age, std::mem::size_of_val(&age));
    println!("Nhiệt độ mùa đông: {}°C (chiếm {} bytes)", nhiet_do, std::mem::size_of_val(&nhiet_do));
    println!("Dân số Việt Nam : {} người (chiếm {} bytes)", derive_num_write_nam, std::mem::size_of_val(&derive_num_write_nam));
    println!("Khoảng cách trăng: {} km (chiếm {} bytes)", pos_value_distance, std::mem::size_of_val(&pos_value_distance));

    println!("\n=== 5. KIỂU LOGIC VÀ KÝ TỰ UNICODE ===");
    let dang_hoc_rust: bool = true;
    let bieu_cam: char = '🎯'; // Ký tự Unicode chiếm trọn vẹn 4 bytes
    let ky_tu_tieng_viet: char = 'Đ';

    println!("Đang say mê học Rust? {}", dang_hoc_rust);
    println!("Mục tiêu học tập    : {}", bieu_cam);
    println!("Chữ cái tiếng Việt  : {}", ky_tu_tieng_viet);
    println!("Kích thước char trên RAM: {} bytes", std::mem::size_of::<char>());

    println!("\n=== 6. ÉP KIỂU AN TOÀN VỚI TỪ KHÓA 'as' ===");
    let point_transfer_can: u8 = 9;
    let exam_score: f32 = 8.5;
    // Để cộng số nguyên với số thực, ta phải chủ động ép kiểu (explicit casting)
    let diem_tong_ket = (point_transfer_can as f32 * 0.3) + (exam_score * 0.7);
    println!("Điểm tổng kết môn học: {:.2}", diem_tong_ket);
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Hệ thống kiểu dữ liệu tĩnh nghiêm ngặt của Rust sẽ giúp bạn bắt lỗi ngay từ lúc viết code. Dưới đây là những lỗi phổ biến:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0384** | `error[E0384]: cannot assign twice to immutable variable 'x'` | Bạn đang cố gán giá trị mới cho một biến được khai báo không có từ khóa `mut`. | Thêm `mut` vào trước tên biến khi khai báo (`let mut x = ...`), hoặc nếu muốn đổi kiểu dữ liệu thì dùng kỹ thuật Shadowing (`let x = ...`). |
| **E0308** | `error[E0308]: mismatched types: expected 'i32', found 'f64'` | Bạn cố tình cộng hoặc gán hai kiểu dữ liệu khác nhau (Rust không bao giờ tự ý ép kiểu ngầm để tránh sai sót). | Sử dụng từ khóa `as` để ép kiểu rõ ràng (ví dụ: `bien_so_nguyen as f64`). |
| **Tràn số biên dịch** | `error: literal out of range for 'u8'` | Bạn viết số `300` vào kiểu `u8` (vốn chỉ chứa được tối đa số `255`). | Đổi sang kiểu dữ liệu có sức chứa lớn hơn như `u16` hoặc `u32`. |
| **Thiếu chú thích kiểu** | `error[E0282]: type annotations needed` | Khi dùng các hàm như `.parse()`, Rust không tự đoán được bạn muốn chuyển đổi chuỗi thành kiểu số nào. | Thêm chú thích kiểu rõ ràng cho biến: `let x: u32 = chuoi.parse().unwrap();`. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Bất biến mặc định (Immutability)**: Mọi biến khai báo bằng `let` đều bị niêm phong không thể sửa đổi; muốn sửa phải chủ động thêm từ khóa `mut`.
2. **Kỹ thuật Shadowing**: Khai báo lại biến cùng tên bằng `let` để dán đè dữ liệu mới, cho phép thay đổi cả kiểu dữ liệu mà không làm mất tính bất biến an toàn.
3. **Phân loại số nguyên**: Bắt đầu bằng `i` là có dấu (âm/dương), bắt đầu bằng `u` là không dấu (chỉ dương); `i32` là kiểu số nguyên tiêu chuẩn mặc định.
4. **Ký tự Unicode 4-byte**: Kiểu `char` trong Rust chiếm 4 bytes, cho phép hiển thị trọn vẹn tiếng Việt và mọi Emoji trên thế giới mà không sợ lỗi font.

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Viết chương trình khai báo thông tin của một chiếc điện thoại di động:
   - Tên dòng máy (chuỗi ký tự).
   - Dung lượng pin tính bằng mAh (số nguyên không dấu).
   - Trọng lượng tính bằng gam (số thực).
   - Đang kết nối Wifi hay không (kiểu logic `bool`).
   - In tất cả các thông tin trên ra màn hình kèm kích thước byte của từng biến.
2. **Bài tập tư duy 2**: Nếu bạn cần lưu trữ thông tin "Số lượng học sinh trong một lớp học (tối đa 50 em)", bạn nên chọn kiểu dữ liệu nào giữa `i8`, `u8`, `i32`, hay `f64`? Hãy giải thích lý do lựa chọn của bạn dựa trên nguyên tắc tiết kiệm bộ nhớ RAM.
3. **Bài tập sửa lỗi (Debugging)**: Cho đoạn mã sau:
   ```rust
   let diem_so = 10;
   diem_so = diem_so + 5;
   println!("Điểm mới: {}", diem_so);
   ```
   Hãy chỉ ra lỗi biên dịch sẽ xuất hiện và đưa ra 2 cách khác nhau để sửa cho đoạn mã này chạy thành công.
