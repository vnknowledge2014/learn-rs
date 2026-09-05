#![allow(dead_code, unused_variables)]
//! Chương 64 — Hệ điều hành từ bên trong: Lập lịch CPU, Phân trang bộ nhớ ảo,
//! Phát hiện bế tắc. Mô phỏng tất định nên kiểm thử được.

use std::collections::{HashMap, HashSet, VecDeque};

// ============================================================================
// 1. TIẾN TRÌNH & KHỐI ĐIỀU KHIỂN TIẾN TRÌNH (PCB)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrangThaiTienTrinh {
    Moi,        // vừa tạo
    SanSang,    // chờ được cấp CPU
    DangChay,   // đang giữ CPU
    Cho,        // chờ I/O
    KetThuc,
}

/// Khối điều khiển tiến trình — thứ mà nhân hệ điều hành lưu cho MỖI tiến trình.
#[derive(Debug, Clone, PartialEq)]
pub struct TienTrinh {
    pub pid: u32,
    pub ten: String,
    pub thoi_diem_den: u64,   // arrival time
    pub thoi_gian_can: u64,   // burst time — tổng CPU cần
    pub con_lai: u64,
    pub uu_tien: u8,          // số nhỏ = ưu tiên cao
    pub trang_thai: TrangThaiTienTrinh,
    pub bat_dau: Option<u64>,
    pub ket_thuc: Option<u64>,
}

impl TienTrinh {
    pub fn moi(pid: u32, ten: &str, den: u64, can: u64, uu_tien: u8) -> Self {
        TienTrinh {
            pid, ten: ten.to_string(), thoi_diem_den: den,
            thoi_gian_can: can, con_lai: can, uu_tien,
            trang_thai: TrangThaiTienTrinh::Moi, bat_dau: None, ket_thuc: None,
        }
    }
    /// Thời gian hoàn thành = lúc xong - lúc đến.
    pub fn thoi_gian_quay_vong(&self) -> Option<u64> {
        self.ket_thuc.map(|k| k - self.thoi_diem_den)
    }
    /// Thời gian chờ = quay vòng - thời gian thực sự dùng CPU.
    pub fn thoi_gian_cho(&self) -> Option<u64> {
        self.thoi_gian_quay_vong().map(|q| q - self.thoi_gian_can)
    }
}

#[derive(Debug, PartialEq)]
pub struct KetQuaLapLich {
    pub duong_thoi_gian: Vec<(u64, u32)>, // (thời điểm, pid đang chạy)
    pub tien_trinh: Vec<TienTrinh>,
    pub cho_trung_binh: f64,
    pub quay_vong_trung_binh: f64,
}

fn tong_ket(tt: Vec<TienTrinh>, dtg: Vec<(u64, u32)>) -> KetQuaLapLich {
    let n = tt.len() as f64;
    let tong_cho: u64 = tt.iter().filter_map(|p| p.thoi_gian_cho()).sum();
    let tong_qv: u64 = tt.iter().filter_map(|p| p.thoi_gian_quay_vong()).sum();
    KetQuaLapLich {
        duong_thoi_gian: dtg,
        cho_trung_binh: tong_cho as f64 / n,
        quay_vong_trung_binh: tong_qv as f64 / n,
        tien_trinh: tt,
    }
}

// ============================================================================
// 2. BA THUẬT TOÁN LẬP LỊCH CPU
// ============================================================================

/// FCFS (First-Come First-Served): ai đến trước chạy trước, chạy tới xong.
/// Nhược điểm kinh điển: "hiệu ứng đoàn xe" — một tiến trình dài chặn tất cả.
pub fn lap_lich_fcfs(mut tt: Vec<TienTrinh>) -> KetQuaLapLich {
    tt.sort_by_key(|p| (p.thoi_diem_den, p.pid));
    let mut dong_ho = 0u64;
    let mut dtg = Vec::new();
    for p in tt.iter_mut() {
        if dong_ho < p.thoi_diem_den {
            dong_ho = p.thoi_diem_den; // CPU rảnh, chờ tiến trình tới
        }
        p.bat_dau = Some(dong_ho);
        for _ in 0..p.thoi_gian_can {
            dtg.push((dong_ho, p.pid));
            dong_ho += 1;
        }
        p.con_lai = 0;
        p.ket_thuc = Some(dong_ho);
        p.trang_thai = TrangThaiTienTrinh::KetThuc;
    }
    tong_ket(tt, dtg)
}

/// SJF không tiếm quyền (Shortest Job First): luôn chọn việc NGẮN NHẤT đang chờ.
/// Tối ưu về thời gian chờ trung bình — nhưng có thể gây "đói" cho việc dài.
pub fn lap_lich_sjf(mut tt: Vec<TienTrinh>) -> KetQuaLapLich {
    let n = tt.len();
    let mut xong = 0;
    let mut dong_ho = 0u64;
    let mut dtg = Vec::new();
    let mut da_chay = vec![false; n];

    while xong < n {
        // Trong số các tiến trình ĐÃ TỚI và chưa chạy, chọn cái ngắn nhất
        let chon = (0..n)
            .filter(|&i| !da_chay[i] && tt[i].thoi_diem_den <= dong_ho)
            .min_by_key(|&i| (tt[i].thoi_gian_can, tt[i].pid));
        match chon {
            Some(i) => {
                tt[i].bat_dau = Some(dong_ho);
                for _ in 0..tt[i].thoi_gian_can {
                    dtg.push((dong_ho, tt[i].pid));
                    dong_ho += 1;
                }
                tt[i].con_lai = 0;
                tt[i].ket_thuc = Some(dong_ho);
                tt[i].trang_thai = TrangThaiTienTrinh::KetThuc;
                da_chay[i] = true;
                xong += 1;
            }
            None => dong_ho += 1, // chưa ai tới, CPU rảnh
        }
    }
    tong_ket(tt, dtg)
}

/// Round-Robin: mỗi tiến trình được một "lượng tử thời gian", hết thì nhường.
/// Đây là thuật toán của hệ điều hành tương tác — bảo đảm không ai bị đói.
pub fn lap_lich_round_robin(mut tt: Vec<TienTrinh>, luong_tu: u64) -> KetQuaLapLich {
    let n = tt.len();
    let mut dong_ho = 0u64;
    let mut dtg = Vec::new();
    let mut hang: VecDeque<usize> = VecDeque::new();
    let mut da_vao = vec![false; n];
    let mut xong = 0;

    // Đưa vào hàng đợi những tiến trình đã tới tại thời điểm 0
    let nap = |dong_ho: u64, hang: &mut VecDeque<usize>, da_vao: &mut Vec<bool>, tt: &Vec<TienTrinh>| {
        let mut moi: Vec<usize> = (0..tt.len())
            .filter(|&i| !da_vao[i] && tt[i].thoi_diem_den <= dong_ho)
            .collect();
        moi.sort_by_key(|&i| (tt[i].thoi_diem_den, tt[i].pid));
        for i in moi { da_vao[i] = true; hang.push_back(i); }
    };
    nap(dong_ho, &mut hang, &mut da_vao, &tt);

    while xong < n {
        match hang.pop_front() {
            Some(i) => {
                if tt[i].bat_dau.is_none() { tt[i].bat_dau = Some(dong_ho); }
                let chay = luong_tu.min(tt[i].con_lai);
                for _ in 0..chay {
                    dtg.push((dong_ho, tt[i].pid));
                    dong_ho += 1;
                    nap(dong_ho, &mut hang, &mut da_vao, &tt); // tiến trình mới tới trong lúc chạy
                }
                tt[i].con_lai -= chay;
                if tt[i].con_lai == 0 {
                    tt[i].ket_thuc = Some(dong_ho);
                    tt[i].trang_thai = TrangThaiTienTrinh::KetThuc;
                    xong += 1;
                } else {
                    hang.push_back(i); // chưa xong -> quay lại cuối hàng
                }
            }
            None => {
                dong_ho += 1;
                nap(dong_ho, &mut hang, &mut da_vao, &tt);
            }
        }
    }
    tong_ket(tt, dtg)
}

// ============================================================================
// 3. BỘ NHỚ ẢO — PHÂN TRANG & THAY TRANG
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct KetQuaThayTrang {
    pub so_loi_trang: usize, // page faults
    pub chuoi_khung: Vec<Vec<u64>>,
}

/// FIFO: trang vào trước ra trước. Đơn giản nhưng có "nghịch lý Belady".
pub fn thay_trang_fifo(chuoi: &[u64], so_khung: usize) -> KetQuaThayTrang {
    let mut khung: VecDeque<u64> = VecDeque::new();
    let mut trong: HashSet<u64> = HashSet::new();
    let mut loi = 0;
    let mut lich_su = Vec::new();
    for &t in chuoi {
        if !trong.contains(&t) {
            loi += 1;
            if khung.len() == so_khung {
                if let Some(cu) = khung.pop_front() { trong.remove(&cu); }
            }
            khung.push_back(t);
            trong.insert(t);
        }
        lich_su.push(khung.iter().copied().collect());
    }
    KetQuaThayTrang { so_loi_trang: loi, chuoi_khung: lich_su }
}

/// LRU (Least Recently Used): thay trang lâu không dùng nhất.
/// Xấp xỉ tốt cho "nguyên lý cục bộ" — chương trình hay dùng lại thứ vừa dùng.
pub fn thay_trang_lru(chuoi: &[u64], so_khung: usize) -> KetQuaThayTrang {
    let mut khung: Vec<u64> = Vec::new();
    let mut lan_cuoi: HashMap<u64, usize> = HashMap::new();
    let mut loi = 0;
    let mut lich_su = Vec::new();
    for (thoi_diem, &t) in chuoi.iter().enumerate() {
        if !khung.contains(&t) {
            loi += 1;
            if khung.len() == so_khung {
                // tìm trang có lần dùng cuối XA NHẤT
                let nan_nhan = khung.iter().copied()
                    .min_by_key(|p| *lan_cuoi.get(p).unwrap_or(&0)).unwrap();
                khung.retain(|&p| p != nan_nhan);
                lan_cuoi.remove(&nan_nhan);
            }
            khung.push(t);
        }
        lan_cuoi.insert(t, thoi_diem);
        lich_su.push(khung.clone());
    }
    KetQuaThayTrang { so_loi_trang: loi, chuoi_khung: lich_su }
}

/// OPT (tối ưu, Bélády): thay trang sẽ được dùng XA NHẤT trong tương lai.
/// Không cài được thật (cần biết tương lai) nhưng là CHUẨN SO SÁNH lý thuyết.
pub fn thay_trang_toi_uu(chuoi: &[u64], so_khung: usize) -> KetQuaThayTrang {
    let mut khung: Vec<u64> = Vec::new();
    let mut loi = 0;
    let mut lich_su = Vec::new();
    for i in 0..chuoi.len() {
        let t = chuoi[i];
        if !khung.contains(&t) {
            loi += 1;
            if khung.len() == so_khung {
                // trang nào KHÔNG xuất hiện lại, hoặc xuất hiện muộn nhất -> loại
                let nan_nhan = khung.iter().copied().max_by_key(|p| {
                    chuoi[i + 1..].iter().position(|x| x == p).unwrap_or(usize::MAX)
                }).unwrap();
                khung.retain(|&p| p != nan_nhan);
            }
            khung.push(t);
        }
        lich_su.push(khung.clone());
    }
    KetQuaThayTrang { so_loi_trang: loi, chuoi_khung: lich_su }
}

// ============================================================================
// 4. BẾ TẮC (Deadlock) — PHÁT HIỆN BẰNG ĐỒ THỊ CHỜ
// ============================================================================

/// Đồ thị "chờ đợi": tiến trình A -> B nghĩa là A đang chờ tài nguyên B giữ.
/// Có CHU TRÌNH trong đồ thị này = có BẾ TẮC.
pub struct DoThiCho {
    canh: HashMap<u32, Vec<u32>>,
}

impl DoThiCho {
    pub fn moi() -> Self { DoThiCho { canh: HashMap::new() } }
    pub fn them_cho(&mut self, ai_cho: u32, cho_ai: u32) {
        self.canh.entry(ai_cho).or_default().push(cho_ai);
    }

    /// Phát hiện bế tắc = tìm chu trình bằng DFS 3 màu.
    pub fn co_be_tac(&self) -> Option<Vec<u32>> {
        let mut mau: HashMap<u32, u8> = HashMap::new(); // 0=trắng 1=xám 2=đen
        let mut duong: Vec<u32> = Vec::new();
        let mut dinh: Vec<u32> = self.canh.keys().copied().collect();
        dinh.sort();
        for d in dinh {
            if mau.get(&d).copied().unwrap_or(0) == 0 {
                if let Some(chu_trinh) = self.dfs(d, &mut mau, &mut duong) {
                    return Some(chu_trinh);
                }
            }
        }
        None
    }

    fn dfs(&self, d: u32, mau: &mut HashMap<u32, u8>, duong: &mut Vec<u32>) -> Option<Vec<u32>> {
        mau.insert(d, 1); // xám = đang thăm
        duong.push(d);
        if let Some(ke) = self.canh.get(&d) {
            let mut ke = ke.clone();
            ke.sort();
            for k in ke {
                match mau.get(&k).copied().unwrap_or(0) {
                    1 => {
                        // gặp lại đỉnh XÁM -> có chu trình
                        let bat_dau = duong.iter().position(|&x| x == k).unwrap();
                        return Some(duong[bat_dau..].to_vec());
                    }
                    0 => {
                        if let Some(c) = self.dfs(k, mau, duong) { return Some(c); }
                    }
                    _ => {}
                }
            }
        }
        duong.pop();
        mau.insert(d, 2); // đen = xong
        None
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   HỆ ĐIỀU HÀNH: LẬP LỊCH CPU · PHÂN TRANG · PHÁT HIỆN BẾ TẮC   ");
    println!("═══════════════════════════════════════════════════════════════");

    let tao = || vec![
        TienTrinh::moi(1, "trinh-duyet", 0, 8, 2),
        TienTrinh::moi(2, "trinh-soan-thao", 1, 4, 1),
        TienTrinh::moi(3, "nen-video", 2, 9, 3),
        TienTrinh::moi(4, "dong-bo-may", 3, 5, 2),
    ];

    println!("\n1. LẬP LỊCH CPU — cùng 4 tiến trình, ba thuật toán");
    for (ten, kq) in [
        ("FCFS       ", lap_lich_fcfs(tao())),
        ("SJF        ", lap_lich_sjf(tao())),
        ("Round-Robin", lap_lich_round_robin(tao(), 3)),
    ] {
        println!("   {} | chờ TB = {:>5.2} | quay vòng TB = {:>5.2}",
                 ten, kq.cho_trung_binh, kq.quay_vong_trung_binh);
    }
    println!("   → SJF tối ưu thời gian chờ, nhưng Round-Robin công bằng hơn (không ai bị đói).");

    println!("\n2. THAY TRANG BỘ NHỚ ẢO (3 khung nhớ)");
    let chuoi = [7u64, 0, 1, 2, 0, 3, 0, 4, 2, 3, 0, 3, 2, 1, 2, 0, 1, 7, 0, 1];
    for (ten, kq) in [
        ("FIFO   ", thay_trang_fifo(&chuoi, 3)),
        ("LRU    ", thay_trang_lru(&chuoi, 3)),
        ("Tối ưu ", thay_trang_toi_uu(&chuoi, 3)),
    ] {
        println!("   {} | {} lỗi trang", ten, kq.so_loi_trang);
    }
    println!("   → Tối ưu là CẬN DƯỚI lý thuyết (cần biết tương lai). LRU bám sát nó nhất.");

    println!("\n3. NGHỊCH LÝ BÉLÁDY — thêm khung nhớ mà LỖI TRANG TĂNG!");
    let belady = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
    println!("   FIFO 3 khung: {} lỗi", thay_trang_fifo(&belady, 3).so_loi_trang);
    println!("   FIFO 4 khung: {} lỗi  ← NHIỀU HƠN dù có thêm bộ nhớ!", thay_trang_fifo(&belady, 4).so_loi_trang);
    println!("   LRU  3 khung: {} lỗi", thay_trang_lru(&belady, 3).so_loi_trang);
    println!("   LRU  4 khung: {} lỗi  ← LRU không bị nghịch lý này", thay_trang_lru(&belady, 4).so_loi_trang);

    println!("\n4. PHÁT HIỆN BẾ TẮC");
    let mut g = DoThiCho::moi();
    g.them_cho(1, 2); // P1 chờ tài nguyên P2 giữ
    g.them_cho(2, 3);
    g.them_cho(3, 1); // ... và P3 chờ P1 -> VÒNG TRÒN
    println!("   Đồ thị P1→P2→P3→P1: {:?}", g.co_be_tac());
    let mut g2 = DoThiCho::moi();
    g2.them_cho(1, 2);
    g2.them_cho(2, 3);
    println!("   Đồ thị P1→P2→P3   : {:?} (không bế tắc)", g2.co_be_tac());

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   HỆ ĐIỀU HÀNH = TRỌNG TÀI PHÂN PHỐI TÀI NGUYÊN CÓ HẠN         ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn mau() -> Vec<TienTrinh> {
        vec![
            TienTrinh::moi(1, "A", 0, 5, 1),
            TienTrinh::moi(2, "B", 1, 3, 2),
            TienTrinh::moi(3, "C", 2, 1, 3),
        ]
    }

    #[test]
    fn fcfs_chay_theo_thu_tu_den() {
        let kq = lap_lich_fcfs(mau());
        // A(0-5), B(5-8), C(8-9)
        assert_eq!(kq.tien_trinh[0].ket_thuc, Some(5));
        assert_eq!(kq.tien_trinh[1].ket_thuc, Some(8));
        assert_eq!(kq.tien_trinh[2].ket_thuc, Some(9));
        assert_eq!(kq.duong_thoi_gian.len(), 9); // tổng burst = 5+3+1
    }

    #[test]
    fn sjf_cho_trung_binh_thap_hon_fcfs() {
        let f = lap_lich_fcfs(mau());
        let s = lap_lich_sjf(mau());
        // SJF tối ưu thời gian chờ trung bình (định lý kinh điển)
        assert!(s.cho_trung_binh <= f.cho_trung_binh,
                "SJF ({}) phải <= FCFS ({})", s.cho_trung_binh, f.cho_trung_binh);
    }

    #[test]
    fn round_robin_khong_bo_doi_ai() {
        let kq = lap_lich_round_robin(mau(), 2);
        // Mọi tiến trình đều hoàn thành
        assert!(kq.tien_trinh.iter().all(|p| p.ket_thuc.is_some()));
        assert!(kq.tien_trinh.iter().all(|p| p.con_lai == 0));
        // Tổng thời gian CPU đúng bằng tổng burst
        assert_eq!(kq.duong_thoi_gian.len(), 9);
    }

    #[test]
    fn moi_thuat_toan_deu_chay_du_tong_burst() {
        for kq in [lap_lich_fcfs(mau()), lap_lich_sjf(mau()), lap_lich_round_robin(mau(), 3)] {
            assert_eq!(kq.duong_thoi_gian.len(), 9, "phải dùng đúng 9 đơn vị CPU");
        }
    }

    #[test]
    fn thay_trang_toi_uu_luon_tot_nhat() {
        let chuoi = [7u64, 0, 1, 2, 0, 3, 0, 4, 2, 3, 0, 3, 2, 1, 2, 0, 1, 7, 0, 1];
        let opt = thay_trang_toi_uu(&chuoi, 3).so_loi_trang;
        let lru = thay_trang_lru(&chuoi, 3).so_loi_trang;
        let fifo = thay_trang_fifo(&chuoi, 3).so_loi_trang;
        // OPT là cận dưới lý thuyết — không thuật toán nào tốt hơn
        assert!(opt <= lru, "OPT({}) phải <= LRU({})", opt, lru);
        assert!(opt <= fifo, "OPT({}) phải <= FIFO({})", opt, fifo);
    }

    #[test]
    fn nghich_ly_belady_co_that_voi_fifo() {
        let chuoi = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
        let ba = thay_trang_fifo(&chuoi, 3).so_loi_trang;
        let bon = thay_trang_fifo(&chuoi, 4).so_loi_trang;
        // NGHỊCH LÝ: thêm khung nhớ mà lỗi trang lại TĂNG
        assert!(bon > ba, "Bélády: FIFO 4 khung ({}) phải nhiều lỗi hơn 3 khung ({})", bon, ba);
    }

    #[test]
    fn lru_khong_bi_nghich_ly_belady() {
        let chuoi = [1u64, 2, 3, 4, 1, 2, 5, 1, 2, 3, 4, 5];
        let ba = thay_trang_lru(&chuoi, 3).so_loi_trang;
        let bon = thay_trang_lru(&chuoi, 4).so_loi_trang;
        // LRU là thuật toán "ngăn xếp" -> thêm khung KHÔNG BAO GIỜ làm tệ hơn
        assert!(bon <= ba, "LRU 4 khung ({}) không được tệ hơn 3 khung ({})", bon, ba);
    }

    #[test]
    fn khung_du_lon_thi_chi_loi_trang_lan_dau() {
        let chuoi = [1u64, 2, 3, 1, 2, 3, 1, 2, 3];
        // 3 trang khác nhau, 5 khung -> chỉ 3 lỗi bắt buộc (compulsory miss)
        assert_eq!(thay_trang_lru(&chuoi, 5).so_loi_trang, 3);
        assert_eq!(thay_trang_fifo(&chuoi, 5).so_loi_trang, 3);
    }

    #[test]
    fn phat_hien_be_tac_khi_co_chu_trinh() {
        let mut g = DoThiCho::moi();
        g.them_cho(1, 2);
        g.them_cho(2, 3);
        g.them_cho(3, 1);
        let ct = g.co_be_tac().expect("phải phát hiện bế tắc");
        assert_eq!(ct.len(), 3);
        assert!(ct.contains(&1) && ct.contains(&2) && ct.contains(&3));
    }

    #[test]
    fn khong_bao_be_tac_khi_do_thi_khong_chu_trinh() {
        let mut g = DoThiCho::moi();
        g.them_cho(1, 2);
        g.them_cho(2, 3);
        g.them_cho(1, 3); // vẫn không có chu trình
        assert_eq!(g.co_be_tac(), None);
    }

    #[test]
    fn be_tac_hai_tien_trinh_kinh_dien() {
        // P1 giữ A chờ B; P2 giữ B chờ A — bế tắc đơn giản nhất
        let mut g = DoThiCho::moi();
        g.them_cho(1, 2);
        g.them_cho(2, 1);
        assert!(g.co_be_tac().is_some());
    }
}
