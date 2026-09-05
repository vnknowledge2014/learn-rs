# Chương 01: Máy tính hoạt động thế nào? CPU, RAM và Ngôn ngữ máy (How Computers Work: CPU, RAM, Bits & Bytes)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn bước vào hành trình chinh phục **Rust** — một trong những ngôn ngữ lập trình hiện đại, mạnh mẽ và được yêu thích nhất trong ngành công nghệ hiện nay. Nếu bạn chưa từng viết một dòng mã nào trong đời, hoặc cảm thấy các khái niệm kỹ thuật máy tính quá phức tạp và xa lạ, chương sách này được thiết kế riêng dành cho bạn.

Mục tiêu học tập của chương:

- Xóa bỏ định kiến máy tính là một "chiếc hộp đen bí ẩn", thấu hiểu bản chất vật lý thực sự đằng sau mọi chương trình máy tính.
- Làm quen với các thành phần cốt lõi của phần cứng máy tính: Bộ vi xử lý (CPU), Bộ nhớ truy xuất ngẫu nhiên (RAM), và Ổ cứng lưu trữ (Storage).
- Hiểu rõ đơn vị thông tin nhỏ nhất của thế giới số: Bit và Byte, cùng cách máy tính quy ước các trạng thái điện thế thành chữ cái, hình ảnh và con số.
- Nắm bắt hành trình kỳ diệu từ khi bạn viết một dòng mã Rust cho đến khi dòng mã đó biến thành tín hiệu điện chạy trong chip vi xử lý.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu cách máy tính vận hành mà không cần bất kỳ công thức toán học nào, hãy cùng quan sát hai hình ảnh vô cùng quen thuộc trong đời sống hàng ngày:

### 1. Bảng 8 công tắc đèn trên tường phòng khách (Bit & Byte)

Hãy tưởng tượng trên bức tường nhà bạn có một bảng gồm **8 chiếc công tắc điện** xếp thành một hàng ngang. Mỗi chiếc công tắc chỉ có đúng 2 trạng thái vật lý đơn giản:

- **Tắt** (tương ứng với số `0`)
- **Bật** (tương ứng với số `1`)

Mỗi chiếc công tắc như vậy được các kỹ sư gọi là một **Bit** (viết tắt của *Binary Digit* - chữ số nhị phân).

- Nếu chỉ có **1 công tắc** (1 bit), bạn chỉ biểu diễn được 2 trạng thái: hoặc là Tối (0), hoặc là Sáng (1).
- Nhưng khi ghép đủ **8 chiếc công tắc** lại với nhau, bạn có một **Byte**.

```
  Công tắc 1    Công tắc 2    Công tắc 3    Công tắc 4    Công tắc 5    Công tắc 6    Công tắc 7    Công tắc 8
  ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐    ┌────────┐
  │  BẬT   │    │  TẮT   │    │  TẮT   │    │  TẮT   │    │  TẮT   │    │  TẮT   │    │  TẮT   │    │  BẬT   │
  │  (1)   │    │  (0)   │    │  (0)   │    │  (0)   │    │  (0)   │    │  (0)   │    │  (0)   │    │  (1)   │
  └────────┘    └────────┘    └────────┘    └────────┘    └────────┘    └────────┘    └────────┘    └────────┘
                                 ===> ĐẠI DIỆN CHO 1 BYTE (8 BITS)
```

Với 8 chiếc công tắc này, bạn có thể tạo ra $2 \times 2 \times 2 \times 2 \times 2 \times 2 \times 2 \times 2 = 256$ tổ hợp bật/tắt khác nhau:

- Trạng thái tất cả đều tắt: `00000000` (quy ước là số `0`).
- Trạng thái bật công tắc cuối cùng: `00000001` (quy ước là số `1`).
- Trạng thái `01000001`: hiệp hội quốc tế quy ước đó là chữ cái in hoa `'A'`.

Mọi thứ bạn nhìn thấy trên màn hình — từ bức ảnh gia đình, video âm nhạc đến trò chơi 3D — bên dưới tận cùng của phần cứng đều chỉ là hàng tỷ chiếc công tắc siêu nhỏ đang được bật hoặc tắt với tốc độ hàng tỷ lần mỗi giây!

---

### 2. Căn bếp của nhà hàng cao cấp (Mô hình CPU, RAM và Ổ cứng)

Hãy hình dung một cỗ máy tính như một căn bếp của một nhà hàng ẩm thực cao cấp:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                             CĂN BẾP NHÀ HÀNG (MÁY TÍNH)                          │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│    ĐẦU BẾP TRƯỞNG       │     MẶT BÀN CHẾ BIẾN          │    KHO LẠNH DƯỚI HẦM   │
│         (CPU)           │          (RAM)                │    (Ổ CỨNG SSD/HDD)    │
│                         │                               │                        │
│ - Tốc độ cực nhanh      │ - Rộng rãi, phẳng lì          │ - Sức chứa khổng lồ    │
│ - Tay cầm dao thớt      │ - Đặt đồ đang nấu dở          │ - Chứa đồ đông lạnh    │
│ - Xử lý từng nhát thái  │ - Với tay lấy trong tích tắc  │ - Đi lấy mất nhiều thời│
│ - Nghỉ ca là buông tay  │ - Tắt điện là lau sạch trơn   │   gian (chậm gấp 1000x)│
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

1. **CPU (Đầu bếp trưởng tài hoa - Central Processing Unit)**:
   - Đầu bếp có tốc độ xào nấu siêu phàm, có thể thái hàng triệu nhát rau củ mỗi giây.
   - Tuy nhiên, đầu bếp chỉ có 2 bàn tay (được gọi là các **Thanh ghi - Registers**). Tại một khoảnh khắc, đầu bếp chỉ có thể cầm một củ cà rốt và một con dao, không thể ôm cả bao tải nguyên liệu trên người.

2. **RAM (Mặt bàn chế biến inox - Random Access Memory)**:
   - Nằm ngay trước mặt đầu bếp. Đây là nơi đặt các đĩa gia vị, thịt, rau củ đã rửa sạch sẵn sàng để nấu.
   - Tốc độ lấy đồ ở mặt bàn cực kỳ nhanh: đầu bếp chỉ cần liếc mắt và đưa tay ra là có ngay nguyên liệu (vài phần tỷ giây - nanosecond).
   - **Đặc điểm sống còn**: Mặt bàn chỉ dùng tạm thời khi ca làm việc đang diễn ra. Khi hết giờ, nhà hàng tắt bếp dọn quán (tắt máy tính), mặt bàn sẽ bị dọn dẹp lau sạch bóng. Không có gì tồn tại vĩnh viễn trên RAM.

3. **Ổ cứng SSD/HDD (Kho lạnh trữ thực phẩm dưới tầng hầm - Storage)**:
   - Nơi lưu trữ hàng tấn bột mì, thùng thịt đông lạnh và gia vị qua nhiều tháng ngày.
   - Khi nhà hàng đóng cửa qua đêm hay cúp điện, thực phẩm trong kho lạnh vẫn còn nguyên vẹn an toàn.
   - Nhưng nhược điểm là: mỗi lần cần một túi muối hay tảng thịt, phụ bếp phải đi thang máy xuống tầng hầm khuân lên. Quá trình này chậm hơn gấp hàng nghìn đến hàng triệu lần so với việc với tay lấy đồ ngay trên mặt bàn RAM!

Khi bạn mở một ứng dụng (ví dụ trình duyệt web hoặc một trò chơi), máy tính sẽ sao chép dữ liệu của chương trình đó từ **Kho lạnh (Ổ cứng)** đặt lên **Mặt bàn (RAM)** để **Đầu bếp (CPU)** trực tiếp thao tác.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

Bây giờ, chúng ta hãy bước sâu hơn vào cơ chế hoạt động thực tế bên trong lòng các linh kiện bán dẫn.

### 1. Transistor — Công tắc điện siêu nhỏ

Trong bộ não vi xử lý (CPU) của máy tính hoặc điện thoại bạn đang dùng có chứa khoảng từ **5 tỷ đến hơn 20 tỷ bóng bán dẫn (Transistor)** được khắc trên một mẩu chip silicon chỉ bằng móng tay.

Mỗi bóng bán dẫn hoạt động như một van đóng mở dòng điện:

- Khi có điện thế chạy qua (ví dụ mức điện áp 3.3V hoặc 1.2V): quy ước là giá trị logic `1`.
- Khi ngắt điện thế (mức điện áp 0V): quy ước là giá trị logic `0`.

### 2. Chu kỳ Nhịp xung và Vòng lặp Tìm nạp - Giải mã - Thực thi (Clock Cycle & Instruction Cycle)

CPU hoạt động theo từng nhịp tim đập đều đặn, được gọi là **Xung nhịp (Clock speed)**. Ví dụ, một con chip có tốc độ `3.5 GHz` đồng nghĩa với việc nó có thể tạo ra `3.5 tỷ nhịp đập mỗi giây`!

Trong mỗi nhịp đập đó, CPU lặp đi lặp lại một chu trình gồm 3 bước cơ bản:

```
  ┌──────────────────────────────────────────────────────────────────┐
  │                    CHU TRÌNH LỆNH CỦA CPU                        │
  │                                                                  │
  │    ┌───────────┐         ┌───────────┐         ┌─────────────┐   │
  │    │  TÌM NẠP  │  ───>   │  GIẢI MÃ  │  ───>   │  THỰC THI   │   │
  │    │  (Fetch)  │         │ (Decode)  │         │  (Execute)  │   │
  │    └───────────┘         └───────────┘         └─────────────┘   │
  │          ▲                                            │          │
  │          └────────────────────────────────────────────┘          │
  └──────────────────────────────────────────────────────────────────┘
```

1. **Tìm nạp (Fetch)**: CPU nhìn vào một con trỏ chỉ thị (Instruction Pointer) để lấy dòng lệnh máy tiếp theo từ RAM nạp vào thanh ghi của mình.
2. **Giải mã (Decode)**: Khối điều khiển bên trong CPU phân tích chuỗi nhị phân đó: "Lệnh này yêu cầu cộng hai số, hay yêu cầu nhảy sang một địa chỉ khác?".
3. **Thực thi (Execute)**: Khối tính toán số học & logic (**ALU - Arithmetic Logic Unit**) thực hiện phép tính và ghi kết quả trở lại thanh ghi hoặc mặt bàn RAM.

### 3. Không gian địa chỉ ô nhớ RAM (Memory Addresses)

Bộ nhớ RAM được chia thành một chuỗi liên tiếp các ô vuông nhỏ. Mỗi ô vuông nhỏ này chứa vừa vặn **1 Byte (8 bits)** dữ liệu.

Điều đặc biệt là: Mỗi ô vuông đều được dán một "số nhà" duy nhất, gọi là **Địa chỉ ô nhớ (Memory Address)**.

```
  Địa chỉ ô nhớ:  [ 0x0001 ]  [ 0x0002 ]  [ 0x0003 ]  [ 0x0004 ]  [ 0x0005 ]
  Dữ liệu chứa:   [01000001]  [00000010]  [11111111]  [00000000]  [00101010]
  Ý nghĩa:           'A'         Số 2       Số 255        Số 0       Số 42
```

Khi CPU muốn đọc hay ghi dữ liệu, nó chỉ cần gửi tín hiệu: *"Hãy lấy cho tôi dữ liệu nằm ở số nhà 0x0001"*. RAM sẽ lập tức trả về giá trị của 8 chiếc công tắc nằm tại vị trí đó.

### 4. Từ Mã nguồn Rust đến Tệp nhị phân thực thi

CPU là một cỗ máy thuần vật lý. Nó **không hề hiểu** tiếng Anh hay tiếng Việt. Nó không biết chữ `println!` hay `fn main()` nghĩa là gì. Thứ duy nhất nó hiểu là các chuỗi số 0 và 1 (Mã máy - Machine Code).

Vậy làm thế nào máy tính hiểu được những gì chúng ta viết?

```
  ┌────────────────────────┐
  │   Mã nguồn Rust (.rs)  │  fn main() { println!("Xin chào!"); }
  │   (Người dễ đọc hiểu)  │
  └──────────┬─────────────┘
             │
             │  (Trình biên dịch rustc phân tích cú pháp,
             │   kiểm tra an toàn bộ nhớ, tối ưu hóa)
             ▼
  ┌───────────────────────┐
  │  Mã máy Nhị phân      │   01001000 10001001 11100101 01001000
  │  (Tệp .exe hoặc file  │   10000011 11101100 00100000 ...
  │   thực thi trên máy)  │
  └──────────┬────────────┘
             │
             │  (Hệ điều hành nạp vào RAM)
             ▼
  ┌───────────────────────┐
  │  CPU thực thi lệnh    │   Các bóng bán dẫn chuyển trạng thái,
  │  (Phần cứng vật lý)   │   chữ "Xin chào!" hiện lên màn hình!
  └───────────────────────┘
```

**Tại sao chúng ta lại học Rust thay vì các ngôn ngữ khác?**

- Các ngôn ngữ cổ điển như **C/C++**: Cho phép bạn trực tiếp can thiệp vào các địa chỉ ô nhớ trên RAM. Điều này giúp chương trình chạy nhanh xé gió, nhưng nếu người lập trình sơ suất trỏ nhầm vào ô nhớ cấm, chương trình sẽ sập ngay lập tức hoặc tạo ra lỗ hổng bảo mật nghiêm trọng để hacker đánh cắp dữ liệu.
- Các ngôn ngữ có "Bộ gom rác" (Garbage Collector) như **Python, Java, Go**: Tự động cử một "nhân viên dọn vệ sinh" chạy ngầm trong máy để dọn rác bộ nhớ. Điều này giúp lập trình viên nhàn nhã hơn, nhưng phải trả giá bằng việc chương trình thỉnh thoảng bị khựng lại (GC pauses), tốn nhiều RAM và không phù hợp cho các hệ thống yêu cầu tốc độ thời gian thực.
- **Rust xuất hiện như một kỳ tích công nghệ**: Mang lại tốc độ tối đa ngang ngửa C/C++ vì không cần bộ gom rác, nhưng lại **an toàn 100%** trước các lỗi bộ nhớ nhờ người gác cổng thông minh mang tên **Borrow Checker** kiểm tra tỉ mỉ ngay từ lúc biên dịch!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là một chương trình Rust hoàn chỉnh. Chương trình này sẽ trực tiếp "hỏi" hệ điều hành và CPU để in ra kích thước vật lý (tính theo Byte) của các kiểu dữ liệu trên thanh RAM máy tính của bạn:

```rust
// File: src/main.rs
// Đây là chương trình Rust hoàn chỉnh đầu tiên của bạn!

fn main() {
    // println! là một công cụ (macro) in các dòng chữ ra màn hình terminal.
    // Dấu chấm than (!) biểu thị rằng đây là một Macro đặc biệt của Rust.
    println!("============================================================");
    println!("  CHƯƠNG TRÌNH KHÁM PHÁ BỘ NHỚ VẬT LÝ VÀ PHẦN CỨNG MÁY TÍNH  ");
    println!("============================================================");

    // 1. Khám phá kích thước của 1 Byte (gồm 8 bits công tắc)
    // std::mem::size_of::<T>() là hàm đo xem kiểu dữ liệu T chiếm bao nhiêu Byte trên RAM.
    let kich_thuoc_u8 = std::mem::size_of::<u8>();
    println!("- Kiểu u8 (số nguyên nhỏ 0..255) chiếm : {} byte ({} bits)", 
             kich_thuoc_u8, kich_thuoc_u8 * 8);

    // 2. Khám phá kiểu số nguyên tiêu chuẩn 32-bit (i32)
    let kich_thuoc_i32 = std::mem::size_of::<i32>();
    println!("- Kiểu i32 (số nguyên chuẩn) chiếm       : {} bytes ({} bits)", 
             kich_thuoc_i32, kich_thuoc_i32 * 8);

    // 3. Khám phá kiểu số nguyên cực lớn 64-bit (i64)
    let kich_thuoc_i64 = std::mem::size_of::<i64>();
    println!("- Kiểu i64 (số nguyên lớn) chiếm         : {} bytes ({} bits)", 
             kich_thuoc_i64, kich_thuoc_i64 * 8);

    // 4. Khám phá kiểu ký tự Unicode (char)
    // Trong Rust, một ký tự có thể là chữ cái tiếng Việt hoặc biểu tượng cảm xúc Emoji!
    let kich_thuoc_char = std::mem::size_of::<char>();
    println!("- Kiểu char (ký tự Unicode/Emoji) chiếm  : {} bytes ({} bits)", 
             kich_thuoc_char, kich_thuoc_char * 8);

    // 5. Khám phá kiểu logic Đúng/Sai (bool)
    let kich_thuoc_bool = std::mem::size_of::<bool>();
    println!("- Kiểu bool (true/false) chiếm           : {} byte (dù chỉ cần 1 bit)", 
             kich_thuoc_bool);

    println!("------------------------------------------------------------");

    // 6. Minh họa trực tiếp cách máy tính nhìn một con số dưới dạng công tắc bật/tắt (nhị phân)
    let con_so_yeu_thich: u8 = 42;
    println!("Con số quen thuộc trong đời thực: {}", con_so_yeu_thich);
    // Cú pháp {:08b} yêu cầu Rust in số này dưới dạng nhị phân 8 bit (0 và 1)
    println!("Dãy 8 công tắc điện thực tế trong chip RAM: {:08b}", con_so_yeu_thich);

    let linh_vat: char = '🦀'; // Cua Ferris - Linh vật chính thức của cộng đồng Rust
    println!("Linh vật đáng yêu của Rust: {}", linh_vat);
    println!("Mã số đại diện trong bộ ký tự quốc tế: U+{:X}", linh_vat as u32);
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi viết chương trình đầu tiên, người mới bắt đầu rất dễ gặp phải một số thông báo lỗi từ trình biên dịch Rust (`rustc`). Đừng hoảng sợ! Trình biên dịch Rust được mệnh danh là người thầy kiên nhẫn nhất thế giới, nó sẽ chỉ rõ vị trí và nguyên nhân gây lỗi.

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
| --- | --- | --- | --- |
| **Lỗi Macro** | `cannot find macro 'prinln' in this scope` | Gõ sai chính tả tên Macro in ấn (ví dụ gõ thiếu chữ `t` trong `println!`). Trình biên dịch sẽ phát hiện và gợi ý tên macro chuẩn. | Kiểm tra lại từng ký tự, sửa đúng thành `println!`. |
| **E0425** | `cannot find value 'x' in this scope` (hoặc `cannot find function`) | Sử dụng một tên biến chưa từng được khai báo bằng `let`, hoặc gọi hàm in ấn mà quên dấu chấm than `!` (ví dụ viết nhầm `prinln("...")` như một hàm thông thường). | Khai báo biến trước khi dùng (`let x = ...;`) hoặc kiểm tra lại tên hàm và bổ sung dấu `!` nếu là Macro. |
| **E0308** | `mismatched types: expected 'u8', found 'i32'` | Gán một biến có kiểu số có dấu hoặc kích thước lớn hơn vào một biến kiểu số nhỏ hơn (ví dụ: `let y: i32 = 10; let x: u8 = y;`). | Dùng phương thức chuyển đổi kiểu dữ liệu an toàn (`.try_into()`) hoặc đồng nhất kiểu dữ liệu của hai biến. *(Lưu ý: Nếu viết trực tiếp số âm `let x: u8 = -1;`, Rust sẽ báo lỗi `E0600: cannot apply unary operator '-' to type 'u8'`)*. |
| **Cảnh báo `unused`** | `warning: variable does not need to be mutable` hoặc `unused variable` | Khai báo một biến trên bộ nhớ nhưng không bao giờ dùng tới trong chương trình. | Xóa biến thừa, hoặc thêm tiền tố dấu gạch dưới `_` (ví dụ `_ten_bien`) để báo cho trình biên dịch biết đây là biến cố ý chưa dùng. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ

1. **Bản chất của dữ liệu số**: Mọi thông tin trong máy tính đều được cấu thành từ các **Bit** (công tắc bật/tắt `1` hoặc `0`). Cứ **8 bits** hợp thành **1 Byte**.
2. **Bộ ba phần cứng**: **CPU** là đầu bếp xử lý siêu tốc nhưng ít chỗ chứa; **RAM** là mặt bàn chế biến tốc độ cao nhưng sẽ bị xóa sạch khi tắt máy; **Ổ cứng** là kho lạnh chứa dữ liệu vĩnh viễn nhưng tốc độ truy xuất chậm.
3. **Địa chỉ bộ nhớ**: Mỗi byte trên thanh RAM đều có một "số nhà" cụ thể. CPU dựa vào địa chỉ này để đọc và ghi thông tin.
4. **Vị thế đặc biệt của Rust**: Rust cho phép kiểm soát tài nguyên phần cứng sát sao như C/C++, mang lại hiệu năng tối đa mà không cần bộ gom rác (Garbage Collector), đồng thời đảm bảo an toàn bộ nhớ tuyệt đối.

### Bài tập rèn luyện tự giải

1. **Bài tập tư duy 1**: Hãy nhẩm tính xem một tệp nhạc dung lượng **5 Megabytes (5 MB)** sẽ tương đương với khoảng bao nhiêu Byte và bao nhiêu công tắc điện (Bits)? Giả sử $1 \text{ MB} \approx 1{,}000{,}000 \text{ Bytes}$.
2. **Bài tập tư duy 2**: Tại sao khi bạn đang soạn thảo một văn bản nhưng đột ngột bị cúp điện (và máy không có pin dự phòng), toàn bộ nội dung bạn chưa kịp bấm nút "Save" lại biến mất hoàn toàn? Hãy giải thích dựa trên hình ảnh căn bếp và mặt bàn RAM.
3. **Bài tập thực hành 3**: Thử gõ lại mã nguồn ở mục thực chiến vào một tệp mới, sau đó bổ sung thêm dòng lệnh đo kích thước của kiểu số thực `f32` và `f64`. Quan sát kết quả in ra màn hình xem chúng chiếm bao nhiêu Byte trên RAM.
