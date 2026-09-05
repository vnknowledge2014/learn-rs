# Chương 23: Danh sách liên kết & Con trỏ thông minh: Box, Rc, RefCell (Linked Lists & Smart Pointers: Box, Rc, RefCell)

## Giới thiệu & Mục tiêu học tập

Trong các ngôn ngữ lập trình truyền thống như C hoặc C++, **Danh sách liên kết (Linked List)** thường là bài tập vỡ lòng đầu tiên mà sinh viên được học sau mảng (array). Nhưng trong thế giới Rust, danh sách liên kết lại được mệnh danh là một trong những "cơn ác mộng" kinh điển nhất đối với người mới bắt đầu! Thậm chí cộng đồng lập trình viên Rust quốc tế còn viết hẳn một cuốn sách nổi tiếng mang tên *"Learn Rust With Entirely Too Many Linked Lists"* chỉ để mổ xẻ cấu trúc dữ liệu này.

Tại sao một cấu trúc dữ liệu cơ bản như vậy lại trở thành thách thức lớn trong Rust? Câu trả lời nằm ở ba trụ cột cốt lõi của ngôn ngữ: **Quy tắc quyền sở hữu (ownership)**, **quy tắc vay mượn (borrow)**, và **thời gian sống (lifetime)**. Trong khi C/C++ cho phép các con trỏ trỏ tự do, chéo nhau và dễ dàng tạo ra các lỗ hổng bảo mật chết người (như con trỏ lơ lửng Dangling Pointer, rò rỉ bộ nhớ Memory Leak, hay giải phóng hai lần Double Free), thì trình kiểm tra mượn (Borrow Checker) của Rust giám sát chặt chẽ từng mối liên kết.

Để giải quyết bài toán này và làm chủ các cấu trúc dữ liệu liên kết động, chúng ta cần sự trợ giúp của bộ ba **con trỏ thông minh (smart pointer)** mạnh mẽ: `Box<T>`, `Rc<T>`, và `RefCell<T>`.

Mục tiêu học tập của chương này:
- Hiểu sâu sắc lý do tại sao cấu trúc tự tham chiếu đệ quy lại có kích thước vô tận (infinite size) và cách con trỏ thông minh (smart pointer) `Box<T>` ấn định kích thước xác định trên Ngăn xếp (Stack).
- Giải mã "nghịch lý" của trình kiểm tra mượn khi quản lý quyền sở hữu (ownership) giữa các nút trong danh sách liên kết.
- Phân biệt vai trò của `Box<T>` (sở hữu độc quyền), `Rc<T>` (đồng sở hữu nhiều người), và `RefCell<T>` (biến đổi nội tại lúc chạy - Interior Mutability).
- Tự tay cài đặt một cấu trúc Danh sách liên kết đơn (Singly Linked List) hoàn chỉnh bằng Safe Rust 100%.
- Nắm vững kỹ thuật viết hàm hủy bộ nhớ `impl Drop` an toàn chống tràn ngăn xếp (Stack Overflow) khi danh sách chứa hàng triệu phần tử.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Hãy cùng hình dung danh sách liên kết và bộ ba con trỏ thông minh qua hai trò chơi đời thực vô cùng gần gũi:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG HÓA DANH SÁCH LIÊN KẾT & CON TRỎ THÔNG MINH               │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [DANH SÁCH LIÊN KẾT: TRÒ CHƠI TRUY TÌM KHO BÁU THEO MẬT THƯ]                     │
│ ┌──────────────┐      ┌──────────────┐      ┌──────────────┐                     │
│ │ Phong bì 1   │      │ Phong bì 2   │      │ Phong bì 3   │      ┌────────┐     │
│ │ [Vàng: 10]   │────► │ [Bạc: 20]    │────► │ [Ngọc: 30]   │────► │ None   │     │
│ │ Chỉ dẫn -> #2│      │ Chỉ dẫn -> #3│      │ Chỉ dẫn: HẾT │      │ (Hết)  │     │
│ └──────────────┘      └──────────────┘      └──────────────┘      └────────┘     │
│                                                                                  │
│ [BỘ BA CON TRỎ THÔNG MINH (SMART POINTERS)]                                      │
│                                                                                  │
│ 1. Box<T> (Hộp khóa độc quyền chính chủ):                                        │
│    - Một chiếc hộp chỉ trao chìa khóa duy nhất cho một chủ nhân.                 │
│    - Khi chủ nhân qua đời, chiếc hộp tự động bị tiêu hủy theo.                   │
│                                                                                  │
│ 2. Rc<T> (Căn hộ chung cư đồng sở hữu - Reference Counting):                     │
│    - 3 người bạn cùng thuê chung 1 căn hộ, mỗi người giữ 1 chìa khóa.            │
│    - Cửa ra vào lắp cảm biến đếm số chìa khóa: Đếm = 3.                          │
│    - Khi 2 người trả phòng: Đếm = 1 (Nhà chưa khóa).                             │
│    - Chỉ khi người cuối cùng trả phòng: Đếm = 0 -> Ban quản lý mới dọn dẹp phòng!│
│                                                                                  │
│ 3. RefCell<T> (Bác bảo vệ trực trước cửa phòng):                                 │
│    - Không bắt bạn xin giấy phép trước từ hôm qua (thời điểm biên dịch).         │
│    - Bác đứng gác cửa: Ai vào sửa đồ thì chỉ được vào MỘT MÌNH.                  │
│    - Nếu có 2 người cùng chen vào sửa một lúc -> Bác bảo vệ tuýt còi báo động!   │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Danh sách liên kết — Trò chơi truy tìm kho báu theo phong bì mật thư
- Khác với Mảng (Array) hay Vector (nơi tất cả các ngăn tủ nằm san sát nhau trên một dãy), trong trò chơi mật thư:
  - **Phong bì 1** được giấu dưới gốc cây đa, bên trong chứa 10 đồng vàng và một mảnh giấy ghi: *"Hãy đến chân cầu thang để tìm phong bì số 2"*.
  - **Phong bì 2** giấu dưới chân cầu thang, chứa 20 đồng bạc và chỉ dẫn đến chiếc ghế đá công viên.
  - **Phong bì 3** chứa ngọc quý và ghi chữ *"HẾT"* (`None`).
- **Ưu điểm**: Muốn giấu thêm một phong bì mới vào đầu hành trình, bạn chỉ cần viết một phong bì mới chỉ về cây đa, hoàn toàn không cần đào xới hay di dời bất kỳ phong bì cũ nào ($O(1)$ thêm phần tử).
- **Nhược điểm**: Muốn lấy kho báu ở phong bì số 100, bạn không thể nhảy dù đến ngay được! Bạn bắt buộc phải lần mò qua 99 phong bì trước đó ($O(N)$ truy cập ngẫu nhiên).

### 2. Nghịch lý của Rust với Danh sách liên kết
Trong các ngôn ngữ như C, lập trình viên có thể cho phong bì 2 trỏ ngược lại phong bì 1 (liên kết đôi). Nhưng trong Rust:
- **Phong bì 1 sở hữu phong bì 2**.
- Nếu phong bì 2 lại sở hữu ngược lại phong bì 1, ai thực sự là chủ nhân của ai?
- Trình kiểm tra mượn (Borrow Checker) sẽ lập tức ngăn chặn vì vi phạm nguyên tắc sở hữu duy nhất. Đó là lý do tại sao chúng ta cần `Rc` (chia sẻ sở hữu) và `RefCell` (mượn linh hoạt lúc chạy) khi xây dựng các cấu trúc liên kết phức tạp.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Tại sao kiểu dữ liệu đệ quy cần `Box<T>`?

Hãy xem xét đoạn mã lỗi minh họa sau đây:
```rust,compile_fail
// compile-fail
// Giả định định nghĩa một Node danh sách liên kết
struct NutLoi {
    gia_tri: i32,
    ke_tiep: Option<NutLoi>, // LỖI BIÊN DỊCH E0072!
}
```
Khi trình biên dịch `rustc` tính toán kích thước vật lý của `NutLoi` trên Ngăn xếp (Stack):
- `NutLoi` chứa một `i32` (4 bytes) + một `Option<NutLoi>`.
- Nhưng `NutLoi` bên trong lại chứa một `NutLoi` con, `NutLoi` con lại chứa `NutLoi` cháu...
- Kích thước của `NutLoi` sẽ là: $4 + 4 + 4 + ... = \infty$ (Vô tận!). Trình biên dịch không thể cấp phát bộ nhớ cho một thứ không rõ kích thước.

**Giải pháp với `Box<T>`**:
```rust
struct NutChuan<T> {
    gia_tri: T,
    ke_tiep: Option<Box<NutChuan<T>>>, // Hợp lệ 100%!
}
```
Bản thân `Box<T>` là một con trỏ thông minh (smart pointer). Kích thước của `Box` trên Stack luôn luôn cố định là **8 bytes** (kích thước một địa chỉ ô nhớ trên hệ điều hành 64-bit), dù dữ liệu thực tế nó trỏ tới trên Heap lớn đến đâu. Chuỗi đệ quy vô hạn đã bị chặn đứng!

### 2. Cơ chế bóc tách của bộ ba Smart Pointers

| Con trỏ thông minh | Vị trí dữ liệu | Cơ chế sở hữu | Tính khả biến (Mutation) | Đa luồng (Thread-Safe)? |
|---|---|---|---|---|
| **`Box<T>`** | Heap | Độc quyền (Single Owner) | Thừa hưởng từ biến chứa nó | Có thể chuyển luồng (`Send`) |
| **`Rc<T>`** | Heap | Đồng sở hữu (Reference Counted) | Bất biến (Immutable) | Không (Dùng đơn luồng, đa luồng dùng `Arc`) |
| **`RefCell<T>`** | Bất kỳ | Theo biến bao bọc | Cho phép sửa đổi nội tại (Interior Mutability) | Không (Đa luồng dùng `Mutex`/`RwLock`) |

> **Quy tắc vàng**: 
> - Muốn chia sẻ 1 dữ liệu cho nhiều nơi đọc: Dùng `Rc<T>`.
> - Muốn chia sẻ 1 dữ liệu cho nhiều nơi và CÓ THỂ SỬA ĐỔI: Phối hợp `Rc<RefCell<T>>`.

### 3. Nguy cơ tràn ngăn xếp (Stack Overflow) khi tự động hủy `Drop`

Khi một danh sách liên kết ra khỏi phạm vi sống, Rust sẽ tự động gọi hàm hủy `Drop`. Mặc định, trình biên dịch sinh mã hủy theo kiểu đệ quy:
```
Hủy Nút 1 -> Hủy Box Nút 2 -> Hủy Box Nút 3 -> ... -> Hủy Nút N
```
Mỗi lần gọi hủy đệ quy, CPU phải tạo thêm một khung ngăn xếp (stack frame) mới trên Stack. Nếu danh sách của bạn chứa **1.000.000 phần tử**, ngăn xếp Stack (vốn chỉ có dung lượng vài Megabytes) sẽ lập tức bị cạn kiệt, dẫn đến lỗi sụp đổ chương trình (Crash do Stack Overflow)!

Do đó, một danh sách liên kết sản xuất chuyên nghiệp trong Rust **bắt buộc phải tự cài đặt `impl<T> Drop`** sử dụng vòng lặp `while let Some(...)` để tháo gỡ từng nút một cách tuần tự trên Heap, giữ độ phức tạp không gian ngăn xếp ở mức $O(1)$.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là bản thiết kế hoàn chỉnh của một Danh sách liên kết đơn (Singly Linked List) an toàn 100%, hỗ trợ thêm/xóa phần tử ở đầu danh sách với thời gian $O(1)$, kiểm tra dữ liệu mượn (borrow), và cài đặt hàm `Drop` an toàn chống tràn ngăn xếp:

```rust
/// Cấu trúc nút bên trong danh sách liên kết
struct Nut<T> {
    gia_tri: T,
    ke_tiep: Option<Box<Nut<T>>>,
}

/// Cấu trúc Danh sách liên kết đơn (Singly Linked List)
pub struct DanhSachLienKet<T> {
    dinh: Option<Box<Nut<T>>>,
    do_dai: usize,
}

impl<T> DanhSachLienKet<T> {
    /// Khởi tạo một danh sách liên kết rỗng
    pub fn new() -> Self {
        DanhSachLienKet {
            dinh: None,
            do_dai: 0,
        }
    }

    /// Thêm một phần tử mới vào đầu danh sách - Độ phức tạp O(1)
    pub fn push_dau(&mut self, gia_tri: T) {
        // Tạo nút mới trên Heap thông qua con trỏ thông minh Box
        // Sử dụng self.dinh.take() để lấy quyền sở hữu đỉnh cũ mà không vi phạm quy tắc mượn
        let nut_moi = Box::new(Nut {
            gia_tri,
            ke_tiep: self.dinh.take(),
        });

        // Gán đỉnh mới cho danh sách
        self.dinh = Some(nut_moi);
        self.do_dai += 1;
    }

    /// Lấy phần tử ở đầu danh sách ra và trả về giá trị - Độ phức tạp O(1)
    pub fn pop_dau(&mut self) -> Option<T> {
        // .take() thay thế đỉnh bằng None và trả về Some(nut_cu)
        self.dinh.take().map(|nut_cu| {
            // Đưa nút kế tiếp lên làm đỉnh mới
            self.dinh = nut_cu.ke_tiep;
            self.do_dai -= 1;
            // Trả về giá trị của nút vừa lấy ra
            nut_cu.gia_tri
        })
    }

    /// Xem giá trị phần tử ở đầu danh sách mà không đoạt quyền sở hữu - Trả về tham chiếu mượn
    pub fn peek_dau(&self) -> Option<&T> {
        self.dinh.as_ref().map(|nut| &nut.gia_tri)
    }

    /// Kiểm tra số lượng phần tử hiện tại trong danh sách
    pub fn len(&self) -> usize {
        self.do_dai
    }

    /// Kiểm tra danh sách có đang rỗng hay không
    pub fn is_empty(&self) -> bool {
        self.do_dai == 0
    }
}

/// Cài đặt hàm hủy bộ nhớ an toàn (Safe Drop)
/// Sử dụng vòng lặp tuần tự thay vì đệ quy để triệt tiêu nguy cơ tràn ngăn xếp (Stack Overflow)
impl<T> Drop for DanhSachLienKet<T> {
    fn drop(&mut self) {
        let mut nut_hien_tai = self.dinh.take();
        // Lặp tuần tự gỡ từng Box trên Heap đưa vào biến cục bộ rồi giải phóng
        while let Some(mut nut) = nut_hien_tai {
            nut_hien_tai = nut.ke_tiep.take();
            // nut tự động được giải phóng tại đây mà không cần gọi đệ quy sâu!
        }
    }
}

// Cài đặt Default trait chuẩn phong cách Rust
impl<T> Default for DanhSachLienKet<T> {
    fn default() -> Self {
        Self::new()
    }
}

fn main() {
    println!("============================================================");
    println!("     HIỆN THỰC DANH SÁCH LIÊN KẾT & SMART POINTERS TRONG RUST");
    println!("============================================================");

    let mut danh_sach: DanhSachLienKet<i32> = DanhSachLienKet::new();
    println!("Khởi tạo danh sách rỗng: len = {}", danh_sach.len());
    assert!(danh_sach.is_empty());

    // 1. Thao tác thêm vào đầu danh sách (Push)
    println!("\n[1] Thêm các phần tử vào đầu danh sách:");
    danh_sach.push_dau(10);
    println!("    - Đã thêm 10. Đỉnh hiện tại: {:?}", danh_sach.peek_dau());
    danh_sach.push_dau(20);
    println!("    - Đã thêm 20. Đỉnh hiện tại: {:?}", danh_sach.peek_dau());
    danh_sach.push_dau(30);
    println!("    - Đã thêm 30. Đỉnh hiện tại: {:?}", danh_sach.peek_dau());
    
    println!("    => Tổng số phần tử: {}", danh_sach.len());
    assert_eq!(danh_sach.len(), 3);
    assert_eq!(danh_sach.peek_dau(), Some(&30));

    // 2. Thao tác lấy phần tử ra khỏi danh sách (Pop)
    println!("\n[2] Lấy các phần tử ra lần lượt (LIFO):");
    let p1 = danh_sach.pop_dau();
    println!("    - Lấy ra lần 1: {:?} (Kỳ vọng: Some(30))", p1);
    assert_eq!(p1, Some(30));

    let p2 = danh_sach.pop_dau();
    println!("    - Lấy ra lần 2: {:?} (Kỳ vọng: Some(20))", p2);
    assert_eq!(p2, Some(20));

    let p3 = danh_sach.pop_dau();
    println!("    - Lấy ra lần 3: {:?} (Kỳ vọng: Some(10))", p3);
    assert_eq!(p3, Some(10));

    let p4 = danh_sach.pop_dau();
    println!("    - Lấy ra khi danh sách rỗng: {:?} (Kỳ vọng: None)", p4);
    assert_eq!(p4, None);
    assert!(danh_sach.is_empty());

    // 3. Kiểm thử khả năng chịu tải chống tràn ngăn xếp (Drop 100.000 phần tử)
    println!("\n[3] Kiểm thử độ bền của hàm hủy Drop an toàn:");
    {
        let mut danh_sach_lon = DanhSachLienKet::new();
        for i in 0..100_000 {
            danh_sach_lon.push_dau(i);
        }
        println!("    - Đã nạp thành công 100.000 phần tử vào danh sách liên kết.");
        println!("    - Bắt đầu giải phóng bộ nhớ khi ra khỏi khối ngoặc nhọn...");
    } // danh_sach_lon bị Drop tại đây. Nhờ vòng lặp tuần tự, không bị tràn Stack!
    println!("    => Giải phóng 100.000 nút bộ nhớ thành công tuyệt đối!");

    println!("============================================================");
    println!("               HOÀN TẤT THỰC NGHIỆM CHƯƠNG 23               ");
    println!("============================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Khi thiết kế danh sách liên kết và sử dụng con trỏ thông minh (smart pointer), người học thường gặp phải các thông báo lỗi đặc thù sau:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0072** | `recursive type '...' has infinite size` | Bạn định nghĩa một struct tự chứa chính nó mà không qua một lớp bọc con trỏ (`ke_tiep: Option<Nut>`). Trình biên dịch không thể tính toán kích thước cố định. | Bao bọc trường đệ quy bằng con trỏ thông minh `Box<T>`: `ke_tiep: Option<Box<Nut<T>>>`. |
| **E0507** | `cannot move out of '...' which is behind a shared reference` | Bạn cố lấy quyền sở hữu một nút bằng cách gán `self.dinh` trong khi hàm chỉ có tham chiếu mượn `&mut self`. | Sử dụng phương thức `.take()` của kiểu `Option` để nhấc giá trị ra an toàn và để lại giá trị `None`. |
| **E0599** | `no method named '...' found for struct 'Box<...>'` | Bạn tưởng rằng phải giải phóng con trỏ thủ công như `free()` trong C. Trong Rust, `Box` tự động giải phóng khi ra khỏi phạm vi sống. | Không cần gọi hàm giải phóng thủ công; tận dụng cơ chế RAII tự động của Rust. |
| **E0506** | `cannot assign to '...' because it is borrowed` | Bạn đang giữ tham chiếu đọc `peek_dau()` nhưng lại cố gọi hàm ghi `push_dau()` làm thay đổi cấu trúc danh sách. | Tách rời phạm vi mượn đọc trước khi thực hiện hành động sửa đổi. |

### Ví dụ phân tích lỗi `E0507` và phương pháp khắc phục với `.take()`:

```rust
struct NutMinhHoa {
    gia_tri: i32,
    ke_tiep: Option<Box<NutMinhHoa>>,
}

// Đoạn mã lỗi minh họa E0507: Cố đoạt quyền sở hữu từ tham chiếu mượn
fn lay_dinh_loi(dinh: &mut Option<Box<NutMinhHoa>>) {
    // let nut_cu = *dinh; // LỖI E0507: cannot move out of `*dinh`!
}

// Cách sửa chữa đúng chuẩn: Sử dụng Option::take()
fn lay_dinh_dung(dinh: &mut Option<Box<NutMinhHoa>>) {
    // .take() sẽ lấy Some(box) ra và gán lại None vào vị trí cũ một cách an toàn
    let nut_cu = dinh.take();
    if let Some(nut) = nut_cu {
        println!("Đã lấy được nút ra an toàn: {}", nut.gia_tri);
    }
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Kiểu đệ quy cần `Box`**: Bất kỳ cấu trúc dữ liệu tự tham chiếu nào trong Rust cũng cần con trỏ thông minh (smart pointer) `Box<T>` để ấn định kích thước con trỏ cố định (8 bytes) trên Ngăn xếp (Stack).
2. **Quyền sở hữu trong danh sách**: Mỗi nút sở hữu nút kế tiếp thông qua `Option<Box<Nut<T>>>`. Đỉnh danh sách sở hữu toàn bộ chuỗi mắt xích phía sau.
3. **Tuyệt chiêu `Option::take()`**: Là chiếc "cờ lê vạn năng" để hoán đổi con trỏ và lấy quyền sở hữu (ownership) mà không bị vi phạm các quy tắc vay mượn (borrow).
4. **Hàm hủy `Drop` tuần tự**: Luôn tự viết hàm `Drop` cho danh sách liên kết để tránh lỗi tràn ngăn xếp (Stack Overflow) khi danh sách chứa số lượng lớn phần tử.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bộ đếm phần tử)**:  
   Không sử dụng trường `do_dai`, hãy viết thêm một phương thức `fn dem_phan_tu_thu_cong(&self) -> usize` cho `DanhSachLienKet`. Phương thức này sử dụng một con trỏ tham chiếu chạy từ đỉnh duyệt lần lượt qua từng nút cho đến khi gặp `None` để đếm tổng số nút. Phân tích độ phức tạp thời gian của phương thức này ($O(N)$).
2. **Bài tập 2 (Tìm kiếm giá trị)**:  
   Cài đặt phương thức `fn chua_phan_tu(&self, gia_tri: &T) -> bool` kiểm tra xem một giá trị có tồn tại trong danh sách liên kết hay không (với điều kiện `T: PartialEq`).
3. **Bài tập 3 (Tư duy con trỏ thông minh)**:  
   Tại sao chúng ta không thể sử dụng `Box<T>` đơn thuần để tạo một Danh sách liên kết đôi (Doubly Linked List - nơi mỗi nút vừa trỏ tới nút kế tiếp `next`, vừa trỏ tới nút đứng trước `prev`)? Hãy giải thích vì sao trường hợp này đòi hỏi sự kết hợp giữa `Rc` và `RefCell` hoặc con trỏ thô (Raw Pointer).
