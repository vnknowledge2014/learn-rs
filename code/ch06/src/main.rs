#![allow(dead_code, unused_variables, unused_imports)]
// File: src/main.rs
// Chương trình thực chiến làm chủ Quy tắc Sở hữu & Cơ chế Di chuyển (Move Semantics)

// 1. Hàm tiếp nhận quyền sở hữu: Biến truyền vào sẽ bị "nuốt chửng" tại đây!
fn consume_series(chuoi_nhan_vao: String) {
    println!("-> [Trong hàm tieu_thu_chuoi]: Đã nhận được: '{}'", chuoi_nhan_vao);
    // Khi hàm này kết thúc tại dấu ngoặc nhọn dưới, chuoi_nhan_vao đi ra khỏi scope
    // Bộ nhớ Heap của chuỗi này sẽ tự động bị giải phóng (DROP) ngay lập tức!
}

// 2. Hàm tiếp nhận và trả lại quyền sở hữu cho người gọi
fn append_suffix(mut series: String) -> String {
    series.push_str(" (Đã được kiểm định)");
    series // Trả lại quyền sở hữu chuỗi mới về cho nơi gọi hàm
}

// 3. Hàm nhận kiểu Copy trên Stack: Không ảnh hưởng gì đến biến gốc
fn print_int(so: i32) {
    println!("-> [Trong hàm in_so_nguyen]: Giá trị số là: {}", so);
}

fn main() {
    println!("============================================================");
    println!("     KHÁM PHÁ QUY TẮC SỞ HỮU & CƠ CHẾ DI CHUYỂN TRONG RUST  ");
    println!("============================================================");

    // --- PHẦN 1: CƠ CHẾ SAO CHÉP TRÊN STACK (COPY TRAIT) ---
    println!("\n1. Kiểm tra kiểu dữ liệu Copy trên Stack:");
    let base_score = 100;
    let point_num_copy = base_score; // Tự động nhân bản trên Stack

    println!("- Điểm gốc: {}, Điểm sao chép: {}", base_score, point_num_copy);
    print_int(base_score);
    // Biến base_score vẫn sử dụng hoàn toàn bình thường sau khi truyền vào hàm!
    println!("- Sau khi gọi hàm, điểm gốc vẫn còn nguyên: {}", base_score);

    // --- PHẦN 2: CƠ CHẾ DI CHUYỂN TRÊN HEAP (MOVE SEMANTICS) ---
    println!("\n2. Kiểm tra cơ chế Di chuyển quyền sở hữu (Move):");
    let security_certificate = String::from("CHUNG_THU_BAO_MAT_2026");
    println!("- Biến 'chung_thu_so' đang là chủ sở hữu hợp pháp duy nhất.");

    // Chuyển giao quyền sở hữu từ security_certificate sang new_owner:
    let new_owner = security_certificate;
    println!("- Đã sang tên đổi chủ thành công cho: {}", new_owner);

    // NẾU BẠN BỎ CHÚ THÍCH DÒNG LỆNH SAU, RUSTC SẼ BÁO LỖI E0382 NGAY:
    // println!("Thử dùng lại biến cũ: {}", security_certificate);

    // --- PHẦN 3: DI CHUYỂN VÀO HÀM VÀ MẤT QUYỀN SỞ HỮU ---
    println!("\n3. Chuyển quyền sở hữu vào một hàm con:");
    let greeting = String::from("Xin chào từ Hà Nội");
    
    // Khi gọi hàm này, greeting bị Move vào hàm con và biến mất khỏi main!
    consume_series(greeting);

    // Dòng sau cũng bị lỗi E0382 vì greeting đã bị Drop bên trong hàm con:
    // println!("Thử in lại thông điệp: {}", greeting);

    // --- PHẦN 4: LẤY LẠI QUYỀN SỞ HỮU THÔNG QUA GIÁ TRỊ TRẢ VỀ ---
    println!("\n4. Chuyển giao đi và nhận lại quyền sở hữu qua return:");
    let profile = String::from("Hồ sơ ứng viên Nguyễn Văn A");
    let decorated_profile = append_suffix(profile);
    // Lúc này 'profile' đã bị move, nhưng 'decorated_profile' là chủ nhân mới nắm giữ kết quả!
    println!("- Kết quả hồ sơ sau khi xử lý: {}", decorated_profile);

    // --- PHẦN 5: NHÂN BẢN SÂU BẰNG .clone() KHI CẦN THIẾT ---
    println!("\n5. Nhân bản sâu toàn diện bằng phương thức .clone():");
    let original_data = String::from("Bản quyền sở hữu trí tuệ");
    let tai_lieu_nhan_ban = original_data.clone(); // Cấp phát thêm một vùng nhớ Heap mới

    println!("- Bản gốc     : {}", original_data);
    println!("- Bản nhân bản: {}", tai_lieu_nhan_ban);
    println!("=> Cả hai biến đều cùng tồn tại và hoạt động độc lập trên 2 vùng Heap riêng biệt!");
}
