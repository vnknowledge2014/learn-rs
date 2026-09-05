# Chương 12: Giao ước hành vi, Kiểu tổng quát và Tổ chức dự án (Traits, Generics, Modules & Crates)

## Giới thiệu & Mục tiêu học tập

Chúc mừng bạn đã tiến bước đến chương tổng kết của giai đoạn Nền tảng (Foundational Rust)! Cho đến lúc này, bạn đã làm chủ các khối gạch cơ bản nhất: biến số, điều khiển dòng chảy, quyền sở hữu (ownership), vay mượn (borrow), cấu trúc dữ liệu, kiểu liệt kê và cơ chế xử lý lỗi.

Tuy nhiên, khi xây dựng các ứng dụng lớn trong thực tế, bạn sẽ bắt gặp hai thách thức to lớn:
1. **Lặp lại mã nguồn**: Bạn viết một hàm in thông tin cho học sinh, sau đó lại phải viết một hàm y hệt chỉ để in thông tin cho giáo viên hoặc nhân viên công ty.
2. **Quản lý sự phức tạp**: Khi mã nguồn dài hàng nghìn dòng, làm sao để chia nhỏ dự án thành nhiều tệp ngăn nắp và bảo vệ các thông tin nội bộ không bị bên ngoài can thiệp bừa bãi?

Rust giải quyết trọn vẹn hai bài toán này thông qua bộ ba công cụ: **Kiểu tổng quát (Generics)**, **Giao ước hành vi (Traits)**, và **Hệ thống tổ chức Mô-đun (Modules & Crates)**.

Mục tiêu học tập của chương này:
- Hiểu khái niệm Kiểu tổng quát (**Generics `<T>`**) và cơ chế "Trừu tượng hóa không tốn phụ phí" (**Zero-Cost Abstraction**) thông qua kỹ thuật Đơn hình hóa (**Monomorphization**).
- Làm chủ khái niệm Giao ước hành vi (**Traits**) — công cụ định nghĩa giao diện cam kết trong Rust.
- Sử dụng thành thạo Ràng buộc Trait (**Trait Bounds**) và cú pháp mệnh đề lọc **`where`**.
- Nhận biết các Trait tiêu chuẩn cốt lõi: `Display`, `Debug`, `Clone`, và `Default`.
- Nắm vững kỹ thuật tổ chức mã nguồn bằng Mô-đun (**`mod`**), phân quyền truy cập thông tin bằng **`pub`**, và liên kết không gian tên bằng **`use`**.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng làm quen với các khái niệm thiết kế kiến trúc này qua 3 hình ảnh đời thường:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│                 HÌNH TƯỢNG ĐỜI SỐNG VỀ GENERICS, TRAITS VÀ MODULES               │
├─────────────────────────┬───────────────────────────────┬────────────────────────┤
│    KHUÔN BÁNH HOA MAI   │       CHUẨN DÂY SẠC TYPE-C    │     TỦ HỒ SƠ NHIỀU TẦNG│
│         (Generics <T>)  │                (Traits)       │       (Modules & pub)  │
│                         │                               │                        │
│ - Chiếc khuôn nhôm đúc  │ - Cắm vừa điện thoại Samsung, │ - Mỗi phòng ban 1 ngăn │
│ - Đổ bột socola, dâu tây│   laptop Dell, tai nghe Sony  │ - Tầng dán nhãn 'pub'  │
│   hay trà xanh vào khuôn│ - Miễn là thiết bị cam kết    │   thì cả công ty được  │
│ - Ra lò những chiếc bánh│   tuân thủ chuẩn chân Type-C  │   mở xem tài liệu      │
│   chuẩn hoa mai mà không│ - Sợi cáp sạc có thể dùng     │ - Tầng không nhãn: khóa│
│   cần đúc 3 cái khuôn!  │   chung cho tất cả mọi máy!   │   riêng tư nội bộ      │
└─────────────────────────┴───────────────────────────────┴────────────────────────┘
```

### 1. Chiếc khuôn nướng bánh hình hoa mai (Kiểu tổng quát - `Generics <T>`)
Hãy tưởng tượng bạn có một chiếc khuôn đúc bánh bằng kim loại sáng bóng:
- Bạn có thể đổ bột socola vào khuôn -> Bạn nướng được chiếc bánh hoa mai vị socola.
- Bạn đổ bột trà xanh vào khuôn -> Bạn nướng được chiếc bánh hoa mai vị trà xanh.
- Bạn đổ bột phô mai vào khuôn -> Bạn nướng được chiếc bánh hoa mai vị phô mai.
Chiếc khuôn bánh chính là **Generics `<T>`** (với `T` là loại bột). Bạn chỉ cần chế tạo chiếc khuôn một lần duy nhất, và chiếc khuôn đó có thể tạo ra hàng trăm loại bánh khác nhau mà bạn không cần phải nhọc công đúc riêng từng chiếc khuôn cho từng loại bột!

### 2. Chuẩn chân cắm sạc USB Type-C (Giao ước hành vi - Trait)
Hãy quan sát chuẩn kết nối Type-C hiện đại:
- Bất kể đó là chiếc điện thoại thông minh, máy tính xách tay bảng mạch lớn, hay chiếc tai nghe không dây tí hon:
- Chỉ cần nhà sản xuất cam kết thiết bị của họ có cổng Type-C (`impl ChuanTypeC for ThietBi`), bạn đều có thể cắm chung một sợi dây sạc duy nhất để truyền điện và truyền dữ liệu.
- Trong Rust, **Trait giống như một bản hợp đồng cam kết**: Nếu một kiểu dữ liệu đồng ý ký vào bản hợp đồng đó, nó phải thực hiện đúng những hành vi đã hứa.

### 3. Tủ tài liệu công ty nhiều tầng có khóa bảo mật (Modules và `pub`)
Trong một văn phòng công ty lớn:
- Phòng kế toán có tủ hồ sơ riêng (`mod phong_ke_toan`), Phòng nhân sự có tủ hồ sơ riêng (`mod phong_nhan_su`).
- **Quy tắc riêng tư mặc định (Private)**: Theo mặc định, ngăn kéo bàn làm việc của ai thì chỉ người đó có chìa khóa mở. Người ngoài bước vào phòng không được tự ý lục lọi.
- **Quy tắc công khai (`pub`)**: Khi trưởng phòng dán một thông báo lên bảng tin có chữ "Công khai" (`pub`), mọi nhân viên trong toàn công ty đều có quyền đọc thông tin đó.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Bí mật tốc độ của Rust: Cơ chế Đơn hình hóa (Monomorphization)

Nhiều người e ngại rằng việc sử dụng Generics sẽ làm chậm chương trình lúc chạy. Nhưng trong Rust, Generics là một tính năng **Trừu tượng hóa không tốn phụ phí (Zero-Cost Abstraction)**!

Làm thế nào Rust làm được điều kỳ diệu này?
Khi bạn viết một hàm generic:
```rust
fn in_du_lieu<T: std::fmt::Display>(gia_tri: T) {
    println!("{}", gia_tri);
}
```
Khi bạn gọi `in_du_lieu(100)` (số nguyên) và `in_du_lieu("Chào")` (chuỗi ký tự), trong quá trình biên dịch, trình biên dịch `rustc` sẽ tự động thực hiện quy trình **Đơn hình hóa (Monomorphization)**:
- Nó tự động sinh ra hai hàm mã máy độc lập:
  - Một hàm tối ưu chuyên biệt cho kiểu số nguyên `i32`.
  - Một hàm tối ưu chuyên biệt cho kiểu chuỗi `&str`.
- Khi chương trình chạy trên CPU, nó thực thi trực tiếp mã máy tối ưu hóa cao nhất, **không có bất kỳ phép kiểm tra hay tra cứu kiểu nào bị trì hoãn lúc chạy**!

### 2. Định nghĩa Trait và Phương thức mặc định (Default Implementations)

Một Trait định nghĩa một tập hợp các phương thức mà các kiểu dữ liệu khác phải cài đặt:
```rust
trait ThietBiBaoDong {
    // Phương thức bắt buộc phải tự cài đặt
    fn ma_thiet_bi(&self) -> &str;

    // Phương thức có sẵn mặc định: Các struct có thể dùng ngay hoặc ghi đè (override)
    fn phat_canh_bao(&self) {
        println!("[CÒI BÁO ĐỘNG] Reng reng! Thiết bị {} phát tín hiệu nguy hiểm!", self.ma_thiet_bi());
    }
}
```

### 3. Ràng buộc Trait (Trait Bounds) và Cú pháp mệnh đề `where`

Khi viết hàm generic, bạn có thể yêu cầu: "Kiểu `T` phải là một kiểu biết tự in ấn (`Display`) và biết tự nhân bản (`Clone`)":
- **Cú pháp ngắn gọn**:
  ```rust
  fn thong_bao(item: &(impl Display + Clone)) { ... }
  ```
- **Cú pháp đầy đủ**:
  ```rust
  fn thong_bao<T: Display + Clone>(item: &T) { ... }
  ```
- **Cú pháp mệnh đề `where` (khi có nhiều kiểu phức tạp)**:
  ```rust
  fn so_sanh_he_thong<T, U>(thiet_bi_a: &T, thiet_bi_b: &U) -> bool
  where
      T: Display + Clone,
      U: Debug + PartialEq,
  {
      // Thân hàm thoáng đãng và cực kỳ dễ đọc
  }
  ```

### 4. Hệ thống Crate và Cây Mô-đun (Module Tree)

- **Crate**: Đơn vị biên dịch nhỏ nhất của Rust. Gồm 2 loại:
  1. *Binary Crate (Crate nhị phân)*: Có tệp `src/main.rs`, biên dịch thành tệp chạy được.
  2. *Library Crate (Crate thư viện)*: Có tệp `src/lib.rs`, chứa các hàm dùng chung để chia sẻ cho các dự án khác.
- **Module (`mod`)**: Chia nhỏ mã nguồn theo không gian tên bên trong cùng một Crate.
- **Quy tắc truy cập**: Mọi thứ bên trong module mặc định là riêng tư (`private`). Muốn hàm hoặc struct dùng được từ module cha hoặc bên ngoài, bắt buộc phải thêm từ khóa `pub`.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Chương trình hoàn chỉnh dưới đây minh họa một hệ thống kiểm soát Tự động hóa Tòa nhà Thông minh (Smart Building), tích hợp Trait có phương thức mặc định, hàm Generics có Trait Bounds với mệnh đề `where`, và cách phân chia các mô-đun với `pub` và `use`:

```rust
// File: src/main.rs
// Chương trình thực chiến làm chủ Generics, Traits & Tổ chức Mô-đun trong Rust

// ============================================================================
// MÔ-ĐUN 1: CÁC GIAO ƯỚC VÀ THIẾT BỊ PHẦN CỨNG
// ============================================================================
mod thiet_bi_thong_minh {
    use std::fmt::Display;

    // 1. Định nghĩa Trait giao ước cho mọi cảm biến trong tòa nhà
    pub trait CamBien: Display {
        // Phương thức bắt buộc mọi cảm biến phải tự hiện thực
        fn doc_gia_tri(&self) -> f64;
        fn don_vi_do(&self) -> &str;

        // Phương thức mặc định (Default implementation): Dùng chung cho tất cả cảm biến
        fn kiem_tra_tinh_trang(&self) {
            println!("-> Cảm biến [{}] đang hoạt động bình thường.", self);
        }
    }

    // 2. Struct Cảm biến Nhiệt độ phòng
    pub struct CamBienNhietDo {
        pub vi_tri: String,
        pub do_c: f64,
    }

    // Cài đặt Display cho CamBienNhietDo (thỏa mãn điều kiện CamBien: Display)
    impl Display for CamBienNhietDo {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "Cảm biến Nhiệt độ tại {}", self.vi_tri)
        }
    }

    // Triển khai Trait CamBien cho CamBienNhietDo
    impl CamBien for CamBienNhietDo {
        fn doc_gia_tri(&self) -> f64 { self.do_c }
        fn don_vi_do(&self) -> &str { "°C" }
    }

    // 3. Struct Cảm biến Khói báo cháy
    pub struct CamBienKhoi {
        pub khu_vuc: String,
        pub mat_do_khoi_ppm: f64,
    }

    impl Display for CamBienKhoi {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(f, "Cảm biến Khói tại {}", self.khu_vuc)
        }
    }

    impl CamBien for CamBienKhoi {
        fn doc_gia_tri(&self) -> f64 { self.mat_do_khoi_ppm }
        fn don_vi_do(&self) -> &str { "PPM" }
    }
}

// ============================================================================
// MÔ-ĐUN 2: TRUNG TÂM GIÁM SÁT TỔNG HỢP VÀ HÀM GENERICS
// ============================================================================
mod trung_tam_dieu_khien {
    use super::thiet_bi_thong_minh::CamBien;

    // Hàm Generics nhận bất kỳ cảm biến nào tuân thủ Trait CamBien
    // Sử dụng mệnh đề 'where' để cấu trúc mã sạch đẹp và chuyên nghiệp
    pub fn giam_sat_thong_so<T>(cam_bien: &T, nguong_canh_bao: f64)
    where
        T: CamBien,
    {
        println!("------------------------------------------------------------");
        // Gọi phương thức mặc định của Trait
        cam_bien.kiem_tra_tinh_trang();

        let gia_tri = cam_bien.doc_gia_tri();
        let don_vi = cam_bien.don_vi_do();

        println!("Chỉ số đo được : {:.2} {}", gia_tri, don_vi);

        if gia_tri >= nguong_canh_bao {
            println!("[CẢNH BÁO NGUY HIỂM] Chỉ số vượt ngưỡng an toàn ({:.2} {})!", 
                     nguong_canh_bao, don_vi);
        } else {
            println!("[AN TOÀN] Chỉ số nằm trong giới hạn cho phép.");
        }
    }
}

// Sử dụng lệnh 'use' để đưa các thành phần cần thiết vào phạm vi làm việc
use thiet_bi_thong_minh::{CamBienNhietDo, CamBienKhoi};
use trung_tam_dieu_khien::giam_sat_thong_so;

fn main() {
    println!("============================================================");
    println!("     HỆ THỐNG ĐIỀU HÀNH TỰ ĐỘNG HÓA TÒA NHÀ THÔNG MINH      ");
    println!("============================================================");

    // Khởi tạo cảm biến nhiệt độ phòng máy chủ
    let cb_nhiet = CamBienNhietDo {
        vi_tri: String::from("Phòng Máy Chủ Tầng 5"),
        do_c: 28.5,
    };

    // Khởi tạo cảm biến khói khu nhà bếp
    let cb_khoi = CamBienKhoi {
        khu_vuc: String::from("Khu Bếp Nhà Hàng Tầng 1"),
        mat_do_khoi_ppm: 65.0,
    };

    // Cùng một hàm giam_sat_thong_so nhưng nhận hai kiểu dữ liệu khác nhau!
    // Trình biên dịch Rust áp dụng Monomorphization tối ưu hóa mã máy hoàn hảo:
    println!("\n1. Giám sát hệ thống cảm biến nhiệt độ:");
    giam_sat_thong_so(&cb_nhiet, 35.0); // Ngưỡng cảnh báo nhiệt độ là 35°C

    println!("\n2. Giám sát hệ thống cảm biến khói báo cháy:");
    giam_sat_thong_so(&cb_khoi, 50.0);  // Ngưỡng cảnh báo mật độ khói là 50 PPM

    println!("\n============================================================");
    println!("   CHÚC MỪNG BẠN ĐÃ HOÀN THÀNH TOÀN BỘ 12 CHƯƠNG NỀN TẢNG!  ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi thường gặp nhất khi làm việc với Generics, Traits và Modules trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `the trait bound 'T: Display' is not satisfied` | Bạn cố dùng `{}` để in một biến có kiểu generic `T` nhưng chưa thêm ràng buộc `T: Display` vào định nghĩa hàm. | Thêm Trait Bound cho kiểu generic: `<T: std::fmt::Display>` hoặc thêm vào mệnh đề `where`. |
| **E0603** | `struct/function '...' is private` | Bạn cố truy cập một hàm, struct hoặc trường dữ liệu nằm trong một module khác nhưng nó chưa được đánh dấu từ khóa `pub`. | Thêm từ khóa `pub` vào phía trước struct, hàm hoặc trường dữ liệu trong module đó (`pub fn ...`, `pub struct ...`). |
| **E0432** | `unresolved import 'super::...'` | Đường dẫn trong câu lệnh `use` bị sai lệch cấp bậc thư mục hoặc tên module không tồn tại. | Kiểm tra lại cây thư mục module, dùng `crate::` cho đường dẫn từ gốc hoặc `super::` để lùi ra một cấp cha. |
| **E0046** | `not all trait items implemented, missing: '...'` | Bạn viết `impl Trait for Type` nhưng quên chưa viết mã cho các phương thức bắt buộc của Trait đó. | Kiểm tra định nghĩa của Trait và triển khai đầy đủ tất cả các phương thức còn thiếu. |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Generics `<T>` & Monomorphization**: Giúp tái sử dụng mã nguồn cho mọi kiểu dữ liệu; trình biên dịch tự sinh mã máy chuyên biệt lúc biên dịch, đem lại hiệu năng tối đa mà không tốn phụ phí lúc chạy (Zero-Cost Abstraction).
2. **Traits**: Bản hợp đồng giao ước hành vi chung; cho phép gắn kết các phương thức bắt buộc và phương thức mặc định vào nhiều kiểu dữ liệu khác nhau.
3. **Mệnh đề `where`**: Giúp khai báo các ràng buộc Trait phức tạp một cách trong sáng, ngăn nắp và dễ bảo trì.
4. **Kiểm soát phạm vi với `mod` & `pub`**: Mặc định mọi thứ là riêng tư (Private) để bảo mật thông tin nội bộ; chỉ sử dụng `pub` cho những giao diện thực sự cần công khai ra bên ngoài.

### Bài tập rèn luyện tự giải:
1. **Bài tập thực hành 1**: Định nghĩa một Trait mang tên `CoDienTich` có một phương thức `fn tinh_dien_tich(&self) -> f64;`. Hãy triển khai Trait này cho hai struct: `HinhTron { ban_kinh: f64 }` và `HinhVuong { canh: f64 }`. Sau đó viết một hàm generic `in_dien_tich<T: CoDienTich>(hinh: &T)` để in diện tích của cả hai hình.
2. **Bài tập tư duy 2**: Cơ chế Monomorphization của Rust mang lại tốc độ thực thi tuyệt đỉnh, nhưng nó có thể dẫn đến nhược điểm gì về kích thước tệp thực thi nhị phân (Binary Size) và thời gian biên dịch nếu có quá nhiều kiểu dữ liệu cùng dùng chung một hàm generic đồ sộ?
3. **Bài tập tổ chức mô-đun 3**: Hãy tổ chức một dự án nhỏ gồm 2 module: `mod quan_ly_kho` (chứa struct `HangHoa` có trường `ten` và `gia` được đánh dấu `pub`) và `mod ban_hang` (chứa hàm `xuat_hoa_don`). Thực hành sử dụng từ khóa `pub` và `use` để hai module tương tác trơn tru với nhau trong hàm `main`.
