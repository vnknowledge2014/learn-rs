# Chương 16: Bộ lặp lười biếng & Toàn bộ đường ống dữ liệu: map, filter_map, fold, collect (Iterators & the Complete Data Pipeline Toolkit)

## Giới thiệu & Mục tiêu học tập

Trong lập trình truyền thống, vòng lặp `for` và `while` là những công cụ quen thuộc nhất để duyệt qua một danh sách. Tuy nhiên, cách tiếp cận này buộc lập trình viên phải tự quản lý chỉ số (index), tự kiểm soát điều kiện dừng, và tự tạo các vùng nhớ đệm (buffer) trung gian để chứa kết quả lọc tạm thời. Điều này không chỉ khiến mã nguồn trở nên rối rắm mà còn tiềm ẩn nguy cơ lỗi truy cập bộ nhớ ngoài biên (out-of-bounds error).

Rust giải quyết triệt để vấn đề này bằng một mẫu thiết kế đỉnh cao: **Bộ lặp duyệt dữ liệu (Iterator Pattern)**. Trong Rust, Iterator không đơn thuần là một công cụ duyệt danh sách thông thường, mà là một cỗ máy xử lý dòng dữ liệu sở hữu tính chất **Đánh giá lười biếng (Lazy Evaluation)** và cam kết **Trừu tượng hóa không chi phí (Zero-Cost Abstraction)**. Bạn có thể ghép nối hàng chục phép biến đổi liên tiếp (`map`, `filter`, `take`, `zip`) mà không làm tiêu tốn thêm bất kỳ byte bộ nhớ RAM trung gian nào, đồng thời tốc độ thực thi cuối cùng trên CPU nhanh ngang vòng lặp viết tay (đôi khi còn nhỉnh hơn vòng lặp dùng chỉ số, vì không phải kiểm tra biên)!

Mục tiêu học tập của chương này:
- Nắm vững cấu tạo cốt lõi của Trait **`Iterator`**, kiểu dữ liệu liên kết **`type Item`**, và phương thức then chốt **`next(&mut self)`**.
- Thấu hiểu cơ chế **Đánh giá lười biếng (Lazy Evaluation)**: Vì sao một chuỗi adapter không hề tốn điện năng hay xung nhịp CPU cho đến khi có một hàm tiêu thụ (consumer) kích hoạt.
- Phân biệt 3 phương thức tạo Iterator trên một tập hợp dữ liệu:
  - **`.iter()`**: Mượn đọc từng phần tử (`&T`) theo cơ chế vay mượn (borrow).
  - **`.iter_mut()`**: Mượn sửa từng phần tử (`&mut T`).
  - **`.into_iter()`**: Đoạt quyền sở hữu (ownership) và tiêu thụ toàn bộ tập hợp (`T`).
- Thành thạo các bộ điều hợp trung gian: **`map`**, **`filter`**, **`filter_map`**, **`flat_map`**, **`flatten`**, **`enumerate`**, **`take`**, **`take_while`**, **`skip`**, **`skip_while`**, **`zip`**, **`chain`**, **`rev`**, **`step_by`**, **`scan`**, **`peekable`**, **`inspect`**.
- Làm chủ các hàm tiêu thụ kết thúc: **`collect`**, **`fold`**, **`reduce`**, **`try_fold`**, **`sum`**, **`product`**, **`count`**, **`find`**, **`position`**, **`any`**, **`all`**, **`min_by_key`**, **`max_by_key`**, **`partition`**, **`unzip`**.
- Hiểu ba trait nằm sau hậu trường: **`IntoIterator`** (vì sao `for x in &vec` chạy được), **`FromIterator`** (vì sao `collect()` đổi được kiểu đích), và **`Extend`**.
- Phân biệt **gấp trái (`fold`) và gấp phải (`rfold`)**, biết khi nào thứ tự gộp ảnh hưởng tới kết quả.
- Tự cài đặt `Iterator` và `IntoIterator` cho kiểu dữ liệu của riêng mình.
- Biết cách **song song hóa một đường ống** bằng `rayon` — phần thưởng cụ thể của tính thuần túy đã hứa ở Chương 13.
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
- Khi bạn viết `list.iter().map(...).filter(...)`, bạn mới chỉ đang **lắp ráp các cỗ máy lên khung băng chuyền**.
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
- Nếu vẫn còn dữ liệu, nó nhả ra `Some(value)`.
- Khi đã duyệt hết phần tử cuối cùng, nó nhả ra `None` để báo hiệu kết thúc dòng chảy.

```rust
let list = vec![10, 20];
let mut iter = list.iter(); // iter phải là mut vì vị trí con trỏ dịch chuyển

assert_eq!(iter.next(), Some(&10));
assert_eq!(iter.next(), Some(&20));
assert_eq!(iter.next(), None); // Đã cạn kiệt phần tử
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
  - Tính chất: **Luôn lười biếng (Lazy)**. Nếu bạn viết `list.iter().map(|x| x * 2);` mà không hứng kết quả bằng một consumer, trình biên dịch `rustc` sẽ cảnh báo: *warning: unused `Map` that must be used*.
- **Hàm tiêu thụ kết thúc (Consumers)**:
  - Đặc điểm: Chủ động gọi liên tục phương thức `next()` cho đến khi nhận được `None`, tổng hợp dữ liệu thành kết quả cụ thể.
  - Các hàm tiêu biểu: `.collect()`, `.fold()`, `.sum()`, `.count()`, `.find()`, `.any()`, `.all()`.

### 4. Bí mật Tốc độ: Tại sao Iterator chạy nhanh ngang Vòng lặp thủ công?

Nhiều lập trình viên từ các ngôn ngữ khác e ngại rằng việc bọc dữ liệu qua hàng loạt struct (`Map<Filter<Iter<...>>>`) sẽ làm chậm chương trình do chi phí gọi hàm ảo (virtual call overhead). Nhưng trong Rust:
1. **Đơn hình hóa và Nội tuyến (Monomorphization & Inlining)**: Trình biên dịch bung toàn bộ chuỗi adapter thành một cấu trúc phẳng duy nhất lúc biên dịch.
2. **Triệt tiêu kiểm tra biên giới hạn (Bounds Check Elimination)**: Trong vòng lặp `for i in 0..v.len()` dùng chỉ số, phép so sánh `i < len` để dừng vòng lặp thì vòng lặp nào cũng có (iterator cũng phải so con trỏ hiện tại với con trỏ cuối). Cái tốn thêm nằm ở chỗ khác: mỗi lần truy cập `v[i]`, Rust chèn **thêm một phép kiểm tra biên** để panic nếu `i` vượt quá độ dài — LLVM chỉ xoá được kiểm tra này khi chứng minh được `i` luôn hợp lệ. Iterator thì không bao giờ đánh chỉ số, nên không có kiểm tra biên nào để xoá; vòng lặp gọn hơn và LLVM dễ tự động vector hóa bằng các lệnh SIMD trên CPU hiện đại. Kết quả: iterator **nhanh ngang** vòng lặp viết tay tốt nhất, và thường nhanh hơn vòng lặp chỉ số mà LLVM không gỡ được kiểm tra biên.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là chương trình hoàn chỉnh xây dựng **Hệ thống Phân tích Dữ liệu Cảm biến Nhà máy Thông minh (Industrial IoT Telemetry Pipeline)**. Chương trình sử dụng đầy đủ `.iter()`, `.iter_mut()`, `.into_iter()`, kết hợp các bộ điều hợp `filter`, `map`, `enumerate` và các hàm tiêu thụ `fold`, `sum`, `collect`.

```rust
// Tệp: src/main.rs
// Chương trình thực chiến làm chủ Iterator: map, filter, fold, collect trong Rust

#[derive(Debug, Clone, PartialEq)]
pub struct SensorRecord {
    pub sensor_id: String,
    pub temp_c: f64,
    pub pressure_bar: f64,
    pub is_valid: bool,
}

#[derive(Debug, PartialEq)]
pub struct DangerAlert {
    pub position: usize,
    pub content: String,
    pub severity: String,
}

fn main() {
    println!("============================================================");
    println!("   HỆ THỐNG XỬ LÝ DÒNG DỮ LIỆU CẢM BIẾN NHÀ MÁY (IOT FP)   ");
    println!("============================================================");

    // 1. Khởi tạo danh sách dữ liệu cảm biến thô ban đầu
    let mut raw_data: Vec<SensorRecord> = vec![
        SensorRecord {
            sensor_id: String::from("CB-LO-01"),
            temp_c: 85.5,
            pressure_bar: 3.2,
            is_valid: true,
        },
        SensorRecord {
            sensor_id: String::from("CB-LO-02"),
            temp_c: -999.0, // Dữ liệu lỗi do đứt dây cáp
            pressure_bar: 0.0,
            is_valid: false,
        },
        SensorRecord {
            sensor_id: String::from("CB-LO-03"),
            temp_c: 125.0, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            pressure_bar: 4.8,
            is_valid: true,
        },
        SensorRecord {
            sensor_id: String::from("CB-LO-04"),
            temp_c: 72.0,
            pressure_bar: 2.9,
            is_valid: true,
        },
        SensorRecord {
            sensor_id: String::from("CB-LO-05"),
            temp_c: 110.5, // Nhiệt độ quá ngưỡng cảnh báo (> 100°C)
            pressure_bar: 5.1,
            is_valid: true,
        },
    ];

    println!("Số lượng bản ghi thu thập được: {}", raw_data.len());

    // ------------------------------------------------------------------------
    // KỸ THUẬT 1: Dùng .iter_mut() để hiệu chỉnh dữ liệu trực tiếp tại chỗ
    // Giả sử cảm biến có sai số cố định +0.5°C cần được bù trừ
    // ------------------------------------------------------------------------
    println!("\n1. Tiến hành bù trừ sai số thiết bị qua .iter_mut():");
    raw_data
        .iter_mut()
        .filter(|record| record.is_valid)
        .for_each(|record| {
            record.temp_c -= 0.5; // Trừ trực tiếp trên ô nhớ RAM
        });
    println!("-> Đã hiệu chỉnh sai số cho tất cả cảm biến hợp lệ thành công.");

    // ------------------------------------------------------------------------
    // KỸ THUẬT 2: Dùng .iter(), .filter(), .map() xây dựng đường ống lọc & trích xuất
    // Lấy danh sách nhiệt độ của các cảm biến an toàn (nhiệt độ <= 100°C)
    // ------------------------------------------------------------------------
    println!("\n2. Trích xuất danh sách nhiệt độ hoạt động an toàn (<= 100°C):");
    let safe_temps: Vec<f64> = raw_data
        .iter()
        .filter(|record| record.is_valid) // Lọc bỏ cảm biến hỏng
        .filter(|record| record.temp_c <= 100.0) // Lọc cảm biến trong ngưỡng an toàn
        .map(|record| record.temp_c) // Chỉ trích xuất lấy số đo nhiệt độ
        .collect(); // Gom tụ thành Vector mới

    println!("-> Các mức nhiệt độ an toàn: {:?}", safe_temps);

    // ------------------------------------------------------------------------
    // KỸ THUẬT 3: Dùng .fold() để tổng hợp thống kê phức tạp trong một lượt duyệt duy nhất
    // Tính tổng nhiệt độ và đếm số lượng cảm biến an toàn để tính trung bình
    // ------------------------------------------------------------------------
    println!("\n3. Tính nhiệt độ trung bình của phân xưởng qua .fold():");
    let (total_temp, valid_count) = raw_data
        .iter()
        .filter(|record| record.is_valid)
        .fold((0.0, 0usize), |(total, count), record| {
            (total + record.temp_c, count + 1)
        });

    if valid_count > 0 {
        let mean = total_temp / (valid_count as f64);
        println!(
            "-> Tổng nhiệt độ: {:.2}°C trên {} cảm biến.",
            total_temp, valid_count
        );
        println!("-> Nhiệt độ trung bình toàn xưởng: {:.2}°C", mean);
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 4: Kết hợp .enumerate(), .filter(), và .collect()
    // Tạo danh sách cảnh báo khẩn cấp cho các cảm biến vượt ngưỡng (> 100°C)
    // ------------------------------------------------------------------------
    println!("\n4. Phát hiện nguy cơ và tổng hợp danh sách cảnh báo khẩn cấp:");
    let alerts: Vec<DangerAlert> = raw_data
        .iter()
        .enumerate() // Cung cấp chỉ số thứ tự (0, 1, 2...) đi kèm với phần tử
        .filter(|(_, record)| record.is_valid && record.temp_c > 100.0)
        .map(|(index, record)| DangerAlert {
            position: index + 1,
            content: format!(
                "Cảm biến [{}] vượt ngưỡng nhiệt độ: {:.2}°C",
                record.sensor_id, record.temp_c
            ),
            severity: String::from("KHẨN CẤP"),
        })
        .collect();

    for alert in &alerts {
        println!(
            "  [!] Vị trí #{}: {} (Mức độ: {})",
            alert.position, alert.content, alert.severity
        );
    }

    // ------------------------------------------------------------------------
    // KỸ THUẬT 5: Dùng .into_iter() để tiêu thụ toàn bộ dữ liệu và giải phóng bộ nhớ
    // ------------------------------------------------------------------------
    println!("\n5. Di chuyển quyền sở hữu toàn bộ qua .into_iter():");
    let all_sensor_ids: Vec<String> = raw_data
        .into_iter()
        .map(|record| record.sensor_id) // Đoạt quyền sở hữu trường String mà không cần clone!
        .collect();

    println!(
        "-> Danh sách mã thiết bị sau khi thu hồi: {:?}",
        all_sensor_ids
    );
    // raw_data đã bị tiêu thụ tại đây, giải phóng bộ nhớ sạch sẽ!

    println!("\n============================================================");
    println!("     XỬ LÝ TOÀN BỘ ĐƯỜNG ỐNG ITERATOR THÀNH CÔNG RỰC RỠ     ");
    println!("============================================================");
}
```

---

## Mở rộng: Bộ công cụ Iterator đầy đủ (The Complete Iterator Toolkit)

Phần trên đã dạy bộ khung. Nhưng trong công việc thực tế, phần lớn sức mạnh của Rust nằm ở những bộ điều hợp mà người tự học hiếm khi tình cờ gặp. Mục này liệt kê **toàn bộ** nhóm công cụ đáng dùng hằng ngày, kèm một chương trình chạy được minh họa từng cái.

### 1. Bảng tra cứu bộ điều hợp (Adapters — lười biếng, trả về iterator mới)

| Bộ điều hợp | Chữ ký rút gọn | Dùng khi nào |
|---|---|---|
| `map(f)` | `A -> B` | Biến đổi từng phần tử, giữ nguyên số lượng |
| `filter(p)` | `&A -> bool` | Giữ lại phần tử thỏa điều kiện |
| **`filter_map(f)`** | `A -> Option<B>` | **Lọc và biến đổi cùng lúc** — bỏ qua phần tử hỏng khi phân tích dữ liệu |
| **`flat_map(f)`** | `A -> IntoIterator<B>` | Mỗi phần tử nở ra thành nhiều phần tử (tách từ, mở danh sách lồng) |
| **`flatten()`** | `Iterator<Iterator<A>>` | Làm phẳng một tầng lồng nhau |
| `enumerate()` | → `(usize, A)` | Cần chỉ số đi kèm |
| `take(n)` / `skip(n)` | | Lấy / bỏ `n` phần tử đầu |
| **`take_while(p)`** / **`skip_while(p)`** | `&A -> bool` | Lấy / bỏ **cho tới khi** điều kiện sai — dừng sớm, khác hẳn `filter` |
| `zip(other)` | → `(A, B)` | Ghép hai dòng dữ liệu song song; dừng ở dòng ngắn hơn |
| `chain(other)` | | Nối hai iterator thành một |
| `rev()` | | Duyệt ngược (cần `DoubleEndedIterator`) |
| `step_by(n)` | | Lấy cách quãng: phần tử 0, n, 2n… |
| **`scan(init, f)`** | | Như `fold` nhưng **nhả ra giá trị trung gian ở mỗi bước** (tính tổng lũy kế) |
| `peekable()` | | Cho phép "nhìn trộm" phần tử kế tiếp mà chưa tiêu thụ nó |
| `inspect(f)` | | Chèn `println!` để gỡ lỗi giữa đường ống mà không đổi dữ liệu |

> **`filter_map` là bộ điều hợp bị bỏ quên nhiều nhất.** Khi phân tích dữ liệu bẩn từ tệp hay mạng, `.filter_map(|s| s.parse::<i32>().ok())` vừa thử chuyển đổi vừa bỏ qua dòng hỏng, chỉ trong một bước.

### 2. Bảng tra cứu hàm tiêu thụ (Consumers — chạy thật, trả về giá trị)

| Hàm tiêu thụ | Trả về | Dùng khi nào |
|---|---|---|
| `collect()` | `Vec`, `String`, `HashMap`, `HashSet`, `Result`, `Option`… | Gom kết quả (xem mục 4) |
| `fold(init, f)` | một giá trị | Gộp có giá trị khởi tạo — luôn dùng được, kể cả danh sách rỗng |
| **`reduce(f)`** | `Option<A>` | Gộp **không** cần giá trị khởi tạo; trả `None` nếu rỗng |
| **`try_fold(init, f)`** | `Result` / `Option` | Gộp **có thể thất bại**, dừng ngay ở lỗi đầu tiên |
| `sum()` / `product()` | số | Đứng sau là trait `Sum` / `Product` (chính là vị nhóm ở Chương 18) |
| `count()` | `usize` | Đếm phần tử |
| `find(p)` / `position(p)` | `Option<A>` / `Option<usize>` | Tìm phần tử / vị trí đầu tiên thỏa điều kiện; **dừng ngay khi thấy** |
| `any(p)` / `all(p)` | `bool` | Có ít nhất một / tất cả đều thỏa; đều **ngắn mạch** |
| `min_by_key(f)` / `max_by_key(f)` | `Option<A>` | Tìm cực trị theo một tiêu chí |
| **`partition(p)`** | `(Vec, Vec)` | Chia đôi thành "thỏa" và "không thỏa" trong một lượt |
| `unzip()` | `(Vec, Vec)` | Tách một dòng cặp thành hai danh sách |
| `for_each(f)` | `()` | Chỉ dùng khi cần tác dụng phụ (in, ghi log) |

### 3. Gấp trái và gấp phải: khi thứ tự gộp có ý nghĩa

```rust
let nums = [10i32, 3, 2];

// Phép CỘNG: giao hoán + kết hợp -> hai chiều cho CÙNG kết quả
assert_eq!(nums.iter().fold(0, |a, b| a + b), nums.iter().rfold(0, |a, b| a + b)); // 15 == 15

// NỐI CHUỖI: kết hợp nhưng KHÔNG giao hoán -> hai chiều cho kết quả KHÁC NHAU
let left: String = nums.iter().fold(String::new(), |a, b| a + &b.to_string());   // "1032"
let right: String = nums.iter().rfold(String::new(), |a, b| a + &b.to_string());  // "2310"
assert_ne!(left, right);
```

Hãy phân biệt cho thật rõ hai tính chất, vì chúng trả lời hai câu hỏi khác nhau:

| Tính chất | Đẳng thức | Nó cho phép điều gì? |
|---|---|---|
| **Kết hợp** (associative) | `(a⊕b)⊕c = a⊕(b⊕c)` | **Chia nhỏ dữ liệu ra nhiều luồng** rồi ghép lại |
| **Giao hoán** (commutative) | `a⊕b = b⊕a` | **Đảo thứ tự phần tử** mà kết quả không đổi |

`fold` và `rfold` duyệt theo hai chiều ngược nhau, nên chúng cho cùng kết quả khi phép gộp **giao hoán** (cộng, nhân, max, min), và cho kết quả khác nhau khi phép gộp **không giao hoán** (nối chuỗi, nối danh sách). Đây là lý do bạn phải biết mình đang gộp bằng phép gì trước khi động tới song song hóa — chủ đề đầy đủ nằm ở Chương 18.

`rfold` và `rev()` đòi hỏi iterator cài đặt **`DoubleEndedIterator`** — tức là biết đi từ hai đầu. `Vec`, mảng, `VecDeque` có; còn iterator đọc từ mạng thì không.

### 4. `collect()` không chỉ tạo ra `Vec`

Đây là chỗ nhiều người học bỏ lỡ nhiều nhất. `collect()` gom được vào **bất kỳ kiểu nào cài đặt `FromIterator`**:

```rust
let v: Vec<i32>              = (1..4).collect();
let s: String                = ['R','u','s','t'].into_iter().collect();
let set: HashSet<i32>        = [1, 2, 2, 3].into_iter().collect();
let map: HashMap<&str, i32>  = [("a", 1), ("b", 2)].into_iter().collect();
let res: Result<Vec<i32>, _> = ["1","2"].iter().map(|s| s.parse::<i32>()).collect();
```

Dòng cuối cùng đặc biệt quan trọng: gom `Iterator<Result<T,E>>` thành `Result<Vec<T>, E>`. Nếu **mọi** phần tử đều `Ok` thì được cả danh sách; chỉ cần **một** phần tử `Err` là toàn bộ trả lỗi. Chúng ta sẽ gọi đúng tên kỹ thuật này ở Chương 19 (*Traversable*).

### 5. Ba trait đứng sau hậu trường

- **`IntoIterator`** — trả lời câu hỏi *"vì sao `for x in &vec` chạy được?"*. Vòng lặp `for` trong Rust chỉ là đường cú pháp cho `IntoIterator::into_iter`. `Vec<T>` có ba cài đặt: cho `Vec<T>` (cho ra `T`), cho `&Vec<T>` (cho ra `&T`), và cho `&mut Vec<T>` (cho ra `&mut T`). Đó chính xác là ba chế độ duyệt bạn đã học ở mục 2.
- **`FromIterator`** — trả lời câu hỏi *"vì sao `collect()` đổi được kiểu đích?"*. Mỗi kiểu tự khai báo cách dựng chính nó từ một iterator.
- **`Extend`** — cho phép **nối thêm** vào một tập hợp đã có: `v.extend(iter)`. Dùng khi bạn muốn gom vào một `Vec` sẵn có thay vì tạo cái mới.

### 6. Vòng lặp bằng gì trong lập trình hàm? Đệ quy — và cạm bẫy của Rust

Ở Chương 13 chúng ta nói lập trình hàm "không dùng vòng lặp và biến thay đổi". Câu hỏi hiển nhiên tiếp theo là: *vậy lặp bằng gì?* Câu trả lời kinh điển của các ngôn ngữ hàm là **đệ quy**.

```rust
// Đệ quy thông thường: phép cộng diễn ra SAU khi lời gọi con trả về
fn sum_rec(list: &[i64]) -> i64 {
    match list {
        [] => 0,
        [first, remaining @ ..] => first + sum_rec(remaining),  // còn việc phải làm sau lời gọi
    }
}

// Đệ quy ĐUÔI (tail recursion): lời gọi đệ quy là việc CUỐI CÙNG,
// kết quả tích lũy được mang theo trong tham số `acc`.
fn sum_tail(list: &[i64], acc: i64) -> i64 {
    match list {
        [] => acc,
        [first, remaining @ ..] => sum_tail(remaining, acc + first),  // không còn việc gì sau đó
    }
}
```

Trong Haskell, PureScript hay Scheme, dạng thứ hai được trình biên dịch **biến thành vòng lặp** (tối ưu hóa lời gọi đuôi — *tail call optimization*), nên chạy với ngăn xếp cố định dù danh sách dài bao nhiêu.

> **⚠️ CẠM BẪY LỚN NHẤT KHI MANG THÓI QUEN FP SANG RUST:**
> **Rust KHÔNG bảo đảm tối ưu hóa lời gọi đuôi.** Trình tối ưu hóa LLVM *đôi khi* làm được ở bản `--release`, nhưng đây **không phải cam kết của ngôn ngữ**. Ở bản `debug` thì gần như chắc chắn không.
>
> Hệ quả rất thật: `sum_tail(&one_million_items, 0)` sẽ **tràn ngăn xếp và sập chương trình**. Đừng bao giờ viết đệ quy có độ sâu tỉ lệ với kích thước dữ liệu người dùng đưa vào.

**Ba lối đi đúng trong Rust:**

| Cách | Khi nào dùng | Ví dụ |
|---|---|---|
| **Iterator** *(ưu tiên số 1)* | Hầu hết mọi trường hợp | `list.iter().sum()` — vừa an toàn vừa nhanh nhất |
| **Vòng lặp với biến tích lũy** | Khi logic quá phức tạp cho iterator | `let mut acc = 0; for x in list { acc += x; }` |
| **`loop` + `ControlFlow`** | Máy trạng thái, thuật toán lặp | Biến đệ quy đuôi thành vòng lặp bằng tay |

Ví dụ chuyển đệ quy đuôi thành vòng lặp — chính là việc mà trình biên dịch Haskell làm giúp bạn:

```rust
fn sum_loop(list: &[i64]) -> i64 {
    let mut remaining = list;
    let mut acc = 0;
    while let [first, rest @ ..] = remaining {   // "lời gọi đệ quy" trở thành phép gán
        acc += first;
        remaining = rest;
    }
    acc
}
```

Đệ quy vẫn hoàn toàn phù hợp trong Rust khi độ sâu **có giới hạn tự nhiên** — ví dụ duyệt cây nhị phân cân bằng (độ sâu ~log N, một triệu nút chỉ sâu 20 tầng, xem Chương 29). Vấn đề chỉ nảy sinh khi độ sâu tỉ lệ **tuyến tính** với dữ liệu.

> **Ghi nhớ**: trong Rust, `fold` chính là "đệ quy đuôi đã được viết sẵn thành vòng lặp cho bạn". Mỗi khi định viết đệ quy tích lũy, hãy tự hỏi trước: *"cái này có phải là một `fold` không?"*

### 7. Phần thưởng của tính thuần túy: song song hóa bằng `rayon`

Ở Chương 13 chúng ta đã hứa rằng hàm thuần túy giúp xử lý đa luồng an toàn mà không cần khóa. Đây là lúc trả lời hứa đó:

```toml
# Cargo.toml
[dependencies]
rayon = "1"
```

```rust
use rayon::prelude::*;

// Tuần tự — chạy trên 1 nhân CPU
let total: u64 = data.iter().map(|x| heavy_compute(x)).sum();

// Song song — chạy trên TOÀN BỘ nhân CPU. Khác biệt: iter -> par_iter
let total: u64 = data.par_iter().map(|x| heavy_compute(x)).sum();
```

**Đổi đúng một từ.** Và bạn được bảo đảm ba điều:
1. Kết quả giống hệt bản tuần tự — vì phép `sum` là một **vị nhóm kết hợp** (Chương 18), chia nhỏ rồi ghép lại không đổi kết quả.
2. Không có tranh chấp dữ liệu — vì closure trong `map` là **hàm thuần túy**, trình biên dịch kiểm tra điều này qua trait `Send`/`Sync` và **từ chối biên dịch** nếu bạn cố sửa trạng thái dùng chung.
3. Không cần một dòng `Mutex` nào.

> Nếu closure của bạn *không* thuần túy (ví dụ ghi vào một biến `mut` bên ngoài), `rayon` sẽ không cho biên dịch. Tính thuần túy ở đây không phải lời khuyên đạo đức — nó là **điều kiện kỹ thuật bắt buộc**, và trình biên dịch là người kiểm tra.

---

## Mã nguồn minh họa mở rộng (Extended Runnable Blueprint)

Chương trình dưới đây phân tích **nhật ký bán hàng thô** — dữ liệu bẩn, có dòng hỏng, đúng như đời thực — và dùng lần lượt toàn bộ bộ công cụ ở trên.

```rust
// Tệp: src/main.rs
// Bộ công cụ Iterator đầy đủ: từ filter_map tới FromIterator

// Cố ý viết `fold(0, |a, b| a + b)` thay vì `sum()` để so sánh fold/rfold/reduce,
// nên tắt lint gợi ý rút gọn fold của clippy cho cả tệp.
#![allow(clippy::unnecessary_fold)]

use std::collections::{HashMap, HashSet};

// ============================================================================
// PHẦN 1: TỰ CÀI ĐẶT MỘT ITERATOR
// ============================================================================

/// Bộ đếm ngược: minh họa việc chỉ cần cài `next()` là có ngay hàng chục
/// phương thức miễn phí (map, filter, take, sum...).
pub struct Countdown {
    current: u32,
}

impl Countdown {
    pub fn new(start: u32) -> Self {
        Countdown { current: start }
    }
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.current == 0 {
            None
        } else {
            self.current -= 1;
            Some(self.current + 1)
        }
    }
}

// ============================================================================
// PHẦN 2: TỰ CÀI ĐẶT IntoIterator CHO KIỂU CỦA MÌNH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Cart {
    items: Vec<String>,
}

impl Cart {
    pub fn new(items: Vec<String>) -> Self {
        Cart { items }
    }
}

/// Nhờ trait này, `for x in cart` chạy được — đúng như với Vec.
impl IntoIterator for Cart {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}

/// Và nhờ trait này, `for x in &cart` cũng chạy được (chỉ mượn đọc).
impl<'a> IntoIterator for &'a Cart {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

/// Và nhờ FromIterator, `collect()` gom thẳng được vào Cart.
impl FromIterator<String> for Cart {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        Cart {
            items: iter.into_iter().collect(),
        }
    }
}

// ============================================================================
// PHẦN 3: MIỀN DỮ LIỆU — NHẬT KÝ BÁN HÀNG THÔ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Trade {
    pub id: String,
    pub region: String,
    pub amount: u64,
}

/// Phân tích một dòng thô "MÃ|KHU_VỰC|SỐ_TIỀN". Trả None nếu dòng hỏng.
pub fn parse_trade(line: &str) -> Option<Trade> {
    let parts: Vec<&str> = line.split('|').map(|s| s.trim()).collect();
    if parts.len() != 3 {
        return None;
    }
    let amount = parts[2].parse::<u64>().ok()?;
    if parts[0].is_empty() || parts[1].is_empty() {
        return None;
    }
    Some(Trade {
        id: parts[0].to_string(),
        region: parts[1].to_string(),
        amount,
    })
}

fn raw_data() -> Vec<&'static str> {
    vec![
        "GD-001 | Hà Nội       | 1250000",
        "GD-002 | TP.HCM       | 890000",
        "dòng hỏng không có dấu gạch",
        "GD-003 | Đà Nẵng      | 450000",
        "GD-004 | Hà Nội       | không phải số",
        "GD-005 | TP.HCM       | 2100000",
        "GD-006 | Hà Nội       | 320000",
        "       |              | 999",
        "GD-007 | Cần Thơ      | 780000",
    ]
}

fn main() {
    println!("============================================================");
    println!("        BỘ CÔNG CỤ ITERATOR ĐẦY ĐỦ CỦA RUST                ");
    println!("============================================================");

    let raw = raw_data();
    println!("\nDữ liệu thô: {} dòng (có cả dòng hỏng)", raw.len());

    // ------------------------------------------------------------------
    // 1. filter_map — LỌC VÀ BIẾN ĐỔI CÙNG LÚC
    // ------------------------------------------------------------------
    let trades: Vec<Trade> = raw.iter().filter_map(|d| parse_trade(d)).collect();
    println!(
        "\n1. filter_map: {} dòng hợp lệ / {} dòng thô",
        trades.len(),
        raw.len()
    );
    for g in trades.iter().take(3) {
        println!("   {:?}", g);
    }
    println!("   (đã dùng luôn `take(3)` để chỉ in 3 dòng đầu)");

    // ------------------------------------------------------------------
    // 2. any / all / find / position — ĐỀU NGẮN MẠCH
    // ------------------------------------------------------------------
    println!("\n2. any / all / find / position (đều dừng sớm)");
    println!(
        "   Có giao dịch nào > 2 triệu?     : {}",
        trades.iter().any(|g| g.amount > 2_000_000)
    );
    println!(
        "   Mọi giao dịch đều > 100 nghìn?  : {}",
        trades.iter().all(|g| g.amount > 100_000)
    );
    println!(
        "   Giao dịch đầu ở Đà Nẵng         : {:?}",
        trades.iter().find(|g| g.region == "Đà Nẵng").map(|g| &g.id)
    );
    println!(
        "   Vị trí giao dịch đầu ở TP.HCM   : {:?}",
        trades.iter().position(|g| g.region == "TP.HCM")
    );

    // ------------------------------------------------------------------
    // 3. min_by_key / max_by_key
    // ------------------------------------------------------------------
    println!("\n3. min_by_key / max_by_key");
    println!(
        "   Giao dịch nhỏ nhất: {:?}",
        trades
            .iter()
            .min_by_key(|g| g.amount)
            .map(|g| (&g.id, g.amount))
    );
    println!(
        "   Giao dịch lớn nhất: {:?}",
        trades
            .iter()
            .max_by_key(|g| g.amount)
            .map(|g| (&g.id, g.amount))
    );

    // ------------------------------------------------------------------
    // 4. partition — CHIA ĐÔI TRONG MỘT LƯỢT
    // ------------------------------------------------------------------
    let (large, small): (Vec<&Trade>, Vec<&Trade>) =
        trades.iter().partition(|g| g.amount >= 800_000);
    println!(
        "\n4. partition: {} đơn lớn (>=800k), {} đơn nhỏ",
        large.len(),
        small.len()
    );

    // ------------------------------------------------------------------
    // 5. fold / reduce / try_fold — BA KIỂU GỘP
    // ------------------------------------------------------------------
    println!("\n5. fold vs reduce vs try_fold");
    let sum_fold: u64 = trades.iter().map(|g| g.amount).fold(0, |a, b| a + b);
    let sum_reduce: Option<u64> = trades.iter().map(|g| g.amount).reduce(|a, b| a + b);
    println!("   fold  (có giá trị khởi tạo)  : {}", sum_fold);
    println!("   reduce(không có, trả Option) : {:?}", sum_reduce);

    let empty: Vec<u64> = Vec::new();
    println!(
        "   Trên danh sách RỖNG -> fold: {}, reduce: {:?}",
        empty.iter().fold(0u64, |a, b| a + b),
        empty.iter().copied().reduce(|a: u64, b: u64| a + b)
    );

    // try_fold: gộp CÓ THỂ THẤT BẠI, dừng ngay ở lỗi đầu tiên
    let safe: Option<u64> = trades.iter().try_fold(0u64, |a, g| a.checked_add(g.amount));
    println!("   try_fold (chống tràn số)     : {:?}", safe);
    let overflowed: Option<u64> = [u64::MAX, 1]
        .iter()
        .try_fold(0u64, |a, b| a.checked_add(*b));
    println!(
        "   try_fold khi tràn số         : {:?} (dừng ngay, không panic)",
        overflowed
    );

    // ------------------------------------------------------------------
    // 6. scan — GIỐNG fold NHƯNG NHẢ RA TỪNG BƯỚC TRUNG GIAN
    // ------------------------------------------------------------------
    let cumulative: Vec<u64> = trades
        .iter()
        .scan(0u64, |acc, g| {
            *acc += g.amount;
            Some(*acc)
        })
        .collect();
    println!("\n6. scan (tổng lũy kế từng bước): {:?}", cumulative);

    // ------------------------------------------------------------------
    // 7. take_while / skip_while — DỪNG SỚM, KHÁC HẲN filter
    // ------------------------------------------------------------------
    println!("\n7. take_while vs filter");
    let numbers = [1, 3, 5, 4, 7, 9];
    let tw: Vec<i32> = numbers.iter().copied().take_while(|x| x % 2 == 1).collect();
    let ft: Vec<i32> = numbers.iter().copied().filter(|x| x % 2 == 1).collect();
    println!("   dãy gốc              : {:?}", numbers);
    println!(
        "   take_while(lẻ)       : {:?}  ← DỪNG ngay khi gặp số chẵn đầu tiên",
        tw
    );
    println!(
        "   filter(lẻ)           : {:?}  ← duyệt HẾT, giữ mọi số lẻ",
        ft
    );
    let sw: Vec<i32> = numbers.iter().copied().skip_while(|x| x % 2 == 1).collect();
    println!("   skip_while(lẻ)       : {:?}", sw);

    // ------------------------------------------------------------------
    // 8. zip / unzip / chain / rev / step_by
    // ------------------------------------------------------------------
    println!("\n8. zip / unzip / chain / rev / step_by");
    let ids: Vec<&str> = trades.iter().map(|g| g.id.as_str()).collect();
    let amounts: Vec<u64> = trades.iter().map(|g| g.amount).collect();
    let zipped: Vec<(&&str, &u64)> = ids.iter().zip(amounts.iter()).take(3).collect();
    println!("   zip 3 cặp đầu : {:?}", zipped);

    let (ids_back, amounts_back): (Vec<&str>, Vec<u64>) =
        ids.iter().copied().zip(amounts.iter().copied()).unzip();
    println!(
        "   unzip tách lại: {} mã, {} số tiền",
        ids_back.len(),
        amounts_back.len()
    );

    let concat: Vec<i32> = (1..3).chain(10..12).collect();
    println!("   chain         : {:?}", concat);
    // CHÚ Ý: `rev()` đòi hỏi trait `DoubleEndedIterator` — iterator phải biết đi
    // từ CẢ HAI đầu. `Countdown` tự viết chỉ cài `Iterator` (một chiều), nên
    // `Countdown::new(5).rev()` KHÔNG biên dịch được:
    //     error[E0277]: the trait bound `Countdown: DoubleEndedIterator` is not satisfied
    // `Vec` thì có, nên ta gom lại trước rồi mới đảo:
    let inverse: Vec<u32> = Countdown::new(5)
        .collect::<Vec<u32>>()
        .into_iter()
        .rev()
        .collect();
    println!("   rev (cần DoubleEndedIterator): {:?}", inverse);
    let stepped: Vec<i32> = (0..10).step_by(3).collect();
    println!("   step_by(3)    : {:?}", stepped);

    // ------------------------------------------------------------------
    // 9. flat_map / flatten
    // ------------------------------------------------------------------
    println!("\n9. flat_map / flatten");
    let sentences = ["Rust rất nhanh", "và an toàn"];
    let words: Vec<&str> = sentences
        .iter()
        .flat_map(|c| c.split_whitespace())
        .collect();
    println!("   flat_map tách từ: {:?}", words);

    let nested: Vec<Vec<i32>> = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let flat: Vec<i32> = nested.into_iter().flatten().collect();
    println!("   flatten làm phẳng: {:?}", flat);

    let with_none: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
    let without_none: Vec<i32> = with_none.into_iter().flatten().collect();
    println!("   flatten bỏ None  : {:?}", without_none);

    // ------------------------------------------------------------------
    // 10. collect VÀO NHIỀU KIỂU KHÁC NHAU
    // ------------------------------------------------------------------
    println!("\n10. collect() gom vào nhiều kiểu đích");
    let text: String = ids.join(", ");
    println!("   -> String     : {}", text);

    let regions: HashSet<&str> = trades.iter().map(|g| g.region.as_str()).collect();
    let mut sorted_regions: Vec<&&str> = regions.iter().collect();
    sorted_regions.sort();
    println!(
        "   -> HashSet    : {:?} ({} khu vực)",
        sorted_regions,
        regions.len()
    );

    let table: HashMap<&str, u64> = trades.iter().map(|g| (g.id.as_str(), g.amount)).collect();
    println!(
        "   -> HashMap    : tra cứu GD-003 = {:?}",
        table.get("GD-003")
    );

    let all_ok: Result<Vec<i32>, _> = ["1", "2", "3"].iter().map(|s| s.parse::<i32>()).collect();
    let has_bad: Result<Vec<i32>, _> = ["1", "x", "3"].iter().map(|s| s.parse::<i32>()).collect();
    println!("   -> Result (ổn) : {:?}", all_ok);
    println!("   -> Result (hỏng): có lỗi = {}", has_bad.is_err());

    // ------------------------------------------------------------------
    // 11. TỔNG HỢP THEO NHÓM — MẪU DÙNG HẰNG NGÀY
    // ------------------------------------------------------------------
    println!("\n11. Tổng doanh thu theo khu vực (fold + entry API)");
    let by_region: HashMap<&str, u64> = trades.iter().fold(HashMap::new(), |mut table, g| {
        *table.entry(g.region.as_str()).or_insert(0) += g.amount;
        table
    });
    let mut pairs: Vec<(&&str, &u64)> = by_region.iter().collect();
    pairs.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (k, v) in pairs {
        println!("   {:<10} {:>10} đ", k, v);
    }

    // ------------------------------------------------------------------
    // 12. fold vs rfold — KHI THỨ TỰ CÓ Ý NGHĨA
    // ------------------------------------------------------------------
    println!("\n12. fold vs rfold");
    let m = [10i32, 3, 2];
    println!(
        "   Phép CỘNG (giao hoán)      : fold={}, rfold={}  -> GIỐNG nhau",
        m.iter().fold(0, |a, b| a + b),
        m.iter().rfold(0, |a, b| a + b)
    );
    let folded_left: String = m.iter().fold(String::new(), |a, b| a + &b.to_string());
    let folded_right: String = m.iter().rfold(String::new(), |a, b| a + &b.to_string());
    println!(
        "   NỐI CHUỖI (không giao hoán): fold={:?}, rfold={:?}  -> KHÁC nhau",
        folded_left, folded_right
    );
    println!("   → Trước khi song song hóa, phải biết phép gộp của mình có tính gì!");

    // ------------------------------------------------------------------
    // 13. ITERATOR TỰ VIẾT VÀ IntoIterator TỰ VIẾT
    // ------------------------------------------------------------------
    println!("\n13. Iterator và IntoIterator tự cài đặt");
    let count: Vec<u32> = Countdown::new(5).collect();
    println!("   Countdown::new(5)           : {:?}", count);
    println!(
        "   Miễn phí luôn map/filter/sum: {}",
        Countdown::new(100).filter(|x| x % 7 == 0).sum::<u32>()
    );

    let cart = Cart::new(vec!["Bàn phím".into(), "Chuột".into(), "Màn hình".into()]);
    print!("   for x in &cart -> ");
    for m in &cart {
        print!("[{}] ", m);
    }
    println!();

    let long_names: Cart = cart.into_iter().filter(|m| m.chars().count() > 5).collect(); // ← nhờ FromIterator tự cài
    println!("   collect() thẳng vào Cart    : {:?}", long_names);

    // ------------------------------------------------------------------
    // 14. Extend — NỐI THÊM VÀO TẬP HỢP ĐÃ CÓ
    // ------------------------------------------------------------------
    let mut extended: Vec<i32> = vec![1, 2];
    extended.extend(3..6);
    println!("\n14. Extend: {:?}", extended);

    println!("\n============================================================");
    println!("   MỘT `next()` — HÀNG CHỤC CÔNG CỤ MIỄN PHÍ ĐI KÈM         ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_map_skips_bad_lines() {
        let trades: Vec<Trade> = raw_data().iter().filter_map(|d| parse_trade(d)).collect();
        assert_eq!(trades.len(), 6, "9 dòng thô, 3 dòng hỏng -> còn 6");
    }

    #[test]
    fn take_while_differs_from_filter() {
        let numbers = [1, 3, 5, 4, 7, 9];
        let tw: Vec<i32> = numbers.iter().copied().take_while(|x| x % 2 == 1).collect();
        let ft: Vec<i32> = numbers.iter().copied().filter(|x| x % 2 == 1).collect();
        assert_eq!(tw, vec![1, 3, 5]); // dừng ở số 4
        assert_eq!(ft, vec![1, 3, 5, 7, 9]); // duyệt hết
    }

    #[test]
    fn reduce_returns_none_when_empty() {
        let empty: Vec<u64> = Vec::new();
        assert_eq!(empty.iter().copied().reduce(|a, b| a + b), None);
        assert_eq!(empty.iter().fold(0u64, |a, b| a + b), 0); // fold vẫn có câu trả lời
    }

    #[test]
    fn try_fold_stops_on_overflow() {
        let result: Option<u64> = [u64::MAX, 1, 2]
            .iter()
            .try_fold(0u64, |a, b| a.checked_add(*b));
        assert_eq!(result, None);
    }

    #[test]
    fn scan_emits_intermediate_steps() {
        let cumulative: Vec<i32> = [1, 2, 3, 4]
            .iter()
            .scan(0, |t, x| {
                *t += x;
                Some(*t)
            })
            .collect();
        assert_eq!(cumulative, vec![1, 3, 6, 10]);
    }

    #[test]
    fn fold_and_rfold_differ_only_when_non_commutative() {
        let m = [10i32, 3, 2];
        // Phép cộng GIAO HOÁN -> duyệt hai chiều cho cùng kết quả
        assert_eq!(
            m.iter().fold(0, |a, b| a + b),
            m.iter().rfold(0, |a, b| a + b)
        );
        // Nối chuỗi KHÔNG giao hoán -> duyệt hai chiều cho kết quả khác nhau
        let left: String = m.iter().fold(String::new(), |a, b| a + &b.to_string());
        let right: String = m.iter().rfold(String::new(), |a, b| a + &b.to_string());
        assert_eq!(left, "1032");
        assert_eq!(right, "2310");
        assert_ne!(left, right);
    }

    #[test]
    fn collect_targets_many_types() {
        let v: Vec<i32> = (1..4).collect();
        assert_eq!(v, vec![1, 2, 3]);
        let s: String = ['R', 'u', 's', 't'].into_iter().collect();
        assert_eq!(s, "Rust");
        let t: HashSet<i32> = [1, 2, 2, 3].into_iter().collect();
        assert_eq!(t.len(), 3);
        let b: HashMap<&str, i32> = [("a", 1), ("b", 2)].into_iter().collect();
        assert_eq!(b.get("b"), Some(&2));
        let r: Result<Vec<i32>, _> = ["1", "2"].iter().map(|s| s.parse::<i32>()).collect();
        assert_eq!(r, Ok(vec![1, 2]));
    }

    #[test]
    fn partition_splits_into_two_groups() {
        let (evens, odds): (Vec<i32>, Vec<i32>) = (1..8).partition(|x| x % 2 == 0);
        assert_eq!(evens, vec![2, 4, 6]);
        assert_eq!(odds, vec![1, 3, 5, 7]);
    }

    #[test]
    fn custom_iterator_works() {
        assert_eq!(Countdown::new(3).collect::<Vec<u32>>(), vec![3, 2, 1]);
        assert_eq!(Countdown::new(10).filter(|x| x % 3 == 0).sum::<u32>(), 18); // 9+6+3
    }

    #[test]
    fn custom_into_and_from_iterator() {
        let cart = Cart::new(vec!["Bàn phím".into(), "Chuột".into()]);
        let names: Vec<&String> = (&cart).into_iter().collect();
        assert_eq!(names.len(), 2);
        let filtered: Cart = cart.into_iter().filter(|m| m.chars().count() > 5).collect();
        assert_eq!(filtered, Cart::new(vec!["Bàn phím".into()]));
    }

    #[test]
    fn totals_by_region_are_correct() {
        let trades: Vec<Trade> = raw_data().iter().filter_map(|d| parse_trade(d)).collect();
        let by_region: HashMap<&str, u64> = trades.iter().fold(HashMap::new(), |mut b, g| {
            *b.entry(g.region.as_str()).or_insert(0) += g.amount;
            b
        });
        assert_eq!(by_region.get("Hà Nội"), Some(&1_570_000)); // 1250000 + 320000
        assert_eq!(by_region.get("Cần Thơ"), Some(&780_000));
    }
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Các lỗi biên dịch phổ biến nhất khi làm việc với Iterator trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0283** | `type annotations needed` | Bạn gọi `.collect()` nhưng không ghi rõ kiểu dữ liệu mong muốn nhận về. Trình biên dịch không biết bạn muốn gom dữ liệu thành `Vec`, `HashSet` hay kiểu tập hợp nào. | Chú thích kiểu tường minh ở biến hứng: `let res: Vec<i32> = ...;` hoặc dùng cú pháp Turbofish: `.collect::<Vec<_>>()`. |
| **E0507** | `cannot move out of '*s' which is behind a shared reference` | Bạn đang dùng `.iter()` (chỉ mượn tham chiếu `&T`) nhưng trong closure của `.map()` bạn lại cố lấy quyền sở hữu của phần tử không có thuộc tính `Copy` (như `String`). | Đổi sang `.into_iter()` nếu muốn lấy quyền sở hữu, hoặc gọi `.clone()`, hoặc chỉ thao tác trên tham chiếu `&`. |
| **E0599** | `the method 'map' exists for struct 'Vec<{integer}>', but its trait bounds were not satisfied` | Bạn cố gọi một phương thức iterator (như `.map()`) trực tiếp trên một tập hợp (`Vec` không phải là `Iterator`) mà quên chưa biến nó thành bộ lặp qua `.iter()`. | Gọi phương thức `.iter()`, `.iter_mut()`, hoặc `.into_iter()` trước khi gọi các adapter. |
| **E0308** | `mismatched types` | Trong hàm `.fold(init, \|acc, item\| ...)`, giá trị trả về của closure không khớp với kiểu của giá trị khởi tạo `init` (vd `init` là `0` kiểu số nguyên nhưng closure trả `f64`). | Kiểm tra lại kiểu của biểu thức cuối cùng trong thân closure của `.fold()`, đảm bảo nó khớp chính xác với kiểu khởi tạo. |

### Phân tích lỗi thực tế `E0283` (Thiếu chú thích kiểu khi gọi `collect`):

```rust
// Đoạn mã lỗi minh họa:
fn broken_collect() {
    let arr = vec![1, 2, 3];
    // LỖI E0283: rustc không biết gom thành kiểu gì
    // let result = arr.iter().map(|x| x * 2).collect();
}

// Cách sửa chữa chuẩn mực:
fn correct_example() {
    let arr = vec![1, 2, 3];
    // Cách A: Chú thích kiểu ở phía biến
    let result_a: Vec<i32> = arr.iter().map(|x| x * 2).collect();

    // Cách B: Sử dụng cú pháp "cá" Turbofish ::<Vec<_>>()
    let result_b = arr.iter().map(|x| x * 2).collect::<Vec<_>>();
    println!("{:?} - {:?}", result_a, result_b);
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 7 Điểm cốt lõi cần ghi nhớ:
1. **Bản chất Trait `Iterator`**: Chỉ cần cài đặt duy nhất một phương thức `fn next(&mut self) -> Option<Self::Item>`, bạn lập tức sở hữu miễn phí hàng chục phương thức biến đổi dữ liệu cao cấp.
2. **Tính lười biếng (Lazy Evaluation)**: Chuỗi adapter không thực thi bất kỳ phép tính nào cho đến khi hàm tiêu thụ (consumer) như `collect` hay `fold` yêu cầu kết quả.
3. **Ba chế độ duyệt**:
   - `.iter()`: Mượn đọc (`&T`).
   - `.iter_mut()`: Mượn sửa trực tiếp (`&mut T`).
   - `.into_iter()`: Đoạt quyền sở hữu (`T`) và tiêu thụ tập hợp gốc.
4. **Hiệu năng Zero-Cost**: Không tốn chi phí gọi hàm trung gian nhờ cơ chế Monomorphization và tối ưu hóa loại bỏ kiểm tra biên giới hạn của LLVM.
5. **Bộ công cụ đầy đủ**: ngoài `map`/`filter`, hãy nhớ `filter_map` (lọc + biến đổi), `flat_map` (nở ra nhiều phần tử), `take_while` (dừng sớm, khác `filter`), `partition` (chia đôi một lượt), `reduce` (gộp không cần khởi tạo), `try_fold` (gộp có thể thất bại) và `scan` (nhả từng bước trung gian).
6. **`collect()` không chỉ tạo `Vec`**: nó gom được vào `String`, `HashMap`, `HashSet`, `Result`, `Option` — hay bất kỳ kiểu nào cài `FromIterator`, kể cả kiểu của chính bạn.
7. **Song song hóa gần như miễn phí**: với `rayon`, đổi `.iter()` thành `.par_iter()` là chạy trên toàn bộ nhân CPU. Điều kiện duy nhất: closure phải thuần túy — và trình biên dịch kiểm tra giúp bạn.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phân tách Chẵn - Lẻ qua Iterator)**:  
   Cho một danh sách số nguyên: `let numbers = vec![12, 7, 19, 24, 30, 5, 8];`.  
   Hãy dùng đường ống Iterator để:
   - Lọc ra các số chẵn.
   - Bình phương từng số chẵn đó.
   - Thu gom vào một `Vec<i32>` mới bằng `.collect()`.

2. **Bài tập 2 (Xây dựng Bộ tính toán với `.fold()`)**:  
   Dùng phương thức `.fold()` để tìm giá trị lớn nhất trong một lát cắt số nguyên `&[i32]` mà không sử dụng phương thức `.max()` có sẵn của Rust. Khởi tạo giá trị ban đầu một cách khéo léo để chương trình hoạt động chính xác.

3. **Bài tập 3 (Tự tạo Trait Iterator đơn giản)**:  
   Tạo một struct mang tên `CountDown { current: u32 }`. Triển khai Trait `Iterator` cho struct này sao cho mỗi lần gọi `.next()`, nó đếm lùi từ một con số cho trước về `1`, và trả về `None` khi số hiện tại chạm mốc `0`. Kiểm tra hoạt động của nó với vòng lặp `for`.

4. **Bài tập 4 (Làm sạch dữ liệu bẩn bằng `filter_map`)**:  
   Cho `let raw = ["12", "abc", "7", "", "30", "-5"];`. Hãy dùng **một** đường ống duy nhất để: bỏ qua mọi dòng không phân tích được thành `u32`, rồi tính tổng các số hợp lệ. Không dùng vòng lặp `for`, không dùng `unwrap()`.

5. **Bài tập 5 (Traversable — "được ăn cả, ngã về không")**:  
   Vẫn dữ liệu trên, nhưng lần này yêu cầu ngược lại: nếu **mọi** dòng đều hợp lệ thì trả về `Ok(Vec<u32>)`; chỉ cần **một** dòng hỏng là trả về `Err`. Viết bằng đúng một lời gọi `.collect()`.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Ba yêu cầu ứng đúng ba mắt xích `.filter()` → `.map()` → `.collect()`. Nhớ rằng `.iter()` cho ra `&i32`, nên hãy dùng mẫu `|&x|` trong closure để bóc tham chiếu, và chú thích kiểu ở biến hứng để `collect()` biết phải gom vào đâu.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
fn main() {
    let numbers = vec![12, 7, 19, 24, 30, 5, 8];

    let even_squares: Vec<i32> = numbers
        .iter()
        .filter(|&&x| x % 2 == 0)   // 12, 24, 30, 8
        .map(|&x| x * x)            // 144, 576, 900, 64
        .collect();

    assert_eq!(even_squares, vec![144, 576, 900, 64]);
    println!("{:?}", even_squares);
}
```
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Câu hỏi then chốt: **giá trị khởi tạo phải là gì?** Nếu khởi tạo bằng `0` thì một mảng toàn số âm sẽ cho kết quả sai. Hãy nghĩ tới "âm vô cực" — trong Rust nó có tên là `i32::MIN`. (Nếu bạn đã đọc Chương 18: đây chính là *phần tử đơn vị* của vị nhóm `max`.)
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
pub fn max(list: &[i32]) -> Option<i32> {
    if list.is_empty() {
        return None;   // câu trả lời trung thực cho danh sách rỗng
    }
    // i32::MIN là "âm vô cực": gộp với bất cứ số nào cũng thua.
    Some(list.iter().fold(i32::MIN, |max, &x| if x > max { x } else { max }))
}

fn main() {
    assert_eq!(max(&[3, 9, 2, 7]), Some(9));
    assert_eq!(max(&[-30, -9, -100]), Some(-9)); // khởi tạo bằng 0 sẽ SAI ở đây!
    assert_eq!(max(&[]), None);
    println!("{:?}", max(&[-30, -9, -100]));
}
```

Hai bài học: (1) chọn sai phần tử khởi tạo là một lỗi im lặng, chỉ lộ ra với dữ liệu âm; (2) trả `Option` thay vì một con số bịa ra chính là biến hàm bộ phận thành **hàm toàn phần** (Chương 13).
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Bạn chỉ phải viết đúng **một** phương thức: `fn next(&mut self) -> Option<Self::Item>`. Hãy cẩn thận thứ tự: giảm `current` trước rồi trả về, hay trả về trước rồi giảm? Hãy tự kiểm bằng cách viết ra kỳ vọng: `CountDown { current: 3 }` phải cho ra `3, 2, 1`.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
pub struct CountDown {
    pub current: u32,
}

impl Iterator for CountDown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.current == 0 {
            None
        } else {
            let value = self.current;
            self.current -= 1;
            Some(value)
        }
    }
}

fn main() {
    // Dùng với vòng lặp for (nhờ IntoIterator có sẵn cho mọi Iterator)
    print!("Đếm ngược: ");
    for n in (CountDown { current: 5 }) {
        print!("{} ", n);
    }
    println!("Phóng!");

    // PHẦN THƯỞNG: chỉ cài `next()` mà được dùng ngay hàng chục phương thức khác
    let list: Vec<u32> = CountDown { current: 5 }.collect();
    assert_eq!(list, vec![5, 4, 3, 2, 1]);

    let even_sum: u32 = CountDown { current: 10 }.filter(|n| n % 2 == 0).sum();
    assert_eq!(even_sum, 30); // 10+8+6+4+2

    let first_three: Vec<u32> = CountDown { current: 100 }.take(3).collect();
    assert_eq!(first_three, vec![100, 99, 98]);
}
```

Đây là minh chứng rõ nhất cho sức mạnh của trait `Iterator`: **một** phương thức bắt buộc, **hàng chục** phương thức miễn phí đi kèm.
</details>

<details>
<summary><b>Bài tập 4 — Gợi ý</b></summary>

`"abc".parse::<u32>()` trả về `Result`. Đổi nó thành `Option` bằng `.ok()`, rồi để `filter_map` tự vứt bỏ những `None`.
</details>

<details>
<summary><b>Bài tập 4 — Lời giải</b></summary>

```rust
fn main() {
    let raw = ["12", "abc", "7", "", "30", "-5"];

    let total: u32 = raw.iter().filter_map(|s| s.parse::<u32>().ok()).sum();

    // "abc", "" và "-5" đều bị bỏ qua ("-5" không phải u32 hợp lệ)
    assert_eq!(total, 49); // 12 + 7 + 30
    println!("Tổng các số hợp lệ: {}", total);
}
```

Một dòng duy nhất, không `unwrap()`, không vòng lặp, không biến `mut`. Đây là mẫu bạn sẽ dùng gần như mỗi khi đọc dữ liệu từ tệp hay mạng.
</details>

<details>
<summary><b>Bài tập 5 — Gợi ý</b></summary>

Điểm mấu chốt nằm ở **kiểu của biến hứng**, không phải ở đường ống. Hãy thử `let res: Result<Vec<u32>, _> = ...` và bỏ `.ok()` đi — `collect()` sẽ tự hiểu bạn muốn gì.
</details>

<details>
<summary><b>Bài tập 5 — Lời giải</b></summary>

```rust
fn main() {
    let bad = ["12", "abc", "7"];
    let good = ["12", "7", "30"];

    // KHÁC BIỆT DUY NHẤT so với bài 4: kiểu của biến hứng, và không có `.ok()`
    let result_bad: Result<Vec<u32>, _> = bad.iter().map(|s| s.parse::<u32>()).collect();
    let result_good: Result<Vec<u32>, _> = good.iter().map(|s| s.parse::<u32>()).collect();

    assert!(result_bad.is_err());                  // MỘT dòng hỏng -> TOÀN BỘ hỏng
    assert_eq!(result_good, Ok(vec![12, 7, 30]));   // mọi dòng tốt  -> được cả danh sách
    println!("{:?}\n{:?}", result_bad, result_good);
}
```

Hãy so sánh bài 4 và bài 5 — cùng dữ liệu, cùng đường ống, chỉ khác kiểu đích:
- **`filter_map` + `.ok()`** = "bỏ qua dòng hỏng, cứu được bao nhiêu hay bấy nhiêu" (dùng cho nhật ký, dữ liệu thống kê).
- **`collect::<Result<Vec<_>, _>>()`** = "được ăn cả, ngã về không" (dùng cho tệp cấu hình, giao dịch tài chính).

Chọn đúng một trong hai là một **quyết định thiết kế**, không phải chuyện phong cách. Kỹ thuật thứ hai có tên chính thức là *Traversable* — Chương 19 sẽ nói kỹ.
</details>
