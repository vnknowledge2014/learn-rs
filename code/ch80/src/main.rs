#![allow(dead_code)]
//! Chương 80 — Kỹ nghệ hiệu năng CPU: phân cấp bộ nhớ, cục bộ cache, dự đoán
//! nhánh, song song mức lệnh, và mã không nhánh.
//!
//! Theo tinh thần các bài tập của [LeetCPU](https://www.leetcpu.com/) — nền
//! tảng luyện hiệu năng CPU có mô phỏng vi kiến trúc phản hồi. Ở đây ta ĐẾM
//! số lần trượt cache và dự đoán sai bằng mô phỏng tất định, thay vì đo đồng
//! hồ treo tường — nhờ vậy kết quả tái lập được và kiểm thử được.

use std::collections::HashMap;

// ============================================================================
// 1. PHÂN CẤP BỘ NHỚ — những con số cần thuộc lòng
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TangBoNho { ThanhGhi, L1, L2, L3, Ram, SsdNvme, DiaQuay }

impl TangBoNho {
    /// Độ trễ tính bằng CHU KỲ CPU. Cách nhìn này quan trọng hơn nano-giây:
    /// nó cho biết CPU phải ngồi chơi bao nhiêu nhịp.
    pub fn chu_ky(self) -> u64 {
        match self {
            TangBoNho::ThanhGhi => 1,
            TangBoNho::L1 => 4,
            TangBoNho::L2 => 12,
            TangBoNho::L3 => 40,
            TangBoNho::Ram => 200,
            TangBoNho::SsdNvme => 200_000,
            TangBoNho::DiaQuay => 20_000_000,
        }
    }
    pub fn ten(self) -> &'static str {
        match self {
            TangBoNho::ThanhGhi => "Thanh ghi", TangBoNho::L1 => "Cache L1",
            TangBoNho::L2 => "Cache L2", TangBoNho::L3 => "Cache L3",
            TangBoNho::Ram => "RAM", TangBoNho::SsdNvme => "SSD NVMe",
            TangBoNho::DiaQuay => "Đĩa quay",
        }
    }
    pub fn tat_ca() -> [TangBoNho; 7] {
        [TangBoNho::ThanhGhi, TangBoNho::L1, TangBoNho::L2, TangBoNho::L3,
         TangBoNho::Ram, TangBoNho::SsdNvme, TangBoNho::DiaQuay]
    }
}

pub const BYTE_MOI_DONG_CACHE: usize = 64;

// ============================================================================
// 2. MÔ PHỎNG CACHE LIÊN KẾT TẬP HỢP
// ============================================================================
// Cache thật không phải "có hay không có" — nó chia thành TẬP HỢP, mỗi tập
// chứa vài ĐƯỜNG. Địa chỉ quyết định tập nào; trong tập thì thay theo LRU.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThongKeCache {
    pub so_truy_cap: u64,
    pub so_trung: u64,
    pub so_truot: u64,
    /// Trượt vì lần đầu chạm tới — không tránh được.
    pub truot_bat_buoc: u64,
    /// Trượt vì cache quá nhỏ hoặc bị đá ra — CÓ THỂ tránh được.
    pub truot_do_dung_luong: u64,
}

impl ThongKeCache {
    pub fn ty_le_trung(&self) -> f64 {
        if self.so_truy_cap == 0 { 0.0 } else { self.so_trung as f64 / self.so_truy_cap as f64 }
    }
    /// Tổng chu kỳ phải trả — thước đo thật sự, không phải số lần trượt.
    pub fn tong_chu_ky(&self) -> u64 {
        self.so_trung * TangBoNho::L1.chu_ky() + self.so_truot * TangBoNho::Ram.chu_ky()
    }
}

pub struct MoPhongCache {
    pub so_tap: usize,
    pub so_duong: usize,
    /// tập → danh sách (thẻ, dấu thời gian dùng gần nhất), dài tối đa `so_duong`
    tap: Vec<Vec<(u64, u64)>>,
    da_thay: std::collections::HashSet<u64>,
    dong_ho: u64,
    pub tk: ThongKeCache,
}

impl MoPhongCache {
    /// `kich_thuoc_byte` là tổng dung lượng; `so_duong` là số đường mỗi tập.
    pub fn moi(kich_thuoc_byte: usize, so_duong: usize) -> Self {
        let so_dong = kich_thuoc_byte / BYTE_MOI_DONG_CACHE;
        let so_tap = (so_dong / so_duong).max(1);
        MoPhongCache {
            so_tap, so_duong,
            tap: vec![Vec::with_capacity(so_duong); so_tap],
            da_thay: std::collections::HashSet::new(),
            dong_ho: 0,
            tk: ThongKeCache { so_truy_cap: 0, so_trung: 0, so_truot: 0,
                               truot_bat_buoc: 0, truot_do_dung_luong: 0 },
        }
    }

    /// Truy cập một địa chỉ byte. Trả `true` nếu trúng cache.
    pub fn truy_cap(&mut self, dia_chi: usize) -> bool {
        self.dong_ho += 1;
        self.tk.so_truy_cap += 1;
        let so_dong = (dia_chi / BYTE_MOI_DONG_CACHE) as u64;
        let chi_so_tap = (so_dong as usize) % self.so_tap;
        let the = so_dong;

        let dh = self.dong_ho;
        let t = &mut self.tap[chi_so_tap];
        if let Some(e) = t.iter_mut().find(|(x, _)| *x == the) {
            e.1 = dh;
            self.tk.so_trung += 1;
            return true;
        }
        // Trượt
        self.tk.so_truot += 1;
        if self.da_thay.insert(the) {
            self.tk.truot_bat_buoc += 1;
        } else {
            self.tk.truot_do_dung_luong += 1;
        }
        if t.len() == self.so_duong {
            // Đá ra đường LÂU NHẤT KHÔNG DÙNG
            let vt = t.iter().enumerate().min_by_key(|(_, (_, d))| *d).map(|(i, _)| i).unwrap();
            t.swap_remove(vt);
        }
        t.push((the, dh));
        false
    }

    pub fn dat_lai(&mut self) {
        for t in self.tap.iter_mut() { t.clear(); }
        self.da_thay.clear();
        self.dong_ho = 0;
        self.tk = ThongKeCache { so_truy_cap: 0, so_trung: 0, so_truot: 0,
                                 truot_bat_buoc: 0, truot_do_dung_luong: 0 };
    }
}

// ============================================================================
// 3. CỤC BỘ CACHE — cùng phép tính, hai cách duyệt
// ============================================================================

/// Duyệt ma trận THEO HÀNG. Rust lưu mảng theo hàng, nên hai phần tử kề nhau
/// trong hàng cũng kề nhau trong bộ nhớ → mỗi dòng cache 64 byte nạp về được
/// dùng cho 8 phần tử `f64`.
pub fn duyet_theo_hang(mp: &mut MoPhongCache, n: usize, byte_moi_o: usize) -> u64 {
    mp.dat_lai();
    for i in 0..n {
        for j in 0..n {
            mp.truy_cap((i * n + j) * byte_moi_o);
        }
    }
    mp.tk.so_truot
}

/// Duyệt THEO CỘT. Hai phần tử liên tiếp cách nhau `n` ô → mỗi lần chạm là
/// một dòng cache mới. Nạp 64 byte về chỉ để dùng 8 byte, phí 87,5%.
pub fn duyet_theo_cot(mp: &mut MoPhongCache, n: usize, byte_moi_o: usize) -> u64 {
    mp.dat_lai();
    for j in 0..n {
        for i in 0..n {
            mp.truy_cap((i * n + j) * byte_moi_o);
        }
    }
    mp.tk.so_truot
}

/// Nhân ma trận ngây thơ: vòng lặp i-j-k. Vòng trong quét CỘT của ma trận B.
pub fn nhan_ma_tran_ngay_tho(mp: &mut MoPhongCache, n: usize, byte_moi_o: usize) -> u64 {
    mp.dat_lai();
    let goc_a = 0usize;
    let goc_b = n * n * byte_moi_o;
    for i in 0..n {
        for j in 0..n {
            for k in 0..n {
                mp.truy_cap(goc_a + (i * n + k) * byte_moi_o);
                mp.truy_cap(goc_b + (k * n + j) * byte_moi_o); // quét cột!
            }
        }
    }
    mp.tk.so_truot
}

/// Nhân ma trận theo KHỐI: chia thành các khối vừa lọt cache, làm xong khối
/// này mới sang khối khác. Cùng số phép nhân, nhưng dữ liệu được TÁI SỬ DỤNG
/// khi còn nóng trong cache.
pub fn nhan_ma_tran_khoi(mp: &mut MoPhongCache, n: usize, khoi: usize,
                         byte_moi_o: usize) -> u64 {
    mp.dat_lai();
    let goc_a = 0usize;
    let goc_b = n * n * byte_moi_o;
    for ii in (0..n).step_by(khoi) {
        for jj in (0..n).step_by(khoi) {
            for kk in (0..n).step_by(khoi) {
                for i in ii..(ii + khoi).min(n) {
                    for j in jj..(jj + khoi).min(n) {
                        for k in kk..(kk + khoi).min(n) {
                            mp.truy_cap(goc_a + (i * n + k) * byte_moi_o);
                            mp.truy_cap(goc_b + (k * n + j) * byte_moi_o);
                        }
                    }
                }
            }
        }
    }
    mp.tk.so_truot
}

// ============================================================================
// 4. DỰ ĐOÁN NHÁNH
// ============================================================================
// CPU hiện đại có đường ống 15–20 tầng. Gặp một `if`, nó ĐOÁN kết quả và chạy
// tiếp. Đoán đúng: không mất gì. Đoán sai: xả sạch đường ống, mất 15–20 chu kỳ.

pub const PHAT_DU_DOAN_SAI: u64 = 18;

/// Bộ đếm bão hoà 2 bit — bộ dự đoán nhánh kinh điển.
/// Trạng thái: 0 = chắc chắn không, 1 = có lẽ không, 2 = có lẽ có, 3 = chắc có.
/// Cần SAI HAI LẦN liên tiếp mới đổi ý → chống nhiễu cho vòng lặp.
#[derive(Debug, Clone)]
pub struct DuDoanNhanh {
    trang_thai: HashMap<usize, u8>,
    pub so_nhanh: u64,
    pub so_du_doan_sai: u64,
}

impl DuDoanNhanh {
    pub fn moi() -> Self {
        DuDoanNhanh { trang_thai: HashMap::new(), so_nhanh: 0, so_du_doan_sai: 0 }
    }

    /// `id_nhanh` là vị trí lệnh nhánh; `thuc_te` là kết quả thật.
    pub fn du_doan(&mut self, id_nhanh: usize, thuc_te: bool) -> bool {
        self.so_nhanh += 1;
        let tt = self.trang_thai.entry(id_nhanh).or_insert(1);
        let doan = *tt >= 2;
        if doan != thuc_te { self.so_du_doan_sai += 1; }
        // Bão hoà: 3 không lên nữa, 0 không xuống nữa
        if thuc_te { *tt = (*tt + 1).min(3); } else { *tt = tt.saturating_sub(1); }
        doan == thuc_te
    }

    pub fn ty_le_sai(&self) -> f64 {
        if self.so_nhanh == 0 { 0.0 } else { self.so_du_doan_sai as f64 / self.so_nhanh as f64 }
    }
    /// Số chu kỳ mất trắng vì đoán sai.
    pub fn chu_ky_phi(&self) -> u64 { self.so_du_doan_sai * PHAT_DU_DOAN_SAI }
}

/// Đếm phần tử lớn hơn ngưỡng, CÓ nhánh. Trên dữ liệu ĐÃ SẮP XẾP, nhánh cực
/// dễ đoán (một chuỗi dài "không" rồi một chuỗi dài "có"). Trên dữ liệu lộn
/// xộn, nó gần như tung đồng xu.
pub fn dem_co_nhanh(du_lieu: &[i32], nguong: i32, dd: &mut DuDoanNhanh) -> (usize, u64) {
    let mut dem = 0;
    for &x in du_lieu {
        let dieu_kien = x >= nguong;
        dd.du_doan(0xB1, dieu_kien); // một vị trí nhánh duy nhất
        if dieu_kien { dem += 1; }
    }
    (dem, dd.so_du_doan_sai)
}

/// Cùng phép tính nhưng KHÔNG có nhánh: biến điều kiện thành số học.
/// CPU không phải đoán gì cả → không bao giờ đoán sai.
pub fn dem_khong_nhanh(du_lieu: &[i32], nguong: i32) -> usize {
    du_lieu.iter().map(|&x| (x >= nguong) as usize).sum()
}

// ============================================================================
// 5. SONG SONG MỨC LỆNH
// ============================================================================
// CPU hiện đại chạy 4–6 lệnh mỗi chu kỳ — NẾU chúng độc lập. Một chuỗi phụ
// thuộc (mỗi lệnh cần kết quả lệnh trước) làm mọi cổng thực thi khác ngồi chơi.

#[derive(Debug, PartialEq)]
pub struct PhanTichIlp {
    pub so_phep_tinh: u64,
    /// Chuỗi phụ thuộc dài nhất — cận dưới của số chu kỳ, bất kể CPU rộng bao nhiêu.
    pub duong_toi_han: u64,
    pub ilp: f64,
    /// Số chu kỳ ước tính trên CPU rộng `do_rong` lệnh/chu kỳ.
    pub chu_ky_uoc_tinh: u64,
}

/// Cộng dồn vào MỘT biến: mỗi phép cộng phải chờ phép trước.
/// Đường tới hạn = n. CPU rộng 4 cũng vô dụng.
pub fn phan_tich_tong_mot_bien(n: u64, _do_rong: u64) -> PhanTichIlp {
    PhanTichIlp {
        so_phep_tinh: n,
        duong_toi_han: n,
        ilp: 1.0,
        chu_ky_uoc_tinh: n.max(1), // bị chặn bởi chuỗi phụ thuộc, không bởi độ rộng
    }
}

/// Cộng dồn vào `k` biến rồi gộp cuối: `k` chuỗi độc lập chạy song song.
/// Đây là "bung vòng lặp có nhiều bộ tích luỹ" — thủ thuật hiệu năng cổ điển.
pub fn phan_tich_tong_nhieu_bien(n: u64, k: u64, do_rong: u64) -> PhanTichIlp {
    let k = k.max(1);
    // Mỗi chuỗi dài n/k, cộng thêm log2(k) bước gộp các bộ tích luỹ lại
    let duong_toi_han = n / k + k.next_power_of_two().trailing_zeros() as u64;
    PhanTichIlp {
        so_phep_tinh: n,
        duong_toi_han,
        ilp: n as f64 / duong_toi_han.max(1) as f64,
        chu_ky_uoc_tinh: duong_toi_han.max(n / do_rong.max(1)),
    }
}

/// Kiểm chứng: nhiều bộ tích luỹ phải cho CÙNG kết quả với một bộ.
pub fn tong_mot_bien(du_lieu: &[i64]) -> i64 { du_lieu.iter().sum() }

pub fn tong_nhieu_bien(du_lieu: &[i64], k: usize) -> i64 {
    let k = k.max(1);
    let mut acc = vec![0i64; k];
    for (i, &x) in du_lieu.iter().enumerate() { acc[i % k] += x; }
    acc.iter().sum()
}

// ============================================================================
// 6. SIMD — một lệnh, nhiều dữ liệu
// ============================================================================
// Thanh ghi vector 256 bit chứa 4 số `f64` hoặc 8 số `f32`. Một lệnh cộng
// vector làm 4 phép cộng cùng lúc. Trình biên dịch TỰ vector hoá được vòng
// lặp đơn giản, nhưng chỉ khi không có phụ thuộc và không có nhánh bên trong.

#[derive(Debug, PartialEq)]
pub struct PhanTichSimd {
    pub so_phan_tu: usize,
    pub be_rong_vector: usize,
    pub so_lenh_vector: usize,
    pub so_phan_tu_du: usize,
    pub tang_toc_ly_thuyet: f64,
}

pub fn phan_tich_simd(so_phan_tu: usize, be_rong_vector: usize) -> PhanTichSimd {
    let w = be_rong_vector.max(1);
    let du = so_phan_tu % w;
    let so_lenh_vector = so_phan_tu / w;
    // Phần dư phải xử lý từng phần tử một — đó là cái giá của mảng không chia hết
    let tong_lenh = so_lenh_vector + du;
    PhanTichSimd {
        so_phan_tu,
        be_rong_vector: w,
        so_lenh_vector,
        so_phan_tu_du: du,
        tang_toc_ly_thuyet: if tong_lenh == 0 { 1.0 }
                            else { so_phan_tu as f64 / tong_lenh as f64 },
    }
}

/// Cộng hai mảng theo lô `w` phần tử — mô phỏng cách trình biên dịch vector hoá.
pub fn cong_mang_theo_lo(a: &[f64], b: &[f64], w: usize) -> Vec<f64> {
    let n = a.len().min(b.len());
    let mut ra = vec![0.0; n];
    let w = w.max(1);
    let het_lo = n - n % w;
    for i in (0..het_lo).step_by(w) {
        for j in 0..w { ra[i + j] = a[i + j] + b[i + j]; }
    }
    for i in het_lo..n { ra[i] = a[i] + b[i]; }
    ra
}

// ============================================================================
// 7. SINH DỮ LIỆU TẤT ĐỊNH
// ============================================================================

pub fn sinh_du_lieu(n: usize, hat_giong: u64) -> Vec<i32> {
    let mut s = hat_giong;
    (0..n).map(|_| {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((s >> 33) % 256) as i32
    }).collect()
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   KỸ NGHỆ HIỆU NĂNG CPU: CACHE · NHÁNH · ILP · SIMD       ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. PHÂN CẤP BỘ NHỚ — những con số cần thuộc");
    println!("   {:<12} {:>14} {:>16}", "tầng", "chu kỳ", "so với L1");
    for t in TangBoNho::tat_ca() {
        println!("   {:<12} {:>14} {:>15.0}x",
                 t.ten(), t.chu_ky(), t.chu_ky() as f64 / TangBoNho::L1.chu_ky() as f64);
    }
    println!("   → Một lần trượt xuống RAM tốn bằng 50 lần chạm L1.");

    println!("\n2. CỤC BỘ CACHE — cùng phép duyệt, khác thứ tự");
    let mut mp = MoPhongCache::moi(32 * 1024, 8); // L1 32 KB, 8 đường
    let n = 256;
    let theo_hang = duyet_theo_hang(&mut mp, n, 8);
    let ck_hang = mp.tk.tong_chu_ky();
    let theo_cot = duyet_theo_cot(&mut mp, n, 8);
    let ck_cot = mp.tk.tong_chu_ky();
    println!("   Ma trận {}x{} f64 ({} KB):", n, n, n * n * 8 / 1024);
    println!("   Theo hàng: {:>8} lần trượt · {:>10} chu kỳ", theo_hang, ck_hang);
    println!("   Theo cột : {:>8} lần trượt · {:>10} chu kỳ", theo_cot, ck_cot);
    println!("   → Cùng {} phép truy cập, chỉ khác thứ tự, chậm gấp {:.1} lần.",
             n * n, ck_cot as f64 / ck_hang as f64);

    println!("\n3. NHÂN MA TRẬN — chia khối để tái dùng dữ liệu nóng");
    let n = 96;
    let mut mp = MoPhongCache::moi(32 * 1024, 8);
    let ngay_tho = nhan_ma_tran_ngay_tho(&mut mp, n, 8);
    println!("   Ngây thơ (i-j-k): {:>9} lần trượt", ngay_tho);
    for k in [8usize, 16, 32] {
        let mut mp2 = MoPhongCache::moi(32 * 1024, 8);
        let theo_khoi = nhan_ma_tran_khoi(&mut mp2, n, k, 8);
        println!("   Chia khối {:>2}x{:<2}   : {:>9} lần trượt → giảm {:.0}%",
                 k, k, theo_khoi, (1.0 - theo_khoi as f64 / ngay_tho as f64) * 100.0);
    }
    println!("   → CÙNG số phép nhân. Chỉ đổi thứ tự truy cập bộ nhớ.");

    println!("\n4. DỰ ĐOÁN NHÁNH — vì sao sắp xếp trước lại nhanh hơn");
    let lon_xon = sinh_du_lieu(100_000, 42);
    let mut da_sap = lon_xon.clone();
    da_sap.sort_unstable();
    for (ten, d) in [("lộn xộn ", &lon_xon), ("đã sắp  ", &da_sap)] {
        let mut dd = DuDoanNhanh::moi();
        let (dem, sai) = dem_co_nhanh(d, 128, &mut dd);
        println!("   {} → {} phần tử · {:>6} lần đoán sai ({:>5.1}%) · phí {:>8} chu kỳ",
                 ten, dem, sai, dd.ty_le_sai() * 100.0, dd.chu_ky_phi());
    }
    println!("   Bản KHÔNG NHÁNH: {} phần tử · 0 lần đoán sai · 0 chu kỳ phí",
             dem_khong_nhanh(&da_sap, 128));
    println!("   → Sắp xếp trước không làm phép đếm nhanh hơn; nó làm CPU ĐOÁN ĐÚNG hơn.");

    println!("\n5. SONG SONG MỨC LỆNH");
    let n = 1_000_000u64;
    println!("   {:<22} {:>14} {:>8} {:>16}",
             "cách viết", "đường tới hạn", "ILP", "chu kỳ ước tính");
    let a = phan_tich_tong_mot_bien(n, 4);
    println!("   {:<22} {:>14} {:>8.1} {:>16}",
             "1 bộ tích luỹ", a.duong_toi_han, a.ilp, a.chu_ky_uoc_tinh);
    for k in [2u64, 4, 8] {
        let b = phan_tich_tong_nhieu_bien(n, k, 4);
        println!("   {:<22} {:>14} {:>8.1} {:>16}",
                 format!("{} bộ tích luỹ", k), b.duong_toi_han, b.ilp, b.chu_ky_uoc_tinh);
    }
    let d: Vec<i64> = (1..=1000).collect();
    println!("   Kết quả vẫn giống hệt nhau: {}",
             tong_mot_bien(&d) == tong_nhieu_bien(&d, 4));

    println!("\n6. SIMD");
    println!("   {:>10} {:>10} {:>14} {:>10} {:>12}",
             "phần tử", "bề rộng", "lệnh vector", "phần dư", "tăng tốc");
    for (n, w) in [(1024usize, 4usize), (1024, 8), (1001, 8), (7, 8)] {
        let p = phan_tich_simd(n, w);
        println!("   {:>10} {:>10} {:>14} {:>10} {:>11.2}x",
                 p.so_phan_tu, p.be_rong_vector, p.so_lenh_vector,
                 p.so_phan_tu_du, p.tang_toc_ly_thuyet);
    }
    println!("   → Mảng 7 phần tử với vector 8 làn: KHÔNG tăng tốc chút nào.");
    println!("     Đó là lý do người ta đệm mảng cho tròn bội số bề rộng vector.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   CÙNG THUẬT TOÁN, KHÁC CÁCH CHẠM BỘ NHỚ, KHÁC HÀNG CHỤC LẦN");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    // ---------- Phân cấp bộ nhớ ----------
    #[test]
    fn do_tre_tang_dan_theo_khoang_cach() {
        let t = TangBoNho::tat_ca();
        for w in t.windows(2) {
            assert!(w[0].chu_ky() < w[1].chu_ky(),
                    "{} phải nhanh hơn {}", w[0].ten(), w[1].ten());
        }
    }

    #[test]
    fn khoang_cach_giua_cac_tang_dung_bac_do_lon() {
        assert_eq!(TangBoNho::Ram.chu_ky() / TangBoNho::L1.chu_ky(), 50,
                   "trượt xuống RAM tốn bằng 50 lần chạm L1");
        assert!(TangBoNho::SsdNvme.chu_ky() > TangBoNho::Ram.chu_ky() * 500,
                "SSD chậm hơn RAM cả ba bậc độ lớn");
    }

    // ---------- Mô phỏng cache ----------
    #[test]
    fn lan_dau_cham_luon_truot_lan_sau_trung() {
        let mut mp = MoPhongCache::moi(32 * 1024, 8);
        assert!(!mp.truy_cap(0), "lần đầu phải trượt");
        assert!(mp.truy_cap(0), "lần hai phải trúng");
        assert_eq!(mp.tk.truot_bat_buoc, 1);
        assert_eq!(mp.tk.truot_do_dung_luong, 0);
    }

    #[test]
    fn ca_dong_cache_duoc_nap_ve_cung_luc() {
        // Chạm byte 0 thì byte 1..63 cũng vào cache theo — đó chính là lý do
        // duyệt tuần tự nhanh hơn duyệt nhảy cóc.
        let mut mp = MoPhongCache::moi(32 * 1024, 8);
        mp.truy_cap(0);
        for b in 1..BYTE_MOI_DONG_CACHE {
            assert!(mp.truy_cap(b), "byte {} phải nằm cùng dòng cache với byte 0", b);
        }
        assert_eq!(mp.tk.so_truot, 1, "64 byte chỉ tốn MỘT lần trượt");
    }

    #[test]
    fn buoc_nhay_bang_dong_cache_thi_lan_nao_cung_truot() {
        let mut mp = MoPhongCache::moi(32 * 1024, 8);
        for i in 0..100 { mp.truy_cap(i * BYTE_MOI_DONG_CACHE); }
        assert_eq!(mp.tk.so_truot, 100, "mỗi lần chạm một dòng mới");
        assert_eq!(mp.tk.ty_le_trung(), 0.0);
    }

    #[test]
    fn du_lieu_vuot_cache_thi_bi_da_ra() {
        // Cache 1 KB = 16 dòng. Quét vòng qua 64 dòng thì lần nào cũng trượt.
        let mut mp = MoPhongCache::moi(1024, 4);
        for _ in 0..3 {
            for i in 0..64 { mp.truy_cap(i * BYTE_MOI_DONG_CACHE); }
        }
        assert!(mp.tk.truot_do_dung_luong > 0, "phải có trượt do bị đá ra");
        assert!(mp.tk.ty_le_trung() < 0.1, "quét vòng lớn hơn cache → gần như trượt hết");
    }

    #[test]
    fn du_lieu_vua_cache_thi_lan_hai_trung_het() {
        let mut mp = MoPhongCache::moi(32 * 1024, 8); // 512 dòng
        for _ in 0..5 {
            for i in 0..100 { mp.truy_cap(i * BYTE_MOI_DONG_CACHE); }
        }
        assert_eq!(mp.tk.so_truot, 100, "chỉ 100 lần trượt bắt buộc, sau đó trúng hết");
        assert_eq!(mp.tk.truot_do_dung_luong, 0);
        assert!(mp.tk.ty_le_trung() > 0.79);
    }

    #[test]
    fn thong_ke_luon_can_bang() {
        let mut mp = MoPhongCache::moi(4096, 4);
        for i in 0..1000 { mp.truy_cap(i * 7); }
        assert_eq!(mp.tk.so_trung + mp.tk.so_truot, mp.tk.so_truy_cap);
        assert_eq!(mp.tk.truot_bat_buoc + mp.tk.truot_do_dung_luong, mp.tk.so_truot);
    }

    // ---------- Cục bộ ----------
    #[test]
    fn duyet_theo_hang_it_truot_hon_han_theo_cot() {
        // Đây là bài học trung tâm của chương.
        let mut mp = MoPhongCache::moi(32 * 1024, 8);
        let n = 256;
        let hang = duyet_theo_hang(&mut mp, n, 8);
        let cot = duyet_theo_cot(&mut mp, n, 8);
        assert!(cot > hang * 5,
                "theo cột {} lần trượt phải nhiều hơn hẳn theo hàng {}", cot, hang);
        // Theo hàng: mỗi dòng cache 64 byte phục vụ 8 phần tử f64
        assert_eq!(hang, (n * n / 8) as u64, "đúng bằng số dòng cache của cả ma trận");
    }

    #[test]
    fn duyet_hai_kieu_cham_dung_cung_so_o_nho() {
        let mut mp = MoPhongCache::moi(32 * 1024, 8);
        let n = 64;
        duyet_theo_hang(&mut mp, n, 8);
        let a = mp.tk.so_truy_cap;
        duyet_theo_cot(&mut mp, n, 8);
        assert_eq!(a, mp.tk.so_truy_cap, "cùng khối lượng việc, chỉ khác thứ tự");
    }

    #[test]
    fn o_nho_nho_hon_thi_moi_dong_cache_phuc_vu_nhieu_phan_tu_hon() {
        let mut mp = MoPhongCache::moi(32 * 1024, 8);
        let n = 128;
        let f64_ = duyet_theo_hang(&mut mp, n, 8);
        let f32_ = duyet_theo_hang(&mut mp, n, 4);
        assert!(f32_ < f64_, "dùng f32 thay f64 giảm một nửa số lần trượt");
        assert_eq!(f64_, f32_ * 2);
    }

    // ---------- Nhân ma trận ----------
    #[test]
    fn chia_khoi_giam_so_lan_truot() {
        let n = 96;
        let mut a = MoPhongCache::moi(32 * 1024, 8);
        let ngay_tho = nhan_ma_tran_ngay_tho(&mut a, n, 8);
        let mut b = MoPhongCache::moi(32 * 1024, 8);
        let theo_khoi = nhan_ma_tran_khoi(&mut b, n, 16, 8);
        assert!(theo_khoi < ngay_tho,
                "chia khối {} phải ít trượt hơn ngây thơ {}", theo_khoi, ngay_tho);
    }

    #[test]
    fn chia_khoi_lam_dung_so_phep_truy_cap() {
        // Bất biến: tối ưu không được đổi KHỐI LƯỢNG VIỆC, chỉ đổi thứ tự.
        let n = 48;
        let mut a = MoPhongCache::moi(32 * 1024, 8);
        nhan_ma_tran_ngay_tho(&mut a, n, 8);
        let mut b = MoPhongCache::moi(32 * 1024, 8);
        nhan_ma_tran_khoi(&mut b, n, 16, 8);
        assert_eq!(a.tk.so_truy_cap, b.tk.so_truy_cap,
                   "cùng 2·n³ phép truy cập, chỉ khác thứ tự");
        assert_eq!(a.tk.so_truy_cap, 2 * (n * n * n) as u64);
    }

    // ---------- Dự đoán nhánh ----------
    #[test]
    fn bo_dem_bao_hoa_can_sai_hai_lan_moi_doi_y() {
        // Đây là lý do bộ đếm 2 bit tốt hơn 1 bit: một lần chệch không làm
        // nó đổi ý, nên vòng lặp dài không bị phạt ở lần lặp bất thường.
        let mut d = DuDoanNhanh::moi();
        for _ in 0..10 { d.du_doan(1, true); } // học "luôn đúng"
        let sai_truoc = d.so_du_doan_sai;
        d.du_doan(1, false); // một lần chệch
        assert_eq!(d.so_du_doan_sai, sai_truoc + 1);
        assert!(d.du_doan(1, true), "một lần chệch KHÔNG làm nó đổi ý");
    }

    #[test]
    fn nhanh_luon_dung_thi_gan_nhu_khong_doan_sai() {
        let mut d = DuDoanNhanh::moi();
        for _ in 0..10_000 { d.du_doan(1, true); }
        assert!(d.so_du_doan_sai <= 2, "chỉ sai vài lần lúc học, thực tế {}", d.so_du_doan_sai);
        assert!(d.ty_le_sai() < 0.001);
    }

    #[test]
    fn nhanh_lat_lien_tuc_thi_doan_sai_gan_het() {
        // Trường hợp tệ nhất của bộ đếm 2 bit: mẫu luân phiên.
        let mut d = DuDoanNhanh::moi();
        for i in 0..10_000 { d.du_doan(1, i % 2 == 0); }
        assert!(d.ty_le_sai() > 0.4, "mẫu luân phiên phải làm nó sai rất nhiều");
    }

    #[test]
    fn du_lieu_da_sap_xep_it_doan_sai_hon_han() {
        // Câu hỏi phỏng vấn kinh điển: "vì sao sắp xếp mảng trước lại làm
        // vòng lặp đếm chạy nhanh hơn?" — không phải vì phép đếm nhanh hơn,
        // mà vì CPU đoán nhánh đúng hơn.
        let lon_xon = sinh_du_lieu(50_000, 42);
        let mut da_sap = lon_xon.clone();
        da_sap.sort_unstable();

        let mut d1 = DuDoanNhanh::moi();
        let (a, sai_lon_xon) = dem_co_nhanh(&lon_xon, 128, &mut d1);
        let mut d2 = DuDoanNhanh::moi();
        let (b, sai_da_sap) = dem_co_nhanh(&da_sap, 128, &mut d2);

        assert_eq!(a, b, "kết quả phải giống hệt — chỉ hiệu năng khác");
        assert!(sai_da_sap * 20 < sai_lon_xon,
                "đã sắp: {} lần sai, lộn xộn: {} lần sai", sai_da_sap, sai_lon_xon);
        assert!(d1.chu_ky_phi() > d2.chu_ky_phi() * 20);
    }

    #[test]
    fn ban_khong_nhanh_cho_cung_ket_qua() {
        for hat in [1u64, 42, 2024] {
            let d = sinh_du_lieu(10_000, hat);
            let mut dd = DuDoanNhanh::moi();
            let (a, _) = dem_co_nhanh(&d, 128, &mut dd);
            assert_eq!(a, dem_khong_nhanh(&d, 128),
                       "mã không nhánh phải cho cùng đáp số");
        }
    }

    #[test]
    fn ban_khong_nhanh_khong_bao_gio_doan_sai() {
        // Không có nhánh thì không có gì để đoán — và không có gì để đoán sai.
        // Đây cũng là nền của mã mật mã chạy thời gian không đổi (Chương 57).
        let lon_xon = sinh_du_lieu(10_000, 7);
        let dd = DuDoanNhanh::moi();
        dem_khong_nhanh(&lon_xon, 128);
        assert_eq!(dd.so_du_doan_sai, 0);
        assert_eq!(dd.chu_ky_phi(), 0);
    }

    // ---------- ILP ----------
    #[test]
    fn mot_bo_tich_luy_bi_chan_boi_chuoi_phu_thuoc() {
        let a = phan_tich_tong_mot_bien(1_000_000, 4);
        assert_eq!(a.ilp, 1.0, "chuỗi phụ thuộc thuần → không song song được gì");
        assert_eq!(a.chu_ky_uoc_tinh, 1_000_000,
                   "CPU rộng 4 lệnh/chu kỳ cũng không giúp được gì");
    }

    #[test]
    fn nhieu_bo_tich_luy_tang_ilp() {
        let mut ilp_truoc = 0.0;
        for k in [1u64, 2, 4, 8] {
            let b = phan_tich_tong_nhieu_bien(1_000_000, k, 4);
            assert!(b.ilp > ilp_truoc, "k={} phải cho ILP cao hơn", k);
            ilp_truoc = b.ilp;
        }
        let b4 = phan_tich_tong_nhieu_bien(1_000_000, 4, 4);
        assert!(b4.ilp > 3.9, "4 bộ tích luỹ phải đạt ILP gần 4, thực tế {:.2}", b4.ilp);
    }

    #[test]
    fn do_rong_cpu_chan_tren_toc_do() {
        // Dù có 64 bộ tích luỹ, CPU rộng 4 vẫn chỉ chạy 4 lệnh mỗi chu kỳ.
        let b = phan_tich_tong_nhieu_bien(1_000_000, 64, 4);
        assert!(b.chu_ky_uoc_tinh >= 1_000_000 / 4,
                "không thể nhanh hơn giới hạn độ rộng CPU");
    }

    #[test]
    fn nhieu_bo_tich_luy_cho_cung_ket_qua() {
        // Cộng số nguyên có tính kết hợp nên đổi thứ tự vẫn đúng.
        // (Với f64 thì KHÔNG — đó là lý do trình biên dịch không tự làm việc
        // này cho số thực trừ khi bạn cho phép nới lỏng ngữ nghĩa dấu phẩy động.)
        let d: Vec<i64> = (1..=10_000).collect();
        let mong_doi = tong_mot_bien(&d);
        for k in [1usize, 2, 3, 4, 8, 16] {
            assert_eq!(tong_nhieu_bien(&d, k), mong_doi, "k={}", k);
        }
    }

    #[test]
    fn tong_mang_rong_bang_khong() {
        assert_eq!(tong_mot_bien(&[]), 0);
        assert_eq!(tong_nhieu_bien(&[], 4), 0);
    }

    // ---------- SIMD ----------
    #[test]
    fn simd_tang_toc_dung_be_rong_khi_chia_het() {
        let p = phan_tich_simd(1024, 4);
        assert_eq!(p.so_lenh_vector, 256);
        assert_eq!(p.so_phan_tu_du, 0);
        assert!((p.tang_toc_ly_thuyet - 4.0).abs() < 1e-9);
    }

    #[test]
    fn phan_du_lam_giam_tang_toc() {
        let chia_het = phan_tich_simd(1024, 8);
        assert_eq!(chia_het.so_phan_tu_du, 0);
        let le = phan_tich_simd(1001, 8);
        assert_eq!(le.so_phan_tu_du, 1);
        assert!(le.tang_toc_ly_thuyet < chia_het.tang_toc_ly_thuyet);
    }

    #[test]
    fn mang_qua_ngan_thi_simd_vo_dung() {
        // 7 phần tử với vector 8 làn: không lô nào đầy, mọi phần tử xử lý lẻ.
        let p = phan_tich_simd(7, 8);
        assert_eq!(p.so_lenh_vector, 0);
        assert_eq!(p.so_phan_tu_du, 7);
        assert!((p.tang_toc_ly_thuyet - 1.0).abs() < 1e-9, "không tăng tốc chút nào");
    }

    #[test]
    fn simd_be_rong_bat_thuong_khong_lam_hong_gi() {
        let p = phan_tich_simd(100, 1);
        assert!((p.tang_toc_ly_thuyet - 1.0).abs() < 1e-9);
        let p0 = phan_tich_simd(100, 0);
        assert_eq!(p0.be_rong_vector, 1, "bề rộng 0 phải được chặn thành 1");
        let rong = phan_tich_simd(0, 8);
        assert!((rong.tang_toc_ly_thuyet - 1.0).abs() < 1e-9, "mảng rỗng không panic");
    }

    #[test]
    fn cong_mang_theo_lo_cho_cung_ket_qua_voi_moi_be_rong() {
        let a: Vec<f64> = (0..103).map(|i| i as f64).collect();
        let b: Vec<f64> = (0..103).map(|i| (i * 2) as f64).collect();
        let mong_doi: Vec<f64> = a.iter().zip(b.iter()).map(|(x, y)| x + y).collect();
        for w in [1usize, 2, 4, 8, 16] {
            assert_eq!(cong_mang_theo_lo(&a, &b, w), mong_doi,
                       "vector hoá bề rộng {} phải cho cùng kết quả", w);
        }
    }

    #[test]
    fn cong_mang_do_dai_khac_nhau_lay_phan_chung() {
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![10.0, 20.0];
        assert_eq!(cong_mang_theo_lo(&a, &b, 4), vec![11.0, 22.0]);
    }

    // ---------- Sinh dữ liệu ----------
    #[test]
    fn sinh_du_lieu_tat_dinh() {
        assert_eq!(sinh_du_lieu(100, 5), sinh_du_lieu(100, 5));
        assert_ne!(sinh_du_lieu(100, 5), sinh_du_lieu(100, 6));
    }

    #[test]
    fn du_lieu_sinh_ra_trai_deu_hai_phia_nguong() {
        let d = sinh_du_lieu(100_000, 42);
        let tren = d.iter().filter(|&&x| x >= 128).count();
        assert!((tren as f64 / d.len() as f64 - 0.5).abs() < 0.05,
                "phải chia đôi quanh ngưỡng để nhánh thật sự khó đoán");
    }
}
