#![allow(dead_code, unused_variables, unused_imports)]
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
    let text_addr = sample_target_function as fn() as usize;
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
