#![allow(dead_code)]
//! Chương 68 — Lập trình Game: vòng lặp game bước cố định, ECS hướng dữ liệu,
//! toán vector, phát hiện va chạm và phân hoạch không gian.
//!
//! Toàn bộ mã ở đây là LÕI THUẦN TÚY — không vẽ, không cửa sổ, không thời gian
//! thực. Đúng theo "lõi hàm, vỏ mệnh lệnh" của Chương 20: nhờ vậy mà logic
//! game kiểm thử được tất định, còn Bevy/macroquad chỉ là lớp vỏ hiển thị.

use std::collections::HashMap;

// ============================================================================
// 1. TOÁN VECTOR — ngôn ngữ của mọi trò chơi
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 { pub x: f32, pub y: f32 }

impl Vec2 {
    pub const KHONG: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub fn new(x: f32, y: f32) -> Vec2 { Vec2 { x, y } }
    pub fn gate(self, k: Vec2) -> Vec2 { Vec2::new(self.x + k.x, self.y + k.y) }
    pub fn subtract(self, k: Vec2) -> Vec2 { Vec2::new(self.x - k.x, self.y - k.y) }
    pub fn nhan(self, s: f32) -> Vec2 { Vec2::new(self.x * s, self.y * s) }
    pub fn dot(self, k: Vec2) -> f32 { self.x * k.x + self.y * k.y }
    /// Bình phương độ dài — dùng nó thay `length()` khi CHỈ cần so sánh,
    /// vì `sqrt` đắt và ta so sánh khoảng cách hàng nghìn lần mỗi khung hình.
    pub fn length_squared(self) -> f32 { self.dot(self) }
    pub fn length(self) -> f32 { self.length_squared().sqrt() }
    /// Chuẩn hóa an toàn: vector không thì trả về không, không sinh NaN.
    pub fn normalize(self) -> Vec2 {
        let d = self.length();
        if d < 1e-6 { Vec2::KHONG } else { self.nhan(1.0 / d) }
    }
    /// Nội suy tuyến tính — dùng để LÀM MƯỢT hình ảnh giữa hai bước vật lý.
    pub fn lerp(self, den: Vec2, t: f32) -> Vec2 {
        self.gate(den.subtract(self).nhan(t))
    }
    /// Phản xạ quanh pháp tuyến — quả bóng nảy khỏi tường.
    pub fn part_remote(self, normal: Vec2) -> Vec2 {
        let n = normal.normalize();
        self.subtract(n.nhan(2.0 * self.dot(n)))
    }
}

// ============================================================================
// 2. VÒNG LẶP GAME BƯỚC CỐ ĐỊNH — bài "Fix Your Timestep" kinh điển
// ============================================================================

/// Nếu để bước vật lý phụ thuộc tốc độ khung hình, cùng một trò chơi sẽ chạy
/// KHÁC NHAU trên máy mạnh và máy yếu — nhân vật xuyên tường, nhảy khác độ cao.
/// Giải pháp: tích lũy thời gian rồi chạy vật lý theo bước CỐ ĐỊNH.
pub struct AccumulatorUnit {
    pub step_has_peak: f32,
    accumulate: f32,
    pub max_step_one_frame: u32,
}

#[derive(Debug, PartialEq)]
pub struct FrameClock {
    pub physics_steps: u32,
    /// Phần dư dùng để nội suy hình ảnh — nhờ nó mà 60 bước/giây vẫn
    /// hiển thị mượt trên màn hình 144 Hz.
    pub lerp_factor: f32,
    pub is_unit_step: bool,
}

impl AccumulatorUnit {
    pub fn new(hz: f32) -> Self {
        AccumulatorUnit { step_has_peak: 1.0 / hz, accumulate: 0.0, max_step_one_frame: 5 }
    }
    pub fn new_frame(&mut self, delta_thuc: f32) -> FrameClock {
        self.accumulate += delta_thuc;
        let mut num_step = 0;
        while self.accumulate >= self.step_has_peak && num_step < self.max_step_one_frame {
            self.accumulate -= self.step_has_peak;
            num_step += 1;
        }
        // "Xoắn ốc tử thần": máy quá chậm → nợ thời gian chồng chất → càng chậm.
        // Cắt nợ để game giữ được phản hồi, chấp nhận chạy chậm hơn thời gian thật.
        let is_unit = self.accumulate >= self.step_has_peak;
        if is_unit { self.accumulate = 0.0; }
        FrameClock {
            physics_steps: num_step,
            lerp_factor: self.accumulate / self.step_has_peak,
            is_unit_step: is_unit,
        }
    }
}

/// PHIÊN BẢN CHỐNG TRÔI: đếm thời gian bằng NANO-GIÂY nguyên thay vì `f32`.
///
/// Cộng dồn 144 lần `1.0/144.0` kiểu `f32` KHÔNG cho ra đúng 1.0 — sai số nhị
/// phân tích lũy làm mất hẳn một bước vật lý mỗi giây. Với game nhiều người
/// chơi hay bản phát lại (replay), một bước lệch là hỏng toàn bộ tính tất định.
/// Số nguyên không có sai số làm tròn, nên phép cộng là chính xác tuyệt đối.
pub struct IntegerAccumulator {
    pub step_nanos: u64,
    accumulated_nanos: u64,
    pub max_step_one_frame: u32,
}

impl IntegerAccumulator {
    pub fn new(hz: u64) -> Self {
        IntegerAccumulator { step_nanos: 1_000_000_000 / hz, accumulated_nanos: 0, max_step_one_frame: 5 }
    }
    pub fn new_frame(&mut self, delta_ns: u64) -> FrameClock {
        self.accumulated_nanos += delta_ns;
        let mut num_step = 0;
        while self.accumulated_nanos >= self.step_nanos && num_step < self.max_step_one_frame {
            self.accumulated_nanos -= self.step_nanos;
            num_step += 1;
        }
        let is_unit = self.accumulated_nanos >= self.step_nanos;
        if is_unit { self.accumulated_nanos = 0; }
        FrameClock {
            physics_steps: num_step,
            lerp_factor: self.accumulated_nanos as f32 / self.step_nanos as f32,
            is_unit_step: is_unit,
        }
    }
}

// ============================================================================
// 3. VẬT LÝ — Euler tường minh vs Euler nửa ẩn
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsBody {
    pub pos_value: Vec2,
    pub velocity: Vec2,
    pub quantity: f32,
}

/// Euler tường minh: dùng vận tốc CŨ để cập nhật vị trí. Đơn giản nhưng
/// TÍCH LŨY NĂNG LƯỢNG — quỹ đạo tròn dần biến thành xoắn ốc bay ra ngoài.
pub fn explicit_euler_step(t: PhysicsBody, gia_toc: Vec2, dt: f32) -> PhysicsBody {
    PhysicsBody {
        pos_value: t.pos_value.gate(t.velocity.nhan(dt)),      // dùng vận tốc CŨ
        velocity: t.velocity.gate(gia_toc.nhan(dt)),
        ..t
    }
}

/// Euler nửa ẩn (symplectic): cập nhật vận tốc TRƯỚC rồi mới dùng nó cho vị trí.
/// Chỉ đổi thứ tự hai dòng, nhưng năng lượng được bảo toàn ổn định — đây là
/// bộ tích phân mặc định của gần như mọi game engine.
pub fn semi_implicit_euler_step(t: PhysicsBody, gia_toc: Vec2, dt: f32) -> PhysicsBody {
    let new_velocity = t.velocity.gate(gia_toc.nhan(dt));
    PhysicsBody {
        pos_value: t.pos_value.gate(new_velocity.nhan(dt)),    // dùng vận tốc MỚI
        velocity: new_velocity,
        ..t
    }
}

// ============================================================================
// 4. VA CHẠM — hình bao AABB và hình tròn
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HopReport { pub min: Vec2, pub max: Vec2 }

impl HopReport {
    pub fn self_centered(tam: Vec2, half_extent: Vec2) -> HopReport {
        HopReport { min: tam.subtract(half_extent), max: tam.gate(half_extent) }
    }
    /// Định lý trục tách: hai hộp KHÔNG chạm nhau nếu tồn tại MỘT trục mà
    /// hình chiếu của chúng rời nhau. Với AABB chỉ cần thử 2 trục X và Y.
    pub fn intersect(&self, k: &HopReport) -> bool {
        self.min.x <= k.max.x && self.max.x >= k.min.x &&
        self.min.y <= k.max.y && self.max.y >= k.min.y
    }
    pub fn contains_point(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn tam(&self) -> Vec2 { self.min.gate(self.max).nhan(0.5) }
    /// Vector đẩy tối thiểu: đẩy hộp ra khỏi nhau theo trục CHỒNG LẤN ÍT NHẤT.
    pub fn day_ra(&self, k: &HopReport) -> Option<Vec2> {
        if !self.intersect(k) { return None; }
        let chong_x = (self.max.x - k.min.x).min(k.max.x - self.min.x);
        let chong_y = (self.max.y - k.min.y).min(k.max.y - self.min.y);
        Some(if chong_x < chong_y {
            let first = if self.tam().x < k.tam().x { -1.0 } else { 1.0 };
            Vec2::new(chong_x * first, 0.0)
        } else {
            let first = if self.tam().y < k.tam().y { -1.0 } else { 1.0 };
            Vec2::new(0.0, chong_y * first)
        })
    }
}

/// Va chạm hình tròn — so BÌNH PHƯƠNG khoảng cách để né phép căn bậc hai.
pub fn intersect_merge(tam_a: Vec2, ban_kinh_a: f32, tam_b: Vec2, ban_kinh_b: f32) -> bool {
    let tong_bk = ban_kinh_a + ban_kinh_b;
    tam_a.subtract(tam_b).length_squared() <= tong_bk * tong_bk
}

// ============================================================================
// 5. PHÂN HOẠCH KHÔNG GIAN — từ O(n²) xuống gần O(n)
// ============================================================================

/// Kiểm tra mọi cặp là O(n²): 1 000 vật thể = 499 500 phép thử mỗi khung hình.
/// Băm không gian chia thế giới thành ô lưới; chỉ so các vật CÙNG ô hoặc ô kề.
pub struct LuoiBam {
    size_cell: f32,
    o: HashMap<(i32, i32), Vec<usize>>,
}

impl LuoiBam {
    pub fn new(size_cell: f32) -> Self {
        LuoiBam { size_cell, o: HashMap::new() }
    }
    fn toa_do_o(&self, p: Vec2) -> (i32, i32) {
        ((p.x / self.size_cell).floor() as i32, (p.y / self.size_cell).floor() as i32)
    }
    pub fn build_use(&mut self, hop: &[HopReport]) {
        self.o.clear();
        for (i, h) in hop.iter().enumerate() {
            let (x0, y0) = self.toa_do_o(h.min);
            let (x1, y1) = self.toa_do_o(h.max);
            // Vật lớn nằm trên nhiều ô -> phải ghi vào TẤT CẢ ô nó chạm.
            for x in x0..=x1 {
                for y in y0..=y1 {
                    self.o.entry((x, y)).or_default().push(i);
                }
            }
        }
    }
    /// Trả về các cặp CÓ THỂ va chạm (đã khử trùng lặp và sắp xếp tất định).
    pub fn suspicious_pairs(&self) -> Vec<(usize, usize)> {
        let mut cap: Vec<(usize, usize)> = Vec::new();
        for list in self.o.values() {
            for i in 0..list.len() {
                for j in (i + 1)..list.len() {
                    let (a, b) = (list[i].min(list[j]), list[i].max(list[j]));
                    cap.push((a, b));
                }
            }
        }
        cap.sort_unstable();
        cap.dedup(); // một cặp có thể xuất hiện ở nhiều ô chung
        cap
    }
}

/// Phép so sánh chuẩn: duyệt mọi cặp. Dùng làm ĐỐI CHỨNG cho lưới băm.
pub fn va_cham_vet_can(hop: &[HopReport]) -> Vec<(usize, usize)> {
    let mut kq = Vec::new();
    for i in 0..hop.len() {
        for j in (i + 1)..hop.len() {
            if hop[i].intersect(&hop[j]) { kq.push((i, j)); }
        }
    }
    kq
}

pub fn va_cham_qua_luoi(hop: &[HopReport], size_cell: f32) -> (Vec<(usize, usize)>, usize) {
    let mut luoi = LuoiBam::new(size_cell);
    luoi.build_use(hop);
    let kha_nghi = luoi.suspicious_pairs();
    let num_op_thu = kha_nghi.len();
    let that: Vec<(usize, usize)> = kha_nghi.into_iter()
        .filter(|&(a, b)| hop[a].intersect(&hop[b]))
        .collect();
    (that, num_op_thu)
}

// ============================================================================
// 6. ECS — Thực thể · Thành phần · Hệ thống
// ============================================================================
// Ý tưởng cốt lõi: KHÔNG dùng kế thừa ("Quái vật kế thừa Sinh vật kế thừa
// Thực thể"). Thay vào đó, thực thể chỉ là một CON SỐ; dữ liệu nằm trong các
// mảng song song. Hệ thống duyệt mảng liên tiếp trong bộ nhớ -> cache CPU
// hoạt động hết công suất. Đây là "thiết kế hướng dữ liệu".

pub type RealPosition = u32;

#[derive(Debug, Default)]
pub struct BoundedPos {
    next: RealPosition,
    pub con_song: Vec<RealPosition>,
    pub pos_value: HashMap<RealPosition, Vec2>,
    pub velocity: HashMap<RealPosition, Vec2>,
    pub mau: HashMap<RealPosition, i32>,
    pub contact_damage: HashMap<RealPosition, i32>,
    pub ban_kinh: HashMap<RealPosition, f32>,
}

impl BoundedPos {
    pub fn new() -> Self { BoundedPos::default() }

    pub fn tao(&mut self) -> RealPosition {
        let e = self.next;
        self.next += 1;
        self.con_song.push(e);
        e
    }
    pub fn cancel(&mut self, e: RealPosition) {
        self.con_song.retain(|&x| x != e);
        self.pos_value.remove(&e);
        self.velocity.remove(&e);
        self.mau.remove(&e);
        self.contact_damage.remove(&e);
        self.ban_kinh.remove(&e);
    }
    /// Truy vấn: các thực thể có ĐỦ cả vị trí lẫn vận tốc.
    /// Trong ECS thật, đây là chỗ dùng "archetype" để duyệt liên tiếp.
    pub fn has_position_and_velocity(&self) -> Vec<RealPosition> {
        let mut v: Vec<RealPosition> = self.con_song.iter().copied()
            .filter(|e| self.pos_value.contains_key(e) && self.velocity.contains_key(e))
            .collect();
        v.sort_unstable(); // tất định — điều kiện tiên quyết để kiểm thử được
        v
    }
}

/// HỆ THỐNG là hàm thuần túy về mặt logic: `&mut BoundedPos` vào, thế giới đổi ra.
/// Mỗi hệ thống chỉ đụng đúng những thành phần nó cần.
pub fn he_thong_move(tg: &mut BoundedPos, dt: f32) {
    for e in tg.has_position_and_velocity() {
        let v = tg.velocity[&e];
        if let Some(p) = tg.pos_value.get_mut(&e) { *p = p.gate(v.nhan(dt)); }
    }
}

pub fn gravity_system(tg: &mut BoundedPos, g: f32, dt: f32) {
    for e in tg.con_song.clone() {
        if let Some(v) = tg.velocity.get_mut(&e) { v.y -= g * dt; }
    }
}

/// Va chạm gây sát thương, rồi thu dọn xác. Trả về số thực thể đã chết.
pub fn collision_damage_system(tg: &mut BoundedPos) -> usize {
    let list: Vec<RealPosition> = {
        let mut v: Vec<RealPosition> = tg.con_song.iter().copied()
            .filter(|e| tg.pos_value.contains_key(e) && tg.ban_kinh.contains_key(e))
            .collect();
        v.sort_unstable(); v
    };
    let mut sat_thuong: HashMap<RealPosition, i32> = HashMap::new();
    for i in 0..list.len() {
        for j in (i + 1)..list.len() {
            let (a, b) = (list[i], list[j]);
            if intersect_merge(tg.pos_value[&a], tg.ban_kinh[&a], tg.pos_value[&b], tg.ban_kinh[&b]) {
                if let Some(&st) = tg.contact_damage.get(&a) { *sat_thuong.entry(b).or_insert(0) += st; }
                if let Some(&st) = tg.contact_damage.get(&b) { *sat_thuong.entry(a).or_insert(0) += st; }
            }
        }
    }
    for (e, st) in sat_thuong {
        if let Some(m) = tg.mau.get_mut(&e) { *m -= st; }
    }
    let chet: Vec<RealPosition> = tg.con_song.iter().copied()
        .filter(|e| tg.mau.get(e).map_or(false, |&m| m <= 0)).collect();
    for e in &chet { tg.cancel(*e); }
    chet.len()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LẬP TRÌNH GAME: VÒNG LẶP · VẬT LÝ · VA CHẠM · ECS        ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÒNG LẶP BƯỚC CỐ ĐỊNH 60 Hz");
    let mut bt = AccumulatorUnit::new(60.0);
    for (name, dt) in [("máy mạnh 144 fps", 1.0 / 144.0), ("máy yếu 30 fps", 1.0 / 30.0),
                      ("khựng 0.5 giây", 0.5)] {
        let n = bt.new_frame(dt);
        println!("   {:<18} → {} bước vật lý, nội suy {:.2}{}",
                 name, n.physics_steps, n.lerp_factor,
                 if n.is_unit_step { "  ⚠ cắt nợ để tránh xoắn ốc tử thần" } else { "" });
    }

    println!("\n2. HAI BỘ TÍCH PHÂN — vật rơi tự do 1 giây, dt = 1/60");
    let bd = PhysicsBody { pos_value: Vec2::new(0.0, 100.0), velocity: Vec2::KHONG, quantity: 1.0 };
    let g = Vec2::new(0.0, -9.81);
    let (mut a, mut b) = (bd, bd);
    for _ in 0..60 {
        a = explicit_euler_step(a, g, 1.0 / 60.0);
        b = semi_implicit_euler_step(b, g, 1.0 / 60.0);
    }
    let that = 100.0 - 0.5 * 9.81;
    println!("   Nghiệm giải tích : y = {:.4}", that);
    println!("   Euler tường minh : y = {:.4} (lệch {:.4})", a.pos_value.y, (a.pos_value.y - that).abs());
    println!("   Euler nửa ẩn     : y = {:.4} (lệch {:.4})", b.pos_value.y, (b.pos_value.y - that).abs());

    println!("\n3. VA CHẠM & VECTOR ĐẨY TỐI THIỂU");
    let h1 = HopReport::self_centered(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
    let h2 = HopReport::self_centered(Vec2::new(1.5, 0.2), Vec2::new(1.0, 1.0));
    println!("   Hai hộp chồng nhau: {} | đẩy ra: {:?}", h1.intersect(&h2), h1.day_ra(&h2));
    println!("   Bóng bay (1,-1) đập sàn (pháp tuyến 0,1) → {:?}",
             Vec2::new(1.0, -1.0).part_remote(Vec2::new(0.0, 1.0)));

    println!("\n4. BĂM KHÔNG GIAN — 400 vật thể rải trên lưới 100×100");
    let hop: Vec<HopReport> = (0..400).map(|i| {
        let x = (i % 20) as f32 * 5.0;
        let y = (i / 20) as f32 * 5.0;
        HopReport::self_centered(Vec2::new(x, y), Vec2::new(1.2, 1.2))
    }).collect();
    let vet_can = va_cham_vet_can(&hop);
    let (qua_luoi, so_thu) = va_cham_qua_luoi(&hop, 6.0);
    let cap_vet_can = hop.len() * (hop.len() - 1) / 2;
    println!("   Vét cạn : {} phép thử → {} va chạm", cap_vet_can, vet_can.len());
    println!("   Lưới băm: {} phép thử → {} va chạm", so_thu, qua_luoi.len());
    println!("   Cùng kết quả: {} | giảm {:.0}% khối lượng tính toán",
             vet_can == qua_luoi, 100.0 - so_thu as f64 * 100.0 / cap_vet_can as f64);

    println!("\n5. ECS — 1 người chơi, 3 quái, mô phỏng 3 khung hình");
    let mut tg = BoundedPos::new();
    let nguoi_choi = tg.tao();
    tg.pos_value.insert(nguoi_choi, Vec2::new(0.0, 0.0));
    tg.velocity.insert(nguoi_choi, Vec2::new(1.0, 0.0));
    tg.mau.insert(nguoi_choi, 100);
    tg.ban_kinh.insert(nguoi_choi, 1.0);
    for i in 0..3 {
        let q = tg.tao();
        tg.pos_value.insert(q, Vec2::new(2.0 + i as f32 * 0.5, 0.0));
        tg.mau.insert(q, 10);
        tg.ban_kinh.insert(q, 1.0);
        tg.contact_damage.insert(q, 4);
    }
    tg.contact_damage.insert(nguoi_choi, 6);
    for frame in 1..=3 {
        he_thong_move(&mut tg, 1.0);
        let chet = collision_damage_system(&mut tg);
        println!("   Khung {}: người chơi ở x={:.1} · máu {:?} · {} thực thể chết · còn {} sống",
                 frame, tg.pos_value.get(&nguoi_choi).map_or(0.0, |p| p.x),
                 tg.mau.get(&nguoi_choi), chet, tg.con_song.len());
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GAME = MỘT HÀM THUẦN TÚY CHẠY 60 LẦN MỖI GIÂY            ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gan_bang(a: f32, b: f32) -> bool { (a - b).abs() < 1e-4 }

    // ---------- Vector ----------
    #[test]
    fn normalizing_zero_vector_avoids_nan() {
        let v = Vec2::KHONG.normalize();
        assert_eq!(v, Vec2::KHONG, "chia cho 0 phải bị chặn, không được ra NaN");
        assert!(!v.x.is_nan() && !v.y.is_nan());
    }

    #[test]
    fn normalize_yields_unit_length() {
        for v in [Vec2::new(3.0, 4.0), Vec2::new(-7.0, 0.5), Vec2::new(0.0, -2.0)] {
            assert!(gan_bang(v.normalize().length(), 1.0));
        }
    }

    #[test]
    fn length_squared_matches_length() {
        let v = Vec2::new(3.0, 4.0);
        assert!(gan_bang(v.length(), 5.0));
        assert!(gan_bang(v.length_squared(), 25.0));
    }

    #[test]
    fn reflect_preserves_magnitude_and_flips_axis() {
        let toi = Vec2::new(1.0, -1.0);
        let ra = toi.part_remote(Vec2::new(0.0, 1.0));
        assert!(gan_bang(ra.x, 1.0), "thành phần song song mặt phẳng giữ nguyên");
        assert!(gan_bang(ra.y, 1.0), "thành phần vuông góc đổi dấu");
        assert!(gan_bang(ra.length(), toi.length()), "va chạm đàn hồi giữ nguyên tốc độ");
    }

    #[test]
    fn lerp_correct_at_ends_and_midpoint() {
        let a = Vec2::new(0.0, 0.0);
        let b = Vec2::new(10.0, 20.0);
        assert_eq!(a.lerp(b, 0.0), a);
        assert_eq!(a.lerp(b, 1.0), b);
        assert_eq!(a.lerp(b, 0.5), Vec2::new(5.0, 10.0));
    }

    // ---------- Vòng lặp game ----------
    #[test]
    fn f32_accumulator_drifts() {
        // LỖI THẬT, KHÔNG PHẢI GIẢ ĐỊNH: 1.0/144.0 không biểu diễn chính xác
        // được bằng nhị phân. Cộng dồn 144 lần cho ra số HƠI NHỎ HƠN 1.0,
        // nên mất hẳn một bước vật lý sau mỗi giây.
        let mut bt = AccumulatorUnit::new(60.0);
        bt.max_step_one_frame = 1000;
        let tong: u32 = (0..144).map(|_| bt.new_frame(1.0 / 144.0).physics_steps).sum();
        assert_eq!(tong, 59, "đáng lẽ 60 — một bước bị nuốt mất vì trôi dấu phẩy động");
    }

    #[test]
    fn integer_accumulator_is_fps_independent() {
        // Cùng 1 giây thời gian thực → CHÍNH XÁC 60 bước vật lý, ở MỌI fps.
        //
        // Chú ý cách lấy delta: hiệu của hai MỐC ĐỒNG HỒ TUYỆT ĐỐI, chứ không
        // phải hằng số `1e9 / fps` chia sẵn. Phép chia nguyên bị cắt cụt sẽ
        // làm hụt thời gian y như trôi dấu phẩy động. Game thật luôn đọc đồng
        // hồ tuyệt đối rồi trừ — nhờ vậy sai số không bao giờ tích lũy.
        for fps in [30u64, 60, 144, 240] {
            let mut bt = IntegerAccumulator::new(60);
            bt.max_step_one_frame = 1000;
            let moc = |i: u64| i * 1_000_000_000 / fps; // mốc tuyệt đối, chính xác
            let tong: u32 = (1..=fps)
                .map(|i| bt.new_frame(moc(i) - moc(i - 1)).physics_steps)
                .sum();
            assert_eq!(tong, 60, "ở {} fps vẫn phải đúng 60 bước", fps);
        }
    }

    #[test]
    fn integer_accumulator_also_clamps_on_long_frames() {
        let mut bt = IntegerAccumulator::new(60);
        let n = bt.new_frame(2_000_000_000); // khựng 2 giây
        assert_eq!(n.physics_steps, 5);
        assert!(n.is_unit_step);
        assert_eq!(bt.new_frame(16_666_666).physics_steps, 1, "không mang nợ sang khung sau");
    }

    #[test]
    fn lerp_factor_stays_in_unit_range() {
        let mut bt = AccumulatorUnit::new(60.0);
        for i in 0..200 {
            let n = bt.new_frame(0.001 * (i % 37) as f32);
            assert!((0.0..1.0).contains(&n.lerp_factor),
                    "hệ số nội suy {} nằm ngoài [0,1)", n.lerp_factor);
        }
    }

    #[test]
    fn clamping_dt_avoids_death_spiral() {
        let mut bt = AccumulatorUnit::new(60.0);
        let n = bt.new_frame(2.0); // khựng 2 giây = đáng lẽ 120 bước
        assert_eq!(n.physics_steps, 5, "bị chặn ở trần 5 bước");
        assert!(n.is_unit_step);
        // Khung sau phải trở lại bình thường, không mang theo nợ
        let next = bt.new_frame(1.0 / 60.0);
        assert_eq!(next.physics_steps, 1, "nợ đã bị cắt, không dồn sang khung sau");
    }

    // ---------- Vật lý ----------
    #[test]
    fn under_constant_accel_both_integrators_err_symmetrically() {
        // Kết quả có thể gây bất ngờ: khi gia tốc KHÔNG ĐỔI, Euler nửa ẩn
        // KHÔNG chính xác hơn. Hai bộ lệch đúng bằng nhau — một cái vượt,
        // một cái hụt — vì sai số đều là 0.5·g·dt².
        // Ưu thế của nửa ẩn nằm ở chỗ khác: sự ỔN ĐỊNH của hệ dao động,
        // xem bài kiểm thử quỹ đạo tròn ngay bên dưới.
        let bd = PhysicsBody { pos_value: Vec2::new(0.0, 100.0), velocity: Vec2::KHONG, quantity: 1.0 };
        let g = Vec2::new(0.0, -9.81);
        let (mut a, mut b) = (bd, bd);
        for _ in 0..60 {
            a = explicit_euler_step(a, g, 1.0 / 60.0);
            b = semi_implicit_euler_step(b, g, 1.0 / 60.0);
        }
        let that = 100.0 - 0.5 * 9.81;
        let sai_a = a.pos_value.y - that;
        let sai_b = b.pos_value.y - that;
        assert!(sai_a > 0.0, "tường minh rơi CHẬM hơn thực tế");
        assert!(sai_b < 0.0, "nửa ẩn rơi NHANH hơn thực tế");
        assert!((sai_a.abs() - sai_b.abs()).abs() < 1e-3,
                "hai sai số phải bằng nhau về độ lớn: {} vs {}", sai_a, sai_b);
    }

    #[test]
    fn semi_implicit_euler_keeps_orbit_stable() {
        // Cùng bài toán khiến Euler tường minh văng ra ngoài (xem bên dưới),
        // nửa ẩn giữ bán kính dao động trong biên hẹp — đây mới là lý do
        // thật sự khiến mọi game engine chọn nó.
        let mut t = PhysicsBody { pos_value: Vec2::new(1.0, 0.0), velocity: Vec2::new(0.0, 1.0), quantity: 1.0 };
        let mut r_lon_nhat: f32 = 0.0;
        for _ in 0..1000 {
            let huong_tam = t.pos_value.normalize().nhan(-1.0);
            t = semi_implicit_euler_step(t, huong_tam, 0.01);
            r_lon_nhat = r_lon_nhat.max(t.pos_value.length());
        }
        assert!(r_lon_nhat < 1.02, "bán kính phải bị chặn, thực tế phình tới {}", r_lon_nhat);
    }

    #[test]
    fn both_integrators_agree_on_velocity() {
        // Chỉ VỊ TRÍ khác nhau — vận tốc cập nhật giống hệt nhau.
        let bd = PhysicsBody { pos_value: Vec2::KHONG, velocity: Vec2::new(1.0, 0.0), quantity: 1.0 };
        let g = Vec2::new(0.0, -10.0);
        let a = explicit_euler_step(bd, g, 0.1);
        let b = semi_implicit_euler_step(bd, g, 0.1);
        assert_eq!(a.velocity, b.velocity);
        assert_ne!(a.pos_value, b.pos_value);
    }

    #[test]
    fn explicit_euler_injects_energy_in_circular_orbit() {
        // Bài kiểm chứng kinh điển: vật quay quanh tâm bằng lực hướng tâm.
        // Euler tường minh làm bán kính LỚN DẦN — vật văng ra ngoài.
        let mut t = PhysicsBody { pos_value: Vec2::new(1.0, 0.0), velocity: Vec2::new(0.0, 1.0), quantity: 1.0 };
        let r_dau = t.pos_value.length();
        for _ in 0..1000 {
            let huong_tam = t.pos_value.normalize().nhan(-1.0);
            t = explicit_euler_step(t, huong_tam, 0.01);
        }
        assert!(t.pos_value.length() > r_dau * 1.01,
                "bán kính phải phình ra: {} → {}", r_dau, t.pos_value.length());
    }

    // ---------- Va chạm ----------
    #[test]
    fn aabb_overlap_handles_touching_edges() {
        let a = HopReport::self_centered(Vec2::KHONG, Vec2::new(1.0, 1.0));      // [-1,1]²
        let slow_peak = HopReport::self_centered(Vec2::new(2.0, 2.0), Vec2::new(1.0, 1.0));
        let roi_nhau = HopReport::self_centered(Vec2::new(2.1, 0.0), Vec2::new(1.0, 1.0));
        assert!(a.intersect(&slow_peak), "chạm đúng một điểm vẫn tính là giao");
        assert!(!a.intersect(&roi_nhau));
    }

    #[test]
    fn overlap_is_symmetric() {
        let a = HopReport::self_centered(Vec2::new(0.0, 0.0), Vec2::new(2.0, 1.0));
        let b = HopReport::self_centered(Vec2::new(1.0, 0.5), Vec2::new(1.0, 3.0));
        assert_eq!(a.intersect(&b), b.intersect(&a));
    }

    #[test]
    fn pushout_picks_axis_of_least_overlap() {
        let a = HopReport::self_centered(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        // chồng 0.2 theo X nhưng 1.8 theo Y -> phải đẩy theo X
        let b = HopReport::self_centered(Vec2::new(1.8, 0.2), Vec2::new(1.0, 1.0));
        let d = a.day_ra(&b).expect("hai hộp có chồng lấn");
        assert!(gan_bang(d.y, 0.0), "phải đẩy theo trục X, không phải Y");
        assert!(d.x < 0.0, "a nằm bên trái nên bị đẩy sang trái");
        assert!(gan_bang(d.x.abs(), 0.2));
    }

    #[test]
    fn pushout_actually_separates_boxes() {
        let a = HopReport::self_centered(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        let b = HopReport::self_centered(Vec2::new(1.5, 0.3), Vec2::new(1.0, 1.0));
        let d = a.day_ra(&b).unwrap();
        let a_moi = HopReport { min: a.min.gate(d), max: a.max.gate(d) };
        // sau khi đẩy, hai hộp chỉ còn chạm nhau chứ không chồng lên nhau
        assert!(gan_bang(a_moi.max.x, b.min.x) || gan_bang(a_moi.min.x, b.max.x)
                || gan_bang(a_moi.max.y, b.min.y) || gan_bang(a_moi.min.y, b.max.y));
    }

    #[test]
    fn no_overlap_means_no_pushout() {
        let a = HopReport::self_centered(Vec2::KHONG, Vec2::new(1.0, 1.0));
        let xa = HopReport::self_centered(Vec2::new(50.0, 50.0), Vec2::new(1.0, 1.0));
        assert_eq!(a.day_ra(&xa), None);
    }

    #[test]
    fn circle_collision_at_exact_contact() {
        assert!(intersect_merge(Vec2::KHONG, 1.0, Vec2::new(2.0, 0.0), 1.0), "chạm nhau vừa đúng");
        assert!(!intersect_merge(Vec2::KHONG, 1.0, Vec2::new(2.01, 0.0), 1.0));
    }

    // ---------- Băm không gian ----------
    #[test]
    fn luoi_bam_cho_ket_qua_y_het_vet_can() {
        let hop: Vec<HopReport> = (0..200).map(|i| {
            let x = ((i * 37) % 100) as f32;
            let y = ((i * 53) % 100) as f32;
            HopReport::self_centered(Vec2::new(x, y), Vec2::new(2.0, 2.0))
        }).collect();
        let (qua_luoi, _) = va_cham_qua_luoi(&hop, 8.0);
        assert_eq!(qua_luoi, va_cham_vet_can(&hop),
                   "tăng tốc KHÔNG được đổi kết quả — đây là bất biến quan trọng nhất");
    }

    #[test]
    fn spatial_hash_cuts_pair_tests() {
        let hop: Vec<HopReport> = (0..400).map(|i| {
            HopReport::self_centered(Vec2::new((i % 20) as f32 * 5.0, (i / 20) as f32 * 5.0),
                           Vec2::new(1.2, 1.2))
        }).collect();
        let vet_can = hop.len() * (hop.len() - 1) / 2; // 79 800
        let (_, so_thu) = va_cham_qua_luoi(&hop, 6.0);
        assert!(so_thu * 10 < vet_can,
                "lưới băm phải cắt hơn 90% phép thử: {} so với {}", so_thu, vet_can);
    }

    #[test]
    fn spatial_hash_catches_multi_cell_bodies() {
        // Một vật RẤT LỚN trải qua nhiều ô phải va chạm được với mọi vật nhỏ.
        let mut hop = vec![HopReport::self_centered(Vec2::new(25.0, 25.0), Vec2::new(25.0, 25.0))];
        for i in 0..10 {
            hop.push(HopReport::self_centered(Vec2::new(i as f32 * 5.0, i as f32 * 5.0), Vec2::new(0.5, 0.5)));
        }
        let (qua_luoi, _) = va_cham_qua_luoi(&hop, 5.0);
        assert_eq!(qua_luoi, va_cham_vet_can(&hop), "vật lớn phải được ghi vào MỌI ô nó chạm");
    }

    #[test]
    fn no_duplicate_pairs_in_result() {
        let hop: Vec<HopReport> = (0..50).map(|i| {
            HopReport::self_centered(Vec2::new((i % 5) as f32, (i / 5) as f32), Vec2::new(3.0, 3.0))
        }).collect();
        let (kq, _) = va_cham_qua_luoi(&hop, 4.0);
        let mut sap = kq.clone();
        sap.sort_unstable();
        sap.dedup();
        assert_eq!(sap.len(), kq.len(), "một cặp chỉ được báo đúng một lần");
        assert!(kq.iter().all(|&(a, b)| a < b), "cặp phải chuẩn hóa a < b");
    }

    // ---------- ECS ----------
    #[test]
    fn entities_are_plain_ids_and_are_never_reused() {
        let mut tg = BoundedPos::new();
        let a = tg.tao();
        let b = tg.tao();
        tg.cancel(a);
        let c = tg.tao();
        assert_ne!(c, a, "ID đã hủy không được cấp lại — tránh lỗi 'con trỏ ma'");
        assert_eq!(tg.con_song, vec![b, c]);
    }

    #[test]
    fn system_touches_only_matching_entities() {
        let mut tg = BoundedPos::new();
        let dong = tg.tao();
        let compute = tg.tao();
        tg.pos_value.insert(dong, Vec2::KHONG);
        tg.velocity.insert(dong, Vec2::new(2.0, 0.0));
        tg.pos_value.insert(compute, Vec2::new(9.0, 9.0)); // KHÔNG có vận tốc
        he_thong_move(&mut tg, 1.0);
        assert_eq!(tg.pos_value[&dong], Vec2::new(2.0, 0.0));
        assert_eq!(tg.pos_value[&compute], Vec2::new(9.0, 9.0), "thiếu thành phần thì hệ thống bỏ qua");
    }

    #[test]
    fn despawn_removes_all_components() {
        let mut tg = BoundedPos::new();
        let e = tg.tao();
        tg.pos_value.insert(e, Vec2::KHONG);
        tg.velocity.insert(e, Vec2::KHONG);
        tg.mau.insert(e, 5);
        tg.cancel(e);
        assert!(!tg.pos_value.contains_key(&e) && !tg.velocity.contains_key(&e)
                && !tg.mau.contains_key(&e), "không được để lại thành phần mồ côi");
    }

    #[test]
    fn collision_deals_damage_and_reaps_dead() {
        let mut tg = BoundedPos::new();
        let strong = tg.tao();
        tg.pos_value.insert(strong, Vec2::KHONG);
        tg.ban_kinh.insert(strong, 1.0);
        tg.mau.insert(strong, 100);
        tg.contact_damage.insert(strong, 50);

        let weak = tg.tao();
        tg.pos_value.insert(weak, Vec2::new(1.0, 0.0)); // chồng lên nhau
        tg.ban_kinh.insert(weak, 1.0);
        tg.mau.insert(weak, 30);
        tg.contact_damage.insert(weak, 10);

        let chet = collision_damage_system(&mut tg);
        assert_eq!(chet, 1, "kẻ yếu phải chết");
        assert_eq!(tg.mau[&strong], 90, "kẻ mạnh mất 10 máu");
        assert!(!tg.con_song.contains(&weak));
    }

    #[test]
    fn no_collision_means_no_damage() {
        let mut tg = BoundedPos::new();
        for i in 0..3 {
            let e = tg.tao();
            tg.pos_value.insert(e, Vec2::new(i as f32 * 100.0, 0.0)); // cách xa nhau
            tg.ban_kinh.insert(e, 1.0);
            tg.mau.insert(e, 10);
            tg.contact_damage.insert(e, 99);
        }
        assert_eq!(collision_damage_system(&mut tg), 0);
        assert!(tg.mau.values().all(|&m| m == 10));
    }

    #[test]
    fn gravity_affects_every_body_with_velocity() {
        let mut tg = BoundedPos::new();
        let e = tg.tao();
        tg.velocity.insert(e, Vec2::KHONG);
        gravity_system(&mut tg, 10.0, 0.5);
        assert!(gan_bang(tg.velocity[&e].y, -5.0));
    }
}
