# Chương 04: Điều khiển dòng chảy: Rẽ nhánh và Vòng lặp (Control Flow: If/Else and Loops)

## Giới thiệu & Mục tiêu học tập

Cho đến thời điểm hiện tại, các chương trình máy tính bạn viết đều vận hành theo một đường thẳng tắp: CPU đọc dòng lệnh số 1, chuyển sang dòng số 2, rồi kết thúc ở dòng số 3. Tuy nhiên, thế giới thực không vận hành đơn giản như vậy. Một ứng dụng thông minh cần biết **đưa ra quyết định** (Nếu người dùng nhập đúng mật khẩu thì mở cửa, ngược lại thì báo lỗi) và biết **lặp đi lặp lại công việc** (Gửi email cho 1.000 khách hàng trong danh sách).

Khả năng này được gọi là **Điều khiển dòng chảy (Control Flow)**.

Mục tiêu học tập của chương này:
- Làm chủ khối điều kiện rẽ nhánh `if`, `else if`, và `else`.
- Khám phá tính năng độc đáo của Rust: `if` hoạt động như một **Biểu thức (Expression)** có khả năng trả về giá trị trực tiếp để gán cho biến.
- Hiểu rõ tại sao Rust nghiêm cấm việc coi số `1` là đúng hay `0` là sai (bắt buộc điều kiện phải là kiểu logic `bool`).
- Thành thạo 3 công cụ lặp dữ liệu:
  - `loop`: Vòng lặp vô tận mạnh mẽ, có thể trả về giá trị thông qua lệnh `break`.
  - `while`: Vòng lặp chạy chừng nào điều kiện kiểm tra còn thỏa mãn.
  - `for .. in`: Vòng lặp an toàn tuyệt đối, duyệt qua từng phần tử của một danh sách mà không bao giờ sợ lỗi tràn mảng.
- Biết cách sử dụng **Nhãn vòng lặp (Loop Labels)** để điều khiển chính xác các vòng lặp lồng nhau phức tạp.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để các cấu trúc điều khiển dòng chảy in sâu vào tâm trí bạn một cách tự nhiên nhất, hãy cùng quan sát 4 hình ảnh vô cùng quen thuộc sau:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                   HÌNH TƯỢNG ĐỜI SỐNG VỀ ĐIỀU KHIỂN DÒNG CHẢY                    │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│    NGÃ TƯ ĐÈN GIAO THÔNG│     ẤM ĐUN NƯỚC SIÊU TỐC      │      BÁC BƯU TÁ PHÁT THƯ│
│       (Cấu trúc if/else)│          (Vòng lặp while)     │       (Vòng lặp for)   │
│                         │                               │                        │
│ - Đèn xanh -> Đi thẳng  │ - Cắm điện đun liên tục       │ - Đi từng số nhà cụ thể│
│ - Đèn đỏ   -> Dừng lại  │ - CHỪNG NÀO nước chưa sôi     │ - Từ nhà số 1 đến số 10│
│ - Chỉ có 1 nhánh đường  │ - Đạt 100°C -> Rơ-le tự nảy   │ - Không phát nhầm nhà  │
│   được chọn tại một lúc │   ngắt điện an toàn           │   số 11 ngoài phố      │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Ngã tư đèn tín hiệu giao thông (`if`, `else if`, `else`)
Bạn đang lái xe trên đường và gặp một ngã tư:
- Nếu đèn tín hiệu màu **Xanh**: Bạn nhấn ga đi thẳng.
- Ngược lại, nếu đèn màu **Vàng**: Bạn đạp nhẹ phanh giảm tốc độ.
- Ngược lại hoàn toàn (đèn màu **Đỏ**): Bạn dừng xe lại trước vạch kẻ.
Tại một thời điểm, bạn chỉ có thể chọn duy nhất một quyết định. Không ai có thể vừa đạp ga phóng đi vừa phanh đứng xe lại cùng một lúc.

### 2. Người chạy bộ quanh sân vận động (`loop` và `break`)
Hãy hình dung một vận động viên chạy vòng quanh sân vận động:
- Anh ta cứ thế chạy hết vòng này đến vòng khác một cách vô tận (`loop`).
- Nhưng trên cổ tay anh ta có một chiếc đồng hồ thông minh theo dõi lượng calo tiêu hao.
- Khi đồng hồ báo đã tiêu hao đủ `500 calo`, anh ta lập tức dừng chân (`break`), bước ra khỏi đường chạy và mang theo kết quả: "Hôm nay tôi đã chạy được 10 vòng!".

### 3. Chiếc ấm đun nước siêu tốc tự ngắt (`while`)
Bạn cắm chiếc ấm siêu tốc vào ổ điện và nhấn nút:
- Ấm sẽ tiếp tục đun nóng dây may-so **CHỪNG NÀO** nhiệt độ nước còn thấp hơn 100°C (`while nuoc_chua_soi`).
- Ngay khi nhiệt độ đạt 100°C, hơi nước bốc lên làm giãn nở thanh kim loại nhiệt, rơ-le nảy "tách" một cái và tự ngắt nguồn điện dừng lại.

### 4. Bác bưu tá phát báo đầu ngõ (`for .. in`)
Bác bưu tá có một xấp báo và danh sách các ngôi nhà từ nhà số 1 đến nhà số 5 trên một con phố:
- Bác đi đến nhà số 1, thả một tờ báo vào hòm thư.
- Bác bước sang nhà số 2, thả tiếp một tờ báo.
- Cứ thế lần lượt cho đến hết nhà số 5, bác kết thúc công việc và ra về.
Bác bưu tá không cần phải đếm xem mình đã bước bao nhiêu bước chân. Bác chỉ ghé đúng những ngôi nhà có thật trên danh sách, và tuyệt đối không bao giờ bước nhầm vào một ngôi nhà số 6 ma quái không hề tồn tại trên phố!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Phân biệt then chốt: Câu lệnh (Statement) vs Biểu thức (Expression)

Đây là một trong những nét đặc sắc nhất của triết lý thiết kế ngôn ngữ Rust:
- **Câu lệnh (Statement)**: Là các chỉ thị thực hiện một hành động nào đó nhưng **không tạo ra giá trị trả về**. Trong Rust, các câu lệnh luôn kết thúc bằng dấu chấm phẩy `;`.
  - Ví dụ: `let x = 6;` là một câu lệnh. Bạn không thể viết `let y = (let x = 6);` vì câu lệnh không sinh ra giá trị.
- **Biểu thức (Expression)**: Tính toán một điều gì đó và **sinh ra một giá trị cụ thể**. Biểu thức **không** kết thúc bằng dấu chấm phẩy.
  - Ví dụ: `5 + 5` là một biểu thức trả về giá trị `10`.
  - Một khối mã nằm trong cặp ngoặc nhọn `{ ... }` cũng là một biểu thức! Giá trị của dòng cuối cùng (không có dấu `;`) chính là giá trị trả về của toàn bộ khối mã đó.

### 2. `if` trong Rust là một biểu thức trả về giá trị

Vì `if` là một biểu thức, bạn có thể gán trực tiếp kết quả của khối `if` vào một biến thông qua từ khóa `let`:
```rust
let condition = true;
let con_so = if condition { 5 } else { 10 };
```

> **Quy tắc sắt đá của Trình biên dịch**: Vì Rust là ngôn ngữ định kiểu tĩnh, kiểu dữ liệu của biến `con_so` phải được xác định duy nhất ngay tại thời điểm biên dịch. Do đó, **tất cả các nhánh `if` và `else` bắt buộc phải trả về cùng một kiểu dữ liệu**! Bạn không thể để nhánh `if` trả về số `5` còn nhánh `else` lại trả về chữ `"Mười"`.

### 3. Nghiêm cấm "Ép kiểu chân lý ngầm" (Không có Truthy/Falsy)

Trong các ngôn ngữ như C, C++, JavaScript hay Python, bạn có thể viết:
```javascript
// Mã JavaScript: Số 1 được coi là đúng (Truthy)
let x = 1;
if (x) { ... }
```
Nhưng trong Rust, điều này **bị cấm tuyệt đối**! Trình biên dịch sẽ ném ra lỗi `E0308 (mismatched types)` ngay lập tức. Điều kiện trong `if` bắt buộc phải là một giá trị mang kiểu logic thuần túy `bool` (`true` hoặc `false`). Sự khắt khe này giúp lập trình viên tránh được hàng ngàn lỗi logic tai hại khi vô tình nhầm lẫn giữa số 0 và giá trị sai.

### 4. Vòng lặp `loop` mang giá trị về thông qua `break`

Trong Rust, `loop` đại diện cho một vòng lặp vô tận. Điểm đặc biệt là bạn có thể đặt một giá trị ngay sau từ khóa `break`:
```rust
let mut count = 0;
let ket_qua = loop {
    count += 1;
    if count == 10 {
        break count * 2; // Thoát vòng lặp và mang giá trị 20 về gán cho ket_qua!
    }
};
```
Nhờ cơ chế này, bạn có thể thực hiện các phép thử nghiệm (ví dụ thử kết nối lại mạng máy tính) và khi thành công thì mang dữ liệu thu được ra ngoài mà không cần khai báo biến tạm lộn xộn.

### 5. Nhãn vòng lặp (Loop Labels) giải quyết bài toán lồng nhau

Khi bạn viết một vòng lặp bên trong một vòng lặp khác, lệnh `break` thông thường chỉ giúp bạn thoát khỏi vòng lặp con gần nhất. Nếu muốn thoát văng hẳn ra khỏi vòng lặp cha bên ngoài, Rust cung cấp cú pháp **Nhãn vòng lặp** bắt đầu bằng dấu nháy đơn:
```rust
'vong_lap_ngoai: loop {
    loop {
        break 'vong_lap_ngoai; // Thoát ngay lập tức ra khỏi cả 2 vòng lặp!
    }
}
```

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình dưới đây mô phỏng một trạm kiểm soát phóng tên lửa không gian, kết hợp hoàn hảo giữa `if` biểu thức, `loop` có giá trị trả về, `while` và `for`:

```rust
// File: src/main.rs
// Chương trình mô phỏng Trạm Kiểm Soát Phóng Tên Lửa Vũ Trụ

fn main() {
    println!("============================================================");
    println!("       TRẠM ĐIỀU HÀNH VŨ TRỤ - CHƯƠNG TRÌNH PHÓNG TÊN LỬA    ");
    println!("============================================================");

    // 1. Sử dụng if như một biểu thức để xác định trạng thái thời tiết
    let wind_kmh = 25;
    let is_raining = false;

    // if/else trả về trực tiếp chuỗi trạng thái được gán vào biến
    let weather = if wind_kmh < 40 && !is_raining {
        "Hoàn hảo để phóng"
    } else if wind_kmh < 60 {
        "Cần theo dõi thêm sức gió"
    } else {
        "Hủy lịch phóng vì thời tiết xấu"
    };
    println!("Tình trạng khí tượng hiện tại: {}", weather);

    // 2. Sử dụng vòng lặp 'loop' có 'break' mang giá trị về:
    // Kiểm tra áp suất nhiên liệu buồng đốt đến khi đạt chuẩn an toàn
    let mut current_pressure = 80;
    println!("\nBắt đầu kích áp buồng đốt nhiên liệu...");

    let threshold_pressure = loop {
        current_pressure += 5;
        println!("- Áp suất đang tăng: {} PSI", current_pressure);

        if current_pressure >= 100 {
            // Khi áp suất đạt ngưỡng 100 PSI, thoát vòng lặp và mang giá trị về!
            break current_pressure;
        }
    };
    println!("==> Áp suất buồng đốt đã khóa an toàn tại mức: {} PSI", threshold_pressure);

    // 3. Sử dụng vòng lặp 'while' để nạp năng lượng bình ắc-quy phụ
    let mut battery_capacity = 85;
    println!("\nĐang sạc bù hệ thống năng lượng dự phòng:");
    while battery_capacity < 100 {
        battery_capacity += 5;
        println!("  Đang sạc... mức pin hiện tại: {}%", battery_capacity);
    }
    println!("==> Hệ thống ắc-quy phụ đã đạt 100%!");

    // 4. Sử dụng vòng lặp lồng nhau với Nhãn (Loop Labels) để quét cảm biến
    println!("\nBắt đầu diễn tập kịch bản ngắt khẩn cấp trên 3 tầng tên lửa:");
    let mut has_emitted = false;

    'kiem_tra_tang_ten_lua: for tang in 1..=3 {
        println!("* Đang quét tầng tên lửa số {}", tang);
        for cam_bien in 1..=4 {
            if tang == 2 && cam_bien == 3 {
                has_emitted = true; // Kích hoạt sự cố mô phỏng!
                println!("  [!] Phát hiện sự cố tại tầng {}, cảm biến {}! Kích hoạt ngắt khẩn cấp!", 
                         tang, cam_bien);
                // Thoát thẳng ra ngoài cả hai vòng lặp nhờ nhãn:
                break 'kiem_tra_tang_ten_lua;
            }
            println!("  - Cảm biến {}.{} hoạt động bình thường", tang, cam_bien);
        }
    }

    if has_emitted {
        println!("==> Cơ chế ngắt khẩn cấp bằng nhãn đã dừng kiểm tra an toàn!");
        println!("==> Đội kỹ thuật đã khắc phục xong sự cố cảm biến 2.3.");
    }

    // 5. Vòng lặp 'for' an toàn đếm ngược thời gian phóng tên lửa
    // (1..=5).rev() tạo ra dãy số: 5, 4, 3, 2, 1
    println!("\nTẤT CẢ HỆ THỐNG SẴN SÀNG! ĐẾM NGƯỢC ĐỂ PHÓNG:");
    for giay in (1..=5).rev() {
        println!("T-minus {} giây...", giay);
    }

    println!("\n🚀 KHAI HỎA ĐỘNG CƠ CHÍNH! TÊN LỬA ĐÃ RỜI BỆ PHÓNG THÀNH CÔNG! 🚀");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi viết các cấu trúc điều khiển dòng chảy trong Rust, bạn sẽ thường gặp các lỗi sau từ trình biên dịch:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0308** | `error[E0308]: mismatched types: expected integer, found '&str'` | Các nhánh của biểu thức `if / else` trả về kiểu dữ liệu khác nhau (ví dụ: nhánh `if` trả về số `10`, nhánh `else` trả về chữ `"Lỗi"`). | Đảm bảo tất cả các nhánh `if` và `else` đều trả về cùng một kiểu dữ liệu thống nhất. |
| **E0308** | `error[E0308]: mismatched types: expected 'bool', found integer` | Truyền một con số vào điều kiện `if` (ví dụ: viết `if quantity { ... }` thay vì so sánh rõ ràng). | Viết biểu thức so sánh rõ ràng trả về `bool` (ví dụ: `if quantity > 0 { ... }`). |
| **Thiếu nhánh else** | `error[E0317]: 'if' may be missing an 'else' clause` | Bạn dùng `let x = if ...` nhưng lại không viết phần `else`. Trình biên dịch không biết nếu điều kiện sai thì biến `x` sẽ nhận giá trị gì. | Luôn bổ sung nhánh `else` đầy đủ khi sử dụng `if` dưới dạng biểu thức gán giá trị cho biến. |
| **Cảnh báo unreachable**| `warning: unreachable statement` | Đặt các dòng lệnh ở phía sau từ khóa `break` hoặc `return`. Do vòng lặp đã thoát trước đó, những dòng lệnh này sẽ không bao giờ được chạm tới. | Xóa bỏ hoặc di chuyển các dòng lệnh bị cảnh báo lên phía trước lệnh `break`. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Biểu thức `if` trả về giá trị**: Trong Rust, `if` có thể tạo ra giá trị để gán trực tiếp cho biến; yêu cầu tất cả các nhánh phải trả về cùng một kiểu dữ liệu.
2. **Kiểu điều kiện khắt khe**: Biểu thức kiểm tra trong `if` và `while` bắt buộc phải là kiểu `bool` (`true`/`false`), Rust không chấp nhận số nguyên đại diện cho chân lý.
3. **Sức mạnh của `loop`**: Vòng lặp vô hạn `loop` có thể đưa dữ liệu ra ngoài phạm vi vòng lặp thông qua cú pháp `break value;`.
4. **Vòng lặp `for` an toàn**: Cú pháp `for phan_tu in list` giúp duyệt dữ liệu tiện lợi, loại bỏ triệt để lỗi chỉ mục vượt giới hạn mảng (Index Out of Bounds).

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Viết chương trình xếp loại học lực học sinh dựa vào điểm trung bình (thang điểm 10):
   - Nếu điểm từ 9.0 trở lên: Xếp loại "Xuất sắc".
   - Từ 8.0 đến dưới 9.0: Xếp loại "Giỏi".
   - Từ 6.5 đến dưới 8.0: Xếp loại "Khá".
   - Dưới 6.5: Xếp loại "Cần nỗ lực hơn".
   - *Yêu cầu*: Sử dụng `if / else` như một biểu thức để gán danh hiệu trực tiếp vào biến `danh_hieu`.
2. **Bài tập thực hành 2**: Sử dụng vòng lặp `for` và khoảng số `1..=100` để tính tổng tất cả các số chẵn từ 1 đến 100. In kết quả cuối cùng ra màn hình (gợi ý: dùng toán tử chia lấy dư `% 2 == 0`).
3. **Bài tập tư duy 3**: Trong tình huống nào bạn nên dùng vòng lặp `while`, và trong tình huống nào bạn bắt buộc phải dùng `loop` kết hợp với `break`? Hãy giải thích qua ví dụ thực tế về việc người dùng nhập mật khẩu đăng nhập.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Dùng `if/else if/else` như một **biểu thức**: cả chuỗi trả về một giá trị, gán thẳng vào `danh_hieu`. Không cần `mut`.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
fn xep_loai(diem: f64) -> &'static str {
    // if/else là BIỂU THỨC: cả khối trả về một giá trị -> gán thẳng, không cần mut.
    // Xét từ mốc cao xuống thấp: nhánh đầu khớp là dừng.
    if diem >= 9.0 {
        "Xuất sắc"
    } else if diem >= 8.0 {
        "Giỏi"
    } else if diem >= 6.5 {
        "Khá"
    } else {
        "Cần nỗ lực hơn"
    }
}

fn main() {
    let diem = 8.5;
    let danh_hieu = xep_loai(diem);   // gán trực tiếp kết quả biểu thức if
    println!("Điểm {diem} -> {danh_hieu}");
}

#[test]
fn xep_loai_dung_cac_moc() {
    assert_eq!(xep_loai(9.0), "Xuất sắc");   // đúng mốc 9.0 -> tính là Xuất sắc
    assert_eq!(xep_loai(8.9), "Giỏi");        // sát dưới 9.0
    assert_eq!(xep_loai(6.5), "Khá");         // đúng mốc 6.5
    assert_eq!(xep_loai(6.4), "Cần nỗ lực hơn");
}
```

Hai điều đáng học:
1. **Thứ tự xét phải từ cao xuống thấp.** Vì mỗi nhánh chỉ kiểm `>=` một mốc, nhánh đầu tiên khớp sẽ thắng. Đảo ngược (xét 6.5 trước) thì điểm 9.0 cũng rơi vào "Khá" — sai.
2. **`if` là biểu thức, không phải câu lệnh.** Trong nhiều ngôn ngữ bạn phải khai báo `danh_hieu` rỗng rồi gán trong từng nhánh. Ở Rust cả khối `if/else` *là* một giá trị, nên gán một phát — sạch hơn và không có nguy cơ quên gán một nhánh.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

`1..=100` là khoảng bao gồm cả 100. Lọc số chẵn bằng `% 2 == 0` rồi cộng dồn. Có thể làm tay bằng `for` hoặc gọn bằng iterator.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
fn tong_so_chan() -> u32 {
    let mut tong = 0;
    // 1..=100 BAO GỒM cả 100 (khác 1..100 là dừng ở 99).
    for i in 1..=100 {
        if i % 2 == 0 {   // chia 2 dư 0 -> số chẵn
            tong += i;
        }
    }
    tong
}

fn main() {
    println!("Tổng số chẵn 1..=100 = {}", tong_so_chan());

    // Cách gọn hơn bằng iterator — cùng kết quả:
    let tong: u32 = (1..=100).filter(|n| n % 2 == 0).sum();
    println!("Kiểm tra lại = {tong}");
}

#[test]
fn tong_dung_bang_2550() {
    // 2+4+...+100 = 2·(1+2+...+50) = 2·1275 = 2550
    assert_eq!(tong_so_chan(), 2550);
    assert_eq!((1..=100).filter(|n| n % 2 == 0).sum::<u32>(), 2550);
}
```

Kết quả **2550** kiểm chứng được bằng tay: các số chẵn là 2+4+…+100 = 2·(1+2+…+50) = 2·1275. Bài này cũng cho thấy hai phong cách Rust song song tồn tại: vòng `for` tường minh (dễ đọc từng bước) và chuỗi iterator `filter().sum()` (ngắn, nói thẳng *ý định*). Cùng một bài toán, bạn sẽ dần nghiêng về kiểu iterator khi quen tay.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

So sánh: khi nào bạn **biết trước điều kiện dừng** so với khi bạn **phải chạy ít nhất một lần rồi mới biết có dừng không**.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

Phân biệt qua chính ví dụ nhập mật khẩu:

**Dùng `loop` + `break`** khi bạn **phải làm ít nhất một lần** rồi mới kiểm được điều kiện dừng. Nhập mật khẩu đúng kiểu này: bạn *bắt buộc* phải hỏi người dùng một lần trước, rồi mới biết họ gõ đúng hay sai.
```text
loop {
    let nhap = doc_mat_khau();       // luôn chạy ít nhất một lần
    if nhap == mat_khau_dung {
        break;                        // đúng -> thoát
    }
    println!("Sai rồi, thử lại.");
}
```

**Dùng `while`** khi điều kiện dừng **kiểm được ngay từ đầu**, và có khả năng thân vòng lặp *không chạy lần nào*. Ví dụ: "trừ dần số lần thử còn lại cho tới khi hết lượt".
```text
let mut so_lan_con_lai = 3;
while so_lan_con_lai > 0 {   // nếu vào vòng đã là 0 thì bỏ qua luôn
    thu_dang_nhap();
    so_lan_con_lai -= 1;
}
```

Nguyên tắc gọn: **`while` kiểm điều kiện *trước* mỗi vòng** (có thể chạy 0 lần); **`loop` chạy rồi mới kiểm và `break` ở *giữa* hoặc *cuối*** (chạy ít nhất 1 lần). Chọn sai dẫn tới hoặc hỏi mật khẩu thừa một lần, hoặc bỏ sót không hỏi lần nào. Ngoài ra `loop` còn có sức mạnh riêng: nó là biểu thức, `break x` trả được giá trị ra ngoài — thứ `while` không làm được.
</details>
