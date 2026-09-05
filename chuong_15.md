# Chương 15: Vòng lặp lười biếng & Bộ chuyển đổi dòng chảy: map, filter, fold, collect (Iterators & Consumers: map, filter, fold, collect)

## Giới thiệu & Mục tiêu học tập

Trong lập trình truyền thống, vòng lặp `for` và `while` là những công cụ quen thuộc nhất để duyệt qua một danh sách. Tuy nhiên, cách tiếp cận này buộc lập trình viên phải tự quản lý chỉ số (index), tự kiểm soát điều kiện dừng, và tự tạo các vùng nhớ đệm (buffer) trung gian để chứa kết quả lọc tạm thời. Điều này không chỉ khiến mã nguồn trở nên rối rắm mà còn tiềm ẩn nguy cơ lỗi truy cập bộ nhớ ngoài biên (out-of-bounds error).

Rust giải quyết triệt để vấn đề này bằng một mẫu thiết kế đỉnh cao: **Bộ lặp duyệt dữ liệu (Iterator Pattern)**. Trong Rust, Iterator không đơn thuần là một công cụ duyệt danh sách thông thường, mà là một cỗ máy xử lý dòng dữ liệu sở hữu tính chất **Đánh giá lười biếng (Lazy Evaluation)** và cam kết **Trừu tượng hóa không chi phí (Zero-Cost Abstraction)**. Bạn có thể ghép nối hàng chục phép biến đổi liên tiếp (`map`, `filter`, `take`, `zip`) mà không làm tiêu tốn thêm bất kỳ byte bộ nhớ RAM trung gian nào, đồng thời tốc độ thực thi cuối cùng trên CPU nhanh tương đương hoặc thậm chí vượt trội hơn vòng lặp C viết tay!

Mục tiêu học tập của chương này:
- Nắm vững cấu tạo cốt lõi của Trait **`Iterator`**, kiểu dữ liệu liên kết **`type Item`**, và phương thức then chốt **`next(&mut self)`**.
- Thấu hiểu cơ chế **Đánh giá lười biếng (Lazy Evaluation)**: Vì sao một chuỗi adapter không hề tốn điện năng hay xung nhịp CPU cho đến khi có một hàm tiêu thụ (consumer) kích hoạt.
- Phân biệt 3 phương thức tạo Iterator trên một tập hợp dữ liệu:
  - **`.iter()`**: Mượn đọc từng phần tử (`&T`) theo cơ chế vay mượn (borrow).
  - **`.iter_mut()`**: Mượn sửa từng phần tử (`&mut T`).
  - **`.into_iter()`**: Đoạt quyền sở hữu (ownership) và tiêu thụ toàn bộ tập hợp (`T`).
- Thành thạo các bộ điều hợp trung gian phổ biến: **`map`**, **`filter`**, **`enumerate`**, **`take`**, **`zip`**.
- Làm chủ các hàm tiêu thụ kết thúc: **`collect`**, **`fold`**, **`sum`**, **`find`**, **`any`**, **`all`**.
- Khám phá bí quyết biên dịch tối ưu hóa của LLVM giúp Iterator đạt hiệu năng siêu đỉnh.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy tưởng tượng bạn đang tham quan một **Nhà máy chế biến bánh kẹo tự động hiện đại** với dây chuyền băng chuyền thông minh:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG ĐỜI SỐNG: DÂY CHUYỀN BĂNG CHUYỀN NHÀ MÁY THÔNG MINH       │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   [Kho bánh mộc] ──► [Máy phết bơ] ──► [Máy sàng lọc] ──► [Thùng đóng gói]       │
│        (Data)            (map)            (filter)            (collect)          │
│                                                                                  │
│   - Trạng thái 1: KHI CHƯA CÓ NGƯỜI ĐẶT THÙNG Ở ĐẦU RA (LAZY EVALUATION)        │
│     Băng chuyền đứng im hoàn toàn! Cọ phết bơ không quẹt, rây lọc không rung.    │
│     Không tốn 1 watt điện nào dù bạn đã lắp ráp 10 chiếc máy vào dây chuyền!    │
│                                                                                  │
│   - Trạng thái 2: QUẢN ĐỐC ĐẶT THÙNG HÀNG VÀ BẤM NÚT (CONSUMER: COLLECT/FOLD)    │
│     Chiếc thùng đầu ra kéo một cái: Bánh mộc chạy qua, được phết bơ, lọc bánh vỡ │
│     và rơi ngay ngắn vào thùng. Bánh làm đến đâu đóng thùng đến đó!              │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Băng chuyền ngủ đông (Tính lười biếng - Lazy Evaluation)
- Khi bạn viết `danh_sach.iter().map(...).filter(...)`, bạn mới chỉ đang **lắp ráp các cỗ máy lên khung băng chuyền**.
- Toàn bộ hệ thống vẫn đang trong trạng thái "ngủ đông". Băng chuyền chưa hề quay một milimét nào, chưa có một hạt bụi nào bị lọc, chưa có phép tính nào được thực hiện.
- Chỉ đến khi bạn gọi một hàm tiêu thụ như `.collect()` hay `.fold()` (hành động người công nhân đặt thùng carton ở cuối băng chuyền và bấm nút kéo hàng), dòng dữ liệu mới bắt đầu dịch chuyển từng phần tử một qua các khâu xử lý.

### 2. Cọ quét bơ (`map`)
- Mỗi chiếc bánh đi qua khay sẽ được cây cọ quét thêm một lớp bơ sữa thơm ngon.
- `.map()` nhận từng phần tử đầu vào, biến đổi nó theo một công thức toán học hoặc quy tắc nghiệp vụ, và đưa ra một phần tử có hình thái mới ở đầu ra. Số lượng bánh trước và sau khi qua cọ quét là hoàn toàn bằng nhau.

### 3. Chiếc rây sàng gạo (`filter`)
- Những chiếc bánh đạt chuẩn kích thước sẽ lọt qua mắt rây để đi tiếp. Những chiếc bánh bị vỡ vụn hoặc méo mó sẽ bị giữ lại và loại bỏ.
- `.filter()` chỉ giữ lại những phần tử thỏa mãn một điều kiện đúng (`true`), loại bỏ tất cả những phần tử không đạt chuẩn.

### 4. Nồi nấu cao cô đặc (`fold`)
- Thay vì lấy từng chiếc bánh riêng lẻ ra thùng, bạn gom toàn bộ nguyên liệu trên băng chuyền bỏ vào một chiếc nồi lớn, đun lửa chậm và cô đặc chúng lại thành một thỏi sô-cô-la duy nhất.
- `.fold()` nhận một giá trị khởi tạo ban đầu, sau đó kết hợp tuần tự từng phần tử trong danh sách với biến tích lũy để tạo ra duy nhất **một kết quả tổng hợp** (như tính tổng, tính giá trị trung bình, hoặc xây dựng một cây dữ liệu phức tạp).

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Giải phẫu Trait `Iterator`: Phương thức `next()`

Tất cả các bộ lặp trong Rust đều phải hiện thực Trait `Iterator` được định nghĩa sẵn trong thư viện chuẩn `core::iter::Iterator`:

```rust
pub trait Iterator {
    type Item; // Kiểu dữ liệu của từng phần tử mà bộ lặp sẽ nhả ra

    // Phương thức cốt lõi duy nhất bắt buộc phải tự cài đặt:
    fn next(&mut self) -> Option<Self::Item>;

    // Hàng chục phương thức tiện ích khác (map, filter, fold...) 
    // đã được cài đặt sẵn mặc định (default methods) dựa trên next()!
}
```

Mỗi lần phương thức `next()` được gọi:
- Nếu vẫn còn dữ liệu, nó nhả ra `Some(gia_tri)`.
- Khi đã duyệt hết phần tử cuối cùng, nó nhả ra `None` để báo hiệu kết thúc dòng chảy.

```rust
let danh_sach = vec![10, 20];
let mut bo_lap = danh_sach.iter(); // bo_lap phải là mut vì vị trí con trỏ dịch chuyển

assert_eq!(bo_lap.next(), Some(&10));
assert_eq!(bo_lap.next(), Some(&20));
assert_eq!(bo_lap.next(), None); // Đã cạn kiệt phần tử
```

### 2. Ba phương thức khởi tạo Iterator: Mượn đọc vs Mượn sửa vs Tiêu thụ

Tùy theo mục đích sử dụng bộ nhớ và quyền sở hữu (ownership), Rust cung cấp 3 cách lấy bộ lặp từ một tập hợp (như `Vec<T>`):

| Phương thức | Chữ ký phương thức | Kiểu phần tử sinh ra (`Item`) | Tác động lên tập hợp gốc |
|---|---|---|---|
| **`.iter()`** | `fn iter(&self) -> Iter<'_, T>` | Tham chiếu bất biến `&T` | Tập hợp gốc nguyên vẹn, chỉ đọc, có thể gọi nhiều lần. |
| **`.iter_mut()`** | `fn iter_mut(&mut self) -> IterMut<'_, T>` | Tham chiếu khả biến `&mut T` | Cho phép sửa đổi trực tiếp dữ liệu gốc tại chỗ trên bộ nhớ. |
| **`.into_iter()`** | `fn into_iter(self) -> IntoIter<T>` | Giá trị sở hữu `T` | Tập hợp gốc bị tiêu thụ (Move), biến gốc không thể dùng lại! |

### 3. Phân biệt Bộ điều hợp (Adapters) và Hàm tiêu thụ (Consumers)

- **Bộ điều hợp trung gian (Iterator Adapters)**:
  - Đặc điểm: Biến đổi một bộ lặp thành một bộ lặp mới.
  - Các hàm tiêu biểu: `.map()`, `.filter()`, `.take(n)`, `.skip(n)`, `.enumerate()`, `.zip()`.
  - Tính chất: **Luôn lười biếng (Lazy)**. Nếu bạn viết `danh_sach.iter().map(|x| x * 2);` mà không hứng kết quả bằng một consumer, trình biên dịch `rustc` sẽ cảnh báo: *warning: unused `Map` that must be used*.
- **Hàm tiêu thụ kết thúc (Consumers)**:
  - Đặc điểm: Chủ động gọi liên tục phương thức `next()` cho đến khi nhận được `None`, tổng hợp dữ liệu thành kết quả cụ thể.
  - Các hàm tiêu biểu: `.collect()`, `.fold()`, `.sum()`, `.count()`, `.find()`, `.any()`, `.all()`.

### 4. Bí mật Tốc độ: Tại sao Iterator chạy nhanh hơn Vòng lặp thủ công?

Nhiều lập trình viên từ các ngôn ngữ khác e ngại rằng việc bọc dữ liệu qua hàng loạt struct (`Map<Filter<Iter<...>>>`) sẽ làm chậm chương trình do chi phí gọi hàm ảo (virtual call overhead). Nhưng trong Rust:
1. **Đơn hình hóa và Nội tuyến (Monomorphization & Inlining)**: Trình biên dịch bung toàn bộ chuỗi adapter thành một cấu trúc phẳng duy nhất lúc biên dịch.
2. **Triệt tiêu kiểm tra biên giới hạn (Bounds Check Elimination)**: Trong vòng lặp `for i in 0..len` truyền thống, CPU phải so sánh `i < len` ở mỗi chu kỳ để tránh tràn ô nhớ. Với Iterator, Rust kiểm soát chặt chẽ điểm đầu và điểm cuối, cho phép trình tối ưu LLVM loại bỏ hoàn toàn các lệnh rẽ nhánh kiểm tra biên, đồng thời tự động vector hóa mã máy bằng các lệnh SIMD siêu tốc trên CPU hiện đại!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là chương trình hoàn chỉnh xây dựng **Hệ thống Phân tích Dữ liệu Cảm biến Nhà máy Thông minh (Industrial IoT Telemetry Pipeline)**. Chương trình sử dụng đầy đủ `.iter()`, `.iter_mut()`, `.into_iter()`, kết hợp các bộ điều hợp `filter`, `map`, `enumerate` và các hàm tiêu thụ `fold`, `sum`, `collect`.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Iterator: map, filter, fold, collect trong Rust

#[derive(Debug, Clone, PartialEq)]
pub struct BanGhiCamBien {
    pub ma_cam_bien: String,
    pub nhiet_do_c: f64,
    pub ap_suat_bar: f64,
    pub hop_le: bool,
}

#[derive(Debug, PartialEq)]
pub struct ThongBaoNguyHiem {
    pub thu_tu_ghi_nhan: usize,
    pub noi_dung: String,
    pub muc_do: String,
}

fn main() {
    println!("============================================================");
    println!("   HỆ THỐNG XỬ LÝ DÒNG DỮ LIỆU CẢM BIẾN NHÀ MÁY (IOT FP)   ");
    println!("============================================================");

    // 1. Khởi tạo danh sách dữ liệu cảm biến thô ban đầu
    let mut du_lieu_tho: Vec<BanGhiCamBien> = vec![
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-01"),
            nhiet_do_c: 85.5,
            ap_suat_bar: 3.2,
            hop_le: true,
        },
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-02"),
            nhiet_do_c: -999.0, // Dữ liệu lỗi do đứt dây cáp
            ap_suat_bar: 0.0,
            hop_le: false,
        },
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-03"),
            nhiet_do_c: 125.0, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            ap_suat_bar: 4.8,
            hop_le: true,
        },
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-04"),
            nhiet_do_c: 72.0,
            ap_suat_bar: 2.9,
            hop_le: true,
        },
        BanGhiCamBien {
            ma_cam_bien: String::from("CB-LO-05"),
            nhiet_do_c: 110.5, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            ap_suat_bar: 5.1,
            hop_le: true,
        },
    ];

    println!("Số lượng bản ghi thu thập được: {}", du_lieu_tho.len());

    // ------------------------------------------------------------------------
    // KỸ THUẬT 1: Dùng .iter_mut() để hiệu chỉnh dữ liệu trực tiếp tại chỗ
    // Giả sử cảm biến có sai số cố định +0.5°C cần được bù trừ
    // ------------------------------------------------------------------------
    println!("\n1. Tiến hành bù trừ sai số thiết bị qua .iter_mut():");
    du_lieu_tho
        .iter_mut()
        .filter(|ban_ghi| ban_ghi.hop_le)
        .for_each(|ban_ghi| {
            ban_ghi.nhiet_do_c -= 0.5; // Trừ trực tiếp trên ô nhớ RAM
        });
    println!("-> Đã hiệu chỉnh sai số cho tất cả cảm biến hợp lệ thành công.");

    // ------------------------------------------------------------------------
    // KỸ THUẬT 2: Dùng .iter(), .filter(), .map() xây dựng đường ống lọc & trích xuất
    // Lấy danh sách nhiệt độ của các cảm biến an toàn (nhiệt độ <= 100°C)
    // ------------------------------------------------------------------------
    println!("\n2. Trích xuất danh sách nhiệt độ hoạt động an toàn (<= 100°C):");
    let nhiet_do_an_toan: Vec<f64> = du_lieu_tho
        .iter()
        .filter(|bg| bg.hop_le)                  // Lọc bỏ cảm biến hỏng
        .filter(|bg| bg.nhiet_do_c <= 100.0)     // Lọc cảm biến trong ngưỡng an toàn
        .map(|bg| bg.nhiet_do_c)                 // Chỉ trích xuất lấy số đo nhiệt độ
        .collect();                              // Gom tụ thành Vector mới

    println!("-> Các mức nhiệt độ an toàn: {:?}", nhiet_do_an_toan);

    // ------------------------------------------------------------------------
    // KỸ THUẬT 3: Dùng .fold() để tổng hợp thống kê phức tạp trong một lượt duyệt duy nhất
    // Tính tổng nhiệt độ và đếm số lượng cảm biến an toàn để tính trung bình
    // ------------------------------------------------------------------------
    println!("\n3. Tính nhiệt độ trung bình của phân xưởng qua .fold():");
    let (tong_nhiet, so_luong) = du_lieu_tho
        .iter()
        .filter(|bg| bg.hop_le)
        .fold((0.0, 0usize), |(tong, dem), bg| {
            (tong + bg.nhiet_do_c, dem + 1)
        });

    if so_luong > 0 {
        let trung_binh = tong_nhiet / (so_luong as f64);
        println!("-> Tổng nhiệt độ: {:.2}°C trên {} cảm biến.", tong_nhiet, so_luong);
        println!("-> Nhiệt độ trung bình toàn xưởng: {:.2}°C", trung_binh);
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 4: Kết hợp .enumerate(), .filter(), và .collect()
    // Tạo danh sách cảnh báo khẩn cấp cho các cảm biến vượt ngưỡng (> 100°C)
    // ------------------------------------------------------------------------
    println!("\n4. Phát hiện nguy cơ và tổng hợp danh sách cảnh báo khẩn cấp:");
    let danh_sach_canh_bao: Vec<ThongBaoNguyHiem> = du_lieu_tho
        .iter()
        .enumerate() // Cung cấp chỉ số thứ tự (0, 1, 2...) đi kèm với phần tử
        .filter(|(_, bg)| bg.hop_le && bg.nhiet_do_c > 100.0)
        .map(|(chi_so, bg)| ThongBaoNguyHiem {
            thu_tu_ghi_nhan: chi_so + 1,
            noi_dung: format!("Cảm biến [{}] vượt ngưỡng nhiệt độ: {:.2}°C", bg.ma_cam_bien, bg.nhiet_do_c),
            muc_do: String::from("KHẨN CẤP"),
        })
        .collect();

    for cb in &danh_sach_canh_bao {
        println!("  [!] Vị trí #{}: {} (Mức độ: {})", 
                 cb.thu_tu_ghi_nhan, cb.noi_dung, cb.muc_do);
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 5: Dùng .into_iter() để tiêu thụ toàn bộ dữ liệu và giải phóng bộ nhớ
    // ------------------------------------------------------------------------
    println!("\n5. Di chuyển quyền sở hữu toàn bộ qua .into_iter():");
    let ma_tat_ca_cam_bien: Vec<String> = du_lieu_tho
        .into_iter()
        .map(|bg| bg.ma_cam_bien) // Đoạt quyền sở hữu trường String mà không cần clone!
        .collect();

    println!("-> Danh sách mã thiết bị sau khi thu hồi: {:?}", ma_tat_ca_cam_bien);
    // du_lieu_tho đã bị tiêu thụ tại đây, giải phóng bộ nhớ sạch sẽ!

    println!("\n============================================================");
    println!("     XỬ LÝ TOÀN BỘ ĐƯỜNG ỐNG ITERATOR THÀNH CÔNG RỰC RỠ     ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Các lỗi biên dịch phổ biến nhất khi làm việc với Iterator trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0282** | `type annotations needed for 'Vec<_>'` | Bạn gọi `.collect()` nhưng không ghi rõ kiểu dữ liệu mong muốn nhận về. Trình biên dịch không biết bạn muốn gom dữ liệu thành `Vec`, `HashSet` hay kiểu tập hợp nào. | Chú thích kiểu tường minh ở biến hứng: `let res: Vec<i32> = ...;` hoặc dùng cú pháp Turbofish: `.collect::<Vec<_>>()`. |
| **E0507** | `cannot move out of '...' which is behind a shared reference` | Bạn đang dùng `.iter()` (chỉ mượn tham chiếu `&T`) nhưng trong closure của `.map()` bạn lại cố lấy quyền sở hữu của phần tử không có thuộc tính `Copy` (như `String`). | Đổi sang `.into_iter()` nếu muốn lấy quyền sở hữu, hoặc gọi `.clone()`, hoặc chỉ thao tác trên tham chiếu `&`. |
| **E0277** | `the trait bound '...: Iterator' is not satisfied` | Bạn cố gọi một phương thức iterator (như `.map()`) trực tiếp trên một tập hợp mà quên chưa biến nó thành bộ lặp qua `.iter()`. | Gọi phương thức `.iter()`, `.iter_mut()`, hoặc `.into_iter()` trước khi gọi các adapter. |
| **E0308** | `mismatched types in closure of fold` | Trong hàm `.fold(khoi_tao, |tich_luy, item| ...)`, giá trị trả về của closure không khớp với kiểu của biến tích lũy `khoi_tao`. | Kiểm tra lại kiểu của biểu thức cuối cùng trong thân closure của `.fold()`, đảm bảo nó khớp chính xác với kiểu khởi tạo. |

### Phân tích lỗi thực tế `E0282` (Thiếu chú thích kiểu khi gọi `collect`):

```rust
// Đoạn mã lỗi minh họa:
fn thu_nghiem_loi_collect() {
    let mang = vec![1, 2, 3];
    // LỖI E0282: rustc không biết gom thành kiểu gì
    // let ket_qua = mang.iter().map(|x| x * 2).collect(); 
}

// Cách sửa chữa chuẩn mực:
fn thu_nghiem_dung() {
    let mang = vec![1, 2, 3];
    // Cách A: Chú thích kiểu ở phía biến
    let ket_qua_a: Vec<i32> = mang.iter().map(|x| x * 2).collect();

    // Cách B: Sử dụng cú pháp cá voi Turbofish ::<Vec<_>>()
    let ket_qua_b = mang.iter().map(|x| x * 2).collect::<Vec<_>>();
    println!("{:?} - {:?}", ket_qua_a, ket_qua_b);
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Bản chất Trait `Iterator`**: Chỉ cần cài đặt duy nhất một phương thức `fn next(&mut self) -> Option<Self::Item>`, bạn lập tức sở hữu miễn phí hàng chục phương thức biến đổi dữ liệu cao cấp.
2. **Tính lười biếng (Lazy Evaluation)**: Chuỗi adapter không thực thi bất kỳ phép tính nào cho đến khi hàm tiêu thụ (consumer) như `collect` hay `fold` yêu cầu kết quả.
3. **Ba chế độ duyệt**:
   - `.iter()`: Mượn đọc (`&T`).
   - `.iter_mut()`: Mượn sửa trực tiếp (`&mut T`).
   - `.into_iter()`: Đoạt quyền sở hữu (`T`) và tiêu thụ tập hợp gốc.
4. **Hiệu năng Zero-Cost**: Không tốn chi phí gọi hàm trung gian nhờ cơ chế Monomorphization và tối ưu hóa loại bỏ kiểm tra biên giới hạn của LLVM.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phân tách Chẵn - Lẻ qua Iterator)**:  
   Cho một danh sách số nguyên: `let so = vec![12, 7, 19, 24, 30, 5, 8];`.  
   Hãy dùng đường ống Iterator để:
   - Lọc ra các số chẵn.
   - Bình phương từng số chẵn đó.
   - Thu gom vào một `Vec<i32>` mới bằng `.collect()`.

2. **Bài tập 2 (Xây dựng Bộ tính toán với `.fold()`)**:  
   Dùng phương thức `.fold()` để tìm giá trị lớn nhất trong một lát cắt số nguyên `&[i32]` mà không sử dụng phương thức `.max()` có sẵn của Rust. Khởi tạo giá trị ban đầu một cách khéo léo để chương trình hoạt động chính xác.

3. **Bài tập 3 (Tự tạo Trait Iterator đơn giản)**:  
   Tạo một struct mang tên `BoDemNguoc { hien_tai: u32 }`. Triển khai Trait `Iterator` cho struct này sao cho mỗi lần gọi `.next()`, nó đếm lùi từ một con số cho trước về `1`, và trả về `None` khi số hiện tại chạm mốc `0`. Kiểm tra hoạt động của nó với vòng lặp `for`.
