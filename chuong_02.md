# Chương 02: Bắt đầu với Rust & Cargo: Cài đặt và chương trình đầu tiên (Getting Started with Rust & Cargo: Installation and First Program)

## Giới thiệu & Mục tiêu học tập

Ở chương trước, bạn đã biết rằng máy tính thực chất là một mạng lưới gồm hàng tỷ chiếc công tắc điện tử, và CPU cần các chỉ thị nhị phân để hoạt động. Nhưng làm thế nào để chúng ta biến những ý tưởng trong đầu thành những chỉ thị đó một cách dễ dàng và bài bản nhất?

Câu trả lời nằm ở bộ công cụ tiêu chuẩn của Rust: **rustup**, **cargo** và **rustc**.

Mục tiêu học tập của chương này:
- Làm quen với cửa sổ dòng lệnh (Terminal / Command Prompt) mà không cảm thấy e ngại.
- Nắm vững vai trò của bộ ba công cụ: `rustup` (người cài đặt và quản lý phiên bản), `cargo` (người quản đốc dự án) và `rustc` (trình biên dịch cốt lõi).
- Hiểu tường tận cấu trúc của một thư mục dự án Rust tiêu chuẩn (`Cargo.toml`, `src/main.rs`, thư mục `target/`).
- Thành thạo các câu lệnh làm việc hàng ngày của lập trình viên Rust: `cargo new`, `cargo check`, `cargo build` và `cargo run`.
- Phân biệt sự khác biệt sống còn giữa chế độ biên dịch thử nghiệm (**Debug**) và chế độ xuất xưởng tối ưu tốc độ (**Release**).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để việc học lập trình trở nên nhẹ nhàng, chúng ta hãy tiếp tục hình tượng hóa các công cụ của Rust qua các nhân vật quen thuộc trong một xưởng may thủ công cao cấp:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                         XƯỞNG MAY SẢN PHẨM RUST CAO CẤP                          │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│      QUẢN LÝ DỰ ÁN      │       GIÁM SÁT KỸ THUẬT       │    HỢP ĐỒNG MAY MẶC    │
│         (CARGO)         │            (RUSTC)            │      (Cargo.toml)      │
│                         │                               │                        │
│ - Chuẩn bị nhà xưởng    │ - Cực kỳ tỉ mỉ, khó tính      │ - Ghi tên sản phẩm     │
│ - Mua chỉ, cúc, phụ liệu│ - Soi từng đường kim mũi chỉ  │ - Tên tác giả          │
│ - Điều phối công đoạn   │ - Bản vẽ sai -> Chặn ngay cổng│ - Danh sách vật tư cần │
│ - Đóng gói áo thành phẩm│ - Đảm bảo áo không bục chỉ    │   mua từ chợ phụ liệu  │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Cargo — Người quản đốc xưởng may tận tụy
Hãy tưởng tượng bạn là một nhà thiết kế thời trang. Bạn chỉ cần vẽ mẫu thiết kế chiếc áo lên giấy (`src/main.rs`) và nói với người quản đốc: *"May cho tôi chiếc áo này!"* (`cargo run`).
- Quản đốc **Cargo** sẽ tự động làm hết mọi việc nặng nhọc:
  - Tự chuẩn bị sẵn mẫu rập tiêu chuẩn khi bắt đầu dự án mới (`cargo new`).
  - Tự động chạy ra "chợ nguyên phụ liệu quốc tế" ([crates.io](https://crates.io)) để mua đúng loại chỉ, cúc áo mà bạn yêu cầu.
  - Chuyển giao vải và bản vẽ cho thợ may chính (`rustc`).
  - Khi áo may xong, đóng gói phẳng phiu vào thùng hàng và trao tận tay bạn để mặc thử!

### 2. rustc — Vị giám sát viên công trình nghiêm khắc nhưng nhân hậu
Trình biên dịch **rustc** là người thợ may kiêm giám sát kỹ thuật tối cao. Ông có một nguyên tắc sắt đá: **Không bao giờ cho phép một sản phẩm lỗi xuất xưởng**.
- Nếu bạn vẽ sai một đường chỉ hoặc chọn nhầm chất liệu dễ cháy, `rustc` sẽ lập tức huýt còi dừng xưởng may lại.
- Nhưng điều tuyệt vời nhất là `rustc` không bao giờ mắng bạn. Ông sẽ chỉ tận tay: *"Chỗ này ở dòng số 5, bạn quên mất dấu chấm phẩy; hãy thêm vào như thế này..."*. Ông chính là người thầy kèm cặp 1-1 kiên nhẫn nhất trên hành trình học lập trình của bạn.

### 3. Cargo.toml — Bản hợp đồng và danh mục vật tư
Mỗi dự án Rust đều có một tệp mang tên `Cargo.toml`. Đây chính là tờ hợp đồng kinh tế dán ngay cửa xưởng:
- Ghi rõ: Chiếc áo này tên là gì? Phiên bản số mấy? Ai là người thiết kế?
- Cần nhập thêm những phụ kiện nào từ bên ngoài để hoàn thành chiếc áo?

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

Bây giờ, chúng ta cùng bóc tách các cơ chế kỹ thuật diễn ra khi một dự án Rust được tạo lập và khởi chạy trên hệ điều hành.

### 1. Bộ ba công cụ phát triển Rust

Hệ sinh thái Rust được phân chia trách nhiệm vô cùng rõ ràng:
1. **`rustup` (The Rust Toolchain Installer)**:
   - Quản lý các phiên bản của ngôn ngữ Rust trên máy tính của bạn (Stable, Beta, Nightly).
   - Giúp bạn cập nhật Rust lên phiên bản mới nhất chỉ bằng một câu lệnh: `rustup update`.
2. **`rustc` (The Rust Compiler)**:
   - Trình biên dịch mã nguồn. Nó nhận tệp văn bản `.rs` chứa chữ viết của bạn, phân tích ngữ pháp, kiểm tra quy tắc an toàn bộ nhớ và biến nó thành các chỉ thị mã máy nhị phân mà CPU hiểu được.
3. **`cargo` (The Rust Package Manager & Build Tool)**:
   - Công cụ bạn sẽ tương tác `99%` thời gian. Bạn hiếm khi phải gọi trực tiếp `rustc`. Thay vào đó, bạn ra lệnh cho `cargo`, và Cargo sẽ tự điều phối `rustc` làm việc.

### 2. Cấu trúc thư mục chuẩn của một dự án Cargo

Khi bạn gõ lệnh tạo dự án mới:
```bash
cargo new du_an_dau_tien
```
Cargo sẽ tự động kiến tạo một không gian làm việc chuẩn mực như sau:

```
du_an_dau_tien/
├── Cargo.toml          <-- Tệp cấu hình dự án (Metadata & Dependencies)
├── Cargo.lock          <-- Tệp ghi chép phiên bản chính xác của các thư viện phụ thuộc
├── .gitignore          <-- Tệp quy ước các thư mục không tải lên Git
└── src/
    └── main.rs         <-- Mã nguồn chính của chương trình
```

- **`src/main.rs`**: Điểm khởi đầu của chương trình. Mọi ứng dụng Rust dạng thực thi đều bắt đầu chạy từ hàm có tên là `fn main()`.
- **`Cargo.toml`**: Được viết theo định dạng TOML (*Tom's Obvious, Minimal Language*) — cực kỳ dễ đọc đối với con người:
  ```toml
  [package]
  name = "du_an_dau_tien"
  version = "0.1.0"
  edition = "2021"

  [dependencies]
  # Nơi khai báo các thư viện muốn tải thêm từ Internet
  ```

### 3. Vòng đời biên dịch và 4 câu lệnh Cargo thiết yếu

Khi phát triển phần mềm bằng Rust, bạn sẽ liên tục sử dụng 4 câu lệnh thần chú sau:

```
                     ┌──────────────────┐
                     │   MÃ NGUỒN .rs   │
                     └────────┬─────────┘
                              │
            ┌─────────────────┼─────────────────┐
            │ cargo check     │ cargo build     │ cargo run
            ▼                 ▼                 ▼
     [ Kiểm tra lỗi ]   [ Biên dịch ra ]  [ Biên dịch ra ]
     [ siêu tốc,    ]   [ tệp thực thi ]  [ tệp thực thi ]
     [ không tạo file]  [ trong target ]  [ và CHẠY LUÔN ]
```

1. **`cargo check` (Kiểm tra nhanh)**:
   - Chỉ kiểm tra xem mã nguồn có lỗi chính tả, sai cú pháp hay vi phạm quy tắc an toàn bộ nhớ hay không mà **không tạo ra tệp nhị phân**.
   - Tốc độ cực nhanh! Các lập trình viên thường dùng lệnh này liên tục trong lúc đang gõ code để biết mình viết đúng hay sai mà không cần chờ đợi.
2. **`cargo build` (Biên dịch đóng gói)**:
   - Tiến hành toàn bộ quá trình biên dịch và tạo ra tệp thực thi đặt trong thư mục `target/debug/`.
3. **`cargo run` (Biên dịch và Chạy ngay)**:
   - Một lệnh "hai trong một": nếu bạn vừa sửa code, nó sẽ tự động biên dịch rồi lập tức khởi chạy chương trình để bạn xem kết quả trên màn hình.
4. **`cargo build --release` (Biên dịch xuất xưởng)**:
   - Bình thường khi gõ `cargo build`, chương trình ở chế độ **Debug**: trình biên dịch giữ lại nhiều thông tin gỡ lỗi để lập trình viên dễ sửa chữa, khiến chương trình chạy chậm hơn.
   - Khi phần mềm đã hoàn thiện và muốn gửi cho người dùng cuối, bạn dùng cờ `--release`. Trình biên dịch sẽ bật toàn bộ các thuật toán tối ưu hóa cấp cao nhất (LLVM Optimizations), loại bỏ các thông tin thừa. Chương trình tạo ra trong `target/release/` thường chạy nhanh hơn bản Debug rất nhiều — điển hình từ **2 đến 20 lần**, và với những đoạn mã nặng tính toán (nhiều vòng lặp, nhiều phép toán trên `Vec`) mức chênh lệch có thể còn lớn hơn nữa. Con số cụ thể phụ thuộc vào từng chương trình, hãy tự đo bằng `cargo build --release` rồi so sánh!

### 4. Tại sao lại có dấu chấm than trong `println!`?

Nhiều người mới học thường thắc mắc: *"Tại sao in chữ ra màn hình lại viết là `println!(...)` mà không phải là `println(...)`?"*.

Trong Rust:
- `ten_ham()`: Là một **Hàm thông thường (Function)**.
- `ten_macro!()`: Là một **Khai báo vĩ mô (Macro)**.

Trong Rust, **cả hàm lẫn macro đều được kiểm tra chặt chẽ ngay lúc biên dịch** — điểm khác biệt nằm ở chỗ khác: một hàm thông thường phải có số lượng tham số cố định và kiểu dữ liệu cố định, trong khi Macro `println!` là một "cỗ máy sinh mã tự động" chạy ngay trong lúc biên dịch nên linh hoạt hơn hẳn:
- Nó cho phép bạn truyền vào số lượng tham số tùy ý (1, 2, 5 hay 10 biến đều được).
- Nó kiểm tra chặt chẽ xem số lượng dấu ngoặc nhọn `{}` có khớp chính xác với số lượng biến bạn muốn in ra hay không. Nếu lệch, trình biên dịch sẽ báo lỗi ngay từ trước khi chương trình được tạo ra, triệt tiêu hoàn toàn các lỗi sập ứng dụng ngớ ngẩn lúc đang chạy!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn hoàn chỉnh của tệp `src/main.rs`. Chương trình minh họa cách sử dụng `println!` với nhiều định dạng khác nhau và cách tổ chức một chương trình chuẩn mực:

```rust
// File: src/main.rs
// Chương trình minh họa việc in ấn thông tin và kiểm tra công cụ Rust

fn main() {
    // 1. In một dòng chữ đơn giản kèm ký tự xuống dòng tự động
    println!("Xin chào! Chào mừng bạn đến với thế giới lập trình Rust!");

    // 2. Sử dụng dấu ngoặc nhọn {} làm "vị trí giữ chỗ định dạng" (Format slot)
    let course_name = "Rust Masterclass Toàn Diện";
    let so_chuong = 12;
    println!("Bạn đang tham gia khóa học: {}", course_name);
    println!("Giai đoạn nền tảng bao gồm: {} chương chuyên sâu.", so_chuong);

    // 3. Truyền nhiều giá trị vào cùng một câu thông báo
    let learner = "Lập trình viên tương lai";
    let level_spend = "Làm chủ bộ nhớ và hệ thống";
    println!("Học viên [{}] đặt mục tiêu: [{}]", learner, level_spend);

    // 4. Các kỹ thuật định dạng văn bản nâng cao với println!
    // In số với khoảng cách căn lề cố định (rất hữu ích khi in bảng biểu dữ liệu)
    println!("------------------------------------------------------------");
    println!("| {:<15} | {:<20} | {:>10} |", "MÃ CHƯƠNG", "CHỦ ĐỀ HỌC", "TRẠNG THÁI");
    println!("------------------------------------------------------------");
    println!("| {:<15} | {:<20} | {:>10} |", "Chương 01", "Phần cứng & CPU", "Hoàn thành");
    println!("| {:<15} | {:<20} | {:>10} |", "Chương 02", "Rust & Cargo", "Đang học");
    println!("| {:<15} | {:<20} | {:>10} |", "Chương 03", "Biến & Kiểu dữ liệu", "Sắp tới");
    println!("------------------------------------------------------------");

    // 5. In biểu diễn số ở các hệ cơ số khác nhau mà không cần tính toán thủ công
    let value_mau = 255;
    println!("Con số {} trong các hệ đếm máy tính:", value_mau);
    println!("- Hệ thập phân (Decimal)     : {}", value_mau);
    println!("- Hệ nhị phân (Binary)       : {:08b}", value_mau);
    println!("- Hệ thập lục phân (Hex)     : 0x{:X}", value_mau);
    println!("- Hệ bát phân (Octal)        : 0o{:o}", value_mau);

    // 6. Thông điệp khích lệ kết thúc chương
    println!("\nChúc mừng! Bạn đã biên dịch và thực thi thành công chương trình thứ hai!");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là những lỗi phổ biến nhất mà bạn sẽ gặp phải khi bắt đầu làm quen với `cargo` và các tệp mã nguồn Rust:

| Mã lỗi / Tình huống | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0601** | `error[E0601]: 'main' function not found in crate` | Quên chưa định nghĩa hàm `fn main() { ... }` trong tệp `src/main.rs`. | Đảm bảo tệp khởi đầu có hàm `fn main()`, vì hệ điều hành cần biết chính xác điểm bắt đầu thực thi chương trình. |
| **Lỗi Cargo.toml** | `error: failed to parse manifest at Cargo.toml` | Viết sai cú pháp định dạng TOML (ví dụ: quên dấu ngoặc kép quanh tên chuỗi, hoặc thiếu dấu ngoặc vuông `[package]`). | Mở tệp `Cargo.toml`, kiểm tra lại các khóa và đảm bảo chuỗi ký tự luôn được bao quanh bởi dấu ngoặc kép `""`. |
| **Lệch vị trí giữ chỗ** | `error: 2 positional arguments in format string, but no arguments were given` | Trong chuỗi `println!` có ghi 2 cặp ngoặc `{}` nhưng đằng sau lại không truyền biến nào vào để lấp chỗ trống. | Đếm số lượng cặp ngoặc nhọn `{}` và truyền đủ số lượng biến hoặc giá trị tương ứng ở phía sau. |
| **Thiếu dấu chấm phẩy** | `error: expected ';', found ...` | Quên đặt dấu chấm phẩy `;` ở cuối câu lệnh. | Thêm dấu chấm phẩy `;` vào cuối dòng lệnh bị báo lỗi. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Bộ ba trụ cột**: `rustup` quản lý phiên bản ngôn ngữ; `cargo` điều phối toàn bộ vòng đời dự án và thư viện; `rustc` là trình biên dịch nghiêm khắc đảm bảo an toàn bộ nhớ.
2. **Cấu trúc dự án**: Luôn đặt mã nguồn vào thư mục `src/` (bắt đầu bằng `src/main.rs`) và quản lý thông tin cấu hình qua `Cargo.toml`.
3. **Quy trình làm việc chuẩn**: Sử dụng `cargo check` thường xuyên để kiểm tra lỗi cú pháp trong tích tắc; sử dụng `cargo run` khi muốn kiểm thử kết quả; dùng cờ `--release` khi muốn xuất bản ứng dụng chạy với tốc độ tối đa.
4. **Bản chất của Macro**: `println!` là một macro (nhận biết qua dấu `!`), kiểm tra tính hợp lệ của định dạng in ấn ngay trong lúc biên dịch để ngăn chặn lỗi sập phần mềm.

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Dùng công cụ dòng lệnh trên máy của bạn, thực hiện tuần tự các bước:
   - Tạo một dự án mới có tên là `so_yeu_ly_lich` bằng lệnh `cargo new so_yeu_ly_lich`.
   - Di chuyển vào thư mục đó và mở tệp `src/main.rs`.
   - Viết chương trình in ra: Họ tên của bạn, năm sinh, và mục tiêu muốn đạt được sau khi học xong ngôn ngữ Rust.
2. **Bài tập thực hành 2**: Thử nghiệm chế độ in bảng biểu: Hãy sử dụng cú pháp căn lề `{:<20}` và `{:>10}` của `println!` để in ra một hóa đơn mua sắm gồm 3 món hàng (Tên món, Số lượng, Đơn giá) thật thẳng hàng và đẹp mắt.
3. **Bài tập thử sai (Error exploration)**: Hãy thử xóa bỏ một biến ở cuối hàm `println!` nhưng vẫn giữ nguyên cặp ngoặc nhọn `{}` trong chuỗi. Chạy lệnh `cargo check` và đọc kỹ thông báo mà trình biên dịch hiển thị. Quan sát cách `rustc` vẽ mũi tên hướng dẫn bạn sửa lỗi.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Nội dung `main.rs` mới cần đúng ba dòng `println!`. Điều quan trọng là hiểu quy trình: `cargo new` dựng khung, bạn chỉ sửa `src/main.rs`, rồi `cargo run`.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

Sau khi chạy `cargo new so_yeu_ly_lich` và `cd so_yeu_ly_lich`, thay nội dung `src/main.rs` bằng:

```rust
fn main() {
    println!("Họ tên: Nguyễn Văn A");
    println!("Năm sinh: 2001");
    println!("Mục tiêu: Thành thạo Rust để viết phần mềm hệ thống an toàn và nhanh.");
}
```

Chạy bằng `cargo run` (từ trong thư mục dự án). Ba điều quy trình này dạy:
1. `cargo new` **dựng sẵn khung**: `Cargo.toml`, thư mục `src/`, một `main.rs` in "Hello, world!", và một kho git. Bạn không bao giờ phải tạo tay những thứ này.
2. Bạn **chỉ sửa `src/main.rs`** — đó là điểm khởi đầu chương trình.
3. `cargo run` gộp hai việc: biên dịch rồi chạy. Đây là vòng lặp bạn sẽ lặp lại hàng nghìn lần.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

`{:<20}` căn **trái** trong 20 ô, `{:>10}` căn **phải** trong 10 ô. Cột chữ căn trái, cột số căn phải thì bảng mới thẳng.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
fn main() {
    // {:<20} = căn trái, rộng 20 ô -> tên hàng dài ngắn khác nhau vẫn thẳng cột.
    // {:>10} = căn phải, rộng 10 ô -> số thẳng hàng đơn vị, dễ đọc.
    println!("{:<20}{:>8}{:>12}", "Tên món", "SL", "Đơn giá");
    println!("{:<20}{:>8}{:>12}", "Bàn phím cơ", 2, 850_000);
    println!("{:<20}{:>8}{:>12}", "Chuột không dây", 1, 320_000);
    println!("{:<20}{:>8}{:>12}", "Tai nghe", 3, 1_200_000);
}

#[test]
fn can_le_dung_be_rong() {
    // Mỗi dòng phải đúng 20+8+12 = 40 ký tự nhờ căn lề cố định.
    let dong = format!("{:<20}{:>8}{:>12}", "Tai nghe", 3, 1_200_000);
    assert_eq!(dong.chars().count(), 40);
}
```

Mấu chốt của bảng đẹp là **mọi dòng có cùng bề rộng cột**. Căn trái (`<`) hợp với chữ vì mắt đọc chữ từ trái sang; căn phải (`>`) hợp với số vì ta so sánh số theo hàng đơn vị — hàng nghìn phải thẳng hàng nghìn. Trộn đúng hai kiểu căn này là toàn bộ bí quyết in bảng trong terminal.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Đây là bài thử sai: cố ý để thừa `{}` rồi đọc thông báo lỗi. Bạn cần *đọc* được lỗi, không phải tránh nó.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

Đoạn mã lỗi cố ý:

```text
fn main() {
    let ten = "An";
    println!("Xin chào {} và {}", ten);   // hai {} nhưng chỉ một biến
}
```

`cargo check` báo lỗi đại ý:

```text
error: 2 positional arguments in format string, but there is 1 argument
 --> src/main.rs:3:14
```

Điều bài này dạy — và là lý do Rust được yêu thích:
- Lỗi bị bắt **lúc biên dịch, trước khi chạy**. Trong nhiều ngôn ngữ khác, `{}` thừa chỉ in ra rác hoặc nổ lúc chạy trên máy người dùng. Rust chặn ngay tại bàn của bạn.
- Thông báo **chỉ đúng chỗ**: "2 chỗ trống nhưng chỉ có 1 biến", kèm số dòng và mũi tên. `rustc` được thiết kế để *dạy* bạn sửa, không chỉ để phàn nàn.

Cách sửa: hoặc thêm biến thứ hai, hoặc bỏ bớt một `{}`. Nhưng bài tập không nằm ở việc sửa — nó nằm ở việc **tập đọc lỗi**, kỹ năng bạn dùng mỗi ngày về sau.
</details>
