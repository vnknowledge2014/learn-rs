# Chương 16: Bộ lặp lười biếng & Toàn bộ đường ống dữ liệu: map, filter_map, fold, collect (Iterators & the Complete Data Pipeline Toolkit)

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
| `zip(khac)` | → `(A, B)` | Ghép hai dòng dữ liệu song song; dừng ở dòng ngắn hơn |
| `chain(khac)` | | Nối hai iterator thành một |
| `rev()` | | Duyệt ngược (cần `DoubleEndedIterator`) |
| `step_by(n)` | | Lấy cách quãng: phần tử 0, n, 2n… |
| **`scan(kt, f)`** | | Như `fold` nhưng **nhả ra giá trị trung gian ở mỗi bước** (tính tổng lũy kế) |
| `peekable()` | | Cho phép "nhìn trộm" phần tử kế tiếp mà chưa tiêu thụ nó |
| `inspect(f)` | | Chèn `println!` để gỡ lỗi giữa đường ống mà không đổi dữ liệu |

> **`filter_map` là bộ điều hợp bị bỏ quên nhiều nhất.** Khi phân tích dữ liệu bẩn từ tệp hay mạng, `.filter_map(|s| s.parse::<i32>().ok())` vừa thử chuyển đổi vừa bỏ qua dòng hỏng, chỉ trong một bước.

### 2. Bảng tra cứu hàm tiêu thụ (Consumers — chạy thật, trả về giá trị)

| Hàm tiêu thụ | Trả về | Dùng khi nào |
|---|---|---|
| `collect()` | `Vec`, `String`, `HashMap`, `HashSet`, `Result`, `Option`… | Gom kết quả (xem mục 4) |
| `fold(kt, f)` | một giá trị | Gộp có giá trị khởi tạo — luôn dùng được, kể cả danh sách rỗng |
| **`reduce(f)`** | `Option<A>` | Gộp **không** cần giá trị khởi tạo; trả `None` nếu rỗng |
| **`try_fold(kt, f)`** | `Result` / `Option` | Gộp **có thể thất bại**, dừng ngay ở lỗi đầu tiên |
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
let so = [10i32, 3, 2];

// Phép CỘNG: giao hoán + kết hợp -> hai chiều cho CÙNG kết quả
assert_eq!(so.iter().fold(0, |a, b| a + b), so.iter().rfold(0, |a, b| a + b)); // 15 == 15

// NỐI CHUỖI: kết hợp nhưng KHÔNG giao hoán -> hai chiều cho kết quả KHÁC NHAU
let trai: String = so.iter().fold(String::new(), |a, b| a + &b.to_string());   // "1032"
let phai: String = so.iter().rfold(String::new(), |a, b| a + &b.to_string());  // "2310"
assert_ne!(trai, phai);
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
let tap: HashSet<i32>        = [1, 2, 2, 3].into_iter().collect();
let bang: HashMap<&str, i32> = [("a", 1), ("b", 2)].into_iter().collect();
let kq: Result<Vec<i32>, _>  = ["1","2"].iter().map(|s| s.parse::<i32>()).collect();
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
fn tong(ds: &[i64]) -> i64 {
    match ds {
        [] => 0,
        [dau, con_lai @ ..] => dau + tong(con_lai),  // còn việc phải làm sau lời gọi
    }
}

// Đệ quy ĐUÔI (tail recursion): lời gọi đệ quy là việc CUỐI CÙNG,
// kết quả tích lũy được mang theo trong tham số `tich_luy`.
fn tong_duoi(ds: &[i64], tich_luy: i64) -> i64 {
    match ds {
        [] => tich_luy,
        [dau, con_lai @ ..] => tong_duoi(con_lai, tich_luy + dau),  // không còn việc gì sau đó
    }
}
```

Trong Haskell, PureScript hay Scheme, dạng thứ hai được trình biên dịch **biến thành vòng lặp** (tối ưu hóa lời gọi đuôi — *tail call optimization*), nên chạy với ngăn xếp cố định dù danh sách dài bao nhiêu.

> **⚠️ CẠM BẪY LỚN NHẤT KHI MANG THÓI QUEN FP SANG RUST:**
> **Rust KHÔNG bảo đảm tối ưu hóa lời gọi đuôi.** Trình tối ưu hóa LLVM *đôi khi* làm được ở bản `--release`, nhưng đây **không phải cam kết của ngôn ngữ**. Ở bản `debug` thì gần như chắc chắn không.
>
> Hệ quả rất thật: `tong_duoi(&mang_mot_trieu_phan_tu, 0)` sẽ **tràn ngăn xếp và sập chương trình**. Đừng bao giờ viết đệ quy có độ sâu tỉ lệ với kích thước dữ liệu người dùng đưa vào.

**Ba lối đi đúng trong Rust:**

| Cách | Khi nào dùng | Ví dụ |
|---|---|---|
| **Iterator** *(ưu tiên số 1)* | Hầu hết mọi trường hợp | `ds.iter().sum()` — vừa an toàn vừa nhanh nhất |
| **Vòng lặp với biến tích lũy** | Khi logic quá phức tạp cho iterator | `let mut acc = 0; for x in ds { acc += x; }` |
| **`loop` + `ControlFlow`** | Máy trạng thái, thuật toán lặp | Biến đệ quy đuôi thành vòng lặp bằng tay |

Ví dụ chuyển đệ quy đuôi thành vòng lặp — chính là việc mà trình biên dịch Haskell làm giúp bạn:

```rust
fn tong_lap(ds: &[i64]) -> i64 {
    let mut con_lai = ds;
    let mut tich_luy = 0;
    while let [dau, duoi @ ..] = con_lai {   // "lời gọi đệ quy" trở thành phép gán
        tich_luy += dau;
        con_lai = duoi;
    }
    tich_luy
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
let tong: u64 = du_lieu.iter().map(|x| tinh_toan_nang(x)).sum();

// Song song — chạy trên TOÀN BỘ nhân CPU. Khác biệt: iter -> par_iter
let tong: u64 = du_lieu.par_iter().map(|x| tinh_toan_nang(x)).sum();
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

use std::collections::{HashMap, HashSet};

// ============================================================================
// PHẦN 1: TỰ CÀI ĐẶT MỘT ITERATOR
// ============================================================================

/// Bộ đếm ngược: minh họa việc chỉ cần cài `next()` là có ngay hàng chục
/// phương thức miễn phí (map, filter, take, sum...).
pub struct DemNguoc {
    hien_tai: u32,
}

impl DemNguoc {
    pub fn moi(bat_dau: u32) -> Self {
        DemNguoc { hien_tai: bat_dau }
    }
}

impl Iterator for DemNguoc {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.hien_tai == 0 {
            None
        } else {
            self.hien_tai -= 1;
            Some(self.hien_tai + 1)
        }
    }
}

// ============================================================================
// PHẦN 2: TỰ CÀI ĐẶT IntoIterator CHO KIỂU CỦA MÌNH
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GioHang {
    mat_hang: Vec<String>,
}

impl GioHang {
    pub fn moi(mat_hang: Vec<String>) -> Self {
        GioHang { mat_hang }
    }
}

/// Nhờ trait này, `for x in gio_hang` chạy được — đúng như với Vec.
impl IntoIterator for GioHang {
    type Item = String;
    type IntoIter = std::vec::IntoIter<String>;
    fn into_iter(self) -> Self::IntoIter {
        self.mat_hang.into_iter()
    }
}

/// Và nhờ trait này, `for x in &gio_hang` cũng chạy được (chỉ mượn đọc).
impl<'a> IntoIterator for &'a GioHang {
    type Item = &'a String;
    type IntoIter = std::slice::Iter<'a, String>;
    fn into_iter(self) -> Self::IntoIter {
        self.mat_hang.iter()
    }
}

/// Và nhờ FromIterator, `collect()` gom thẳng được vào GioHang.
impl FromIterator<String> for GioHang {
    fn from_iter<I: IntoIterator<Item = String>>(iter: I) -> Self {
        GioHang { mat_hang: iter.into_iter().collect() }
    }
}

// ============================================================================
// PHẦN 3: MIỀN DỮ LIỆU — NHẬT KÝ BÁN HÀNG THÔ
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct GiaoDich {
    pub ma: String,
    pub khu_vuc: String,
    pub so_tien: u64,
}

/// Phân tích một dòng thô "MA|KHU_VUC|SO_TIEN". Trả None nếu dòng hỏng.
pub fn phan_tich_dong(dong: &str) -> Option<GiaoDich> {
    let phan: Vec<&str> = dong.split('|').map(|s| s.trim()).collect();
    if phan.len() != 3 {
        return None;
    }
    let so_tien = phan[2].parse::<u64>().ok()?;
    if phan[0].is_empty() || phan[1].is_empty() {
        return None;
    }
    Some(GiaoDich {
        ma: phan[0].to_string(),
        khu_vuc: phan[1].to_string(),
        so_tien,
    })
}

fn du_lieu_tho() -> Vec<&'static str> {
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

    let tho = du_lieu_tho();
    println!("\nDữ liệu thô: {} dòng (có cả dòng hỏng)", tho.len());

    // ------------------------------------------------------------------
    // 1. filter_map — LỌC VÀ BIẾN ĐỔI CÙNG LÚC
    // ------------------------------------------------------------------
    let gd: Vec<GiaoDich> = tho.iter().filter_map(|d| phan_tich_dong(d)).collect();
    println!("\n1. filter_map: {} dòng hợp lệ / {} dòng thô", gd.len(), tho.len());
    for g in gd.iter().take(3) {
        println!("   {:?}", g);
    }
    println!("   (đã dùng luôn `take(3)` để chỉ in 3 dòng đầu)");

    // ------------------------------------------------------------------
    // 2. any / all / find / position — ĐỀU NGẮN MẠCH
    // ------------------------------------------------------------------
    println!("\n2. any / all / find / position (đều dừng sớm)");
    println!("   Có giao dịch nào > 2 triệu?     : {}", gd.iter().any(|g| g.so_tien > 2_000_000));
    println!("   Mọi giao dịch đều > 100 nghìn?  : {}", gd.iter().all(|g| g.so_tien > 100_000));
    println!("   Giao dịch đầu ở Đà Nẵng         : {:?}", gd.iter().find(|g| g.khu_vuc == "Đà Nẵng").map(|g| &g.ma));
    println!("   Vị trí giao dịch đầu ở TP.HCM   : {:?}", gd.iter().position(|g| g.khu_vuc == "TP.HCM"));

    // ------------------------------------------------------------------
    // 3. min_by_key / max_by_key
    // ------------------------------------------------------------------
    println!("\n3. min_by_key / max_by_key");
    println!("   Giao dịch nhỏ nhất: {:?}", gd.iter().min_by_key(|g| g.so_tien).map(|g| (&g.ma, g.so_tien)));
    println!("   Giao dịch lớn nhất: {:?}", gd.iter().max_by_key(|g| g.so_tien).map(|g| (&g.ma, g.so_tien)));

    // ------------------------------------------------------------------
    // 4. partition — CHIA ĐÔI TRONG MỘT LƯỢT
    // ------------------------------------------------------------------
    let (lon, nho): (Vec<&GiaoDich>, Vec<&GiaoDich>) =
        gd.iter().partition(|g| g.so_tien >= 800_000);
    println!("\n4. partition: {} đơn lớn (>=800k), {} đơn nhỏ", lon.len(), nho.len());

    // ------------------------------------------------------------------
    // 5. fold / reduce / try_fold — BA KIỂU GỘP
    // ------------------------------------------------------------------
    println!("\n5. fold vs reduce vs try_fold");
    let tong_fold: u64 = gd.iter().map(|g| g.so_tien).fold(0, |a, b| a + b);
    let tong_reduce: Option<u64> = gd.iter().map(|g| g.so_tien).reduce(|a, b| a + b);
    println!("   fold  (có giá trị khởi tạo)  : {}", tong_fold);
    println!("   reduce(không có, trả Option) : {:?}", tong_reduce);

    let rong: Vec<u64> = Vec::new();
    println!("   Trên danh sách RỖNG -> fold: {}, reduce: {:?}",
             rong.iter().fold(0u64, |a, b| a + b),
             rong.iter().copied().reduce(|a: u64, b: u64| a + b));

    // try_fold: gộp CÓ THỂ THẤT BẠI, dừng ngay ở lỗi đầu tiên
    let an_toan: Option<u64> = gd.iter().try_fold(0u64, |a, g| a.checked_add(g.so_tien));
    println!("   try_fold (chống tràn số)     : {:?}", an_toan);
    let se_tran: Option<u64> = [u64::MAX, 1].iter().try_fold(0u64, |a, b| a.checked_add(*b));
    println!("   try_fold khi tràn số         : {:?} (dừng ngay, không panic)", se_tran);

    // ------------------------------------------------------------------
    // 6. scan — GIỐNG fold NHƯNG NHẢ RA TỪNG BƯỚC TRUNG GIAN
    // ------------------------------------------------------------------
    let luy_ke: Vec<u64> = gd
        .iter()
        .scan(0u64, |tong, g| {
            *tong += g.so_tien;
            Some(*tong)
        })
        .collect();
    println!("\n6. scan (tổng lũy kế từng bước): {:?}", luy_ke);

    // ------------------------------------------------------------------
    // 7. take_while / skip_while — DỪNG SỚM, KHÁC HẲN filter
    // ------------------------------------------------------------------
    println!("\n7. take_while vs filter");
    let so = [1, 3, 5, 4, 7, 9];
    let tw: Vec<i32> = so.iter().copied().take_while(|x| x % 2 == 1).collect();
    let ft: Vec<i32> = so.iter().copied().filter(|x| x % 2 == 1).collect();
    println!("   dãy gốc              : {:?}", so);
    println!("   take_while(lẻ)       : {:?}  ← DỪNG ngay khi gặp số chẵn đầu tiên", tw);
    println!("   filter(lẻ)           : {:?}  ← duyệt HẾT, giữ mọi số lẻ", ft);
    let sw: Vec<i32> = so.iter().copied().skip_while(|x| x % 2 == 1).collect();
    println!("   skip_while(lẻ)       : {:?}", sw);

    // ------------------------------------------------------------------
    // 8. zip / unzip / chain / rev / step_by
    // ------------------------------------------------------------------
    println!("\n8. zip / unzip / chain / rev / step_by");
    let ma: Vec<&str> = gd.iter().map(|g| g.ma.as_str()).collect();
    let tien: Vec<u64> = gd.iter().map(|g| g.so_tien).collect();
    let ghep: Vec<(&&str, &u64)> = ma.iter().zip(tien.iter()).take(3).collect();
    println!("   zip 3 cặp đầu : {:?}", ghep);

    let (lai_ma, lai_tien): (Vec<&str>, Vec<u64>) =
        ma.iter().copied().zip(tien.iter().copied()).unzip();
    println!("   unzip tách lại: {} mã, {} số tiền", lai_ma.len(), lai_tien.len());

    let noi: Vec<i32> = (1..3).chain(10..12).collect();
    println!("   chain         : {:?}", noi);
    // CHÚ Ý: `rev()` đòi hỏi trait `DoubleEndedIterator` — iterator phải biết đi
    // từ CẢ HAI đầu. `DemNguoc` tự viết chỉ cài `Iterator` (một chiều), nên
    // `DemNguoc::moi(5).rev()` KHÔNG biên dịch được:
    //     error[E0277]: the trait bound `DemNguoc: DoubleEndedIterator` is not satisfied
    // `Vec` thì có, nên ta gom lại trước rồi mới đảo:
    let nguoc: Vec<u32> = DemNguoc::moi(5).collect::<Vec<u32>>().into_iter().rev().collect();
    println!("   rev (cần DoubleEndedIterator): {:?}", nguoc);
    let cach_quang: Vec<i32> = (0..10).step_by(3).collect();
    println!("   step_by(3)    : {:?}", cach_quang);

    // ------------------------------------------------------------------
    // 9. flat_map / flatten
    // ------------------------------------------------------------------
    println!("\n9. flat_map / flatten");
    let cau = ["Rust rất nhanh", "và an toàn"];
    let tu: Vec<&str> = cau.iter().flat_map(|c| c.split_whitespace()).collect();
    println!("   flat_map tách từ: {:?}", tu);

    let long: Vec<Vec<i32>> = vec![vec![1, 2], vec![], vec![3, 4, 5]];
    let phang: Vec<i32> = long.into_iter().flatten().collect();
    println!("   flatten làm phẳng: {:?}", phang);

    let co_none: Vec<Option<i32>> = vec![Some(1), None, Some(3)];
    let bo_none: Vec<i32> = co_none.into_iter().flatten().collect();
    println!("   flatten bỏ None  : {:?}", bo_none);

    // ------------------------------------------------------------------
    // 10. collect VÀO NHIỀU KIỂU KHÁC NHAU
    // ------------------------------------------------------------------
    println!("\n10. collect() gom vào nhiều kiểu đích");
    let chuoi: String = ma.iter().copied().collect::<Vec<&str>>().join(", ");
    println!("   -> String     : {}", chuoi);

    let khu_vuc: HashSet<&str> = gd.iter().map(|g| g.khu_vuc.as_str()).collect();
    let mut kv: Vec<&&str> = khu_vuc.iter().collect();
    kv.sort();
    println!("   -> HashSet    : {:?} ({} khu vực)", kv, khu_vuc.len());

    let bang: HashMap<&str, u64> = gd.iter().map(|g| (g.ma.as_str(), g.so_tien)).collect();
    println!("   -> HashMap    : tra cứu GD-003 = {:?}", bang.get("GD-003"));

    let tot: Result<Vec<i32>, _> = ["1", "2", "3"].iter().map(|s| s.parse::<i32>()).collect();
    let xau: Result<Vec<i32>, _> = ["1", "x", "3"].iter().map(|s| s.parse::<i32>()).collect();
    println!("   -> Result (ổn) : {:?}", tot);
    println!("   -> Result (hỏng): có lỗi = {}", xau.is_err());

    // ------------------------------------------------------------------
    // 11. TỔNG HỢP THEO NHÓM — MẪU DÙNG HẰNG NGÀY
    // ------------------------------------------------------------------
    println!("\n11. Tổng doanh thu theo khu vực (fold + entry API)");
    let theo_kv: HashMap<&str, u64> =
        gd.iter().fold(HashMap::new(), |mut bang, g| {
            *bang.entry(g.khu_vuc.as_str()).or_insert(0) += g.so_tien;
            bang
        });
    let mut cac_kv: Vec<(&&str, &u64)> = theo_kv.iter().collect();
    cac_kv.sort_by(|a, b| b.1.cmp(a.1).then(a.0.cmp(b.0)));
    for (k, v) in cac_kv {
        println!("   {:<10} {:>10} đ", k, v);
    }

    // ------------------------------------------------------------------
    // 12. fold vs rfold — KHI THỨ TỰ CÓ Ý NGHĨA
    // ------------------------------------------------------------------
    println!("\n12. fold vs rfold");
    let m = [10i32, 3, 2];
    println!("   Phép CỘNG (giao hoán)      : fold={}, rfold={}  -> GIỐNG nhau",
             m.iter().fold(0, |a, b| a + b), m.iter().rfold(0, |a, b| a + b));
    let noi_trai: String = m.iter().fold(String::new(), |a, b| a + &b.to_string());
    let noi_phai: String = m.iter().rfold(String::new(), |a, b| a + &b.to_string());
    println!("   NỐI CHUỖI (không giao hoán): fold={:?}, rfold={:?}  -> KHÁC nhau",
             noi_trai, noi_phai);
    println!("   → Trước khi song song hóa, phải biết phép gộp của mình có tính gì!");

    // ------------------------------------------------------------------
    // 13. ITERATOR TỰ VIẾT VÀ IntoIterator TỰ VIẾT
    // ------------------------------------------------------------------
    println!("\n13. Iterator và IntoIterator tự cài đặt");
    let dem: Vec<u32> = DemNguoc::moi(5).collect();
    println!("   DemNguoc(5)                 : {:?}", dem);
    println!("   Miễn phí luôn map/filter/sum: {}", DemNguoc::moi(100).filter(|x| x % 7 == 0).sum::<u32>());

    let gio = GioHang::moi(vec!["Bàn phím".into(), "Chuột".into(), "Màn hình".into()]);
    print!("   for x in &gio_hang -> ");
    for m in &gio {
        print!("[{}] ", m);
    }
    println!();

    let gio_moi: GioHang = gio
        .into_iter()
        .filter(|m| m.chars().count() > 5)
        .collect(); // ← nhờ FromIterator tự cài
    println!("   collect() thẳng vào GioHang : {:?}", gio_moi);

    // ------------------------------------------------------------------
    // 14. Extend — NỐI THÊM VÀO TẬP HỢP ĐÃ CÓ
    // ------------------------------------------------------------------
    let mut kho: Vec<i32> = vec![1, 2];
    kho.extend(3..6);
    println!("\n14. Extend: {:?}", kho);

    println!("\n============================================================");
    println!("   MỘT `next()` — HÀNG CHỤC CÔNG CỤ MIỄN PHÍ ĐI KÈM         ");
    println!("============================================================");
}

// ============================================================================
// KIỂM THỬ
// ============================================================================

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn filter_map_bo_qua_dong_hong() {
        let gd: Vec<GiaoDich> = du_lieu_tho().iter().filter_map(|d| phan_tich_dong(d)).collect();
        assert_eq!(gd.len(), 6, "9 dòng thô, 3 dòng hỏng -> còn 6");
    }

    #[test]
    fn take_while_khac_filter() {
        let so = [1, 3, 5, 4, 7, 9];
        let tw: Vec<i32> = so.iter().copied().take_while(|x| x % 2 == 1).collect();
        let ft: Vec<i32> = so.iter().copied().filter(|x| x % 2 == 1).collect();
        assert_eq!(tw, vec![1, 3, 5]); // dừng ở số 4
        assert_eq!(ft, vec![1, 3, 5, 7, 9]); // duyệt hết
    }

    #[test]
    fn reduce_tra_none_khi_rong() {
        let rong: Vec<u64> = Vec::new();
        assert_eq!(rong.iter().copied().reduce(|a, b| a + b), None);
        assert_eq!(rong.iter().fold(0u64, |a, b| a + b), 0); // fold vẫn có câu trả lời
    }

    #[test]
    fn try_fold_dung_ngay_khi_tran_so() {
        let kq: Option<u64> = [u64::MAX, 1, 2].iter().try_fold(0u64, |a, b| a.checked_add(*b));
        assert_eq!(kq, None);
    }

    #[test]
    fn scan_nha_ra_tung_buoc_trung_gian() {
        let luy_ke: Vec<i32> = [1, 2, 3, 4]
            .iter()
            .scan(0, |t, x| { *t += x; Some(*t) })
            .collect();
        assert_eq!(luy_ke, vec![1, 3, 6, 10]);
    }

    #[test]
    fn fold_va_rfold_chi_khac_nhau_voi_phep_khong_giao_hoan() {
        let m = [10i32, 3, 2];
        // Phép cộng GIAO HOÁN -> duyệt hai chiều cho cùng kết quả
        assert_eq!(m.iter().fold(0, |a, b| a + b), m.iter().rfold(0, |a, b| a + b));
        // Nối chuỗi KHÔNG giao hoán -> duyệt hai chiều cho kết quả khác nhau
        let trai: String = m.iter().fold(String::new(), |a, b| a + &b.to_string());
        let phai: String = m.iter().rfold(String::new(), |a, b| a + &b.to_string());
        assert_eq!(trai, "1032");
        assert_eq!(phai, "2310");
        assert_ne!(trai, phai);
    }

    #[test]
    fn collect_gom_duoc_nhieu_kieu_dich() {
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
    fn partition_chia_dung_hai_nhom() {
        let (chan, le): (Vec<i32>, Vec<i32>) = (1..8).partition(|x| x % 2 == 0);
        assert_eq!(chan, vec![2, 4, 6]);
        assert_eq!(le, vec![1, 3, 5, 7]);
    }

    #[test]
    fn iterator_tu_viet_hoat_dong() {
        assert_eq!(DemNguoc::moi(3).collect::<Vec<u32>>(), vec![3, 2, 1]);
        assert_eq!(DemNguoc::moi(10).filter(|x| x % 3 == 0).sum::<u32>(), 18); // 9+6+3
    }

    #[test]
    fn into_iterator_va_from_iterator_tu_viet() {
        let gio = GioHang::moi(vec!["Bàn phím".into(), "Chuột".into()]);
        let ten: Vec<&String> = (&gio).into_iter().collect();
        assert_eq!(ten.len(), 2);
        let loc: GioHang = gio.into_iter().filter(|m| m.chars().count() > 5).collect();
        assert_eq!(loc, GioHang::moi(vec!["Bàn phím".into()]));
    }

    #[test]
    fn tong_hop_theo_khu_vuc_dung() {
        let gd: Vec<GiaoDich> = du_lieu_tho().iter().filter_map(|d| phan_tich_dong(d)).collect();
        let theo_kv: HashMap<&str, u64> = gd.iter().fold(HashMap::new(), |mut b, g| {
            *b.entry(g.khu_vuc.as_str()).or_insert(0) += g.so_tien;
            b
        });
        assert_eq!(theo_kv.get("Hà Nội"), Some(&1_570_000)); // 1250000 + 320000
        assert_eq!(theo_kv.get("Cần Thơ"), Some(&780_000));
    }
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
5. **Bộ công cụ đầy đủ**: ngoài `map`/`filter`, hãy nhớ `filter_map` (lọc + biến đổi), `flat_map` (nở ra nhiều phần tử), `take_while` (dừng sớm, khác `filter`), `partition` (chia đôi một lượt), `reduce` (gộp không cần khởi tạo), `try_fold` (gộp có thể thất bại) và `scan` (nhả từng bước trung gian).
6. **`collect()` không chỉ tạo `Vec`**: nó gom được vào `String`, `HashMap`, `HashSet`, `Result`, `Option` — hay bất kỳ kiểu nào cài `FromIterator`, kể cả kiểu của chính bạn.
7. **Song song hóa gần như miễn phí**: với `rayon`, đổi `.iter()` thành `.par_iter()` là chạy trên toàn bộ nhân CPU. Điều kiện duy nhất: closure phải thuần túy — và trình biên dịch kiểm tra giúp bạn.

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

4. **Bài tập 4 (Làm sạch dữ liệu bẩn bằng `filter_map`)**:  
   Cho `let tho = ["12", "abc", "7", "", "30", "-5"];`. Hãy dùng **một** đường ống duy nhất để: bỏ qua mọi dòng không phân tích được thành `u32`, rồi tính tổng các số hợp lệ. Không dùng vòng lặp `for`, không dùng `unwrap()`.

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
    let so = vec![12, 7, 19, 24, 30, 5, 8];

    let binh_phuong_chan: Vec<i32> = so
        .iter()
        .filter(|&&x| x % 2 == 0)   // 12, 24, 30, 8
        .map(|&x| x * x)            // 144, 576, 900, 64
        .collect();

    assert_eq!(binh_phuong_chan, vec![144, 576, 900, 64]);
    println!("{:?}", binh_phuong_chan);
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
pub fn lon_nhat(ds: &[i32]) -> Option<i32> {
    if ds.is_empty() {
        return None;   // câu trả lời trung thực cho danh sách rỗng
    }
    // i32::MIN là "âm vô cực": gộp với bất cứ số nào cũng thua.
    Some(ds.iter().fold(i32::MIN, |lon_nhat, &x| if x > lon_nhat { x } else { lon_nhat }))
}

fn main() {
    assert_eq!(lon_nhat(&[3, 9, 2, 7]), Some(9));
    assert_eq!(lon_nhat(&[-30, -9, -100]), Some(-9)); // khởi tạo bằng 0 sẽ SAI ở đây!
    assert_eq!(lon_nhat(&[]), None);
    println!("{:?}", lon_nhat(&[-30, -9, -100]));
}
```

Hai bài học: (1) chọn sai phần tử khởi tạo là một lỗi im lặng, chỉ lộ ra với dữ liệu âm; (2) trả `Option` thay vì một con số bịa ra chính là biến hàm bộ phận thành **hàm toàn phần** (Chương 13).
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Bạn chỉ phải viết đúng **một** phương thức: `fn next(&mut self) -> Option<Self::Item>`. Hãy cẩn thận thứ tự: giảm `hien_tai` trước rồi trả về, hay trả về trước rồi giảm? Hãy tự kiểm bằng cách viết ra kỳ vọng: `BoDemNguoc { hien_tai: 3 }` phải cho ra `3, 2, 1`.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

```rust
pub struct BoDemNguoc {
    pub hien_tai: u32,
}

impl Iterator for BoDemNguoc {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.hien_tai == 0 {
            None
        } else {
            let ra = self.hien_tai;
            self.hien_tai -= 1;
            Some(ra)
        }
    }
}

fn main() {
    // Dùng với vòng lặp for (nhờ IntoIterator có sẵn cho mọi Iterator)
    print!("Đếm ngược: ");
    for n in (BoDemNguoc { hien_tai: 5 }) {
        print!("{} ", n);
    }
    println!("Phóng!");

    // PHẦN THƯỞNG: chỉ cài `next()` mà được dùng ngay hàng chục phương thức khác
    let ds: Vec<u32> = BoDemNguoc { hien_tai: 5 }.collect();
    assert_eq!(ds, vec![5, 4, 3, 2, 1]);

    let tong_chan: u32 = BoDemNguoc { hien_tai: 10 }.filter(|n| n % 2 == 0).sum();
    assert_eq!(tong_chan, 30); // 10+8+6+4+2

    let ba_dau: Vec<u32> = BoDemNguoc { hien_tai: 100 }.take(3).collect();
    assert_eq!(ba_dau, vec![100, 99, 98]);
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
    let tho = ["12", "abc", "7", "", "30", "-5"];

    let tong: u32 = tho.iter().filter_map(|s| s.parse::<u32>().ok()).sum();

    // "abc", "" và "-5" đều bị bỏ qua ("-5" không phải u32 hợp lệ)
    assert_eq!(tong, 49); // 12 + 7 + 30
    println!("Tổng các số hợp lệ: {}", tong);
}
```

Một dòng duy nhất, không `unwrap()`, không vòng lặp, không biến `mut`. Đây là mẫu bạn sẽ dùng gần như mỗi khi đọc dữ liệu từ tệp hay mạng.
</details>

<details>
<summary><b>Bài tập 5 — Gợi ý</b></summary>

Điểm mấu chốt nằm ở **kiểu của biến hứng**, không phải ở đường ống. Hãy thử `let kq: Result<Vec<u32>, _> = ...` và bỏ `.ok()` đi — `collect()` sẽ tự hiểu bạn muốn gì.
</details>

<details>
<summary><b>Bài tập 5 — Lời giải</b></summary>

```rust
fn main() {
    let hong = ["12", "abc", "7"];
    let tot = ["12", "7", "30"];

    // KHÁC BIỆT DUY NHẤT so với bài 4: kiểu của biến hứng, và không có `.ok()`
    let kq_hong: Result<Vec<u32>, _> = hong.iter().map(|s| s.parse::<u32>()).collect();
    let kq_tot: Result<Vec<u32>, _> = tot.iter().map(|s| s.parse::<u32>()).collect();

    assert!(kq_hong.is_err());                 // MỘT dòng hỏng -> TOÀN BỘ hỏng
    assert_eq!(kq_tot, Ok(vec![12, 7, 30]));   // mọi dòng tốt  -> được cả danh sách
    println!("{:?}\n{:?}", kq_hong, kq_tot);
}
```

Hãy so sánh bài 4 và bài 5 — cùng dữ liệu, cùng đường ống, chỉ khác kiểu đích:
- **`filter_map` + `.ok()`** = "bỏ qua dòng hỏng, cứu được bao nhiêu hay bấy nhiêu" (dùng cho nhật ký, dữ liệu thống kê).
- **`collect::<Result<Vec<_>, _>>()`** = "được ăn cả, ngã về không" (dùng cho tệp cấu hình, giao dịch tài chính).

Chọn đúng một trong hai là một **quyết định thiết kế**, không phải chuyện phong cách. Kỹ thuật thứ hai có tên chính thức là *Traversable* — Chương 19 sẽ nói kỹ.
</details>
