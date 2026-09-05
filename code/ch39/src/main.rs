#![allow(dead_code, unused_variables, unused_imports)]
use std::alloc::{alloc, dealloc, Layout};
use std::ffi::CStr;
use std::os::raw::c_char;

/// Cấu trúc dữ liệu tương thích 100% với định dạng bộ nhớ C ABI
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct NativePoint {
    pub x: i32,
    pub y: i32,
}

/// Một cấu trúc bao bọc an toàn (Safe Abstraction Wrapper)
/// tự quản lý con trỏ thô cấp thấp trên Heap mà không gây rò rỉ bộ nhớ
pub struct SafeRawBuffer {
    ptr: *mut u8,
    capacity: usize,
    layout: Layout,
}

impl SafeRawBuffer {
    /// Khởi tạo bộ đệm với dung lượng chỉ định (Cấp phát thô an toàn)
    pub fn with_capacity(capacity: usize) -> Result<Self, &'static str> {
        if capacity == 0 {
            return Err("Dung lượng bộ đệm phải lớn hơn 0");
        }

        // Tạo bố cục bộ nhớ (Memory Layout) với căn lề 8 bytes
        let layout = Layout::array::<u8>(capacity)
            .map_err(|_| "Lỗi tính toán kích thước bố cục bộ nhớ")?;

        // Thao tác cấp phát thô nằm trong khối unsafe
        let raw_ptr = unsafe { alloc(layout) };

        if raw_ptr.is_null() {
            return Err("Hệ thống cạn kiệt bộ nhớ: Cấp phát con trỏ thô thất bại!");
        }

        // Khởi tạo các byte về 0 để tránh đọc dữ liệu rác
        unsafe {
            std::ptr::write_bytes(raw_ptr, 0, capacity);
        }

        Ok(Self {
            ptr: raw_ptr,
            capacity,
            layout,
        })
    }

    /// Ghi dữ liệu vào vị trí offset với kiểm tra biên tuyệt đối
    pub fn write_byte(&mut self, offset: usize, value: u8) -> Result<(), &'static str> {
        if offset >= self.capacity {
            return Err("Chỉ số vượt quá giới hạn dung lượng bộ đệm!");
        }

        // Thao tác unsafe được kiểm chứng an toàn 100% bởi ranh giới offset < capacity
        unsafe {
            let target_ptr = self.ptr.add(offset);
            *target_ptr = value;
        }

        Ok(())
    }

    /// Đọc dữ liệu tại vị trí offset an toàn
    pub fn read_byte(&self, offset: usize) -> Option<u8> {
        if offset >= self.capacity {
            return None;
        }

        unsafe {
            let target_ptr = self.ptr.add(offset);
            Some(*target_ptr)
        }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// Tự động giải phóng con trỏ thô khi cấu trúc ra khỏi phạm vi (RAII Pattern)
impl Drop for SafeRawBuffer {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            println!("    [Drop] Đang giải phóng con trỏ thô tại địa chỉ {:p}...", self.ptr);
            unsafe {
                dealloc(self.ptr, self.layout);
            }
            self.ptr = std::ptr::null_mut();
        }
    }
}

// Giả lập khai báo hàm FFI tương thích chuẩn C
extern "C" {
    // Gọi hàm đo độ dài chuỗi kinh điển strlen trong thư viện C chuẩn (libc)
    fn strlen(s: *const c_char) -> usize;
}

fn main() {
    println!("==================================================================");
    println!("   KIEM CHUNG AN TOAN BO NHO: UNSAFE RUST & FFI DONG GOI CHUAN   ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM BỘ ĐỆM CẤP THẤP ĐÓNG GÓI AN TOÀN (SAFE WRAPPER)
    // -------------------------------------------------------------
    println!("\n[1] Khoi tao SafeRawBuffer dong goi con tro tho Heap:");
    {
        let mut my_buffer = SafeRawBuffer::with_capacity(32).expect("Khoi tao that bai");
        println!("    - Khoi tao thanh cong bo dem dung luong: {} bytes", my_buffer.capacity());

        // Ghi dữ liệu an toàn
        my_buffer.write_byte(0, 0xDE).unwrap();
        my_buffer.write_byte(1, 0xAD).unwrap();
        my_buffer.write_byte(2, 0xBE).unwrap();
        my_buffer.write_byte(3, 0xEF).unwrap();

        println!("    - Doc byte tai index 0: 0x{:02X}", my_buffer.read_byte(0).unwrap());
        println!("    - Doc byte tai index 1: 0x{:02X}", my_buffer.read_byte(1).unwrap());

        // Thử nghiệm truy cập ngoài biên an toàn
        let out_of_bounds = my_buffer.write_byte(100, 0xFF);
        println!("    - Thu ghi vao index = 100: {:?}", out_of_bounds);
        assert!(out_of_bounds.is_err());
        println!("    => Lop vo Safe Wrapper da chan dung hanh vi vi pham bien!");
    } // my_buffer tự động được giải phóng an toàn tại đây thông qua drop()!

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM GIAO TIẾP HÀM NGOẠI LAI (FFI VỚI C ABI)
    // -------------------------------------------------------------
    println!("\n[2] Thu nghiem Foreign Function Interface (FFI) voi C Library:");

    // Tạo chuỗi an toàn tương thích C kết thúc bằng byte \0
    let c_greeting = std::ffi::CString::new("Hello from Rust via C ABI!").unwrap();

    // Gọi hàm strlen của C bên trong khối unsafe có kiểm soát
    let length_from_c = unsafe {
        let raw_c_ptr = c_greeting.as_ptr();
        strlen(raw_c_ptr)
    };

    println!("    - Chuoi gui sang C : {:?}", c_greeting);
    println!("    - Do dai do boi C strlen: {} bytes", length_from_c);
    assert_eq!(length_from_c, 26);

    // -------------------------------------------------------------
    // 3. THỬ NGHIỆM CẤU TRÚC ĐỊNH DẠNG TƯƠNG THÍCH #[repr(C)]
    // -------------------------------------------------------------
    println!("\n[3] Kiem tra tuong thich bo cuc bo nho #[repr(C)]:");
    let pt = NativePoint { x: 100, y: 200 };
    println!("    - Toa do diem C-compatible: x = {}, y = {}", pt.x, pt.y);
    println!("    - Kich thuoc struct NativePoint: {} bytes (dung bang 2 * i32)", std::mem::size_of::<NativePoint>());
    assert_eq!(std::mem::size_of::<NativePoint>(), 8);

    println!("\n==================================================================");
    println!("   XAC NHAN: UNSAFE & FFI HOAT DONG AN TOAN DUNG QUY CHUAN!      ");
    println!("==================================================================");
}
