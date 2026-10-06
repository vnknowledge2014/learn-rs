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
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }
    pub fn dot(self, k: Vec2) -> f32 {
        self.x * k.x + self.y * k.y
    }
    /// Bình phương độ dài — dùng nó thay `length()` khi CHỈ cần so sánh,
    /// vì `sqrt` đắt và ta so sánh khoảng cách hàng nghìn lần mỗi khung hình.
    pub fn length_squared(self) -> f32 {
        self.dot(self)
    }
    pub fn length(self) -> f32 {
        self.length_squared().sqrt()
    }
    /// Chuẩn hóa an toàn: vector không thì trả về không, không sinh NaN.
    pub fn normalize(self) -> Vec2 {
        let d = self.length();
        if d < 1e-6 {
            Vec2::ZERO
        } else {
            self * (1.0 / d)
        }
    }
    /// Nội suy tuyến tính — dùng để LÀM MƯỢT hình ảnh giữa hai bước vật lý.
    pub fn lerp(self, to: Vec2, t: f32) -> Vec2 {
        self + (to - self) * t
    }
    /// Phản xạ quanh pháp tuyến — quả bóng nảy khỏi tường.
    pub fn reflect(self, normal: Vec2) -> Vec2 {
        let n = normal.normalize();
        self - n * (2.0 * self.dot(n))
    }
}

// Toán tử qua trait của `std::ops` — viết `p + v * dt` như trong sách vật lý.
impl std::ops::Add for Vec2 {
    type Output = Vec2;
    fn add(self, k: Vec2) -> Vec2 {
        Vec2::new(self.x + k.x, self.y + k.y)
    }
}
impl std::ops::Sub for Vec2 {
    type Output = Vec2;
    fn sub(self, k: Vec2) -> Vec2 {
        Vec2::new(self.x - k.x, self.y - k.y)
    }
}
impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, s: f32) -> Vec2 {
        Vec2::new(self.x * s, self.y * s)
    }
}
impl std::ops::Neg for Vec2 {
    type Output = Vec2;
    fn neg(self) -> Vec2 {
        Vec2::new(-self.x, -self.y)
    }
}

// ============================================================================
// 2. VÒNG LẶP GAME BƯỚC CỐ ĐỊNH — bài "Fix Your Timestep" kinh điển
// ============================================================================

/// Nếu để bước vật lý phụ thuộc tốc độ khung hình, cùng một trò chơi sẽ chạy
/// KHÁC NHAU trên máy mạnh và máy yếu — nhân vật xuyên tường, nhảy khác độ cao.
/// Giải pháp: tích lũy thời gian rồi chạy vật lý theo bước CỐ ĐỊNH.
pub struct Accumulator {
    pub fixed_step: f32,
    accumulated: f32,
    pub max_steps_per_frame: u32,
}

#[derive(Debug, PartialEq)]
pub struct FrameClock {
    pub physics_steps: u32,
    /// Phần dư dùng để nội suy hình ảnh — nhờ nó mà 60 bước/giây vẫn
    /// hiển thị mượt trên màn hình 144 Hz.
    pub lerp_factor: f32,
    pub steps_dropped: bool,
}

impl Accumulator {
    pub fn new(hz: f32) -> Self {
        Accumulator {
            fixed_step: 1.0 / hz,
            accumulated: 0.0,
            max_steps_per_frame: 5,
        }
    }
    pub fn advance(&mut self, real_dt: f32) -> FrameClock {
        self.accumulated += real_dt;
        let mut num_steps = 0;
        while self.accumulated >= self.fixed_step && num_steps < self.max_steps_per_frame {
            self.accumulated -= self.fixed_step;
            num_steps += 1;
        }
        // "Xoắn ốc tử thần": máy quá chậm → nợ thời gian chồng chất → càng chậm.
        // Cắt nợ để game giữ được phản hồi, chấp nhận chạy chậm hơn thời gian thật.
        let dropped = self.accumulated >= self.fixed_step;
        if dropped {
            self.accumulated = 0.0;
        }
        FrameClock {
            physics_steps: num_steps,
            lerp_factor: self.accumulated / self.fixed_step,
            steps_dropped: dropped,
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
    pub max_steps_per_frame: u32,
}

impl IntegerAccumulator {
    pub fn new(hz: u64) -> Self {
        IntegerAccumulator {
            step_nanos: 1_000_000_000 / hz,
            accumulated_nanos: 0,
            max_steps_per_frame: 5,
        }
    }
    pub fn advance(&mut self, delta_ns: u64) -> FrameClock {
        self.accumulated_nanos += delta_ns;
        let mut num_steps = 0;
        while self.accumulated_nanos >= self.step_nanos && num_steps < self.max_steps_per_frame {
            self.accumulated_nanos -= self.step_nanos;
            num_steps += 1;
        }
        let dropped = self.accumulated_nanos >= self.step_nanos;
        if dropped {
            self.accumulated_nanos = 0;
        }
        FrameClock {
            physics_steps: num_steps,
            lerp_factor: self.accumulated_nanos as f32 / self.step_nanos as f32,
            steps_dropped: dropped,
        }
    }
}

// ============================================================================
// 3. VẬT LÝ — Euler tường minh vs Euler nửa ẩn
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PhysicsBody {
    pub position: Vec2,
    pub velocity: Vec2,
    pub mass: f32,
}

/// Euler tường minh: dùng vận tốc CŨ để cập nhật vị trí. Đơn giản nhưng
/// TÍCH LŨY NĂNG LƯỢNG — quỹ đạo tròn dần biến thành xoắn ốc bay ra ngoài.
pub fn explicit_euler_step(b: PhysicsBody, accel: Vec2, dt: f32) -> PhysicsBody {
    PhysicsBody {
        position: b.position + b.velocity * dt, // dùng vận tốc CŨ
        velocity: b.velocity + accel * dt,
        ..b
    }
}

/// Euler nửa ẩn (symplectic): cập nhật vận tốc TRƯỚC rồi mới dùng nó cho vị trí.
/// Chỉ đổi thứ tự hai dòng, nhưng năng lượng được bảo toàn ổn định — đây là
/// bộ tích phân mặc định của gần như mọi game engine.
pub fn semi_implicit_euler_step(b: PhysicsBody, accel: Vec2, dt: f32) -> PhysicsBody {
    let new_velocity = b.velocity + accel * dt;
    PhysicsBody {
        position: b.position + new_velocity * dt, // dùng vận tốc MỚI
        velocity: new_velocity,
        ..b
    }
}

// ============================================================================
// 4. VA CHẠM — hình bao AABB và hình tròn
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Aabb {
    pub min: Vec2,
    pub max: Vec2,
}

impl Aabb {
    pub fn from_center(center: Vec2, half_extent: Vec2) -> Aabb {
        Aabb {
            min: center - half_extent,
            max: center + half_extent,
        }
    }
    /// Định lý trục tách: hai hộp KHÔNG chạm nhau nếu tồn tại MỘT trục mà
    /// hình chiếu của chúng rời nhau. Với AABB chỉ cần thử 2 trục X và Y.
    pub fn intersects(&self, other: &Aabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
    }
    pub fn contains_point(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn center(&self) -> Vec2 {
        (self.min + self.max) * 0.5
    }
    /// Vector đẩy tối thiểu (MTV): đẩy hộp ra khỏi nhau theo trục CHỒNG LẤN ÍT NHẤT.
    pub fn min_translation(&self, other: &Aabb) -> Option<Vec2> {
        if !self.intersects(other) {
            return None;
        }
        let overlap_x = (self.max.x - other.min.x).min(other.max.x - self.min.x);
        let overlap_y = (self.max.y - other.min.y).min(other.max.y - self.min.y);
        Some(if overlap_x < overlap_y {
            let sign = if self.center().x < other.center().x {
                -1.0
            } else {
                1.0
            };
            Vec2::new(overlap_x * sign, 0.0)
        } else {
            let sign = if self.center().y < other.center().y {
                -1.0
            } else {
                1.0
            };
            Vec2::new(0.0, overlap_y * sign)
        })
    }
}

/// Va chạm hình tròn — so BÌNH PHƯƠNG khoảng cách để né phép căn bậc hai.
pub fn circles_intersect(center_a: Vec2, radius_a: f32, center_b: Vec2, radius_b: f32) -> bool {
    let sum_r = radius_a + radius_b;
    (center_a - center_b).length_squared() <= sum_r * sum_r
}

// ============================================================================
// 5. PHÂN HOẠCH KHÔNG GIAN — từ O(n²) xuống gần O(n)
// ============================================================================

/// Kiểm tra mọi cặp là O(n²): 1 000 vật thể = 499 500 phép thử mỗi khung hình.
/// Băm không gian chia thế giới thành ô lưới; mỗi vật được ghi vào MỌI ô nó
/// chạm, nên chỉ cần so các vật CÙNG ô.
pub struct SpatialHash {
    cell_size: f32,
    cells: HashMap<(i32, i32), Vec<usize>>,
}

impl SpatialHash {
    pub fn new(cell_size: f32) -> Self {
        SpatialHash {
            cell_size,
            cells: HashMap::new(),
        }
    }
    fn cell_of(&self, p: Vec2) -> (i32, i32) {
        (
            (p.x / self.cell_size).floor() as i32,
            (p.y / self.cell_size).floor() as i32,
        )
    }
    pub fn rebuild(&mut self, boxes: &[Aabb]) {
        self.cells.clear();
        for (i, h) in boxes.iter().enumerate() {
            let (x0, y0) = self.cell_of(h.min);
            let (x1, y1) = self.cell_of(h.max);
            // Vật lớn nằm trên nhiều ô -> phải ghi vào TẤT CẢ ô nó chạm.
            for x in x0..=x1 {
                for y in y0..=y1 {
                    self.cells.entry((x, y)).or_default().push(i);
                }
            }
        }
    }
    /// Trả về các cặp CÓ THỂ va chạm (đã khử trùng lặp và sắp xếp tất định).
    pub fn candidate_pairs(&self) -> Vec<(usize, usize)> {
        let mut pairs: Vec<(usize, usize)> = Vec::new();
        for list in self.cells.values() {
            for i in 0..list.len() {
                for j in (i + 1)..list.len() {
                    let (a, b) = (list[i].min(list[j]), list[i].max(list[j]));
                    pairs.push((a, b));
                }
            }
        }
        pairs.sort_unstable();
        pairs.dedup(); // một cặp có thể xuất hiện ở nhiều ô chung
        pairs
    }
}

/// Phép so sánh chuẩn: duyệt mọi cặp. Dùng làm ĐỐI CHỨNG cho lưới băm.
pub fn brute_force_pairs(boxes: &[Aabb]) -> Vec<(usize, usize)> {
    let mut hits = Vec::new();
    for i in 0..boxes.len() {
        for j in (i + 1)..boxes.len() {
            if boxes[i].intersects(&boxes[j]) {
                hits.push((i, j));
            }
        }
    }
    hits
}

/// Trả về (các cặp va chạm thật, số phép thử đã làm).
pub fn grid_pairs(boxes: &[Aabb], cell_size: f32) -> (Vec<(usize, usize)>, usize) {
    let mut grid = SpatialHash::new(cell_size);
    grid.rebuild(boxes);
    let candidates = grid.candidate_pairs();
    let tests = candidates.len();
    let hits: Vec<(usize, usize)> = candidates
        .into_iter()
        .filter(|&(a, b)| boxes[a].intersects(&boxes[b]))
        .collect();
    (hits, tests)
}

// ============================================================================
// 6. ECS — Thực thể · Thành phần · Hệ thống
// ============================================================================
// Ý tưởng cốt lõi: KHÔNG dùng kế thừa ("Quái vật kế thừa Sinh vật kế thừa
// Thực thể"). Thay vào đó, thực thể chỉ là một CON SỐ; dữ liệu nằm trong các
// mảng song song. Hệ thống duyệt mảng liên tiếp trong bộ nhớ -> cache CPU
// hoạt động hết công suất. Đây là "thiết kế hướng dữ liệu".

pub type Entity = u32;

#[derive(Debug, Default)]
pub struct World {
    next: Entity,
    pub alive: Vec<Entity>,
    pub position: HashMap<Entity, Vec2>,
    pub velocity: HashMap<Entity, Vec2>,
    pub health: HashMap<Entity, i32>,
    pub contact_damage: HashMap<Entity, i32>,
    pub radius: HashMap<Entity, f32>,
}

impl World {
    pub fn new() -> Self {
        World::default()
    }

    pub fn spawn(&mut self) -> Entity {
        let e = self.next;
        self.next += 1;
        self.alive.push(e);
        e
    }
    pub fn despawn(&mut self, e: Entity) {
        self.alive.retain(|&x| x != e);
        self.position.remove(&e);
        self.velocity.remove(&e);
        self.health.remove(&e);
        self.contact_damage.remove(&e);
        self.radius.remove(&e);
    }
    /// Truy vấn: các thực thể có ĐỦ cả vị trí lẫn vận tốc.
    /// Trong ECS thật, đây là chỗ dùng "archetype" để duyệt liên tiếp.
    pub fn has_position_and_velocity(&self) -> Vec<Entity> {
        let mut v: Vec<Entity> = self
            .alive
            .iter()
            .copied()
            .filter(|e| self.position.contains_key(e) && self.velocity.contains_key(e))
            .collect();
        v.sort_unstable(); // tất định — điều kiện tiên quyết để kiểm thử được
        v
    }
}

/// HỆ THỐNG là hàm thuần túy về mặt logic: `&mut World` vào, thế giới đổi ra.
/// Mỗi hệ thống chỉ đụng đúng những thành phần nó cần.
pub fn movement_system(w: &mut World, dt: f32) {
    for e in w.has_position_and_velocity() {
        let v = w.velocity[&e];
        if let Some(p) = w.position.get_mut(&e) {
            *p = *p + v * dt;
        }
    }
}

pub fn gravity_system(w: &mut World, g: f32, dt: f32) {
    for v in w.velocity.values_mut() {
        v.y -= g * dt;
    }
}

/// Va chạm gây sát thương, rồi thu dọn xác. Trả về số thực thể đã chết.
pub fn collision_damage_system(w: &mut World) -> usize {
    let list: Vec<Entity> = {
        let mut v: Vec<Entity> = w
            .alive
            .iter()
            .copied()
            .filter(|e| w.position.contains_key(e) && w.radius.contains_key(e))
            .collect();
        v.sort_unstable();
        v
    };
    let mut damage: HashMap<Entity, i32> = HashMap::new();
    for i in 0..list.len() {
        for j in (i + 1)..list.len() {
            let (a, b) = (list[i], list[j]);
            if circles_intersect(w.position[&a], w.radius[&a], w.position[&b], w.radius[&b]) {
                if let Some(&dmg) = w.contact_damage.get(&a) {
                    *damage.entry(b).or_insert(0) += dmg;
                }
                if let Some(&dmg) = w.contact_damage.get(&b) {
                    *damage.entry(a).or_insert(0) += dmg;
                }
            }
        }
    }
    for (e, dmg) in damage {
        if let Some(hp) = w.health.get_mut(&e) {
            *hp -= dmg;
        }
    }
    let dead: Vec<Entity> = w
        .alive
        .iter()
        .copied()
        .filter(|e| w.health.get(e).is_some_and(|&hp| hp <= 0))
        .collect();
    for e in &dead {
        w.despawn(*e);
    }
    dead.len()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LẬP TRÌNH GAME: VÒNG LẶP · VẬT LÝ · VA CHẠM · ECS        ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÒNG LẶP BƯỚC CỐ ĐỊNH 60 Hz");
    let mut acc = Accumulator::new(60.0);
    for (name, dt) in [
        ("máy mạnh 144 fps", 1.0 / 144.0),
        ("máy yếu 30 fps", 1.0 / 30.0),
        ("khựng 0.5 giây", 0.5),
    ] {
        let n = acc.advance(dt);
        println!(
            "   {:<18} → {} bước vật lý, nội suy {:.2}{}",
            name,
            n.physics_steps,
            n.lerp_factor,
            if n.steps_dropped {
                "  ⚠ cắt nợ để tránh xoắn ốc tử thần"
            } else {
                ""
            }
        );
    }

    println!("\n2. HAI BỘ TÍCH PHÂN — vật rơi tự do 1 giây, dt = 1/60");
    let start = PhysicsBody {
        position: Vec2::new(0.0, 100.0),
        velocity: Vec2::ZERO,
        mass: 1.0,
    };
    let g = Vec2::new(0.0, -9.81);
    let (mut a, mut b) = (start, start);
    for _ in 0..60 {
        a = explicit_euler_step(a, g, 1.0 / 60.0);
        b = semi_implicit_euler_step(b, g, 1.0 / 60.0);
    }
    let exact = 100.0 - 0.5 * 9.81;
    println!("   Nghiệm giải tích : y = {:.4}", exact);
    println!(
        "   Euler tường minh : y = {:.4} (lệch {:.4})",
        a.position.y,
        (a.position.y - exact).abs()
    );
    println!(
        "   Euler nửa ẩn     : y = {:.4} (lệch {:.4})",
        b.position.y,
        (b.position.y - exact).abs()
    );

    println!("\n3. VA CHẠM & VECTOR ĐẨY TỐI THIỂU");
    let h1 = Aabb::from_center(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
    let h2 = Aabb::from_center(Vec2::new(1.5, 0.2), Vec2::new(1.0, 1.0));
    println!(
        "   Hai hộp chồng nhau: {} | đẩy ra: {:?}",
        h1.intersects(&h2),
        h1.min_translation(&h2)
    );
    println!(
        "   Bóng bay (1,-1) đập sàn (pháp tuyến 0,1) → {:?}",
        Vec2::new(1.0, -1.0).reflect(Vec2::new(0.0, 1.0))
    );

    println!("\n4. BĂM KHÔNG GIAN — 400 vật thể rải trên lưới 100×100");
    let boxes: Vec<Aabb> = (0..400)
        .map(|i| {
            let x = (i % 20) as f32 * 5.0;
            let y = (i / 20) as f32 * 5.0;
            Aabb::from_center(Vec2::new(x, y), Vec2::new(1.2, 1.2))
        })
        .collect();
    let brute = brute_force_pairs(&boxes);
    let (via_grid, tests) = grid_pairs(&boxes, 6.0);
    let all_pairs = boxes.len() * (boxes.len() - 1) / 2;
    println!(
        "   Vét cạn : {} phép thử → {} va chạm",
        all_pairs,
        brute.len()
    );
    println!(
        "   Lưới băm: {} phép thử → {} va chạm",
        tests,
        via_grid.len()
    );
    println!(
        "   Cùng kết quả: {} | giảm {:.0}% khối lượng tính toán",
        brute == via_grid,
        100.0 - tests as f64 * 100.0 / all_pairs as f64
    );

    println!("\n5. ECS — 1 người chơi, 3 quái, mô phỏng 3 khung hình");
    let mut w = World::new();
    let player = w.spawn();
    w.position.insert(player, Vec2::new(0.0, 0.0));
    w.velocity.insert(player, Vec2::new(1.0, 0.0));
    w.health.insert(player, 100);
    w.radius.insert(player, 1.0);
    for i in 0..3 {
        let q = w.spawn();
        w.position.insert(q, Vec2::new(2.0 + i as f32 * 0.5, 0.0));
        w.health.insert(q, 10);
        w.radius.insert(q, 1.0);
        w.contact_damage.insert(q, 4);
    }
    w.contact_damage.insert(player, 6);
    for frame in 1..=3 {
        movement_system(&mut w, 1.0);
        let dead = collision_damage_system(&mut w);
        println!(
            "   Khung {}: người chơi ở x={:.1} · máu {:?} · {} thực thể chết · còn {} sống",
            frame,
            w.position.get(&player).map_or(0.0, |p| p.x),
            w.health.get(&player),
            dead,
            w.alive.len()
        );
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GAME = MỘT HÀM THUẦN TÚY CHẠY 60 LẦN MỖI GIÂY            ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-4
    }

    // ---------- Vector ----------
    #[test]
    fn normalizing_zero_vector_avoids_nan() {
        let v = Vec2::ZERO.normalize();
        assert_eq!(v, Vec2::ZERO, "chia cho 0 phải bị chặn, không được ra NaN");
        assert!(!v.x.is_nan() && !v.y.is_nan());
    }

    #[test]
    fn normalize_yields_unit_length() {
        for v in [
            Vec2::new(3.0, 4.0),
            Vec2::new(-7.0, 0.5),
            Vec2::new(0.0, -2.0),
        ] {
            assert!(approx_eq(v.normalize().length(), 1.0));
        }
    }

    #[test]
    fn length_squared_matches_length() {
        let v = Vec2::new(3.0, 4.0);
        assert!(approx_eq(v.length(), 5.0));
        assert!(approx_eq(v.length_squared(), 25.0));
    }

    #[test]
    fn reflect_preserves_magnitude_and_flips_axis() {
        let incoming = Vec2::new(1.0, -1.0);
        let outgoing = incoming.reflect(Vec2::new(0.0, 1.0));
        assert!(
            approx_eq(outgoing.x, 1.0),
            "thành phần song song mặt phẳng giữ nguyên"
        );
        assert!(approx_eq(outgoing.y, 1.0), "thành phần vuông góc đổi dấu");
        assert!(
            approx_eq(outgoing.length(), incoming.length()),
            "va chạm đàn hồi giữ nguyên tốc độ"
        );
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
        let mut acc = Accumulator::new(60.0);
        acc.max_steps_per_frame = 1000;
        let total: u32 = (0..144)
            .map(|_| acc.advance(1.0 / 144.0).physics_steps)
            .sum();
        assert_eq!(
            total, 59,
            "đáng lẽ 60 — một bước bị nuốt mất vì trôi dấu phẩy động"
        );
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
            let mut acc = IntegerAccumulator::new(60);
            acc.max_steps_per_frame = 1000;
            let timestamp = |i: u64| i * 1_000_000_000 / fps; // mốc tuyệt đối, chính xác
            let total: u32 = (1..=fps)
                .map(|i| acc.advance(timestamp(i) - timestamp(i - 1)).physics_steps)
                .sum();
            assert_eq!(total, 60, "ở {} fps vẫn phải đúng 60 bước", fps);
        }
    }

    #[test]
    fn integer_accumulator_also_clamps_on_long_frames() {
        let mut acc = IntegerAccumulator::new(60);
        let n = acc.advance(2_000_000_000); // khựng 2 giây
        assert_eq!(n.physics_steps, 5);
        assert!(n.steps_dropped);
        assert_eq!(
            acc.advance(16_666_666).physics_steps,
            1,
            "không mang nợ sang khung sau"
        );
    }

    #[test]
    fn lerp_factor_stays_in_unit_range() {
        let mut acc = Accumulator::new(60.0);
        for i in 0..200 {
            let n = acc.advance(0.001 * (i % 37) as f32);
            assert!(
                (0.0..1.0).contains(&n.lerp_factor),
                "hệ số nội suy {} nằm ngoài [0,1)",
                n.lerp_factor
            );
        }
    }

    #[test]
    fn clamping_dt_avoids_death_spiral() {
        let mut acc = Accumulator::new(60.0);
        let n = acc.advance(2.0); // khựng 2 giây = đáng lẽ 120 bước
        assert_eq!(n.physics_steps, 5, "bị chặn ở trần 5 bước");
        assert!(n.steps_dropped);
        // Khung sau phải trở lại bình thường, không mang theo nợ
        let next = acc.advance(1.0 / 60.0);
        assert_eq!(
            next.physics_steps, 1,
            "nợ đã bị cắt, không dồn sang khung sau"
        );
    }

    // ---------- Vật lý ----------
    #[test]
    fn under_constant_accel_both_integrators_err_symmetrically() {
        // Kết quả có thể gây bất ngờ: khi gia tốc KHÔNG ĐỔI, Euler nửa ẩn
        // KHÔNG chính xác hơn. Hai bộ lệch đúng bằng nhau — một cái vượt,
        // một cái hụt — vì sai số đều là 0.5·g·dt·T (T = tổng thời gian mô phỏng).
        // Ưu thế của nửa ẩn nằm ở chỗ khác: sự ỔN ĐỊNH của hệ dao động,
        // xem bài kiểm thử quỹ đạo tròn ngay bên dưới.
        let start = PhysicsBody {
            position: Vec2::new(0.0, 100.0),
            velocity: Vec2::ZERO,
            mass: 1.0,
        };
        let g = Vec2::new(0.0, -9.81);
        let (mut a, mut b) = (start, start);
        for _ in 0..60 {
            a = explicit_euler_step(a, g, 1.0 / 60.0);
            b = semi_implicit_euler_step(b, g, 1.0 / 60.0);
        }
        let exact = 100.0 - 0.5 * 9.81;
        let err_a = a.position.y - exact;
        let err_b = b.position.y - exact;
        assert!(err_a > 0.0, "tường minh rơi CHẬM hơn thực tế");
        assert!(err_b < 0.0, "nửa ẩn rơi NHANH hơn thực tế");
        assert!(
            (err_a.abs() - err_b.abs()).abs() < 1e-3,
            "hai sai số phải bằng nhau về độ lớn: {} vs {}",
            err_a,
            err_b
        );
    }

    #[test]
    fn semi_implicit_euler_keeps_orbit_stable() {
        // Cùng bài toán khiến Euler tường minh văng ra ngoài (xem bên dưới),
        // nửa ẩn giữ bán kính dao động trong biên hẹp — đây mới là lý do
        // thật sự khiến mọi game engine chọn nó.
        let mut t = PhysicsBody {
            position: Vec2::new(1.0, 0.0),
            velocity: Vec2::new(0.0, 1.0),
            mass: 1.0,
        };
        let mut max_radius: f32 = 0.0;
        for _ in 0..1000 {
            let toward_center = -t.position.normalize();
            t = semi_implicit_euler_step(t, toward_center, 0.01);
            max_radius = max_radius.max(t.position.length());
        }
        assert!(
            max_radius < 1.02,
            "bán kính phải bị chặn, thực tế phình tới {}",
            max_radius
        );
    }

    #[test]
    fn both_integrators_agree_on_velocity() {
        // Chỉ VỊ TRÍ khác nhau — vận tốc cập nhật giống hệt nhau.
        let start = PhysicsBody {
            position: Vec2::ZERO,
            velocity: Vec2::new(1.0, 0.0),
            mass: 1.0,
        };
        let g = Vec2::new(0.0, -10.0);
        let a = explicit_euler_step(start, g, 0.1);
        let b = semi_implicit_euler_step(start, g, 0.1);
        assert_eq!(a.velocity, b.velocity);
        assert_ne!(a.position, b.position);
    }

    #[test]
    fn explicit_euler_injects_energy_in_circular_orbit() {
        // Bài kiểm chứng kinh điển: vật quay quanh tâm bằng lực hướng tâm.
        // Euler tường minh làm bán kính LỚN DẦN — vật văng ra ngoài.
        let mut t = PhysicsBody {
            position: Vec2::new(1.0, 0.0),
            velocity: Vec2::new(0.0, 1.0),
            mass: 1.0,
        };
        let r0 = t.position.length();
        for _ in 0..1000 {
            let toward_center = -t.position.normalize();
            t = explicit_euler_step(t, toward_center, 0.01);
        }
        assert!(
            t.position.length() > r0 * 1.01,
            "bán kính phải phình ra: {} → {}",
            r0,
            t.position.length()
        );
    }

    // ---------- Va chạm ----------
    #[test]
    fn aabb_overlap_handles_touching_edges() {
        let a = Aabb::from_center(Vec2::ZERO, Vec2::new(1.0, 1.0)); // [-1,1]²
        let corner_touch = Aabb::from_center(Vec2::new(2.0, 2.0), Vec2::new(1.0, 1.0));
        let apart = Aabb::from_center(Vec2::new(2.1, 0.0), Vec2::new(1.0, 1.0));
        assert!(
            a.intersects(&corner_touch),
            "chạm đúng một điểm vẫn tính là giao"
        );
        assert!(!a.intersects(&apart));
    }

    #[test]
    fn overlap_is_symmetric() {
        let a = Aabb::from_center(Vec2::new(0.0, 0.0), Vec2::new(2.0, 1.0));
        let b = Aabb::from_center(Vec2::new(1.0, 0.5), Vec2::new(1.0, 3.0));
        assert_eq!(a.intersects(&b), b.intersects(&a));
    }

    #[test]
    fn pushout_picks_axis_of_least_overlap() {
        let a = Aabb::from_center(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        // chồng 0.2 theo X nhưng 1.8 theo Y -> phải đẩy theo X
        let b = Aabb::from_center(Vec2::new(1.8, 0.2), Vec2::new(1.0, 1.0));
        let d = a.min_translation(&b).expect("hai hộp có chồng lấn");
        assert!(approx_eq(d.y, 0.0), "phải đẩy theo trục X, không phải Y");
        assert!(d.x < 0.0, "a nằm bên trái nên bị đẩy sang trái");
        assert!(approx_eq(d.x.abs(), 0.2));
    }

    #[test]
    fn pushout_actually_separates_boxes() {
        let a = Aabb::from_center(Vec2::new(0.0, 0.0), Vec2::new(1.0, 1.0));
        let b = Aabb::from_center(Vec2::new(1.5, 0.3), Vec2::new(1.0, 1.0));
        let d = a.min_translation(&b).unwrap();
        let moved = Aabb {
            min: a.min + d,
            max: a.max + d,
        };
        // sau khi đẩy, hai hộp chỉ còn chạm nhau chứ không chồng lên nhau
        assert!(
            approx_eq(moved.max.x, b.min.x)
                || approx_eq(moved.min.x, b.max.x)
                || approx_eq(moved.max.y, b.min.y)
                || approx_eq(moved.min.y, b.max.y)
        );
    }

    #[test]
    fn no_overlap_means_no_pushout() {
        let a = Aabb::from_center(Vec2::ZERO, Vec2::new(1.0, 1.0));
        let far = Aabb::from_center(Vec2::new(50.0, 50.0), Vec2::new(1.0, 1.0));
        assert_eq!(a.min_translation(&far), None);
    }

    #[test]
    fn circle_collision_at_exact_contact() {
        assert!(
            circles_intersect(Vec2::ZERO, 1.0, Vec2::new(2.0, 0.0), 1.0),
            "chạm nhau vừa đúng"
        );
        assert!(!circles_intersect(
            Vec2::ZERO,
            1.0,
            Vec2::new(2.01, 0.0),
            1.0
        ));
    }

    // ---------- Băm không gian ----------
    #[test]
    fn spatial_hash_matches_brute_force() {
        let boxes: Vec<Aabb> = (0..200)
            .map(|i| {
                let x = ((i * 37) % 100) as f32;
                let y = ((i * 53) % 100) as f32;
                Aabb::from_center(Vec2::new(x, y), Vec2::new(2.0, 2.0))
            })
            .collect();
        let (via_grid, _) = grid_pairs(&boxes, 8.0);
        assert_eq!(
            via_grid,
            brute_force_pairs(&boxes),
            "tăng tốc KHÔNG được đổi kết quả — đây là bất biến quan trọng nhất"
        );
    }

    #[test]
    fn spatial_hash_cuts_pair_tests() {
        let boxes: Vec<Aabb> = (0..400)
            .map(|i| {
                Aabb::from_center(
                    Vec2::new((i % 20) as f32 * 5.0, (i / 20) as f32 * 5.0),
                    Vec2::new(1.2, 1.2),
                )
            })
            .collect();
        let brute = boxes.len() * (boxes.len() - 1) / 2; // 79 800
        let (_, tests) = grid_pairs(&boxes, 6.0);
        assert!(
            tests * 10 < brute,
            "lưới băm phải cắt hơn 90% phép thử: {} so với {}",
            tests,
            brute
        );
    }

    #[test]
    fn spatial_hash_catches_multi_cell_bodies() {
        // Một vật RẤT LỚN trải qua nhiều ô phải va chạm được với mọi vật nhỏ.
        let mut boxes = vec![Aabb::from_center(
            Vec2::new(25.0, 25.0),
            Vec2::new(25.0, 25.0),
        )];
        for i in 0..10 {
            boxes.push(Aabb::from_center(
                Vec2::new(i as f32 * 5.0, i as f32 * 5.0),
                Vec2::new(0.5, 0.5),
            ));
        }
        let (via_grid, _) = grid_pairs(&boxes, 5.0);
        assert_eq!(
            via_grid,
            brute_force_pairs(&boxes),
            "vật lớn phải được ghi vào MỌI ô nó chạm"
        );
    }

    #[test]
    fn no_duplicate_pairs_in_result() {
        let boxes: Vec<Aabb> = (0..50)
            .map(|i| {
                Aabb::from_center(
                    Vec2::new((i % 5) as f32, (i / 5) as f32),
                    Vec2::new(3.0, 3.0),
                )
            })
            .collect();
        let (pairs, _) = grid_pairs(&boxes, 4.0);
        let mut unique = pairs.clone();
        unique.sort_unstable();
        unique.dedup();
        assert_eq!(
            unique.len(),
            pairs.len(),
            "một cặp chỉ được báo đúng một lần"
        );
        assert!(
            pairs.iter().all(|&(a, b)| a < b),
            "cặp phải chuẩn hóa a < b"
        );
    }

    // ---------- ECS ----------
    #[test]
    fn entities_are_plain_ids_and_are_never_reused() {
        let mut w = World::new();
        let a = w.spawn();
        let b = w.spawn();
        w.despawn(a);
        let c = w.spawn();
        assert_ne!(
            c, a,
            "ID đã hủy không được cấp lại — tránh lỗi 'con trỏ ma'"
        );
        assert_eq!(w.alive, vec![b, c]);
    }

    #[test]
    fn system_touches_only_matching_entities() {
        let mut w = World::new();
        let mover = w.spawn();
        let statue = w.spawn();
        w.position.insert(mover, Vec2::ZERO);
        w.velocity.insert(mover, Vec2::new(2.0, 0.0));
        w.position.insert(statue, Vec2::new(9.0, 9.0)); // KHÔNG có vận tốc
        movement_system(&mut w, 1.0);
        assert_eq!(w.position[&mover], Vec2::new(2.0, 0.0));
        assert_eq!(
            w.position[&statue],
            Vec2::new(9.0, 9.0),
            "thiếu thành phần thì hệ thống bỏ qua"
        );
    }

    #[test]
    fn despawn_removes_all_components() {
        let mut w = World::new();
        let e = w.spawn();
        w.position.insert(e, Vec2::ZERO);
        w.velocity.insert(e, Vec2::ZERO);
        w.health.insert(e, 5);
        w.despawn(e);
        assert!(
            !w.position.contains_key(&e)
                && !w.velocity.contains_key(&e)
                && !w.health.contains_key(&e),
            "không được để lại thành phần mồ côi"
        );
    }

    #[test]
    fn collision_deals_damage_and_reaps_dead() {
        let mut w = World::new();
        let strong = w.spawn();
        w.position.insert(strong, Vec2::ZERO);
        w.radius.insert(strong, 1.0);
        w.health.insert(strong, 100);
        w.contact_damage.insert(strong, 50);

        let weak = w.spawn();
        w.position.insert(weak, Vec2::new(1.0, 0.0)); // chồng lên nhau
        w.radius.insert(weak, 1.0);
        w.health.insert(weak, 30);
        w.contact_damage.insert(weak, 10);

        let dead = collision_damage_system(&mut w);
        assert_eq!(dead, 1, "kẻ yếu phải chết");
        assert_eq!(w.health[&strong], 90, "kẻ mạnh mất 10 máu");
        assert!(!w.alive.contains(&weak));
    }

    #[test]
    fn no_collision_means_no_damage() {
        let mut w = World::new();
        for i in 0..3 {
            let e = w.spawn();
            w.position.insert(e, Vec2::new(i as f32 * 100.0, 0.0)); // cách xa nhau
            w.radius.insert(e, 1.0);
            w.health.insert(e, 10);
            w.contact_damage.insert(e, 99);
        }
        assert_eq!(collision_damage_system(&mut w), 0);
        assert!(w.health.values().all(|&m| m == 10));
    }

    #[test]
    fn gravity_affects_every_body_with_velocity() {
        let mut w = World::new();
        let e = w.spawn();
        w.velocity.insert(e, Vec2::ZERO);
        gravity_system(&mut w, 10.0, 0.5);
        assert!(approx_eq(w.velocity[&e].y, -5.0));
    }
}
