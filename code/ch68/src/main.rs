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
    pub fn moi(x: f32, y: f32) -> Vec2 { Vec2 { x, y } }
    pub fn cong(self, k: Vec2) -> Vec2 { Vec2::moi(self.x + k.x, self.y + k.y) }
    pub fn tru(self, k: Vec2) -> Vec2 { Vec2::moi(self.x - k.x, self.y - k.y) }
    pub fn nhan(self, s: f32) -> Vec2 { Vec2::moi(self.x * s, self.y * s) }
    pub fn tich_vo_huong(self, k: Vec2) -> f32 { self.x * k.x + self.y * k.y }
    /// Bình phương độ dài — dùng nó thay `do_dai()` khi CHỈ cần so sánh,
    /// vì `sqrt` đắt và ta so sánh khoảng cách hàng nghìn lần mỗi khung hình.
    pub fn do_dai_binh_phuong(self) -> f32 { self.tich_vo_huong(self) }
    pub fn do_dai(self) -> f32 { self.do_dai_binh_phuong().sqrt() }
    /// Chuẩn hóa an toàn: vector không thì trả về không, không sinh NaN.
    pub fn chuan_hoa(self) -> Vec2 {
        let d = self.do_dai();
        if d < 1e-6 { Vec2::KHONG } else { self.nhan(1.0 / d) }
    }
    /// Nội suy tuyến tính — dùng để LÀM MƯỢT hình ảnh giữa hai bước vật lý.
    pub fn noi_suy(self, den: Vec2, t: f32) -> Vec2 {
        self.cong(den.tru(self).nhan(t))
    }
    /// Phản xạ quanh pháp tuyến — quả bóng nảy khỏi tường.
    pub fn phan_xa(self, phap_tuyen: Vec2) -> Vec2 {
        let n = phap_tuyen.chuan_hoa();
        self.tru(n.nhan(2.0 * self.tich_vo_huong(n)))
    }
}

// ============================================================================
// 2. VÒNG LẶP GAME BƯỚC CỐ ĐỊNH — bài "Fix Your Timestep" kinh điển
// ============================================================================

/// Nếu để bước vật lý phụ thuộc tốc độ khung hình, cùng một trò chơi sẽ chạy
/// KHÁC NHAU trên máy mạnh và máy yếu — nhân vật xuyên tường, nhảy khác độ cao.
/// Giải pháp: tích lũy thời gian rồi chạy vật lý theo bước CỐ ĐỊNH.
pub struct BoTichLuy {
    pub buoc_co_dinh: f32,
    tich_luy: f32,
    pub toi_da_buoc_mot_khung: u32,
}

#[derive(Debug, PartialEq)]
pub struct NhipKhung {
    pub so_buoc_vat_ly: u32,
    /// Phần dư dùng để nội suy hình ảnh — nhờ nó mà 60 bước/giây vẫn
    /// hiển thị mượt trên màn hình 144 Hz.
    pub he_so_noi_suy: f32,
    pub bi_bo_buoc: bool,
}

impl BoTichLuy {
    pub fn moi(hz: f32) -> Self {
        BoTichLuy { buoc_co_dinh: 1.0 / hz, tich_luy: 0.0, toi_da_buoc_mot_khung: 5 }
    }
    pub fn khung_moi(&mut self, delta_thuc: f32) -> NhipKhung {
        self.tich_luy += delta_thuc;
        let mut so_buoc = 0;
        while self.tich_luy >= self.buoc_co_dinh && so_buoc < self.toi_da_buoc_mot_khung {
            self.tich_luy -= self.buoc_co_dinh;
            so_buoc += 1;
        }
        // "Xoắn ốc tử thần": máy quá chậm → nợ thời gian chồng chất → càng chậm.
        // Cắt nợ để game giữ được phản hồi, chấp nhận chạy chậm hơn thời gian thật.
        let bi_bo = self.tich_luy >= self.buoc_co_dinh;
        if bi_bo { self.tich_luy = 0.0; }
        NhipKhung {
            so_buoc_vat_ly: so_buoc,
            he_so_noi_suy: self.tich_luy / self.buoc_co_dinh,
            bi_bo_buoc: bi_bo,
        }
    }
}

/// PHIÊN BẢN CHỐNG TRÔI: đếm thời gian bằng NANO-GIÂY nguyên thay vì `f32`.
///
/// Cộng dồn 144 lần `1.0/144.0` kiểu `f32` KHÔNG cho ra đúng 1.0 — sai số nhị
/// phân tích lũy làm mất hẳn một bước vật lý mỗi giây. Với game nhiều người
/// chơi hay bản phát lại (replay), một bước lệch là hỏng toàn bộ tính tất định.
/// Số nguyên không có sai số làm tròn, nên phép cộng là chính xác tuyệt đối.
pub struct BoTichLuyNguyen {
    pub buoc_ns: u64,
    tich_luy_ns: u64,
    pub toi_da_buoc_mot_khung: u32,
}

impl BoTichLuyNguyen {
    pub fn moi(hz: u64) -> Self {
        BoTichLuyNguyen { buoc_ns: 1_000_000_000 / hz, tich_luy_ns: 0, toi_da_buoc_mot_khung: 5 }
    }
    pub fn khung_moi(&mut self, delta_ns: u64) -> NhipKhung {
        self.tich_luy_ns += delta_ns;
        let mut so_buoc = 0;
        while self.tich_luy_ns >= self.buoc_ns && so_buoc < self.toi_da_buoc_mot_khung {
            self.tich_luy_ns -= self.buoc_ns;
            so_buoc += 1;
        }
        let bi_bo = self.tich_luy_ns >= self.buoc_ns;
        if bi_bo { self.tich_luy_ns = 0; }
        NhipKhung {
            so_buoc_vat_ly: so_buoc,
            he_so_noi_suy: self.tich_luy_ns as f32 / self.buoc_ns as f32,
            bi_bo_buoc: bi_bo,
        }
    }
}

// ============================================================================
// 3. VẬT LÝ — Euler tường minh vs Euler nửa ẩn
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TheVatLy {
    pub vi_tri: Vec2,
    pub van_toc: Vec2,
    pub khoi_luong: f32,
}

/// Euler tường minh: dùng vận tốc CŨ để cập nhật vị trí. Đơn giản nhưng
/// TÍCH LŨY NĂNG LƯỢNG — quỹ đạo tròn dần biến thành xoắn ốc bay ra ngoài.
pub fn buoc_euler_tuong_minh(t: TheVatLy, gia_toc: Vec2, dt: f32) -> TheVatLy {
    TheVatLy {
        vi_tri: t.vi_tri.cong(t.van_toc.nhan(dt)),      // dùng vận tốc CŨ
        van_toc: t.van_toc.cong(gia_toc.nhan(dt)),
        ..t
    }
}

/// Euler nửa ẩn (symplectic): cập nhật vận tốc TRƯỚC rồi mới dùng nó cho vị trí.
/// Chỉ đổi thứ tự hai dòng, nhưng năng lượng được bảo toàn ổn định — đây là
/// bộ tích phân mặc định của gần như mọi game engine.
pub fn buoc_euler_nua_an(t: TheVatLy, gia_toc: Vec2, dt: f32) -> TheVatLy {
    let van_toc_moi = t.van_toc.cong(gia_toc.nhan(dt));
    TheVatLy {
        vi_tri: t.vi_tri.cong(van_toc_moi.nhan(dt)),    // dùng vận tốc MỚI
        van_toc: van_toc_moi,
        ..t
    }
}

// ============================================================================
// 4. VA CHẠM — hình bao AABB và hình tròn
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HopBao { pub min: Vec2, pub max: Vec2 }

impl HopBao {
    pub fn tu_tam(tam: Vec2, nua_kich_thuoc: Vec2) -> HopBao {
        HopBao { min: tam.tru(nua_kich_thuoc), max: tam.cong(nua_kich_thuoc) }
    }
    /// Định lý trục tách: hai hộp KHÔNG chạm nhau nếu tồn tại MỘT trục mà
    /// hình chiếu của chúng rời nhau. Với AABB chỉ cần thử 2 trục X và Y.
    pub fn giao_nhau(&self, k: &HopBao) -> bool {
        self.min.x <= k.max.x && self.max.x >= k.min.x &&
        self.min.y <= k.max.y && self.max.y >= k.min.y
    }
    pub fn chua_diem(&self, p: Vec2) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn tam(&self) -> Vec2 { self.min.cong(self.max).nhan(0.5) }
    /// Vector đẩy tối thiểu: đẩy hộp ra khỏi nhau theo trục CHỒNG LẤN ÍT NHẤT.
    pub fn day_ra(&self, k: &HopBao) -> Option<Vec2> {
        if !self.giao_nhau(k) { return None; }
        let chong_x = (self.max.x - k.min.x).min(k.max.x - self.min.x);
        let chong_y = (self.max.y - k.min.y).min(k.max.y - self.min.y);
        Some(if chong_x < chong_y {
            let dau = if self.tam().x < k.tam().x { -1.0 } else { 1.0 };
            Vec2::moi(chong_x * dau, 0.0)
        } else {
            let dau = if self.tam().y < k.tam().y { -1.0 } else { 1.0 };
            Vec2::moi(0.0, chong_y * dau)
        })
    }
}

/// Va chạm hình tròn — so BÌNH PHƯƠNG khoảng cách để né phép căn bậc hai.
pub fn tron_giao_nhau(tam_a: Vec2, ban_kinh_a: f32, tam_b: Vec2, ban_kinh_b: f32) -> bool {
    let tong_bk = ban_kinh_a + ban_kinh_b;
    tam_a.tru(tam_b).do_dai_binh_phuong() <= tong_bk * tong_bk
}

// ============================================================================
// 5. PHÂN HOẠCH KHÔNG GIAN — từ O(n²) xuống gần O(n)
// ============================================================================

/// Kiểm tra mọi cặp là O(n²): 1 000 vật thể = 499 500 phép thử mỗi khung hình.
/// Băm không gian chia thế giới thành ô lưới; chỉ so các vật CÙNG ô hoặc ô kề.
pub struct LuoiBam {
    kich_thuoc_o: f32,
    o: HashMap<(i32, i32), Vec<usize>>,
}

impl LuoiBam {
    pub fn moi(kich_thuoc_o: f32) -> Self {
        LuoiBam { kich_thuoc_o, o: HashMap::new() }
    }
    fn toa_do_o(&self, p: Vec2) -> (i32, i32) {
        ((p.x / self.kich_thuoc_o).floor() as i32, (p.y / self.kich_thuoc_o).floor() as i32)
    }
    pub fn xay_dung(&mut self, hop: &[HopBao]) {
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
    pub fn cac_cap_kha_nghi(&self) -> Vec<(usize, usize)> {
        let mut cap: Vec<(usize, usize)> = Vec::new();
        for ds in self.o.values() {
            for i in 0..ds.len() {
                for j in (i + 1)..ds.len() {
                    let (a, b) = (ds[i].min(ds[j]), ds[i].max(ds[j]));
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
pub fn va_cham_vet_can(hop: &[HopBao]) -> Vec<(usize, usize)> {
    let mut kq = Vec::new();
    for i in 0..hop.len() {
        for j in (i + 1)..hop.len() {
            if hop[i].giao_nhau(&hop[j]) { kq.push((i, j)); }
        }
    }
    kq
}

pub fn va_cham_qua_luoi(hop: &[HopBao], kich_thuoc_o: f32) -> (Vec<(usize, usize)>, usize) {
    let mut luoi = LuoiBam::moi(kich_thuoc_o);
    luoi.xay_dung(hop);
    let kha_nghi = luoi.cac_cap_kha_nghi();
    let so_phep_thu = kha_nghi.len();
    let that: Vec<(usize, usize)> = kha_nghi.into_iter()
        .filter(|&(a, b)| hop[a].giao_nhau(&hop[b]))
        .collect();
    (that, so_phep_thu)
}

// ============================================================================
// 6. ECS — Thực thể · Thành phần · Hệ thống
// ============================================================================
// Ý tưởng cốt lõi: KHÔNG dùng kế thừa ("Quái vật kế thừa Sinh vật kế thừa
// Thực thể"). Thay vào đó, thực thể chỉ là một CON SỐ; dữ liệu nằm trong các
// mảng song song. Hệ thống duyệt mảng liên tiếp trong bộ nhớ -> cache CPU
// hoạt động hết công suất. Đây là "thiết kế hướng dữ liệu".

pub type ThucThe = u32;

#[derive(Debug, Default)]
pub struct TheGioi {
    ke_tiep: ThucThe,
    pub con_song: Vec<ThucThe>,
    pub vi_tri: HashMap<ThucThe, Vec2>,
    pub van_toc: HashMap<ThucThe, Vec2>,
    pub mau: HashMap<ThucThe, i32>,
    pub sat_thuong_cham: HashMap<ThucThe, i32>,
    pub ban_kinh: HashMap<ThucThe, f32>,
}

impl TheGioi {
    pub fn moi() -> Self { TheGioi::default() }

    pub fn tao(&mut self) -> ThucThe {
        let e = self.ke_tiep;
        self.ke_tiep += 1;
        self.con_song.push(e);
        e
    }
    pub fn huy(&mut self, e: ThucThe) {
        self.con_song.retain(|&x| x != e);
        self.vi_tri.remove(&e);
        self.van_toc.remove(&e);
        self.mau.remove(&e);
        self.sat_thuong_cham.remove(&e);
        self.ban_kinh.remove(&e);
    }
    /// Truy vấn: các thực thể có ĐỦ cả vị trí lẫn vận tốc.
    /// Trong ECS thật, đây là chỗ dùng "archetype" để duyệt liên tiếp.
    pub fn co_vi_tri_va_van_toc(&self) -> Vec<ThucThe> {
        let mut v: Vec<ThucThe> = self.con_song.iter().copied()
            .filter(|e| self.vi_tri.contains_key(e) && self.van_toc.contains_key(e))
            .collect();
        v.sort_unstable(); // tất định — điều kiện tiên quyết để kiểm thử được
        v
    }
}

/// HỆ THỐNG là hàm thuần túy về mặt logic: `&mut TheGioi` vào, thế giới đổi ra.
/// Mỗi hệ thống chỉ đụng đúng những thành phần nó cần.
pub fn he_thong_di_chuyen(tg: &mut TheGioi, dt: f32) {
    for e in tg.co_vi_tri_va_van_toc() {
        let v = tg.van_toc[&e];
        if let Some(p) = tg.vi_tri.get_mut(&e) { *p = p.cong(v.nhan(dt)); }
    }
}

pub fn he_thong_trong_luc(tg: &mut TheGioi, g: f32, dt: f32) {
    for e in tg.con_song.clone() {
        if let Some(v) = tg.van_toc.get_mut(&e) { v.y -= g * dt; }
    }
}

/// Va chạm gây sát thương, rồi thu dọn xác. Trả về số thực thể đã chết.
pub fn he_thong_va_cham_gay_sat_thuong(tg: &mut TheGioi) -> usize {
    let ds: Vec<ThucThe> = {
        let mut v: Vec<ThucThe> = tg.con_song.iter().copied()
            .filter(|e| tg.vi_tri.contains_key(e) && tg.ban_kinh.contains_key(e))
            .collect();
        v.sort_unstable(); v
    };
    let mut sat_thuong: HashMap<ThucThe, i32> = HashMap::new();
    for i in 0..ds.len() {
        for j in (i + 1)..ds.len() {
            let (a, b) = (ds[i], ds[j]);
            if tron_giao_nhau(tg.vi_tri[&a], tg.ban_kinh[&a], tg.vi_tri[&b], tg.ban_kinh[&b]) {
                if let Some(&st) = tg.sat_thuong_cham.get(&a) { *sat_thuong.entry(b).or_insert(0) += st; }
                if let Some(&st) = tg.sat_thuong_cham.get(&b) { *sat_thuong.entry(a).or_insert(0) += st; }
            }
        }
    }
    for (e, st) in sat_thuong {
        if let Some(m) = tg.mau.get_mut(&e) { *m -= st; }
    }
    let chet: Vec<ThucThe> = tg.con_song.iter().copied()
        .filter(|e| tg.mau.get(e).map_or(false, |&m| m <= 0)).collect();
    for e in &chet { tg.huy(*e); }
    chet.len()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   LẬP TRÌNH GAME: VÒNG LẶP · VẬT LÝ · VA CHẠM · ECS        ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. VÒNG LẶP BƯỚC CỐ ĐỊNH 60 Hz");
    let mut bt = BoTichLuy::moi(60.0);
    for (ten, dt) in [("máy mạnh 144 fps", 1.0 / 144.0), ("máy yếu 30 fps", 1.0 / 30.0),
                      ("khựng 0.5 giây", 0.5)] {
        let n = bt.khung_moi(dt);
        println!("   {:<18} → {} bước vật lý, nội suy {:.2}{}",
                 ten, n.so_buoc_vat_ly, n.he_so_noi_suy,
                 if n.bi_bo_buoc { "  ⚠ cắt nợ để tránh xoắn ốc tử thần" } else { "" });
    }

    println!("\n2. HAI BỘ TÍCH PHÂN — vật rơi tự do 1 giây, dt = 1/60");
    let bd = TheVatLy { vi_tri: Vec2::moi(0.0, 100.0), van_toc: Vec2::KHONG, khoi_luong: 1.0 };
    let g = Vec2::moi(0.0, -9.81);
    let (mut a, mut b) = (bd, bd);
    for _ in 0..60 {
        a = buoc_euler_tuong_minh(a, g, 1.0 / 60.0);
        b = buoc_euler_nua_an(b, g, 1.0 / 60.0);
    }
    let that = 100.0 - 0.5 * 9.81;
    println!("   Nghiệm giải tích : y = {:.4}", that);
    println!("   Euler tường minh : y = {:.4} (lệch {:.4})", a.vi_tri.y, (a.vi_tri.y - that).abs());
    println!("   Euler nửa ẩn     : y = {:.4} (lệch {:.4})", b.vi_tri.y, (b.vi_tri.y - that).abs());

    println!("\n3. VA CHẠM & VECTOR ĐẨY TỐI THIỂU");
    let h1 = HopBao::tu_tam(Vec2::moi(0.0, 0.0), Vec2::moi(1.0, 1.0));
    let h2 = HopBao::tu_tam(Vec2::moi(1.5, 0.2), Vec2::moi(1.0, 1.0));
    println!("   Hai hộp chồng nhau: {} | đẩy ra: {:?}", h1.giao_nhau(&h2), h1.day_ra(&h2));
    println!("   Bóng bay (1,-1) đập sàn (pháp tuyến 0,1) → {:?}",
             Vec2::moi(1.0, -1.0).phan_xa(Vec2::moi(0.0, 1.0)));

    println!("\n4. BĂM KHÔNG GIAN — 400 vật thể rải trên lưới 100×100");
    let hop: Vec<HopBao> = (0..400).map(|i| {
        let x = (i % 20) as f32 * 5.0;
        let y = (i / 20) as f32 * 5.0;
        HopBao::tu_tam(Vec2::moi(x, y), Vec2::moi(1.2, 1.2))
    }).collect();
    let vet_can = va_cham_vet_can(&hop);
    let (qua_luoi, so_thu) = va_cham_qua_luoi(&hop, 6.0);
    let cap_vet_can = hop.len() * (hop.len() - 1) / 2;
    println!("   Vét cạn : {} phép thử → {} va chạm", cap_vet_can, vet_can.len());
    println!("   Lưới băm: {} phép thử → {} va chạm", so_thu, qua_luoi.len());
    println!("   Cùng kết quả: {} | giảm {:.0}% khối lượng tính toán",
             vet_can == qua_luoi, 100.0 - so_thu as f64 * 100.0 / cap_vet_can as f64);

    println!("\n5. ECS — 1 người chơi, 3 quái, mô phỏng 3 khung hình");
    let mut tg = TheGioi::moi();
    let nguoi_choi = tg.tao();
    tg.vi_tri.insert(nguoi_choi, Vec2::moi(0.0, 0.0));
    tg.van_toc.insert(nguoi_choi, Vec2::moi(1.0, 0.0));
    tg.mau.insert(nguoi_choi, 100);
    tg.ban_kinh.insert(nguoi_choi, 1.0);
    for i in 0..3 {
        let q = tg.tao();
        tg.vi_tri.insert(q, Vec2::moi(2.0 + i as f32 * 0.5, 0.0));
        tg.mau.insert(q, 10);
        tg.ban_kinh.insert(q, 1.0);
        tg.sat_thuong_cham.insert(q, 4);
    }
    tg.sat_thuong_cham.insert(nguoi_choi, 6);
    for khung in 1..=3 {
        he_thong_di_chuyen(&mut tg, 1.0);
        let chet = he_thong_va_cham_gay_sat_thuong(&mut tg);
        println!("   Khung {}: người chơi ở x={:.1} · máu {:?} · {} thực thể chết · còn {} sống",
                 khung, tg.vi_tri.get(&nguoi_choi).map_or(0.0, |p| p.x),
                 tg.mau.get(&nguoi_choi), chet, tg.con_song.len());
    }

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GAME = MỘT HÀM THUẦN TÚY CHẠY 60 LẦN MỖI GIÂY            ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn gan_bang(a: f32, b: f32) -> bool { (a - b).abs() < 1e-4 }

    // ---------- Vector ----------
    #[test]
    fn chuan_hoa_vector_khong_khong_sinh_nan() {
        let v = Vec2::KHONG.chuan_hoa();
        assert_eq!(v, Vec2::KHONG, "chia cho 0 phải bị chặn, không được ra NaN");
        assert!(!v.x.is_nan() && !v.y.is_nan());
    }

    #[test]
    fn chuan_hoa_cho_do_dai_bang_mot() {
        for v in [Vec2::moi(3.0, 4.0), Vec2::moi(-7.0, 0.5), Vec2::moi(0.0, -2.0)] {
            assert!(gan_bang(v.chuan_hoa().do_dai(), 1.0));
        }
    }

    #[test]
    fn do_dai_binh_phuong_khop_voi_do_dai() {
        let v = Vec2::moi(3.0, 4.0);
        assert!(gan_bang(v.do_dai(), 5.0));
        assert!(gan_bang(v.do_dai_binh_phuong(), 25.0));
    }

    #[test]
    fn phan_xa_bao_toan_do_lon_va_dao_dung_truc() {
        let toi = Vec2::moi(1.0, -1.0);
        let ra = toi.phan_xa(Vec2::moi(0.0, 1.0));
        assert!(gan_bang(ra.x, 1.0), "thành phần song song mặt phẳng giữ nguyên");
        assert!(gan_bang(ra.y, 1.0), "thành phần vuông góc đổi dấu");
        assert!(gan_bang(ra.do_dai(), toi.do_dai()), "va chạm đàn hồi giữ nguyên tốc độ");
    }

    #[test]
    fn noi_suy_dung_o_hai_dau_va_giua() {
        let a = Vec2::moi(0.0, 0.0);
        let b = Vec2::moi(10.0, 20.0);
        assert_eq!(a.noi_suy(b, 0.0), a);
        assert_eq!(a.noi_suy(b, 1.0), b);
        assert_eq!(a.noi_suy(b, 0.5), Vec2::moi(5.0, 10.0));
    }

    // ---------- Vòng lặp game ----------
    #[test]
    fn bo_tich_luy_f32_bi_troi_sai_so_tich_luy() {
        // LỖI THẬT, KHÔNG PHẢI GIẢ ĐỊNH: 1.0/144.0 không biểu diễn chính xác
        // được bằng nhị phân. Cộng dồn 144 lần cho ra số HƠI NHỎ HƠN 1.0,
        // nên mất hẳn một bước vật lý sau mỗi giây.
        let mut bt = BoTichLuy::moi(60.0);
        bt.toi_da_buoc_mot_khung = 1000;
        let tong: u32 = (0..144).map(|_| bt.khung_moi(1.0 / 144.0).so_buoc_vat_ly).sum();
        assert_eq!(tong, 59, "đáng lẽ 60 — một bước bị nuốt mất vì trôi dấu phẩy động");
    }

    #[test]
    fn bo_tich_luy_nguyen_tat_dinh_tuyet_doi_bat_ke_fps() {
        // Cùng 1 giây thời gian thực → CHÍNH XÁC 60 bước vật lý, ở MỌI fps.
        //
        // Chú ý cách lấy delta: hiệu của hai MỐC ĐỒNG HỒ TUYỆT ĐỐI, chứ không
        // phải hằng số `1e9 / fps` chia sẵn. Phép chia nguyên bị cắt cụt sẽ
        // làm hụt thời gian y như trôi dấu phẩy động. Game thật luôn đọc đồng
        // hồ tuyệt đối rồi trừ — nhờ vậy sai số không bao giờ tích lũy.
        for fps in [30u64, 60, 144, 240] {
            let mut bt = BoTichLuyNguyen::moi(60);
            bt.toi_da_buoc_mot_khung = 1000;
            let moc = |i: u64| i * 1_000_000_000 / fps; // mốc tuyệt đối, chính xác
            let tong: u32 = (1..=fps)
                .map(|i| bt.khung_moi(moc(i) - moc(i - 1)).so_buoc_vat_ly)
                .sum();
            assert_eq!(tong, 60, "ở {} fps vẫn phải đúng 60 bước", fps);
        }
    }

    #[test]
    fn bo_tich_luy_nguyen_cung_cat_no_khi_khung_hinh_khung() {
        let mut bt = BoTichLuyNguyen::moi(60);
        let n = bt.khung_moi(2_000_000_000); // khựng 2 giây
        assert_eq!(n.so_buoc_vat_ly, 5);
        assert!(n.bi_bo_buoc);
        assert_eq!(bt.khung_moi(16_666_666).so_buoc_vat_ly, 1, "không mang nợ sang khung sau");
    }

    #[test]
    fn he_so_noi_suy_luon_trong_khoang_0_1() {
        let mut bt = BoTichLuy::moi(60.0);
        for i in 0..200 {
            let n = bt.khung_moi(0.001 * (i % 37) as f32);
            assert!((0.0..1.0).contains(&n.he_so_noi_suy),
                    "hệ số nội suy {} nằm ngoài [0,1)", n.he_so_noi_suy);
        }
    }

    #[test]
    fn cat_no_thoi_gian_de_tranh_xoan_oc_tu_than() {
        let mut bt = BoTichLuy::moi(60.0);
        let n = bt.khung_moi(2.0); // khựng 2 giây = đáng lẽ 120 bước
        assert_eq!(n.so_buoc_vat_ly, 5, "bị chặn ở trần 5 bước");
        assert!(n.bi_bo_buoc);
        // Khung sau phải trở lại bình thường, không mang theo nợ
        let sau = bt.khung_moi(1.0 / 60.0);
        assert_eq!(sau.so_buoc_vat_ly, 1, "nợ đã bị cắt, không dồn sang khung sau");
    }

    // ---------- Vật lý ----------
    #[test]
    fn voi_gia_toc_hang_hai_bo_tich_phan_sai_bang_nhau_ve_hai_phia() {
        // Kết quả có thể gây bất ngờ: khi gia tốc KHÔNG ĐỔI, Euler nửa ẩn
        // KHÔNG chính xác hơn. Hai bộ lệch đúng bằng nhau — một cái vượt,
        // một cái hụt — vì sai số đều là 0.5·g·dt².
        // Ưu thế của nửa ẩn nằm ở chỗ khác: sự ỔN ĐỊNH của hệ dao động,
        // xem bài kiểm thử quỹ đạo tròn ngay bên dưới.
        let bd = TheVatLy { vi_tri: Vec2::moi(0.0, 100.0), van_toc: Vec2::KHONG, khoi_luong: 1.0 };
        let g = Vec2::moi(0.0, -9.81);
        let (mut a, mut b) = (bd, bd);
        for _ in 0..60 {
            a = buoc_euler_tuong_minh(a, g, 1.0 / 60.0);
            b = buoc_euler_nua_an(b, g, 1.0 / 60.0);
        }
        let that = 100.0 - 0.5 * 9.81;
        let sai_a = a.vi_tri.y - that;
        let sai_b = b.vi_tri.y - that;
        assert!(sai_a > 0.0, "tường minh rơi CHẬM hơn thực tế");
        assert!(sai_b < 0.0, "nửa ẩn rơi NHANH hơn thực tế");
        assert!((sai_a.abs() - sai_b.abs()).abs() < 1e-3,
                "hai sai số phải bằng nhau về độ lớn: {} vs {}", sai_a, sai_b);
    }

    #[test]
    fn euler_nua_an_giu_quy_dao_on_dinh() {
        // Cùng bài toán khiến Euler tường minh văng ra ngoài (xem bên dưới),
        // nửa ẩn giữ bán kính dao động trong biên hẹp — đây mới là lý do
        // thật sự khiến mọi game engine chọn nó.
        let mut t = TheVatLy { vi_tri: Vec2::moi(1.0, 0.0), van_toc: Vec2::moi(0.0, 1.0), khoi_luong: 1.0 };
        let mut r_lon_nhat: f32 = 0.0;
        for _ in 0..1000 {
            let huong_tam = t.vi_tri.chuan_hoa().nhan(-1.0);
            t = buoc_euler_nua_an(t, huong_tam, 0.01);
            r_lon_nhat = r_lon_nhat.max(t.vi_tri.do_dai());
        }
        assert!(r_lon_nhat < 1.02, "bán kính phải bị chặn, thực tế phình tới {}", r_lon_nhat);
    }

    #[test]
    fn ca_hai_bo_tich_phan_cho_cung_van_toc() {
        // Chỉ VỊ TRÍ khác nhau — vận tốc cập nhật giống hệt nhau.
        let bd = TheVatLy { vi_tri: Vec2::KHONG, van_toc: Vec2::moi(1.0, 0.0), khoi_luong: 1.0 };
        let g = Vec2::moi(0.0, -10.0);
        let a = buoc_euler_tuong_minh(bd, g, 0.1);
        let b = buoc_euler_nua_an(bd, g, 0.1);
        assert_eq!(a.van_toc, b.van_toc);
        assert_ne!(a.vi_tri, b.vi_tri);
    }

    #[test]
    fn euler_tuong_minh_bom_nang_luong_trong_quy_dao_tron() {
        // Bài kiểm chứng kinh điển: vật quay quanh tâm bằng lực hướng tâm.
        // Euler tường minh làm bán kính LỚN DẦN — vật văng ra ngoài.
        let mut t = TheVatLy { vi_tri: Vec2::moi(1.0, 0.0), van_toc: Vec2::moi(0.0, 1.0), khoi_luong: 1.0 };
        let r_dau = t.vi_tri.do_dai();
        for _ in 0..1000 {
            let huong_tam = t.vi_tri.chuan_hoa().nhan(-1.0);
            t = buoc_euler_tuong_minh(t, huong_tam, 0.01);
        }
        assert!(t.vi_tri.do_dai() > r_dau * 1.01,
                "bán kính phải phình ra: {} → {}", r_dau, t.vi_tri.do_dai());
    }

    // ---------- Va chạm ----------
    #[test]
    fn aabb_giao_nhau_dung_ca_truong_hop_bien() {
        let a = HopBao::tu_tam(Vec2::KHONG, Vec2::moi(1.0, 1.0));      // [-1,1]²
        let cham_dinh = HopBao::tu_tam(Vec2::moi(2.0, 2.0), Vec2::moi(1.0, 1.0));
        let roi_nhau = HopBao::tu_tam(Vec2::moi(2.1, 0.0), Vec2::moi(1.0, 1.0));
        assert!(a.giao_nhau(&cham_dinh), "chạm đúng một điểm vẫn tính là giao");
        assert!(!a.giao_nhau(&roi_nhau));
    }

    #[test]
    fn giao_nhau_co_tinh_doi_xung() {
        let a = HopBao::tu_tam(Vec2::moi(0.0, 0.0), Vec2::moi(2.0, 1.0));
        let b = HopBao::tu_tam(Vec2::moi(1.0, 0.5), Vec2::moi(1.0, 3.0));
        assert_eq!(a.giao_nhau(&b), b.giao_nhau(&a));
    }

    #[test]
    fn day_ra_chon_truc_chong_lan_it_nhat() {
        let a = HopBao::tu_tam(Vec2::moi(0.0, 0.0), Vec2::moi(1.0, 1.0));
        // chồng 0.2 theo X nhưng 1.8 theo Y -> phải đẩy theo X
        let b = HopBao::tu_tam(Vec2::moi(1.8, 0.2), Vec2::moi(1.0, 1.0));
        let d = a.day_ra(&b).expect("hai hộp có chồng lấn");
        assert!(gan_bang(d.y, 0.0), "phải đẩy theo trục X, không phải Y");
        assert!(d.x < 0.0, "a nằm bên trái nên bị đẩy sang trái");
        assert!(gan_bang(d.x.abs(), 0.2));
    }

    #[test]
    fn day_ra_thuc_su_tach_roi_hai_hop() {
        let a = HopBao::tu_tam(Vec2::moi(0.0, 0.0), Vec2::moi(1.0, 1.0));
        let b = HopBao::tu_tam(Vec2::moi(1.5, 0.3), Vec2::moi(1.0, 1.0));
        let d = a.day_ra(&b).unwrap();
        let a_moi = HopBao { min: a.min.cong(d), max: a.max.cong(d) };
        // sau khi đẩy, hai hộp chỉ còn chạm nhau chứ không chồng lên nhau
        assert!(gan_bang(a_moi.max.x, b.min.x) || gan_bang(a_moi.min.x, b.max.x)
                || gan_bang(a_moi.max.y, b.min.y) || gan_bang(a_moi.min.y, b.max.y));
    }

    #[test]
    fn khong_giao_nhau_thi_khong_co_vector_day() {
        let a = HopBao::tu_tam(Vec2::KHONG, Vec2::moi(1.0, 1.0));
        let xa = HopBao::tu_tam(Vec2::moi(50.0, 50.0), Vec2::moi(1.0, 1.0));
        assert_eq!(a.day_ra(&xa), None);
    }

    #[test]
    fn va_cham_tron_dung_o_diem_tiep_xuc() {
        assert!(tron_giao_nhau(Vec2::KHONG, 1.0, Vec2::moi(2.0, 0.0), 1.0), "chạm nhau vừa đúng");
        assert!(!tron_giao_nhau(Vec2::KHONG, 1.0, Vec2::moi(2.01, 0.0), 1.0));
    }

    // ---------- Băm không gian ----------
    #[test]
    fn luoi_bam_cho_ket_qua_y_HET_vet_can() {
        let hop: Vec<HopBao> = (0..200).map(|i| {
            let x = ((i * 37) % 100) as f32;
            let y = ((i * 53) % 100) as f32;
            HopBao::tu_tam(Vec2::moi(x, y), Vec2::moi(2.0, 2.0))
        }).collect();
        let (qua_luoi, _) = va_cham_qua_luoi(&hop, 8.0);
        assert_eq!(qua_luoi, va_cham_vet_can(&hop),
                   "tăng tốc KHÔNG được đổi kết quả — đây là bất biến quan trọng nhất");
    }

    #[test]
    fn luoi_bam_giam_manh_so_phep_thu() {
        let hop: Vec<HopBao> = (0..400).map(|i| {
            HopBao::tu_tam(Vec2::moi((i % 20) as f32 * 5.0, (i / 20) as f32 * 5.0),
                           Vec2::moi(1.2, 1.2))
        }).collect();
        let vet_can = hop.len() * (hop.len() - 1) / 2; // 79 800
        let (_, so_thu) = va_cham_qua_luoi(&hop, 6.0);
        assert!(so_thu * 10 < vet_can,
                "lưới băm phải cắt hơn 90% phép thử: {} so với {}", so_thu, vet_can);
    }

    #[test]
    fn luoi_bam_khong_bo_sot_vat_the_nam_tren_nhieu_o() {
        // Một vật RẤT LỚN trải qua nhiều ô phải va chạm được với mọi vật nhỏ.
        let mut hop = vec![HopBao::tu_tam(Vec2::moi(25.0, 25.0), Vec2::moi(25.0, 25.0))];
        for i in 0..10 {
            hop.push(HopBao::tu_tam(Vec2::moi(i as f32 * 5.0, i as f32 * 5.0), Vec2::moi(0.5, 0.5)));
        }
        let (qua_luoi, _) = va_cham_qua_luoi(&hop, 5.0);
        assert_eq!(qua_luoi, va_cham_vet_can(&hop), "vật lớn phải được ghi vào MỌI ô nó chạm");
    }

    #[test]
    fn khong_co_cap_trung_lap_trong_ket_qua() {
        let hop: Vec<HopBao> = (0..50).map(|i| {
            HopBao::tu_tam(Vec2::moi((i % 5) as f32, (i / 5) as f32), Vec2::moi(3.0, 3.0))
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
    fn thuc_the_chi_la_con_so_va_khong_tai_su_dung_id() {
        let mut tg = TheGioi::moi();
        let a = tg.tao();
        let b = tg.tao();
        tg.huy(a);
        let c = tg.tao();
        assert_ne!(c, a, "ID đã hủy không được cấp lại — tránh lỗi 'con trỏ ma'");
        assert_eq!(tg.con_song, vec![b, c]);
    }

    #[test]
    fn he_thong_chi_dung_toi_thuc_the_du_thanh_phan() {
        let mut tg = TheGioi::moi();
        let dong = tg.tao();
        let tinh = tg.tao();
        tg.vi_tri.insert(dong, Vec2::KHONG);
        tg.van_toc.insert(dong, Vec2::moi(2.0, 0.0));
        tg.vi_tri.insert(tinh, Vec2::moi(9.0, 9.0)); // KHÔNG có vận tốc
        he_thong_di_chuyen(&mut tg, 1.0);
        assert_eq!(tg.vi_tri[&dong], Vec2::moi(2.0, 0.0));
        assert_eq!(tg.vi_tri[&tinh], Vec2::moi(9.0, 9.0), "thiếu thành phần thì hệ thống bỏ qua");
    }

    #[test]
    fn huy_thuc_the_go_sach_moi_thanh_phan() {
        let mut tg = TheGioi::moi();
        let e = tg.tao();
        tg.vi_tri.insert(e, Vec2::KHONG);
        tg.van_toc.insert(e, Vec2::KHONG);
        tg.mau.insert(e, 5);
        tg.huy(e);
        assert!(!tg.vi_tri.contains_key(&e) && !tg.van_toc.contains_key(&e)
                && !tg.mau.contains_key(&e), "không được để lại thành phần mồ côi");
    }

    #[test]
    fn va_cham_gay_sat_thuong_va_thu_don_xac() {
        let mut tg = TheGioi::moi();
        let manh = tg.tao();
        tg.vi_tri.insert(manh, Vec2::KHONG);
        tg.ban_kinh.insert(manh, 1.0);
        tg.mau.insert(manh, 100);
        tg.sat_thuong_cham.insert(manh, 50);

        let yeu = tg.tao();
        tg.vi_tri.insert(yeu, Vec2::moi(1.0, 0.0)); // chồng lên nhau
        tg.ban_kinh.insert(yeu, 1.0);
        tg.mau.insert(yeu, 30);
        tg.sat_thuong_cham.insert(yeu, 10);

        let chet = he_thong_va_cham_gay_sat_thuong(&mut tg);
        assert_eq!(chet, 1, "kẻ yếu phải chết");
        assert_eq!(tg.mau[&manh], 90, "kẻ mạnh mất 10 máu");
        assert!(!tg.con_song.contains(&yeu));
    }

    #[test]
    fn khong_va_cham_thi_khong_ai_mat_mau() {
        let mut tg = TheGioi::moi();
        for i in 0..3 {
            let e = tg.tao();
            tg.vi_tri.insert(e, Vec2::moi(i as f32 * 100.0, 0.0)); // cách xa nhau
            tg.ban_kinh.insert(e, 1.0);
            tg.mau.insert(e, 10);
            tg.sat_thuong_cham.insert(e, 99);
        }
        assert_eq!(he_thong_va_cham_gay_sat_thuong(&mut tg), 0);
        assert!(tg.mau.values().all(|&m| m == 10));
    }

    #[test]
    fn trong_luc_tac_dong_len_moi_vat_co_van_toc() {
        let mut tg = TheGioi::moi();
        let e = tg.tao();
        tg.van_toc.insert(e, Vec2::KHONG);
        he_thong_trong_luc(&mut tg, 10.0, 0.5);
        assert!(gan_bang(tg.van_toc[&e].y, -5.0));
    }
}
