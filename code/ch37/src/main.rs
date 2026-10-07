#![allow(dead_code, unused_variables, unused_imports)]
use std::hint::black_box;
use std::sync::atomic::{AtomicI32, AtomicU64};

// 1. Biến tĩnh toàn cục CÓ THỂ GHI, khởi tạo khác 0 -> phân đoạn .data
//    (một `static` thường, bất biến, sẽ được xếp vào .rodata chỉ đọc!)
static GLOBAL_DATA_VAR: AtomicI32 = AtomicI32::new(2026);

// 1b. Biến tĩnh có thể ghi, khởi tạo bằng 0 -> phân đoạn .bss (không chiếm chỗ trong tệp thực thi)
static GLOBAL_BSS_VAR: AtomicU64 = AtomicU64::new(0);

// 2. Hằng số tĩnh bất biến nằm trong phân đoạn dữ liệu chỉ đọc (.rodata)
static READ_ONLY_STRING: &str = "Bản đồ bộ nhớ Rust Masterclass";

// Một hàm đơn giản nằm trong phân đoạn mã máy (.text)
fn sample_target_function() {
    println!("    [Thực thi] Hàm mục tiêu đang chạy bên trong phân đoạn .text!");
}

// Hàm đệ quy mô phỏng việc đẩy nhiều khung ngăn xếp (Stack Frames) liên tiếp
fn demonstrate_stack_growth(depth: u32, prev_addr: usize) {
    let local_var: u64 = 0xDEADBEEF;
    let current_addr = &local_var as *const u64 as usize;

    println!(
        "    - Stack Frame độ sâu {}: Biến cục bộ tại địa chỉ 0x{:012x}",
        depth, current_addr
    );

    if prev_addr != 0 {
        if current_addr < prev_addr {
            let diff = prev_addr - current_addr;
            println!(
                "      ==> Địa chỉ GIẢM đi {} bytes so với khung trước (Stack phát triển ĐI XUỐNG)!",
                diff
            );
        } else {
            let diff = current_addr - prev_addr;
            println!("      ==> Địa chỉ TĂNG lên {} bytes!", diff);
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
    println!("   KHÁM PHÁ BẢN ĐỒ BỘ NHỚ & KHÔNG GIAN ĐỊA CHỈ ẢO (VIRTUAL MEMORY)  ");
    println!("==================================================================");

    // 1. Phân đoạn Mã lệnh (.text)
    let text_addr = sample_target_function as fn() as usize;
    println!("\n[1] Phân đoạn Mã máy (.text segment):");
    println!(
        "    - Địa chỉ hàm sample_target_function: 0x{:012x}",
        text_addr
    );

    // 2. Phân đoạn Dữ liệu (.data, .bss & .rodata)
    let data_addr = &GLOBAL_DATA_VAR as *const AtomicI32 as usize;
    let bss_addr = &GLOBAL_BSS_VAR as *const AtomicU64 as usize;
    let rodata_addr = READ_ONLY_STRING.as_ptr() as usize;
    println!("\n[2] Phân đoạn Dữ liệu toàn cục (.data, .bss & .rodata):");
    println!(
        "    - Biến toàn cục GLOBAL_DATA_VAR (.data)   : 0x{:012x}",
        data_addr
    );
    println!(
        "    - Biến toàn cục GLOBAL_BSS_VAR (.bss)     : 0x{:012x}",
        bss_addr
    );
    println!(
        "    - Chuỗi hằng READ_ONLY_STRING (.rodata): 0x{:012x}",
        rodata_addr
    );

    // 3. Phân đoạn Vùng nhớ động (Heap segment)
    println!("\n[3] Phân đoạn Vùng nhớ động (Heap segment):");
    let heap_box_1 = Box::new(1000u64);
    let heap_box_2 = Box::new(2000u64);
    let heap_box_3 = Box::new(3000u64);

    let heap_addr_1 = heap_box_1.as_ref() as *const u64 as usize;
    let heap_addr_2 = heap_box_2.as_ref() as *const u64 as usize;
    let heap_addr_3 = heap_box_3.as_ref() as *const u64 as usize;

    println!("    - Khối Heap #1: 0x{:012x}", heap_addr_1);
    println!("    - Khối Heap #2: 0x{:012x}", heap_addr_2);
    println!("    - Khối Heap #3: 0x{:012x}", heap_addr_3);

    if heap_addr_2 > heap_addr_1 {
        println!(
            "    ==> Khoảng cách Heap #2 so với #1: +{} bytes (lần này trình cấp phát cấp địa chỉ TĂNG dần)",
            heap_addr_2 - heap_addr_1
        );
    }

    // 4. Phân đoạn Ngăn xếp (Stack segment)
    println!("\n[4] Phân đoạn Ngăn xếp cuộc gọi (Stack segment):");
    let main_stack_var: u64 = 42;
    println!(
        "    - Biến cục bộ trong hàm main(): 0x{:012x}",
        &main_stack_var as *const u64 as usize
    );
    println!("    - Kiểm tra hướng dịch chuyển của Stack qua các lần gọi hàm:");
    demonstrate_stack_growth(1, 0);

    // 5. Tổng kết so sánh khoảng cách địa chỉ ảo
    println!("\n[5] So sánh tương quan bản đồ địa chỉ ảo:");
    println!(
        "    - Đỉnh cao nhất (Stack)   : ~0x{:012x}",
        &main_stack_var as *const u64 as usize
    );
    println!("    - Vùng trung tâm (Heap)   : ~0x{:012x}", heap_addr_1);
    println!("    - Vùng thấp (Data)        : ~0x{:012x}", data_addr);
    println!("    - Vùng đáy cơ sở (Text)   : ~0x{:012x}", text_addr);

    // Gọi hàm mẫu để đảm bảo logic chạy hoàn hảo
    sample_target_function();

    println!("\n==================================================================");
    println!("   QUAN SÁT THÀNH CÔNG: KHÔNG GIAN BỘ NHỚ HOÀN TOÀN CÁCH LY!     ");
    println!("==================================================================");
}
