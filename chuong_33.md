# Chương 33: Bản đồ bộ nhớ & Không gian địa chỉ ảo (Virtual Address Space & Memory Layout)

## Giới thiệu & Mục tiêu học tập

Chào mừng bạn đến với **Chủ đề 7: An toàn thông tin & Kỹ thuật tấn công/phòng thủ (Cyber Security + Offensive Attacking OSCP)**! Nếu như ở các chủ đề trước, chúng ta đã tìm hiểu cách dữ liệu được tổ chức trên RAM và ghi bền vững xuống đĩa cứng, thì trong chủ đề này, chúng ta sẽ bước sang một vũ đài hoàn toàn mới: **Thế giới ngầm của bảo mật cấp thấp (Low-level Systems Security)**.

Để trở thành một kỹ sư phần mềm hệ thống xuất sắc hay một chuyên gia kiểm thử bảo mật thâm nhập đạt chứng chỉ quốc tế OSCP, vũ khí quan trọng nhất không phải là các công cụ quét tự động, mà là **mô hình tư duy không gian bộ nhớ (Memory Mental Model)**. Mọi cuộc tấn công mạng nguy hiểm nhất lịch sử — từ việc chiếm quyền điều khiển máy chủ, leo thang đặc quyền, đến việc cài cắm mã độc gián điệp — đều bắt nguồn từ sự hiểu lầm hoặc sơ hở trong cách chương trình tương tác với các ô nhớ vật lý.

Trong chương mở đầu của Topic 7, chúng ta sẽ khám phá:
- Bản chất của **Không gian địa chỉ ảo (Virtual Address Space)** và cơ chế ánh xạ trang bộ nhớ (Memory Paging) do Hệ điều hành và Phần cứng (MMU) điều phối.
- Cấu trúc chi tiết của một tiến trình đang chạy trong bộ nhớ: Phân vùng mã lệnh (`.text`), biến toàn cục (`.data`, `.bss`), vùng nhớ động (`Heap`), và ngăn xếp cuộc gọi hàm (`Stack`).
- Khái niệm về bố cục bộ nhớ (memory layout) và cách các kiểu dữ liệu được căn lề byte (alignment) trong máy tính.
- Vai trò của các thanh ghi CPU tối quan trọng: Con trỏ lệnh (`RIP/PC`), con trỏ đỉnh ngăn xếp (`RSP`), và con trỏ đáy ngăn xếp (`RBP`).
- Sự khác biệt về mặt địa chỉ vật lý và logic giữa các biến trên Stack và các dữ liệu được cấp phát trên Heap thông qua con trỏ thông minh (smart pointer).
- Cách đo đạc, quan sát trực tiếp tọa độ byte của từng biến trong Rust và hiểu rõ chiều phát triển đối nghịch giữa Stack (phát triển đi xuống) và Heap (phát triển đi lên).

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu không gian địa chỉ ảo mà không cần bất kỳ công thức toán học hay thuật toán phức tạp nào, hãy cùng quan sát hai hình tượng đời sống trực quan sau:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG HÓA: THÀNH PHỐ ĐỊA CHỈ ẢO & KHÁCH SẠN VẠN PHÒNG           │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. KHÁCH SẠN ĐỊA CHỈ ẢO (VIRTUAL ADDRESS SPACE CỦA MỖI TIẾN TRÌNH)]             │
│ Mỗi chương trình khi khởi động đều được cấp một tấm bản đồ khách sạn riêng biệt: │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Tầng Thượng: Vùng Nhân (Kernel Space - 0xFFFF...) ◄── Cấm xâm phạm!   │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Tầng Cao: Ngăn xếp Stack (Chồng đĩa tiệc cưới)                       │         │
│ │           Xếp đĩa mới xuống dưới ──► Chiều phát triển: Giảm dần      │         │
│ │           ▼ (Đi xuống)                                               │         │
│ │           ... (Vùng trống tự do giữa Stack và Heap) ...              │         │
│ │           ▲ (Đi lên)                                                 │         │
│ │ Tầng Giữa: Đống Heap (Kho gửi đồ công cộng có thẻ giữ đồ)            │         │
│ │           Lấy thêm ô tủ mới ───────► Chiều phát triển: Tăng dần      │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Tầng Trệt: Biến toàn cục (.data & .bss - Bảng thông báo sảnh)        │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Tầng Hầm:  Mã lệnh thực thi (.text - Cuốn sách hướng dẫn cố định)   │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│                                                                                  │
│ [2. ANH LỄ TÂN PHẦN CỨNG MMU (MEMORY MANAGEMENT UNIT)]                           │
│ Chương trình tưởng mình sở hữu cả toà nhà 64-bit từ 0x00 đến 0xFF...             │
│ Nhưng thực tế, anh lễ tân MMU âm thầm ánh xạ từng số phòng ảo sang               │
│ các ô tủ thực tế nằm rải rác trên thanh RAM vật lý của máy tính!                 │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Tấm bản đồ ảo và Anh lễ tân khách sạn (Virtual Memory & MMU)
- Hãy tưởng tượng bạn và một người bạn cùng bước vào một chuỗi khách sạn. Khách sạn đưa cho mỗi người một cuốn sổ tay phòng: Cả hai cuốn sổ đều đánh số phòng từ `101` đến `999`.
- Bạn ở phòng `101` của bạn, bạn của bạn cũng ở phòng `101` của họ. Hai người hoàn toàn không nhìn thấy nhau và không thể bước nhầm vào phòng của nhau.
- Người quản lý thực sự là **Anh lễ tân phần cứng (MMU - Memory Management Unit)**: Khi bạn bảo *"Tôi muốn vào phòng 101"*, anh lễ tân tra sổ mật và dẫn bạn ra chiếc giường số `52` ngoài đời thực. Khi bạn của bạn gọi phòng `101`, anh lễ tân dẫn họ ra chiếc giường số `89`.
- Nhờ cơ chế địa chỉ ảo (virtual address) này, nếu chương trình của bạn bị lỗi hoặc sập, nó chỉ làm hỏng "căn phòng ảo" của chính nó, hoàn toàn không thể làm ảnh hưởng hay đọc trộm dữ liệu của các chương trình khác đang chạy song song trên cùng một hệ điều hành.

### 2. Chồng đĩa tiệc cưới (Stack) vs Kho gửi đồ công cộng (Heap)
- **Ngăn xếp Stack (Chồng đĩa tiệc cưới)**:
  - Khi có một món ăn mới (gọi một hàm), phục vụ đặt một chiếc đĩa mới lên đỉnh chồng đĩa.
  - Mọi gia vị, muỗng nĩa dùng cho món ăn đó (biến cục bộ) được đặt ngay ngắn trên chiếc đĩa này.
  - Khi ăn xong món (hàm kết thúc), người phục vụ nhấc chiếc đĩa trên cùng ra dọn sạch trong nháy mắt. Ngăn nắp, cực kỳ nhanh, và hoàn toàn tự động!
- **Vùng nhớ Heap (Kho gửi đồ công cộng)**:
  - Nếu bạn có một chiếc vali cồng kềnh với kích thước tùy biến (dữ liệu biến thiên như `String` hay `Vec<u8>`), chồng đĩa Stack không thể chứa nổi.
  - Bạn phải mang vali tới quầy gửi đồ công cộng (Heap), nhân viên kho tìm một ô tủ trống vừa vặn, nhét vali vào đó và trao cho bạn một chiếc **Thẻ giữ đồ** (Con trỏ - Pointer).
  - Chiếc thẻ giữ đồ rất nhẹ, bạn có thể cất vào túi áo (Stack), nhưng đồ đạc thực tế thì nằm trong kho lớn (Heap).

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Phân vùng Bộ nhớ Tiến trình (Process Memory Layout)

Trong hệ điều hành 64-bit hiện đại (Linux/macOS/Windows), mỗi tiến trình được cung cấp một không gian địa chỉ ảo khổng lồ (về lý thuyết lên tới $2^{64}$ bytes, trên thực tế thường sử dụng 48-bit tương đương 256 Terabytes). Không gian này được chia thành các phân đoạn có quyền hạn truy cập (Read, Write, Execute) rõ ràng, tạo nên bố cục bộ nhớ (memory layout) quy chuẩn:

1. **Phân đoạn Mã lệnh (`.text segment`)**:
   - Chứa mã máy nhị phân (Machine Instructions) mà CPU sẽ trực tiếp nạp vào để thực thi.
   - **Quyền hạn**: Đọc và Thực thi (`R-X`), **tuyệt đối không được ghi** (`No-Write`). Bất kỳ hành vi nào cố tình ghi đè lên `.text` sẽ bị CPU kích hoạt ngoại lệ `Segmentation Fault` (SIGSEGV) ngay lập tức.
2. **Phân đoạn Biến tĩnh đã khởi tạo (`.data segment`)**:
   - Chứa các biến toàn cục và biến `static` đã được gán giá trị khởi tạo sẵn từ khi biên dịch.
   - **Quyền hạn**: Đọc và Ghi (`RW-`).
3. **Phân đoạn Biến tĩnh chưa khởi tạo (`.bss segment`)**:
   - Viết tắt của *Block Started by Symbol*. Chứa các biến toàn cục chưa được gán giá trị cụ thể. Khi tiến trình nạp vào RAM, hệ điều hành sẽ tự động điền toàn bộ vùng nhớ này bằng các byte `0`.
4. **Vùng nhớ động (`Heap segment`)**:
   - Vùng nhớ dùng để cấp phát động trong lúc chương trình đang chạy (`runtime allocation`).
   - Bắt đầu ngay sau `.bss` và **phát triển dần từ địa chỉ thấp lên địa chỉ cao** (Growing Upwards).
   - Được quản lý thông qua trình cấp phát bộ nhớ (Memory Allocator như `jemalloc` hoặc trình cấp phát mặc định của hệ thống).
5. **Vùng ánh xạ tệp & Thư viện chia sẻ (`Memory Mapping Segment`)**:
   - Vùng nhớ nằm giữa Heap và Stack, nơi hệ điều hành nạp các thư viện liên kết động (`.so` trên Linux, `.dylib` trên macOS, `.dll` trên Windows) và các tệp được ánh xạ qua lời gọi hệ thống `mmap`.
6. **Ngăn xếp cuộc gọi (`Stack segment`)**:
   - Chứa các khung ngăn xếp (Stack Frames) của các hàm đang được thực thi, các biến cục bộ, tham số truyền vào, và địa chỉ trả về (Return Address).
   - Bắt đầu từ gần đỉnh của không gian người dùng và **phát triển dần từ địa chỉ cao xuống địa chỉ thấp** (Growing Downwards).
7. **Vùng không gian nhân (`Kernel Space`)**:
   - Chiếm phần đỉnh cao nhất của không gian địa chỉ. Chỉ có mã nguồn chạy ở cấp độ đặc quyền của Hệ điều hành (Ring 0) mới được phép truy cập. Mã người dùng (Ring 3) bị cấm ngặt nghèo.

### 2. Các Thanh ghi CPU cốt lõi & Cấu trúc Stack Frame

Khi một hàm được gọi, CPU và trình biên dịch phối hợp để tạo ra một **Khung ngăn xếp (Stack Frame)**:

```
Địa chỉ cao (High Address)
┌────────────────────────────────────────────────────────┐
│ Tham số của hàm (Function Arguments)                  │
├────────────────────────────────────────────────────────┤
│ ĐỊA CHỈ TRẢ VỀ (SAVED RETURN ADDRESS - RIP)            │ ◄── Điểm hiểm yếu nhất!
├────────────────────────────────────────────────────────┤
│ Con trỏ khung đáy cũ (SAVED BASE POINTER - RBP)        │
├────────────────────────────────────────────────────────┤
│ Biến cục bộ 1 (Local Variable 1)                      │
├────────────────────────────────────────────────────────┤
│ Biến cục bộ 2 (Local Variable 2)                      │
│ ...                                                    │
├────────────────────────────────────────────────────────┤
│ Đỉnh ngăn xếp hiện tại (STACK POINTER - RSP)           │ ◄── Đỉnh Stack trỏ vào đây
└────────────────────────────────────────────────────────┘
Địa chỉ thấp (Low Address)
```

- **Thanh ghi `RSP` (Stack Pointer)**: Luôn trỏ vào vị trí byte thấp nhất của Stack hiện tại. Mỗi khi thực hiện lệnh `push`, `RSP` bị trừ bớt đi (vì Stack đi xuống); khi gọi lệnh `pop`, `RSP` được cộng thêm.
- **Thanh ghi `RBP` (Base Pointer / Frame Pointer)**: Trỏ vào gốc của Stack Frame hiện tại, giúp chương trình truy cập các biến cục bộ theo độ lệch cố định (ví dụ: `[RBP - 8]`, `[RBP - 16]`).
- **Thanh ghi `RIP` (Instruction Pointer / Program Counter)**: Con trỏ chỉ thẳng vào địa chỉ câu lệnh máy tiếp theo trong phân đoạn `.text` mà CPU chuẩn bị nạp và chạy.
- **Saved Return Address**: Khi hàm `A` gọi hàm `B`, CPU lưu địa chỉ câu lệnh kế tiếp của hàm `A` lên đỉnh Stack. Khi hàm `B` kết thúc (lệnh `ret`), CPU lấy địa chỉ này nạp lại vào `RIP` để nhảy về hàm `A`. **Nếu kẻ tấn công có thể đè hỏng địa chỉ trả về này, chúng có thể bẻ lái CPU thực thi bất kỳ đoạn mã độc nào tùy thích!**

### 3. Con trỏ và Cơ chế kiểm soát địa chỉ trong Rust

Trong Rust, mọi biến đều tuân thủ nghiêm ngặt các quy luật:
- **Quyền sở hữu (ownership)**: Mỗi giá trị trong bộ nhớ chỉ có duy nhất một biến làm chủ sở hữu tại một thời điểm.
- **Mượn (borrow)**: Có thể mượn bất biến `&T` hoặc mượn khả biến `&mut T`, được trình kiểm tra thời gian sống (lifetime) giám sát chặt chẽ.
- **Con trỏ thông minh (smart pointer)**: Các cấu trúc như `Box<T>` nắm giữ địa chỉ trên Heap nhưng tự động giải phóng vùng nhớ khi ra khỏi phạm vi thông qua trait `Drop`.
- **Bộ nhớ đệm (buffer)**: Các lát cắt mảng `&[T]` hay `Vec<T>` luôn lưu kèm thông tin kích thước và con trỏ, triệt tiêu nguy cơ đọc quá biên.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là chương trình Rust hoàn chỉnh giúp bạn trực tiếp "chụp X-quang" toàn bộ không gian địa chỉ ảo của một tiến trình đang hoạt động. Chương trình sẽ in ra địa chỉ chính xác của các phân vùng `.text`, `.data`, Heap, và Stack, đồng thời chứng minh bằng thực nghiệm: **Stack phát triển đi xuống và Heap phát triển đi lên**:

```rust
use std::hint::black_box;

// 1. Biến tĩnh toàn cục nằm trong phân đoạn .data
static GLOBAL_DATA_VAR: i32 = 2026;

// 2. Hằng số tĩnh bất biến nằm trong phân đoạn dữ liệu chỉ đọc (.rodata)
static READ_ONLY_STRING: &str = "Ban do bo nho Rust Masterclass";

// Một hàm đơn giản nằm trong phân đoạn mã máy (.text)
fn sample_target_function() {
    println!("    [Execute] Ham muc tieu dang chay ben trong phan doan .text!");
}

// Hàm đệ quy mô phỏng việc đẩy nhiều khung ngăn xếp (Stack Frames) liên tiếp
fn demonstrate_stack_growth(depth: u32, prev_addr: usize) {
    let local_var: u64 = 0xDEADBEEF;
    let current_addr = &local_var as *const u64 as usize;

    println!(
        "    - Stack Frame do sau {}: Bien cuc bo tai dia chi 0x{:012x}",
        depth, current_addr
    );

    if prev_addr != 0 {
        if current_addr < prev_addr {
            let diff = prev_addr - current_addr;
            println!(
                "      ==> Dia chi GIAM di {} bytes so voi khung truoc (Stack phat trien DI XUONG)!",
                diff
            );
        } else {
            let diff = current_addr - prev_addr;
            println!("      ==> Dia chi TANG len {} bytes!", diff);
        }
    }

    if depth < 3 {
        demonstrate_stack_growth(depth + 1, current_addr);
    }

    // Đảm bảo trình biên dịch không tối ưu hóa làm biến mất biến
    black_box(local_var);
}

fn main() {
    println!("==================================================================");
    println!("   KHAM PHA BAN DO BO NHO & KHONG GIAN DIA CHI AO (VIRTUAL MEMORY)  ");
    println!("==================================================================");

    // 1. Phân đoạn Mã lệnh (.text)
    let text_addr = sample_target_function as usize;
    println!("\n[1] Phan doan Ma may (.text segment):");
    println!("    - Dia chi ham sample_target_function: 0x{:012x}", text_addr);

    // 2. Phân đoạn Dữ liệu (.data & .rodata)
    let data_addr = &GLOBAL_DATA_VAR as *const i32 as usize;
    let rodata_addr = READ_ONLY_STRING.as_ptr() as usize;
    println!("\n[2] Phan doan Du lieu toan cuc (.data & .rodata segments):");
    println!("    - Bien toan cuc GLOBAL_DATA_VAR (.data) : 0x{:012x}", data_addr);
    println!("    - Chuoi hang so READ_ONLY_STRING (.rodata): 0x{:012x}", rodata_addr);

    // 3. Phân đoạn Vùng nhớ động (Heap segment)
    println!("\n[3] Phan doan Vung nho dong (Heap segment):");
    let heap_box_1 = Box::new(1000u64);
    let heap_box_2 = Box::new(2000u64);
    let heap_box_3 = Box::new(3000u64);

    let heap_addr_1 = heap_box_1.as_ref() as *const u64 as usize;
    let heap_addr_2 = heap_box_2.as_ref() as *const u64 as usize;
    let heap_addr_3 = heap_box_3.as_ref() as *const u64 as usize;

    println!("    - Khoi Heap #1: 0x{:012x}", heap_addr_1);
    println!("    - Khoi Heap #2: 0x{:012x}", heap_addr_2);
    println!("    - Khoi Heap #3: 0x{:012x}", heap_addr_3);

    if heap_addr_2 > heap_addr_1 {
        println!(
            "    ==> Khoang cach Heap #2 so voi #1: +{} bytes (Heap phat trien DI LEN)!",
            heap_addr_2 - heap_addr_1
        );
    }

    // 4. Phân đoạn Ngăn xếp (Stack segment)
    println!("\n[4] Phan doan Ngan xep cuoc goi (Stack segment):");
    let main_stack_var: u64 = 42;
    println!(
        "    - Bien cuc bo trong ham main(): 0x{:012x}",
        &main_stack_var as *const u64 as usize
    );
    println!("    - Kiem tra huong dich chuyen cua Stack qua cac lan goi ham:");
    demonstrate_stack_growth(1, 0);

    // 5. Tổng kết so sánh khoảng cách địa chỉ ảo
    println!("\n[5] So sanh tuong quan ban do dia chi ao:");
    println!("    - Dinh cao nhat (Stack)   : ~0x{:012x}", &main_stack_var as *const u64 as usize);
    println!("    - Vung trung tam (Heap)   : ~0x{:012x}", heap_addr_1);
    println!("    - Vung thap (Data)        : ~0x{:012x}", data_addr);
    println!("    - Vung day co so (Text)   : ~0x{:012x}", text_addr);

    // Gọi hàm mẫu để đảm bảo logic chạy hoàn hảo
    sample_target_function();

    println!("\n==================================================================");
    println!("   QUAN SAT THANH CONG: KHONG GIAN BO NHO HOAN TOAN CACH LY!     ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch phổ biến nhất khi lập trình viên bắt đầu làm quen với địa chỉ bộ nhớ, con trỏ và cấu trúc dữ liệu thấp trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0716** | `temporary value dropped while borrowed` | Bạn lấy địa chỉ tham chiếu `&` của một giá trị tạm thời (rvalue) được sinh ra trong biểu thức, giá trị này bị hủy ngay ở cuối dòng lệnh. | Gán giá trị tạm thời đó vào một biến `let` có tên cụ thể trước khi lấy địa chỉ tham chiếu của nó. |
| **E0308** | `mismatched types: expected raw pointer, found reference` | Nhầm lẫn giữa kiểu tham chiếu an toàn (`&T`) và con trỏ thô (`*const T` hoặc `*mut T`). | Sử dụng cú pháp ép kiểu tường minh: `&val as *const T` hoặc phương thức `.as_ptr()`. |
| **E0384** | `cannot assign twice to immutable variable` | Cố gắng thay đổi con trỏ hoặc giá trị mà biến không được khai báo với từ khóa `mut`. | Thêm từ khóa `mut` vào khai báo biến: `let mut ptr = ...`. |
| **E0507** | `cannot move out of a shared reference` | Cố gắng di chuyển quyền sở hữu của một giá trị nằm sau con trỏ mượn. | Sử dụng clone dữ liệu, hoặc chỉ mượn tham chiếu thay vì di chuyển giá trị gốc. |

### Ví dụ phân tích lỗi `E0716` khi lấy địa chỉ của giá trị tạm thời:

```rust
// Đoạn mã lỗi minh họa E0716:
fn vi_du_loi_e0716() {
    // Lỗi: Chuỗi String được tạo ra tạm thời rồi lập tức bị giải phóng
    // let addr = String::from("Rust Security").as_ptr();
    // println!("Địa chỉ: {:p}", addr); // Sử dụng con trỏ trỏ vào vùng nhớ đã chết!
}

// Cách sửa chữa đúng chuẩn:
fn vi_du_dung_e0716() {
    let safe_string = String::from("Rust Security"); // Giữ quyền sở hữu rõ ràng
    let addr = safe_string.as_ptr(); // Con trỏ mượn hợp lệ chừng nào safe_string còn sống
    println!("Địa chỉ an toàn: {:p}", addr);
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Không gian địa chỉ ảo (Virtual Address Space)**: Mỗi tiến trình hoạt động trong một "vũ trụ độc lập" do Hệ điều hành và phần cứng MMU bảo vệ, ngăn chặn triệt để việc các tiến trình xâm phạm vùng nhớ của nhau.
2. **Bố cục bộ nhớ 4 tầng**: Từ thấp lên cao gồm có `.text` (mã máy chỉ đọc), `.data`/`.bss` (biến toàn cục), `Heap` (bộ nhớ động phát triển đi lên), và `Stack` (ngăn xếp hàm phát triển đi xuống).
3. **Các thanh ghi sinh tử**: `RIP` (trỏ lệnh thực thi tiếp theo), `RSP` (đỉnh ngăn xếp), và `RBP` (đáy khung ngăn xếp). Việc bảo vệ địa chỉ trả về lưu trên Stack là phòng tuyến sống còn của an ninh nhị phân.
4. **Rust loại bỏ nguy cơ từ gốc**: Nhờ cơ chế quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) có kiểm tra biên, Rust ngăn chặn hầu hết các lỗi định vị bộ nhớ ngay từ khâu biên dịch.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Đo lường dung lượng Stack Frame)**:  
   Viết một hàm nhận vào hai số nguyên `x: u64` và `y: u64`, khai báo thêm một mảng cục bộ `let buffer = [0u8; 128];`. Hãy in ra địa chỉ của `x`, `y`, và phần tử đầu tiên của `buffer`. Tính toán xem Stack Frame của hàm này chiếm tối thiểu bao nhiêu bytes trên ngăn xếp.
2. **Bài tập 2 (Xác minh địa chỉ Heap độc lập)**:  
   Khởi tạo hai biến `Vec<u8>` có kích thước lần lượt là 64 bytes và 1024 bytes. Sử dụng phương thức `.as_ptr()` để in ra địa chỉ vùng đệm thực tế trên Heap của hai vector này. Nhận xét xem trình cấp phát bộ nhớ có đặt chúng liền kề nhau hay không.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Stack Overflow)**:  
   Nếu bạn viết một hàm đệ quy vô tận không có điểm dừng, chuyện gì sẽ xảy ra về mặt cơ chế bộ nhớ? Khi thanh ghi `RSP` đi xuống quá sâu và vượt qua giới hạn cho phép của hệ điều hành, tín hiệu lỗi nào sẽ được phát ra? Tại sao hệ điều hành không để Stack phát triển vô hạn?
