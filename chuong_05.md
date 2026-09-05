# Chương 05: Hàm, Bộ nhớ Ngăn xếp vs Vùng nhớ tự do, và Nhập/Xuất chuẩn (Functions, Stack vs Heap, and Standard I/O)

## Giới thiệu & Mục tiêu học tập

Khi chương trình phần mềm của bạn bắt đầu lớn dần lên, việc viết hàng nghìn dòng lệnh dồn cục trong một hàm `main` duy nhất sẽ trở thành một cơn ác mộng: khó đọc, khó sửa và không thể tái sử dụng. Để giải quyết vấn đề này, các kỹ sư phần mềm chia nhỏ chương trình thành các khối chức năng độc lập mang tên **Hàm (Function)**.

Đặc biệt hơn, chương này sẽ trang bị cho bạn một "vũ khí bí mật" mang tính chất nền tảng sống còn của toàn bộ ngôn ngữ Rust: **Phân biệt rạch ròi giữa Bộ nhớ Ngăn xếp (Stack) và Vùng nhớ Tự do (Heap)**. Hiểu được cơ chế này chính là chiếc chìa khóa vạn năng giúp bạn chinh phục trọn vẹn khái niệm Quyền sở hữu (Ownership) ở Chủ đề 2 mà không hề bị bỡ ngỡ.

Mục tiêu học tập của chương này:
- Nắm vững cú pháp khai báo và triệu gọi hàm trong Rust với từ khóa `fn`.
- Hiểu sự khác biệt tinh tế giữa việc trả về giá trị ngầm định (**Implicit Return**) bằng biểu thức không có dấu chấm phẩy `;` và câu lệnh `return` rõ ràng.
- Hiểu sâu sắc bản chất phần cứng của **Bộ nhớ Ngăn xếp (Stack)** và **Vùng nhớ Tự do (Heap)**.
- Mổ xẻ cấu trúc ô nhớ của kiểu chuỗi co giãn `String` (gồm con trỏ, độ dài, và sức chứa).
- Tương tác trực tiếp với người dùng qua bàn phím máy tính bằng thư viện Nhập/Xuất chuẩn `std::io`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Chúng ta hãy hữu hình hóa các khái niệm kỹ thuật trừu tượng này thông qua 3 hình ảnh đời thường:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG ĐỜI SỐNG VỀ HÀM VÀ BỘ NHỚ RAM                       │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│   MÁY ÉP HOA QUẢ TỰ ĐỘNG│      ỐNG ĐỰNG BÓNG TENNIS     │     BÃI ĐỖ XE SIÊU THỊ │
│         (Hàm - Function)│     (Ngăn xếp - Stack Memory) │  (Vùng nhớ tự do - Heap)│
│                         │                               │                        │
│ - Cho hoa quả vào phễu  │ - Các quả bóng kích cỡ y hệt  │ - Xe máy, ô tô, xe tải │
│   (Tham số đầu vào)     │ - Xếp chồng lên nhau          │   kích thước to nhỏ    │
│ - Máy xay ép bên trong  │ - Bỏ vào sau -> Lấy ra trước  │ - Bác bảo vệ tìm ô đỗ  │
│   (Thân hàm thực thi)   │ - Rút ra siêu nhanh trong nháy│   và phát vé xe        │
│ - Rót ly nước mát lành  │   mắt, không cần tìm kiếm     │ - Vé xe giữ ở túi áo   │
│   (Giá trị trả về)      │                               │   (vé trên Stack, xe   │
│                         │                               │    nằm trên Heap)      │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Chiếc máy ép nước trái cây tự động (Hàm - Function)
Một hàm giống như một chiếc máy gia dụng đa năng:
- **Miệng phễu tiếp nguyên liệu**: Nơi bạn thả cam, táo, cà rốt vào (chính là các **Tham số - Parameters** truyền vào hàm).
- **Quy trình ép bên trong buồng máy**: Lưỡi dao quay, nghiền nát và lọc bã (chính là **Thân hàm - Function Body** chứa các dòng code xử lý).
- **Vòi rót thành phẩm**: Chảy ra một cốc nước ép mát lạnh (chính là **Giá trị trả về - Return Value** của hàm).
Bạn chỉ cần chế tạo chiếc máy ép một lần, và sau đó có thể mang ra ép hoa quả hàng trăm lần mỗi ngày mà không cần lắp ráp lại từ đầu.

### 2. Ống đựng bóng tennis (Bộ nhớ Ngăn xếp - Stack)
Hãy tưởng tượng một chiếc ống nhựa dài vừa khít để đựng bóng tennis:
- Tất cả các quả bóng đều có kích thước cố định y hệt nhau.
- Bạn thả quả bóng số 1 vào đáy ống, rồi thả quả bóng số 2 đè lên trên, cuối cùng là quả bóng số 3 nằm ở miệng ống.
- Khi muốn lấy bóng ra chơi, bạn bắt buộc phải lấy quả bóng số 3 (bỏ vào sau cùng) ra trước tiên. Cơ chế này gọi là **Vào sau - Ra trước (Last In, First Out - LIFO)**.
- **Ưu điểm vượt trội**: Cực kỳ nhanh! CPU không cần đi tìm kiếm ô nhớ ở đâu xa, nó chỉ việc đặt thêm một quả bóng lên đỉnh hoặc nhấc quả bóng trên đỉnh ra với chi phí thời gian gần như bằng 0.

### 3. Bãi đỗ xe trung tâm thương mại (Vùng nhớ Tự do - Heap)
Bộ nhớ Heap giống như một bãi đỗ xe rộng mênh mông ngoài trời:
- Các phương tiện đến đỗ có kích thước hoàn toàn khác nhau: có người đi xe đạp nhỏ gọn, có người lái ô tô 4 chỗ, có người lái xe tải chở hàng cồng kềnh.
- Khi một chiếc xe tải đến cổng, bác bảo vệ bãi xe phải đi tìm một khoảng đất trống đủ lớn để chứa vừa chiếc xe đó (quá trình **Cấp phát bộ nhớ - Memory Allocation**).
- Sau khi xếp xe vào vị trí, bác bảo vệ ghi tọa độ ô đỗ vào một mảnh giấy nhỏ rồi đưa cho bạn làm **Vé giữ xe (Con trỏ địa chỉ - Pointer)**.
- Bạn cất chiếc vé giữ xe nhỏ xíu vào ví tiền trong túi áo (túi áo chính là **Stack**), trong khi chiếc xe tải to đùng thì đang nằm phơi mình ở góc bãi đỗ (bãi đỗ chính là **Heap**)!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Cú pháp định nghĩa hàm và Quy tắc trả về ngầm định

Trong Rust, hàm được khai báo bằng từ khóa `fn`:
```rust
fn tinh_tong(so_a: i32, so_b: i32) -> i32 {
    so_a + so_b // Không có dấu chấm phẩy: Đây là biểu thức trả về giá trị!
}
```
- **Bắt buộc chú thích kiểu tham số**: Rust yêu cầu bạn phải ghi rõ kiểu dữ liệu của từng tham số (`so_a: i32`). Điều này giúp trình biên dịch kiểm tra tính đúng đắn trên toàn bộ dự án mà không cần phải chạy thử.
- **Mũi tên kiểu trả về `-> Kiểu`**: Nếu hàm có sinh ra kết quả, bạn dùng dấu mũi tên `->` theo sau là kiểu dữ liệu.
- **Biểu thức trả về (Implicit Return)**: Dòng cuối cùng của thân hàm nếu **không có dấu chấm phẩy `;`** sẽ được Rust coi là giá trị trả về của hàm. Bạn vẫn có thể dùng từ khóa `return` rõ ràng khi muốn thoát hàm sớm giữa chừng, nhưng phong cách viết không dùng `return` ở cuối hàm là chuẩn mực thanh lịch (idiomatic) của cộng đồng Rust.

### 2. Khung Ngăn xếp (Stack Frame) và Vòng đời gọi hàm

Mỗi khi một hàm được gọi, hệ thống sẽ cấp phát một vùng nhỏ trên đỉnh Stack gọi là **Khung ngăn xếp (Stack Frame)**:
- Khung này chứa tất cả các tham số truyền vào và các biến cục bộ khai báo bên trong hàm đó.
- Khi hàm thực thi xong và thoát ra, toàn bộ Stack Frame đó sẽ bị "thu hồi" ngay lập tức bằng cách di chuyển con trỏ đỉnh ngăn xếp (Stack Pointer). Bộ nhớ được dọn sạch tinh tươm trong 1 chu kỳ xung nhịp CPU!

```
  ĐỈNH STACK ▲
             │  ┌────────────────────────────────────────┐
             │  │ Stack Frame của hàm con: tinh_bmi()    │ <── Đang thực thi
             │  ├────────────────────────────────────────┤
             │  │ Stack Frame của hàm cha: main()        │ <── Tạm dừng chờ
             │  └────────────────────────────────────────┘
  ĐÁY STACK  ┴────────────────────────────────────────────
```

### 3. So sánh chuyên sâu: Stack vs Heap

| Tiêu chí | Bộ nhớ Ngăn xếp (Stack) | Vùng nhớ Tự do (Heap) |
|---|---|---|
| **Kích thước dữ liệu** | Phải biết chính xác kích thước cố định tại thời điểm biên dịch. | Kích thước linh hoạt, có thể co giãn hoặc phình to tùy ý lúc đang chạy. |
| **Tốc độ truy xuất** | Cực kỳ nhanh (dữ liệu nằm sát cạnh nhau trong bộ nhớ đệm (buffer / CPU cache)). | Chậm hơn (CPU phải đọc địa chỉ con trỏ trước rồi mới nhảy sang ô nhớ Heap). |
| **Cơ chế quản lý** | Tự động hoàn toàn theo cấu trúc LIFO (đẩy vào / rút ra ở đỉnh). | Phải tìm khoảng trống đủ lớn trên RAM (dễ bị phân mảnh bộ nhớ). |
| **Dữ liệu đại diện** | Các kiểu nguyên bản: `i32`, `f64`, `bool`, `char`, mảng cố định `[T; N]`. | Dữ liệu động: Chuỗi co giãn `String`, danh sách động `Vec<T>`, đối tượng `Box<T>`. |

### 4. Mổ xẻ chuỗi `String` trên Stack và Heap

Một trong những ví dụ điển hình nhất minh họa mối quan hệ giữa Stack và Heap là kiểu chuỗi `String`:
```rust
let loi_chao = String::from("Xin chào");
```
Dưới góc nhìn phần cứng, dữ liệu của biến `loi_chao` được tổ chức như sau:

```
    BỘ NHỚ STACK (Chiếm 24 bytes)                  BỘ NHỚ HEAP (Cấp phát động)
   ┌───────────┬─────────────┐                   ┌──────────────┬─────────────┐
   │ Tên trường│   Giá trị   │                   │ Chỉ số byte  │ Ký tự ASCII │
   ├───────────┼─────────────┤                   ├──────────────┼─────────────┤
   │  con trỏ  │  0xAB12CD   │ ──Trỏ sang Heap─> │   0xAB12CD   │     'X'     │
   │  (ptr)    │  (Địa chỉ)  │                   │   0xAB12CE   │     'i'     │
   ├───────────┼─────────────┤                   │   0xAB12CF   │     'n'     │
   │  độ dài   │   8 bytes   │                   │   0xAB12D0   │     ' '     │
   │  (len)    │             │                   │   0xAB12D1   │     'c'     │
   ├───────────┼─────────────┤                   │   0xAB12D2   │     'h'     │
   │ sức chứa  │   8 bytes   │                   │   0xAB12D3   │     'à'     │
   │(capacity) │             │                   │   0xAB12D4   │     'o'     │
   └───────────┴─────────────┘                   └──────────────┴─────────────┘
```
- **Phần nằm trên Stack**: Gồm 3 con số nguyên cố định (mỗi số 8 bytes trên máy 64-bit):
  1. `ptr`: Địa chỉ dẫn tới ô nhớ trên bãi đỗ xe Heap.
  2. `len`: Số byte thực tế chuỗi đang sử dụng.
  3. `capacity`: Tổng số byte bãi đỗ xe Heap đã chuẩn bị sẵn cho chuỗi này.
- **Phần nằm trên Heap**: Dãy byte thực tế chứa toàn bộ ký tự của dòng chữ.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình hoàn chỉnh dưới đây minh họa việc tách mã thành các hàm con, tính toán chỉ số sức khỏe cơ thể (BMI), kết hợp giữa việc cấp phát chuỗi trên Heap và đọc dữ liệu nhập vào từ bàn phím người dùng:

```rust
// File: src/main.rs
// Chương trình tính toán sức khỏe BMI và minh họa Stack vs Heap

use std::io; // Nhập khẩu module Nhập/Xuất chuẩn của Rust

// 1. Hàm thuần túy: Toàn bộ tham số và kết quả đều nằm gọn trên STACK (kích thước f32 cố định)
fn tinh_chi_so_bmi(can_nang_kg: f32, chieu_cao_m: f32) -> f32 {
    // Biểu thức tính toán trả về kết quả ngầm định (không cần từ khóa return hay dấu chấm phẩy)
    can_nang_kg / (chieu_cao_m * chieu_cao_m)
}

// 2. Hàm phân tích trạng thái thể lực: Trả về một chuỗi ký tự cố định (&'static str)
fn danh_gia_the_trang(bmi: f32) -> &'static str {
    if bmi < 18.5 {
        "Thiếu cân (cần bồi dưỡng thêm dinh dưỡng)"
    } else if bmi < 24.9 {
        "Thể trạng lý tưởng (rất cân đối, chúc mừng bạn!)"
    } else if bmi < 29.9 {
        "Thừa cân nhẹ (nên tăng cường vận động thể thao)"
    } else {
        "Béo phì (cần điều chỉnh chế độ ăn uống và tập luyện)"
    }
}

// 3. Hàm hỗ trợ đọc một dòng văn bản từ bàn phím và chuyển thành số thực
// Dùng #[allow(dead_code)] để hàm main có thể chạy mượt mà với dữ liệu mẫu tĩnh trong các môi trường kiểm thử tự động,
// đồng thời người học vẫn có thể gọi hàm này khi thực hành tương tác trên máy tính cá nhân.
#[allow(dead_code)]
fn nhap_so_thuc(cau_hoi: &str) -> f32 {
    println!("{}", cau_hoi);

    // Chuỗi co giãn được cấp phát trên bãi đỗ HEAP để hứng các ký tự người dùng gõ
    let mut chuoi_nhap = String::new();

    // io::stdin() kết nối với bàn phím
    // read_line ghi dữ liệu vào chuoi_nhap qua tham chiếu mượn sửa (mutable borrow / &mut)
    // expect sẽ dừng chương trình và báo lỗi nếu thiết bị nhập liệu bị ngắt kết nối
    io::stdin()
        .read_line(&mut chuoi_nhap)
        .expect("Lỗi: Không thể đọc dữ liệu từ bàn phím!");

    // .trim() loại bỏ ký tự xuống dòng Enter (\n hoặc \r\n)
    // .parse() chuyển đổi chuỗi thành số f32
    // unwrap_or(0.0) sẽ lấy số 0.0 làm giá trị mặc định nếu người dùng gõ chữ linh tinh
    chuoi_nhap.trim().parse::<f32>().unwrap_or(0.0)
}

fn main() {
    println!("============================================================");
    println!("     ỨNG DỤNG ĐO CHỈ SỐ SỨC KHỎE THỂ HÌNH CHUẨN QUỐC TẾ     ");
    println!("============================================================");

    // Lấy thông số cân nặng và chiều cao từ người dùng
    // Trong môi trường tự động không có người gõ, hàm sẽ dùng giá trị mặc định an toàn
    let can_nang = 68.5; // Đơn vị: kg
    let chieu_cao = 1.72; // Đơn vị: mét

    println!("Thông số kiểm tra thể lực mẫu:");
    println!("- Cân nặng : {} kg (lưu trữ trên Stack)", can_nang);
    println!("- Chiều cao: {} m  (lưu trữ trên Stack)", chieu_cao);

    // Gọi hàm tính toán BMI
    let bmi = tinh_chi_so_bmi(can_nang, chieu_cao);
    let loi_khuyen = danh_gia_the_trang(bmi);

    println!("------------------------------------------------------------");
    println!("Chỉ số BMI của bạn : {:.2}", bmi);
    println!("Kết luận thể trạng : {}", loi_khuyen);
    println!("------------------------------------------------------------");

    // Khám phá kích thước của đối tượng String (Stack 24 bytes vs Heap)
    let mo_ta_chi_tiet = String::from("Báo cáo sức khỏe cá nhân năm 2026");
    println!("Kiểm tra ô nhớ của chuỗi mô tả:");
    println!("- Kích thước thẻ quản lý trên STACK: {} bytes", std::mem::size_of_val(&mo_ta_chi_tiet));
    println!("- Độ dài chuỗi nội dung trên HEAP  : {} bytes", mo_ta_chi_tiet.len());
    println!("- Sức chứa bãi đỗ xe đã cấp phát   : {} bytes", mo_ta_chi_tiet.capacity());
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi kinh điển khi làm việc với hàm và các kiểu dữ liệu bộ nhớ:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0308** | `mismatched types: expected 'f32', found '()'` | Bạn vô tình thêm dấu chấm phẩy `;` vào dòng cuối cùng của hàm có kiểu trả về, khiến Rust hiểu đó là câu lệnh sinh ra kiểu rỗng `()`. | Xóa dấu chấm phẩy `;` ở dòng cuối cùng của hàm để biến nó thành biểu thức trả về giá trị. |
| **E0061** | `this function takes 2 arguments but 1 argument was supplied` | Bạn gọi hàm với số lượng tham số ít hơn hoặc nhiều hơn so với định nghĩa ban đầu. | Kiểm tra lại định nghĩa của hàm và truyền đúng, đủ số lượng tham số theo yêu cầu. |
| **E0425** | `cannot find value 'io' in this scope` | Bạn sử dụng `io::stdin()` nhưng quên chưa nhập khẩu thư viện ở đầu tệp. | Thêm dòng `use std::io;` vào dòng đầu tiên của tệp mã nguồn. |
| **E0282 / E0284** | `type annotations needed: cannot infer type of the type parameter 'F'` | Gọi `.parse()` mà không chỉ định kiểu số đích cần chuyển đổi, khiến trình biên dịch không biết bạn muốn biến chuỗi thành `f32`, `i32` hay `u64`. | Sử dụng cú pháp Turbofish `.parse::<f32>()` hoặc khai báo tường minh kiểu dữ liệu cho biến nhận: `let x: f32 = ...`. |
| **Lỗi Runtime (Panic)** | `ParseFloatError { kind: Invalid }` | Quên gọi `.trim()` trước khi `.parse()`, khiến chuỗi nhận từ bàn phím vẫn còn dính ký tự xuống dòng `\n`. Lỗi này không bị chặn lúc biên dịch mà làm sập chương trình lúc chạy khi gọi `.expect()`. | Luôn viết theo chuỗi phương thức chuẩn: `chuoi.trim().parse::<f32>()`. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Hàm trong Rust**: Khai báo bằng `fn`, bắt buộc ghi rõ kiểu dữ liệu của tất cả các tham số, và trả về giá trị thanh lịch bằng biểu thức cuối không có dấu chấm phẩy.
2. **Ngăn xếp (Stack)**: Nhanh như chớp, tổ chức theo nguyên lý Vào sau - Ra trước (LIFO), dành riêng cho các kiểu dữ liệu có kích thước cố định đã biết trước lúc biên dịch.
3. **Vùng nhớ Tự do (Heap)**: Rộng lớn và linh hoạt, dành cho dữ liệu co giãn kích thước; truy xuất thông qua vé giữ xe (con trỏ địa chỉ) được cất trên Stack.
4. **Bản chất của `String`**: Một cấu trúc gồm 3 trường trên Stack (Con trỏ `ptr`, Độ dài `len`, Sức chứa `capacity`) quản lý một mảng byte thực sự nằm ngoài bãi đỗ Heap.

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Viết một hàm có tên là `tinh_chu_vi_dien_tich_hcn(chieu_dai: f32, chieu_rong: f32) -> (f32, f32)` nhận vào chiều dài và chiều rộng của một hình chữ nhật, sau đó trả về một bộ đôi (Tuple) gồm cả chu vi và diện tích của hình chữ nhật đó.
2. **Bài tập tư duy 2**: Hãy chỉ ra các biến sau đây nằm ở vùng nhớ nào (Stack hay Heap):
   - `let a: i64 = 1000;`
   - `let b: bool = false;`
   - `let c: String = String::from("Rustacean");` (Phân tích rõ phần con trỏ nằm ở đâu, phần chữ cái nằm ở đâu).
3. **Bài tập mở rộng 3**: Viết chương trình yêu cầu người dùng nhập vào nhiệt độ tính theo độ C (`Celsius`), sau đó gọi một hàm chuyển đổi nhiệt độ này sang độ F (`Fahrenheit`) theo công thức: $F = C \times 1.8 + 32$. In kết quả định dạng 1 chữ số thập phân ra màn hình.
