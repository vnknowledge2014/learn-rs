#![allow(dead_code)]
//! Chương 76 — Ghi & Phát lại phiên giao dịch: định dạng bản ghi, đồng hồ ảo,
//! phát lại đúng dòng thời gian hoặc tua nhanh, mô hình độ trễ, và mô phỏng
//! khớp lệnh có xét vị trí hàng đợi.
//!
//! Đây là "phòng thí nghiệm" của mọi đội giao dịch nghiêm túc: ghi lại phiên
//! thật một lần, rồi chạy lại hàng nghìn lần với các chiến lược khác nhau,
//! kết quả TÁI LẬP TUYỆT ĐỐI.

use std::collections::BTreeMap;

// ============================================================================
// 1. ĐỊNH DẠNG BẢN GHI — khung có tiền tố độ dài
// ============================================================================
// Mỗi khung: [độ dài u32 BE][thời điểm ns u64 BE][thân bản tin].
// Tiền tố độ dài cho phép đọc tuần tự mà không cần phân tích thân — nên bộ
// ghi có thể lưu BẤT KỲ giao thức nào mà không cần hiểu nó.

pub type Gia = i64;
pub type SoLuong = u32;
pub type MaLenh = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chieu { Mua, Ban }

impl Chieu {
    pub fn nguoc(self) -> Chieu {
        match self { Chieu::Mua => Chieu::Ban, Chieu::Ban => Chieu::Mua }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SuKienThiTruong {
    ThemLenh { ma: MaLenh, chieu: Chieu, gia: Gia, so_luong: SoLuong },
    HuyLenh { ma: MaLenh },
    KhopLenh { gia: Gia, so_luong: SoLuong, chieu_chu_dong: Chieu },
}

#[derive(Debug, Clone, PartialEq)]
pub struct KhungGhi {
    /// Nano-giây kể từ mốc bắt đầu phiên. KHÔNG dùng ngày lịch — múi giờ,
    /// giờ mùa hè và giây nhuận đều là nguồn lỗi không đáng chuốc vào.
    pub thoi_diem_ns: u64,
    pub su_kien: SuKienThiTruong,
}

#[derive(Debug, PartialEq)]
pub enum LoiDoc { KhungCut, DoDaiVoLy(u32), MaSuKienLa(u8) }

/// Bộ ghi phiên. Trong hệ thống thật, `noi_dung` được xả xuống đĩa theo lô;
/// ở đây giữ trong bộ nhớ để kiểm thử được.
#[derive(Debug, Default)]
pub struct BoGhiPhien {
    pub noi_dung: Vec<u8>,
    pub so_khung: u64,
    pub thoi_diem_dau: Option<u64>,
    pub thoi_diem_cuoi: u64,
}

impl BoGhiPhien {
    pub fn moi() -> Self { BoGhiPhien::default() }

    pub fn ghi(&mut self, k: &KhungGhi) {
        let than = ma_hoa_su_kien(&k.su_kien);
        let do_dai = (8 + than.len()) as u32;
        self.noi_dung.extend_from_slice(&do_dai.to_be_bytes());
        self.noi_dung.extend_from_slice(&k.thoi_diem_ns.to_be_bytes());
        self.noi_dung.extend_from_slice(&than);
        self.so_khung += 1;
        if self.thoi_diem_dau.is_none() { self.thoi_diem_dau = Some(k.thoi_diem_ns); }
        self.thoi_diem_cuoi = k.thoi_diem_ns;
    }

    pub fn thoi_luong_ns(&self) -> u64 {
        self.thoi_diem_cuoi - self.thoi_diem_dau.unwrap_or(0)
    }
    pub fn so_byte(&self) -> usize { self.noi_dung.len() }

    /// Đọc lại toàn bộ. Trả lỗi nếu bản ghi bị cắt cụt — chuyện thường gặp khi
    /// tiến trình ghi bị giết giữa chừng, và phải xử lý được chứ không panic.
    pub fn doc_lai(&self) -> Result<Vec<KhungGhi>, LoiDoc> {
        let mut ra = Vec::new();
        let b = &self.noi_dung;
        let mut i = 0usize;
        while i < b.len() {
            if i + 4 > b.len() { return Err(LoiDoc::KhungCut); }
            let do_dai = u32::from_be_bytes(b[i..i + 4].try_into().unwrap()) as usize;
            if do_dai < 8 { return Err(LoiDoc::DoDaiVoLy(do_dai as u32)); }
            if i + 4 + do_dai > b.len() { return Err(LoiDoc::KhungCut); }
            let thoi_diem_ns = u64::from_be_bytes(b[i + 4..i + 12].try_into().unwrap());
            let su_kien = giai_ma_su_kien(&b[i + 12..i + 4 + do_dai])?;
            ra.push(KhungGhi { thoi_diem_ns, su_kien });
            i += 4 + do_dai;
        }
        Ok(ra)
    }
}

fn ma_hoa_su_kien(sk: &SuKienThiTruong) -> Vec<u8> {
    let mut v = Vec::with_capacity(24);
    match sk {
        SuKienThiTruong::ThemLenh { ma, chieu, gia, so_luong } => {
            v.push(b'A');
            v.extend_from_slice(&ma.to_be_bytes());
            v.push(if *chieu == Chieu::Mua { b'B' } else { b'S' });
            v.extend_from_slice(&gia.to_be_bytes());
            v.extend_from_slice(&so_luong.to_be_bytes());
        }
        SuKienThiTruong::HuyLenh { ma } => {
            v.push(b'X');
            v.extend_from_slice(&ma.to_be_bytes());
        }
        SuKienThiTruong::KhopLenh { gia, so_luong, chieu_chu_dong } => {
            v.push(b'T');
            v.extend_from_slice(&gia.to_be_bytes());
            v.extend_from_slice(&so_luong.to_be_bytes());
            v.push(if *chieu_chu_dong == Chieu::Mua { b'B' } else { b'S' });
        }
    }
    v
}

fn giai_ma_su_kien(b: &[u8]) -> Result<SuKienThiTruong, LoiDoc> {
    if b.is_empty() { return Err(LoiDoc::KhungCut); }
    let can = match b[0] {
        b'A' => 22, b'X' => 9, b'T' => 14,
        x => return Err(LoiDoc::MaSuKienLa(x)),
    };
    if b.len() < can { return Err(LoiDoc::KhungCut); }
    Ok(match b[0] {
        b'A' => SuKienThiTruong::ThemLenh {
            ma: u64::from_be_bytes(b[1..9].try_into().unwrap()),
            chieu: if b[9] == b'B' { Chieu::Mua } else { Chieu::Ban },
            gia: i64::from_be_bytes(b[10..18].try_into().unwrap()),
            so_luong: u32::from_be_bytes(b[18..22].try_into().unwrap()),
        },
        b'X' => SuKienThiTruong::HuyLenh {
            ma: u64::from_be_bytes(b[1..9].try_into().unwrap()),
        },
        _ => SuKienThiTruong::KhopLenh {
            gia: i64::from_be_bytes(b[1..9].try_into().unwrap()),
            so_luong: u32::from_be_bytes(b[9..13].try_into().unwrap()),
            chieu_chu_dong: if b[13] == b'B' { Chieu::Mua } else { Chieu::Ban },
        },
    })
}

// ============================================================================
// 2. ĐỒNG HỒ ẢO — thứ khiến phát lại TÁI LẬP ĐƯỢC
// ============================================================================
// Điều kiện sống còn: chiến lược KHÔNG ĐƯỢC gọi đồng hồ hệ thống. Nó chỉ được
// hỏi đồng hồ ảo do bộ phát lại điều khiển. Nhờ vậy hai lần chạy trên cùng dữ
// liệu cho ra kết quả giống hệt nhau, bất kể máy nhanh hay chậm.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DongHoAo { pub bay_gio_ns: u64 }

impl DongHoAo {
    pub fn moi(bat_dau: u64) -> Self { DongHoAo { bay_gio_ns: bat_dau } }
    pub fn tien_toi(&mut self, ns: u64) { if ns > self.bay_gio_ns { self.bay_gio_ns = ns; } }
    pub fn cong_them(&mut self, ns: u64) { self.bay_gio_ns += ns; }
}

/// Tốc độ phát lại.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TocDoPhat {
    /// Đúng nhịp thật: giữ nguyên khoảng cách giữa các sự kiện.
    ThoiGianThuc,
    /// Nhân tốc độ: 2.0 = nhanh gấp đôi, 0.5 = chậm một nửa (để quan sát kỹ).
    HeSo(f64),
    /// Bỏ hẳn thời gian chờ — dùng khi quét tham số hàng nghìn lần.
    NhanhNhatCoThe,
}

impl TocDoPhat {
    /// Thời gian THỰC (nano-giây) phải chờ, ứng với `khoang_cach_ns` trong dữ liệu.
    pub fn cho_bao_lau(&self, khoang_cach_ns: u64) -> u64 {
        match self {
            TocDoPhat::ThoiGianThuc => khoang_cach_ns,
            TocDoPhat::HeSo(h) if *h > 0.0 => (khoang_cach_ns as f64 / h) as u64,
            _ => 0,
        }
    }
}

// ============================================================================
// 3. MÔ HÌNH ĐỘ TRỄ — lệnh của ta KHÔNG tới nơi tức thì
// ============================================================================
// Bỏ qua độ trễ là cách nhanh nhất để dựng ra một chiến lược "thắng" trên
// giấy rồi thua tiền thật. Ở tốc độ HFT, 50 µs là đủ để cơ hội biến mất.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MoHinhDoTre {
    /// Từ lúc sàn phát tin tới lúc ta nhận được.
    pub vao_ns: u64,
    /// Từ lúc ta quyết định tới lúc lệnh tới sàn.
    pub ra_ns: u64,
    /// Dao động cộng thêm (tất định, dựa trên số thứ tự sự kiện).
    pub dao_dong_ns: u64,
}

impl MoHinhDoTre {
    pub fn khong_do_tre() -> Self { MoHinhDoTre { vao_ns: 0, ra_ns: 0, dao_dong_ns: 0 } }
    pub fn dat_thue_rieng() -> Self {
        MoHinhDoTre { vao_ns: 5_000, ra_ns: 8_000, dao_dong_ns: 2_000 }
    }
    pub fn qua_internet() -> Self {
        MoHinhDoTre { vao_ns: 8_000_000, ra_ns: 12_000_000, dao_dong_ns: 5_000_000 }
    }

    /// Tổng thời gian từ lúc SÀN phát tin tới lúc lệnh của ta ĐẾN SÀN.
    /// Đây chính là "tick-to-trade" mà Chương 74 mổ xẻ.
    pub fn khu_hoi_ns(&self, so_thu_tu: u64) -> u64 {
        // Dao động tất định: cùng chuỗi sự kiện luôn cho cùng độ trễ
        let d = if self.dao_dong_ns == 0 { 0 } else {
            (so_thu_tu.wrapping_mul(2654435761) >> 32) % self.dao_dong_ns
        };
        self.vao_ns + self.ra_ns + d
    }
}

// ============================================================================
// 4. SỔ LỆNH RÚT GỌN CHO MÔ PHỎNG
// ============================================================================

#[derive(Debug, Default, Clone)]
pub struct SoRutGon {
    mua: BTreeMap<Gia, u64>, // khoá ÂM → giá cao nhất trước
    ban: BTreeMap<Gia, u64>,
}

impl SoRutGon {
    pub fn them(&mut self, c: Chieu, g: Gia, kl: u64) {
        let (bd, k) = match c {
            Chieu::Mua => (&mut self.mua, -g), Chieu::Ban => (&mut self.ban, g) };
        *bd.entry(k).or_insert(0) += kl;
    }
    pub fn bot(&mut self, c: Chieu, g: Gia, kl: u64) {
        let (bd, k) = match c {
            Chieu::Mua => (&mut self.mua, -g), Chieu::Ban => (&mut self.ban, g) };
        if let Some(v) = bd.get_mut(&k) {
            *v = v.saturating_sub(kl);
            if *v == 0 { bd.remove(&k); }
        }
    }
    pub fn mua_tot_nhat(&self) -> Option<Gia> { self.mua.keys().next().map(|k| -k) }
    pub fn ban_tot_nhat(&self) -> Option<Gia> { self.ban.keys().next().copied() }
    pub fn khoi_luong(&self, c: Chieu, g: Gia) -> u64 {
        let (bd, k) = match c { Chieu::Mua => (&self.mua, -g), Chieu::Ban => (&self.ban, g) };
        bd.get(&k).copied().unwrap_or(0)
    }
}

// ============================================================================
// 5. LỆNH CỦA TA TRONG MÔ PHỎNG
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct LenhCuaTa {
    pub ma: MaLenh,
    pub chieu: Chieu,
    pub gia: Gia,
    pub so_luong: SoLuong,
    pub da_khop: SoLuong,
    /// Khối lượng đứng TRƯỚC ta trong hàng lúc lệnh tới sàn. Phải khớp hết
    /// chỗ đó thì mới tới lượt ta — đây là điểm mà phần lớn bộ kiểm định
    /// nghiệp dư bỏ qua, và vì thế cho kết quả lạc quan phi thực tế.
    pub khoi_luong_truoc_mat: u64,
    pub thoi_diem_toi_san_ns: u64,
}

impl LenhCuaTa {
    pub fn con_lai(&self) -> SoLuong { self.so_luong - self.da_khop }
    pub fn khop_het(&self) -> bool { self.da_khop >= self.so_luong }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KhopCuaTa {
    pub ma_lenh: MaLenh,
    pub chieu: Chieu,
    pub gia: Gia,
    pub so_luong: SoLuong,
    pub thoi_diem_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ViThe { pub so_luong: i64, pub tien_mat: i64 }

impl ViThe {
    pub fn ghep(self, k: ViThe) -> ViThe {
        ViThe { so_luong: self.so_luong + k.so_luong, tien_mat: self.tien_mat + k.tien_mat }
    }
    pub fn tu_khop(c: Chieu, g: Gia, sl: SoLuong) -> ViThe {
        let dau = if c == Chieu::Mua { 1 } else { -1 };
        ViThe { so_luong: dau * sl as i64, tien_mat: -dau * g * sl as i64 }
    }
    pub fn gia_tri_rong(&self, gia_tt: Gia) -> i64 { self.tien_mat + self.so_luong * gia_tt }
}

/// Chiến lược nhìn thấy gì và làm gì. Thuần tuý: cùng đầu vào → cùng đầu ra.
pub trait ChienLuocPhatLai {
    fn ten(&self) -> &str;
    /// Gọi sau MỖI sự kiện thị trường. Trả về các lệnh muốn gửi.
    fn khi_co_su_kien(&mut self, dong_ho: &DongHoAo, so: &SoRutGon,
                      vi_the: &ViThe) -> Vec<(Chieu, Gia, SoLuong)>;
    /// Gọi khi một lệnh của ta được khớp.
    fn khi_duoc_khop(&mut self, _k: &KhopCuaTa) {}
}

// ============================================================================
// 6. BỘ PHÁT LẠI
// ============================================================================

#[derive(Debug, PartialEq)]
pub struct KetQuaPhatLai {
    pub so_su_kien: u64,
    pub so_lenh_gui: u64,
    pub so_lenh_khop: u64,
    pub cac_khop: Vec<KhopCuaTa>,
    pub vi_the_cuoi: ViThe,
    pub gia_tri_cuoi: i64,
    /// Tổng thời gian ẢO đã trôi qua.
    pub thoi_gian_ao_ns: u64,
    /// Tổng thời gian THỰC phải chờ nếu chạy ở tốc độ đã chọn.
    pub thoi_gian_cho_thuc_ns: u64,
}

pub struct BoPhatLai {
    pub do_tre: MoHinhDoTre,
    pub toc_do: TocDoPhat,
}

impl BoPhatLai {
    pub fn moi(do_tre: MoHinhDoTre, toc_do: TocDoPhat) -> Self {
        BoPhatLai { do_tre, toc_do }
    }

    /// Chạy lại phiên. Toàn bộ là hàm THUẦN TUÝ trên `cac_khung` — không đọc
    /// đồng hồ hệ thống, không đọc tệp, không ngẫu nhiên.
    pub fn chay(&self, cac_khung: &[KhungGhi], cl: &mut dyn ChienLuocPhatLai) -> KetQuaPhatLai {
        let mut so = SoRutGon::default();
        // Phải theo dõi từng lệnh của THỊ TRƯỜNG thì mới xử lý được lệnh huỷ.
        // Bỏ qua huỷ lệnh là lỗi mô hình nghiêm trọng: sổ chỉ phình ra, các
        // mức giá cũ không bao giờ biến mất, và chỉ sau vài nghìn sự kiện là
        // sổ bị chéo vĩnh viễn — chiến lược đứng ngoài mà ta không hiểu vì sao.
        // BTreeMap chứ KHÔNG phải HashMap: ta duyệt bản đồ này khi phân bổ
        // khối lượng khớp, mà thứ tự duyệt HashMap trong Rust KHÔNG TẤT ĐỊNH
        // giữa các lần chạy (hạt giống băm ngẫu nhiên chống tấn công HashDoS).
        // Dùng HashMap ở đây làm hỏng luôn tính tái lập của cả bộ phát lại —
        // đúng thứ mà chương này tồn tại để bảo vệ.
        let mut lenh_thi_truong: BTreeMap<MaLenh, (Chieu, Gia, u64)> = BTreeMap::new();
        let mut dong_ho = DongHoAo::moi(cac_khung.first().map_or(0, |k| k.thoi_diem_ns));
        let mut lenh_cho: Vec<LenhCuaTa> = Vec::new();
        let mut cac_khop = Vec::new();
        let mut vi_the = ViThe::default();
        let mut ma_ke = 1u64;
        let mut so_lenh_gui = 0u64;
        let mut cho_thuc = 0u64;
        let mut gia_cuoi: Gia = 0;
        let mut truoc_do = dong_ho.bay_gio_ns;

        for (i, k) in cac_khung.iter().enumerate() {
            cho_thuc += self.toc_do.cho_bao_lau(k.thoi_diem_ns.saturating_sub(truoc_do));
            truoc_do = k.thoi_diem_ns;
            dong_ho.tien_toi(k.thoi_diem_ns);

            // --- Lệnh nào vừa "bay tới sàn" thì chốt vị trí hàng đợi NGAY LÚC ĐÓ,
            //     không phải lúc ta quyết định. Đây là chi tiết quyết định tính
            //     thực tế của toàn bộ mô phỏng.
            for l in lenh_cho.iter_mut() {
                if l.thoi_diem_toi_san_ns <= dong_ho.bay_gio_ns
                    && l.khoi_luong_truoc_mat == u64::MAX {
                    l.khoi_luong_truoc_mat = so.khoi_luong(l.chieu, l.gia);
                }
            }

            // --- Áp dụng sự kiện thị trường ---
            match &k.su_kien {
                SuKienThiTruong::ThemLenh { ma, chieu, gia, so_luong } => {
                    so.them(*chieu, *gia, *so_luong as u64);
                    lenh_thi_truong.insert(*ma, (*chieu, *gia, *so_luong as u64));
                }
                SuKienThiTruong::HuyLenh { ma } => {
                    if let Some((c, g, kl)) = lenh_thi_truong.remove(ma) {
                        so.bot(c, g, kl);
                    }
                }
                SuKienThiTruong::KhopLenh { gia, so_luong, chieu_chu_dong } => {
                    gia_cuoi = *gia;
                    // Lệnh khớp ăn vào bên THỤ ĐỘNG
                    let ben_bi_an = chieu_chu_dong.nguoc();
                    so.bot(ben_bi_an, *gia, *so_luong as u64);
                    // Khớp cũng làm cạn lệnh thị trường ở mức giá đó
                    let mut con_an = *so_luong as u64;
                    let mut can_xoa: Vec<MaLenh> = Vec::new();
                    for (m, (c, g, kl)) in lenh_thi_truong.iter_mut() {
                        if con_an == 0 { break; }
                        if *c != ben_bi_an || *g != *gia { continue; }
                        let an = con_an.min(*kl);
                        *kl -= an;
                        con_an -= an;
                        if *kl == 0 { can_xoa.push(*m); }
                    }
                    for m in can_xoa { lenh_thi_truong.remove(&m); }

                    // Lệnh của ta cùng bên thụ động, cùng giá thì có thể tới lượt
                    let mut con = *so_luong as u64;
                    for l in lenh_cho.iter_mut() {
                        if con == 0 { break; }
                        if l.khop_het() || l.chieu != ben_bi_an || l.gia != *gia { continue; }
                        if l.thoi_diem_toi_san_ns > dong_ho.bay_gio_ns { continue; }
                        // Trước hết phải "ăn" hết phần đứng trước ta
                        let an_truoc = con.min(l.khoi_luong_truoc_mat);
                        l.khoi_luong_truoc_mat -= an_truoc;
                        con -= an_truoc;
                        if l.khoi_luong_truoc_mat > 0 || con == 0 { continue; }
                        // Giờ mới tới lượt ta
                        let khop = con.min(l.con_lai() as u64) as SoLuong;
                        if khop > 0 {
                            l.da_khop += khop;
                            con -= khop as u64;
                            let kq = KhopCuaTa { ma_lenh: l.ma, chieu: l.chieu, gia: *gia,
                                                 so_luong: khop, thoi_diem_ns: dong_ho.bay_gio_ns };
                            vi_the = vi_the.ghep(ViThe::tu_khop(l.chieu, *gia, khop));
                            cl.khi_duoc_khop(&kq);
                            cac_khop.push(kq);
                        }
                    }
                }
            }

            // --- Chiến lược quyết định ---
            for (chieu, gia, sl) in cl.khi_co_su_kien(&dong_ho, &so, &vi_the) {
                if sl == 0 { continue; }
                lenh_cho.push(LenhCuaTa {
                    ma: ma_ke, chieu, gia, so_luong: sl, da_khop: 0,
                    khoi_luong_truoc_mat: u64::MAX, // chốt sau, lúc tới sàn
                    thoi_diem_toi_san_ns: dong_ho.bay_gio_ns + self.do_tre.khu_hoi_ns(i as u64),
                });
                ma_ke += 1;
                so_lenh_gui += 1;
            }
            lenh_cho.retain(|l| !l.khop_het());
        }

        KetQuaPhatLai {
            so_su_kien: cac_khung.len() as u64,
            so_lenh_gui,
            so_lenh_khop: cac_khop.len() as u64,
            gia_tri_cuoi: vi_the.gia_tri_rong(gia_cuoi),
            vi_the_cuoi: vi_the,
            cac_khop,
            thoi_gian_ao_ns: cac_khung.last().map_or(0, |k| k.thoi_diem_ns)
                             - cac_khung.first().map_or(0, |k| k.thoi_diem_ns),
            thoi_gian_cho_thuc_ns: cho_thuc,
        }
    }
}

// ============================================================================
// 7. CHIẾN LƯỢC MẪU
// ============================================================================

/// Tạo lập thị trường: đặt lệnh mua dưới và bán trên giá giữa, ăn chênh lệch.
pub struct TaoLapDonGian {
    pub do_lech_tick: Gia,
    pub co_lenh: SoLuong,
    pub vi_the_toi_da: i64,
    pub buoc: u64,
    pub moi_n_su_kien: u64,
}

impl ChienLuocPhatLai for TaoLapDonGian {
    fn ten(&self) -> &str { "Tạo lập thị trường đơn giản" }

    fn khi_co_su_kien(&mut self, _dh: &DongHoAo, so: &SoRutGon, vt: &ViThe)
        -> Vec<(Chieu, Gia, SoLuong)>
    {
        self.buoc += 1;
        if self.buoc % self.moi_n_su_kien != 0 { return vec![]; }
        let (m, b) = match (so.mua_tot_nhat(), so.ban_tot_nhat()) {
            (Some(m), Some(b)) => (m, b),
            _ => return vec![],
        };
        if b <= m { return vec![]; } // sổ chéo hoặc khoá → đứng ngoài
        let giua = (m + b) / 2;
        let mut ra = Vec::new();
        // Kiểm soát tồn kho: đã ôm nhiều thì thôi mua thêm
        if vt.so_luong < self.vi_the_toi_da {
            ra.push((Chieu::Mua, giua - self.do_lech_tick, self.co_lenh));
        }
        if vt.so_luong > -self.vi_the_toi_da {
            ra.push((Chieu::Ban, giua + self.do_lech_tick, self.co_lenh));
        }
        ra
    }
}

/// Bản CÓ KIỂM SOÁT: đếm cả khối lượng ĐANG TREO chứ không chỉ vị thế đã khớp.
///
/// Đây là khác biệt giữa một mô hình đồ chơi và một chiến lược dám chạy tiền
/// thật. Lệnh đã gửi mà chưa khớp vẫn là RỦI RO: nó có thể khớp bất cứ lúc nào.
/// Chỉ nhìn vị thế đã khớp thì cứ mỗi nhịp lại chào thêm, và khi thị trường
/// quét qua thì tất cả khớp một lượt — vị thế nhảy vọt qua trần.
pub struct TaoLapCoKiemSoat {
    pub do_lech_tick: Gia,
    pub co_lenh: SoLuong,
    pub vi_the_toi_da: i64,
    pub buoc: u64,
    pub moi_n_su_kien: u64,
    treo_mua: i64,
    treo_ban: i64,
}

impl TaoLapCoKiemSoat {
    pub fn moi(do_lech_tick: Gia, co_lenh: SoLuong, vi_the_toi_da: i64, moi_n_su_kien: u64) -> Self {
        TaoLapCoKiemSoat { do_lech_tick, co_lenh, vi_the_toi_da, buoc: 0,
                           moi_n_su_kien, treo_mua: 0, treo_ban: 0 }
    }
    pub fn dang_treo(&self) -> (i64, i64) { (self.treo_mua, self.treo_ban) }
}

impl ChienLuocPhatLai for TaoLapCoKiemSoat {
    fn ten(&self) -> &str { "Tạo lập có kiểm soát tồn kho" }

    fn khi_co_su_kien(&mut self, _dh: &DongHoAo, so: &SoRutGon, vt: &ViThe)
        -> Vec<(Chieu, Gia, SoLuong)>
    {
        self.buoc += 1;
        if self.buoc % self.moi_n_su_kien != 0 { return vec![]; }
        let (m, b) = match (so.mua_tot_nhat(), so.ban_tot_nhat()) {
            (Some(m), Some(b)) => (m, b), _ => return vec![],
        };
        if b <= m { return vec![]; }
        let giua = (m + b) / 2;
        let co = self.co_lenh as i64;
        let mut ra = Vec::new();
        // PHƠI BÀY = vị thế đã khớp + toàn bộ khối lượng đang treo cùng chiều
        if vt.so_luong + self.treo_mua + co <= self.vi_the_toi_da {
            ra.push((Chieu::Mua, giua - self.do_lech_tick, self.co_lenh));
            self.treo_mua += co;
        }
        if vt.so_luong - self.treo_ban - co >= -self.vi_the_toi_da {
            ra.push((Chieu::Ban, giua + self.do_lech_tick, self.co_lenh));
            self.treo_ban += co;
        }
        ra
    }

    fn khi_duoc_khop(&mut self, k: &KhopCuaTa) {
        // Khớp rồi thì phần đó không còn "treo" nữa — nó đã thành vị thế
        match k.chieu {
            Chieu::Mua => self.treo_mua = (self.treo_mua - k.so_luong as i64).max(0),
            Chieu::Ban => self.treo_ban = (self.treo_ban - k.so_luong as i64).max(0),
        }
    }
}

pub struct DungNgoai;
impl ChienLuocPhatLai for DungNgoai {
    fn ten(&self) -> &str { "Đứng ngoài" }
    fn khi_co_su_kien(&mut self, _: &DongHoAo, _: &SoRutGon, _: &ViThe)
        -> Vec<(Chieu, Gia, SoLuong)> { vec![] }
}

// ============================================================================
// 8. SINH PHIÊN TẤT ĐỊNH ĐỂ GHI LẠI
// ============================================================================

/// Sinh một phiên tất định. Hai chi tiết quyết định tính THỰC TẾ của nó:
///
/// 1. **Huỷ lệnh nhắm đúng lệnh CŨ NHẤT còn sống.** Thị trường thật rút báo
///    giá cũ liên tục (>90% lệnh bị huỷ trước khi khớp). Nếu huỷ theo mã ngẫu
///    nhiên, phần lớn lệnh huỷ trúng mã đã biến mất, báo giá cũ nằm lại mãi,
///    và sau vài nghìn sự kiện là sổ CHÉO VĨNH VIỄN.
/// 2. **Số lệnh sống bị chặn trần.** Vượt trần thì lệnh cũ nhất bị đẩy ra —
///    mô phỏng đúng việc thanh khoản cũ tan đi khi giá đã đi xa.
pub fn sinh_phien_ghi(so_su_kien: usize, hat_giong: u64) -> Vec<KhungGhi> {
    const TRAN_LENH_SONG: usize = 120;
    let mut s = hat_giong;
    let mut t = 9 * 3_600 * 1_000_000_000u64; // 9 giờ sáng, tính bằng ns
    let mut giua: Gia = 8_400;
    let mut ma = 1u64;
    let mut song: std::collections::VecDeque<MaLenh> = std::collections::VecDeque::new();
    let mut ra = Vec::with_capacity(so_su_kien);

    for _ in 0..so_su_kien {
        s = s.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        t += 10_000 + (s >> 20) % 500_000; // 10 µs – 0,5 ms giữa các sự kiện
        let r = (s >> 33) % 100;

        // Quá nhiều lệnh cũ thì buộc phải rút bớt, bất kể bốc trúng gì
        if song.len() >= TRAN_LENH_SONG {
            if let Some(cu) = song.pop_front() {
                ra.push(KhungGhi { thoi_diem_ns: t, su_kien: SuKienThiTruong::HuyLenh { ma: cu } });
                continue;
            }
        }

        if r < 55 || song.is_empty() {
            let chieu = if (s >> 41) % 2 == 0 { Chieu::Mua } else { Chieu::Ban };
            let lech = 1 + ((s >> 45) % 10) as i64;
            let gia = match chieu { Chieu::Mua => giua - lech, Chieu::Ban => giua + lech };
            let sl = 100 + ((s >> 49) % 5) as u32 * 100;
            ra.push(KhungGhi { thoi_diem_ns: t,
                su_kien: SuKienThiTruong::ThemLenh { ma, chieu, gia, so_luong: sl } });
            song.push_back(ma);
            ma += 1;
        } else if r < 85 {
            // Rút báo giá CŨ NHẤT — đây là chi tiết giữ cho sổ không bị chéo
            let cu = song.pop_front().unwrap();
            ra.push(KhungGhi { thoi_diem_ns: t, su_kien: SuKienThiTruong::HuyLenh { ma: cu } });
        } else {
            let chieu = if (s >> 41) % 2 == 0 { Chieu::Mua } else { Chieu::Ban };
            let gia = match chieu { Chieu::Mua => giua + 1, Chieu::Ban => giua - 1 };
            let sl = 100 + ((s >> 49) % 5) as u32 * 100;
            ra.push(KhungGhi { thoi_diem_ns: t,
                su_kien: SuKienThiTruong::KhopLenh { gia, so_luong: sl, chieu_chu_dong: chieu } });
            // Giá đi lang thang một chút quanh mốc ban đầu
            giua += if (s >> 57) % 2 == 0 { 1 } else { -1 };
            giua = giua.clamp(8_350, 8_450);
        }
    }
    ra
}

fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("   GHI & PHÁT LẠI PHIÊN GIAO DỊCH                          ");
    println!("═══════════════════════════════════════════════════════════");

    println!("\n1. GHI LẠI MỘT PHIÊN");
    let phien = sinh_phien_ghi(20_000, 2024);
    let mut ghi = BoGhiPhien::moi();
    for k in &phien { ghi.ghi(k); }
    println!("   {} sự kiện · {} byte · {:.2} byte/sự kiện",
             ghi.so_khung, ghi.so_byte(), ghi.so_byte() as f64 / ghi.so_khung as f64);
    println!("   Thời lượng phiên: {:.3} giây", ghi.thoi_luong_ns() as f64 / 1e9);
    let doc = ghi.doc_lai().unwrap();
    println!("   Đọc lại khớp bản gốc từng bit: {}", doc == phien);

    println!("\n2. BẢN GHI BỊ CẮT CỤT — phải báo lỗi, không được panic");
    let mut hong = BoGhiPhien::moi();
    for k in phien.iter().take(5) { hong.ghi(k); }
    hong.noi_dung.truncate(hong.noi_dung.len() - 3); // giả lập tiến trình bị giết
    println!("   Đọc bản ghi cụt → {:?}", hong.doc_lai().unwrap_err());

    println!("\n3. TỐC ĐỘ PHÁT LẠI");
    let mut cl = DungNgoai;
    for (ten, td) in [("thời gian thực", TocDoPhat::ThoiGianThuc),
                      ("nhanh 10 lần  ", TocDoPhat::HeSo(10.0)),
                      ("nhanh 1000 lần", TocDoPhat::HeSo(1000.0)),
                      ("nhanh nhất    ", TocDoPhat::NhanhNhatCoThe)] {
        let kq = BoPhatLai::moi(MoHinhDoTre::khong_do_tre(), td).chay(&phien, &mut cl);
        println!("   {} → thời gian ảo {:.2}s · phải chờ thật {:.4}s",
                 ten, kq.thoi_gian_ao_ns as f64 / 1e9, kq.thoi_gian_cho_thuc_ns as f64 / 1e9);
    }
    println!("   → Quét 1000 tổ hợp tham số: chạy đúng nhịp mất ~{:.0} phút,",
             ghi.thoi_luong_ns() as f64 / 1e9 * 1000.0 / 60.0);
    println!("     chạy ở chế độ nhanh nhất chỉ mất vài giây.");

    println!("\n4. ĐỘ TRỄ ĂN MẤT LỢI NHUẬN NHƯ THẾ NÀO");
    for (ten, dt) in [("không độ trễ  ", MoHinhDoTre::khong_do_tre()),
                      ("đặt thuê riêng", MoHinhDoTre::dat_thue_rieng()),
                      ("qua Internet  ", MoHinhDoTre::qua_internet())] {
        let mut cl = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                     vi_the_toi_da: 500, buoc: 0, moi_n_su_kien: 50 };
        let kq = BoPhatLai::moi(dt, TocDoPhat::NhanhNhatCoThe).chay(&phien, &mut cl);
        println!("   {} → khứ hồi {:>9} ns · gửi {:>4} lệnh · khớp {:>3} · lãi {:>8} tick",
                 ten, dt.khu_hoi_ns(0), kq.so_lenh_gui, kq.so_lenh_khop, kq.gia_tri_cuoi);
    }
    println!("   → Cùng chiến lược, cùng dữ liệu. Chỉ khác chỗ ngồi so với sàn.");

    println!("\n5. KIỂM SOÁT TỒN KHO — đếm cả lệnh ĐANG TREO");
    let tran = 300i64;
    let mut ngay_tho = TaoLapDonGian { do_lech_tick: 1, co_lenh: 100,
                                       vi_the_toi_da: tran, buoc: 0, moi_n_su_kien: 5 };
    let a = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
        .chay(&phien, &mut ngay_tho);
    let mut chat_che = TaoLapCoKiemSoat::moi(1, 100, tran, 5);
    let b = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
        .chay(&phien, &mut chat_che);
    println!("   Trần đặt ra: {}", tran);
    println!("   Chỉ nhìn vị thế đã khớp → vị thế cuối {:>6}  ← VƯỢT TRẦN",
             a.vi_the_cuoi.so_luong);
    println!("   Đếm cả lệnh đang treo   → vị thế cuối {:>6}  ← trong trần",
             b.vi_the_cuoi.so_luong);
    println!("   → Lệnh đã gửi mà chưa khớp VẪN LÀ RỦI RO.");

    println!("\n6. TÁI LẬP TUYỆT ĐỐI");
    let chay = || {
        let mut c = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                    vi_the_toi_da: 500, buoc: 0, moi_n_su_kien: 50 };
        BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
            .chay(&phien, &mut c)
    };
    println!("   Chạy hai lần cho kết quả giống hệt: {}", chay() == chay());
    println!("   → Vì chiến lược chỉ hỏi ĐỒNG HỒ ẢO, không bao giờ hỏi đồng hồ hệ thống.");

    println!("\n═══════════════════════════════════════════════════════════");
    println!("   GHI MỘT LẦN, CHẠY LẠI HÀNG NGHÌN LẦN, KẾT QUẢ KHÔNG ĐỔI  ");
    println!("═══════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn ghi_phien(n: usize, h: u64) -> (Vec<KhungGhi>, BoGhiPhien) {
        let p = sinh_phien_ghi(n, h);
        let mut g = BoGhiPhien::moi();
        for k in &p { g.ghi(k); }
        (p, g)
    }

    // ---------- Định dạng bản ghi ----------
    #[test]
    fn ghi_roi_doc_lai_khop_tung_bit() {
        let (p, g) = ghi_phien(2_000, 1);
        assert_eq!(g.doc_lai().unwrap(), p, "vòng ghi–đọc phải khép kín tuyệt đối");
        assert_eq!(g.so_khung, 2_000);
    }

    #[test]
    fn ghi_moi_loai_su_kien_deu_khep_kin() {
        let cac = vec![
            SuKienThiTruong::ThemLenh { ma: 1, chieu: Chieu::Mua, gia: 8_450, so_luong: 100 },
            SuKienThiTruong::ThemLenh { ma: 2, chieu: Chieu::Ban, gia: -7, so_luong: 1 },
            SuKienThiTruong::HuyLenh { ma: 999 },
            SuKienThiTruong::KhopLenh { gia: 8_400, so_luong: 50, chieu_chu_dong: Chieu::Ban },
        ];
        for sk in cac {
            let mut g = BoGhiPhien::moi();
            let k = KhungGhi { thoi_diem_ns: 123_456_789, su_kien: sk };
            g.ghi(&k);
            assert_eq!(g.doc_lai().unwrap(), vec![k]);
        }
    }

    #[test]
    fn ban_ghi_bi_cat_cut_bao_loi_chu_khong_panic() {
        // Tiến trình ghi bị giết giữa chừng là chuyện bình thường trong vận hành.
        let (_, g) = ghi_phien(10, 2);
        for cat in 1..12usize {
            let mut h = BoGhiPhien::moi();
            h.noi_dung = g.noi_dung[..g.noi_dung.len() - cat].to_vec();
            assert!(matches!(h.doc_lai(), Err(LoiDoc::KhungCut) | Err(LoiDoc::MaSuKienLa(_))),
                    "cắt {} byte cuối phải báo lỗi", cat);
        }
    }

    #[test]
    fn do_dai_khung_vo_ly_bi_tu_choi() {
        let mut g = BoGhiPhien::moi();
        g.noi_dung = vec![0, 0, 0, 3, 1, 2, 3]; // độ dài 3 < 8 byte dấu thời gian
        assert_eq!(g.doc_lai(), Err(LoiDoc::DoDaiVoLy(3)));
    }

    #[test]
    fn ma_su_kien_la_bi_tu_choi() {
        let mut g = BoGhiPhien::moi();
        g.noi_dung.extend_from_slice(&9u32.to_be_bytes());
        g.noi_dung.extend_from_slice(&0u64.to_be_bytes());
        g.noi_dung.push(b'?');
        assert_eq!(g.doc_lai(), Err(LoiDoc::MaSuKienLa(b'?')));
    }

    #[test]
    fn ban_ghi_rong_doc_ra_danh_sach_rong() {
        assert_eq!(BoGhiPhien::moi().doc_lai(), Ok(vec![]));
    }

    #[test]
    fn kich_thuoc_khung_dung_bang_tong_cac_truong() {
        // 4 byte độ dài + 8 byte dấu thời gian + thân.
        // Thân: A = 1+8+1+8+4 = 22 · X = 1+8 = 9 · T = 1+8+4+1 = 14
        let ktra = |sk: SuKienThiTruong, mong: usize| {
            let mut g = BoGhiPhien::moi();
            g.ghi(&KhungGhi { thoi_diem_ns: 1, su_kien: sk });
            assert_eq!(g.so_byte(), mong);
        };
        ktra(SuKienThiTruong::ThemLenh { ma: 1, chieu: Chieu::Mua, gia: 1, so_luong: 1 }, 34);
        ktra(SuKienThiTruong::HuyLenh { ma: 1 }, 21);
        ktra(SuKienThiTruong::KhopLenh { gia: 1, so_luong: 1, chieu_chu_dong: Chieu::Mua }, 26);
    }

    #[test]
    fn dinh_dang_nhi_phan_du_gon_de_ghi_ca_ngay() {
        let (_, g) = ghi_phien(10_000, 3);
        let byte_moi_su_kien = g.so_byte() as f64 / g.so_khung as f64;
        // Phiên trộn ~70% thêm lệnh (34 B), 15% huỷ (21 B), 15% khớp (26 B)
        // → trung bình khoảng 31 byte.
        assert!((21.0..32.0).contains(&byte_moi_su_kien),
                "trung bình {:.2} byte/sự kiện, kỳ vọng trong khoảng 21–32", byte_moi_su_kien);
        // Một phiên sôi động 50 triệu sự kiện vẫn chỉ khoảng 1,5 GB
        let ca_ngay_gb = 50_000_000.0 * byte_moi_su_kien / 1e9;
        assert!(ca_ngay_gb < 2.0, "cả ngày ~{:.2} GB — thừa sức lưu trữ", ca_ngay_gb);
    }

    // ---------- Đồng hồ ảo ----------
    #[test]
    fn dong_ho_ao_khong_bao_gio_chay_lui() {
        let mut d = DongHoAo::moi(1_000);
        d.tien_toi(500); // sự kiện tới muộn, dấu thời gian cũ
        assert_eq!(d.bay_gio_ns, 1_000, "thời gian không được lùi");
        d.tien_toi(2_000);
        assert_eq!(d.bay_gio_ns, 2_000);
        d.cong_them(50);
        assert_eq!(d.bay_gio_ns, 2_050);
    }

    // ---------- Tốc độ phát ----------
    #[test]
    fn toc_do_phat_tinh_dung_thoi_gian_cho() {
        assert_eq!(TocDoPhat::ThoiGianThuc.cho_bao_lau(1_000_000), 1_000_000);
        assert_eq!(TocDoPhat::HeSo(2.0).cho_bao_lau(1_000_000), 500_000);
        assert_eq!(TocDoPhat::HeSo(0.5).cho_bao_lau(1_000_000), 2_000_000,
                   "hệ số < 1 để chạy CHẬM lại mà quan sát kỹ");
        assert_eq!(TocDoPhat::NhanhNhatCoThe.cho_bao_lau(1_000_000), 0);
        assert_eq!(TocDoPhat::HeSo(0.0).cho_bao_lau(1_000_000), 0,
                   "hệ số 0 không được gây chia cho 0");
    }

    #[test]
    fn tua_nhanh_khong_duoc_doi_ket_qua() {
        // Tua nhanh chỉ đổi thời gian ta phải ngồi chờ, KHÔNG đổi những gì xảy ra.
        let p = sinh_phien_ghi(3_000, 5);
        let mut kq: Vec<KetQuaPhatLai> = Vec::new();
        for td in [TocDoPhat::ThoiGianThuc, TocDoPhat::HeSo(100.0), TocDoPhat::NhanhNhatCoThe] {
            let mut c = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                        vi_the_toi_da: 500, buoc: 0, moi_n_su_kien: 25 };
            kq.push(BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), td).chay(&p, &mut c));
        }
        assert_eq!(kq[0].cac_khop, kq[1].cac_khop);
        assert_eq!(kq[1].cac_khop, kq[2].cac_khop);
        assert_eq!(kq[0].thoi_gian_ao_ns, kq[2].thoi_gian_ao_ns);
        assert!(kq[0].thoi_gian_cho_thuc_ns > kq[1].thoi_gian_cho_thuc_ns);
        assert_eq!(kq[2].thoi_gian_cho_thuc_ns, 0);
    }

    // ---------- Mô hình độ trễ ----------
    #[test]
    fn do_tre_tat_dinh_theo_so_thu_tu() {
        let d = MoHinhDoTre::dat_thue_rieng();
        for i in 0..100u64 {
            assert_eq!(d.khu_hoi_ns(i), d.khu_hoi_ns(i), "cùng sự kiện → cùng độ trễ");
        }
        assert_eq!(MoHinhDoTre::khong_do_tre().khu_hoi_ns(42), 0);
    }

    #[test]
    fn do_tre_luon_trong_khoang_hop_ly() {
        let d = MoHinhDoTre::dat_thue_rieng();
        let toi_thieu = d.vao_ns + d.ra_ns;
        for i in 0..1_000u64 {
            let x = d.khu_hoi_ns(i);
            assert!(x >= toi_thieu && x < toi_thieu + d.dao_dong_ns,
                    "độ trễ {} nằm ngoài [{}, {})", x, toi_thieu, toi_thieu + d.dao_dong_ns);
        }
    }

    #[test]
    fn dat_thue_rieng_nhanh_hon_internet_hang_tram_lan() {
        let a = MoHinhDoTre::dat_thue_rieng().khu_hoi_ns(0);
        let b = MoHinhDoTre::qua_internet().khu_hoi_ns(0);
        assert!(b > a * 100, "ngồi cạnh sàn nhanh hơn {} lần", b / a.max(1));
    }

    // ---------- Sổ rút gọn ----------
    #[test]
    fn so_rut_gon_tra_dung_gia_tot_nhat() {
        let mut s = SoRutGon::default();
        s.them(Chieu::Mua, 8_390, 100);
        s.them(Chieu::Mua, 8_400, 200);
        s.them(Chieu::Ban, 8_420, 100);
        s.them(Chieu::Ban, 8_410, 50);
        assert_eq!(s.mua_tot_nhat(), Some(8_400));
        assert_eq!(s.ban_tot_nhat(), Some(8_410));
        s.bot(Chieu::Mua, 8_400, 200);
        assert_eq!(s.mua_tot_nhat(), Some(8_390), "mức hết hàng phải biến mất");
    }

    // ---------- Vị thế ----------
    #[test]
    fn vi_the_la_vi_nhom() {
        let a = ViThe::tu_khop(Chieu::Mua, 100, 10);
        let b = ViThe::tu_khop(Chieu::Ban, 110, 5);
        let c = ViThe::tu_khop(Chieu::Mua, 90, 3);
        assert_eq!(a.ghep(b).ghep(c), a.ghep(b.ghep(c)), "luật kết hợp");
        assert_eq!(a.ghep(ViThe::default()), a, "luật đơn vị");
    }

    #[test]
    fn mua_re_ban_dat_thi_co_lai() {
        let v = ViThe::tu_khop(Chieu::Mua, 8_000, 100)
            .ghep(ViThe::tu_khop(Chieu::Ban, 8_500, 100));
        assert_eq!(v.so_luong, 0);
        assert_eq!(v.gia_tri_rong(0), 50_000);
    }

    // ---------- Phát lại ----------
    #[test]
    fn so_khong_bi_cheo_vinh_vien_vi_bo_qua_huy_lenh() {
        // Bài học mô hình: nếu bộ phát lại bỏ qua bản tin huỷ, sổ chỉ phình
        // ra, các mức giá cũ không bao giờ mất, và chỉ sau vài nghìn sự kiện
        // là sổ chéo vĩnh viễn — chiến lược đứng ngoài mà ta không hiểu vì sao.
        let p = sinh_phien_ghi(20_000, 2024);
        let mut c = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                    vi_the_toi_da: 500, buoc: 0, moi_n_su_kien: 50 };
        let kq = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
            .chay(&p, &mut c);
        // 20 000 sự kiện, cứ 50 sự kiện lại chào giá → phải gửi hàng trăm lệnh
        assert!(kq.so_lenh_gui > 50,
                "chỉ gửi {} lệnh — dấu hiệu sổ bị chéo và chiến lược đứng ngoài",
                kq.so_lenh_gui);
        assert!(kq.so_lenh_khop > 20, "và phải khớp được kha khá, thực tế {}", kq.so_lenh_khop);
    }

    #[test]
    fn huy_lenh_thuc_su_rut_thanh_khoan_khoi_so() {
        let khung = vec![
            KhungGhi { thoi_diem_ns: 1_000,
                su_kien: SuKienThiTruong::ThemLenh { ma: 1, chieu: Chieu::Mua,
                                                     gia: 8_400, so_luong: 500 } },
            KhungGhi { thoi_diem_ns: 2_000,
                su_kien: SuKienThiTruong::ThemLenh { ma: 2, chieu: Chieu::Ban,
                                                     gia: 8_410, so_luong: 300 } },
            KhungGhi { thoi_diem_ns: 3_000, su_kien: SuKienThiTruong::HuyLenh { ma: 1 } },
        ];
        // Dùng một chiến lược chỉ quan sát để đọc trạng thái sổ ở bước cuối
        struct Soi { mua_cuoi: Option<Gia>, ban_cuoi: Option<Gia> }
        impl ChienLuocPhatLai for Soi {
            fn ten(&self) -> &str { "soi sổ" }
            fn khi_co_su_kien(&mut self, _: &DongHoAo, so: &SoRutGon, _: &ViThe)
                -> Vec<(Chieu, Gia, SoLuong)> {
                self.mua_cuoi = so.mua_tot_nhat();
                self.ban_cuoi = so.ban_tot_nhat();
                vec![]
            }
        }
        let mut s = Soi { mua_cuoi: None, ban_cuoi: None };
        BoPhatLai::moi(MoHinhDoTre::khong_do_tre(), TocDoPhat::NhanhNhatCoThe)
            .chay(&khung, &mut s);
        assert_eq!(s.mua_cuoi, None, "lệnh mua đã bị huỷ, bên mua phải rỗng");
        assert_eq!(s.ban_cuoi, Some(8_410), "lệnh bán không bị đụng tới");
    }

    #[test]
    fn phat_lai_tai_lap_tuyet_doi() {
        // BẤT BIẾN QUAN TRỌNG NHẤT của chương. Nếu bài này hỏng thì mọi kết
        // quả kiểm định đều vô nghĩa vì không so sánh được với nhau.
        let p = sinh_phien_ghi(5_000, 2024);
        let chay = || {
            let mut c = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                        vi_the_toi_da: 500, buoc: 0, moi_n_su_kien: 30 };
            BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
                .chay(&p, &mut c)
        };
        assert_eq!(chay(), chay());
        assert_eq!(chay(), chay(), "ba lần vẫn phải giống hệt");
    }

    #[test]
    fn dung_ngoai_thi_khong_lenh_khong_lai_khong_lo() {
        let p = sinh_phien_ghi(2_000, 7);
        let kq = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
            .chay(&p, &mut DungNgoai);
        assert_eq!(kq.so_lenh_gui, 0);
        assert_eq!(kq.so_lenh_khop, 0);
        assert_eq!(kq.vi_the_cuoi, ViThe::default());
        assert_eq!(kq.gia_tri_cuoi, 0);
    }

    #[test]
    fn lenh_khong_the_khop_truoc_khi_toi_san() {
        // Nếu mô phỏng cho lệnh khớp ngay lúc quyết định, ta đã "nhìn trộm
        // tương lai" ở mức tinh vi nhất — và kết quả sẽ đẹp một cách giả tạo.
        let p = sinh_phien_ghi(3_000, 11);
        let mut c = TaoLapDonGian { do_lech_tick: 1, co_lenh: 100,
                                    vi_the_toi_da: 10_000, buoc: 0, moi_n_su_kien: 10 };
        let dt = MoHinhDoTre::qua_internet();
        let kq = BoPhatLai::moi(dt, TocDoPhat::NhanhNhatCoThe).chay(&p, &mut c);
        let dau = p.first().unwrap().thoi_diem_ns;
        let toi_thieu = dt.vao_ns + dt.ra_ns;
        for k in &kq.cac_khop {
            assert!(k.thoi_diem_ns >= dau + toi_thieu,
                    "khớp lúc {} là quá sớm — lệnh chưa kịp bay tới sàn", k.thoi_diem_ns);
        }
    }

    #[test]
    fn do_tre_cang_lon_thi_cang_kho_khop() {
        // Đây là lý do các hãng trả rất nhiều tiền để đặt máy cạnh sàn.
        let p = sinh_phien_ghi(8_000, 2024);
        let dem_khop = |dt: MoHinhDoTre| {
            let mut c = TaoLapDonGian { do_lech_tick: 1, co_lenh: 100,
                                        vi_the_toi_da: 10_000, buoc: 0, moi_n_su_kien: 10 };
            BoPhatLai::moi(dt, TocDoPhat::NhanhNhatCoThe).chay(&p, &mut c).so_lenh_khop
        };
        let nhanh = dem_khop(MoHinhDoTre::dat_thue_rieng());
        let cham = dem_khop(MoHinhDoTre::qua_internet());
        assert!(nhanh >= cham,
                "gần sàn phải khớp được ít nhất bằng: {} so với {}", nhanh, cham);
    }

    #[test]
    fn vi_the_cuoi_bang_dung_tong_cac_lan_khop() {
        // Kế toán phải khớp: vị thế = tổng mọi lần khớp, không thừa không thiếu.
        let p = sinh_phien_ghi(5_000, 17);
        let mut c = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                    vi_the_toi_da: 1_000, buoc: 0, moi_n_su_kien: 20 };
        let kq = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
            .chay(&p, &mut c);
        let dung_lai = kq.cac_khop.iter()
            .fold(ViThe::default(), |a, k| a.ghep(ViThe::tu_khop(k.chieu, k.gia, k.so_luong)));
        assert_eq!(dung_lai, kq.vi_the_cuoi,
                   "dựng lại vị thế từ nhật ký khớp phải ra đúng vị thế cuối");
        assert_eq!(kq.so_lenh_khop as usize, kq.cac_khop.len());
    }

    #[test]
    fn moi_lan_khop_deu_hop_le() {
        let p = sinh_phien_ghi(5_000, 13);
        let mut c = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                    vi_the_toi_da: 1_000, buoc: 0, moi_n_su_kien: 20 };
        let kq = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
            .chay(&p, &mut c);
        for k in &kq.cac_khop {
            assert!(k.so_luong > 0, "không được ghi nhận khớp khối lượng 0");
            assert!(k.gia > 0);
        }
    }

    #[test]
    fn chi_nhin_vi_the_da_khop_thi_VUOT_TRAN() {
        // Bài học đắt tiền, và bài kiểm thử này CỐ Ý ghi lại cái sai:
        // `TaoLapDonGian` chỉ kiểm tra vị thế ĐÃ KHỚP, nên cứ mỗi nhịp lại
        // chào thêm một lệnh nữa. Khi thị trường quét qua, tất cả khớp một
        // lượt và vị thế nhảy vọt qua trần.
        let p = sinh_phien_ghi(10_000, 23);
        let tran = 300i64;
        let mut c = TaoLapDonGian { do_lech_tick: 1, co_lenh: 100,
                                    vi_the_toi_da: tran, buoc: 0, moi_n_su_kien: 5 };
        let kq = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
            .chay(&p, &mut c);
        assert!(kq.vi_the_cuoi.so_luong.abs() > tran,
                "chính vì bỏ qua lệnh đang treo mà vị thế {} vượt trần {}",
                kq.vi_the_cuoi.so_luong, tran);
    }

    #[test]
    fn dem_ca_lenh_dang_treo_thi_giu_duoc_tran() {
        // Bản đúng: phơi bày = vị thế đã khớp + khối lượng đang treo.
        let p = sinh_phien_ghi(10_000, 23);
        let tran = 300i64;
        let mut c = TaoLapCoKiemSoat::moi(1, 100, tran, 5);
        let kq = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
            .chay(&p, &mut c);
        assert!(kq.vi_the_cuoi.so_luong.abs() <= tran,
                "vị thế cuối {} phải nằm trong trần {}", kq.vi_the_cuoi.so_luong, tran);
        assert!(kq.so_lenh_gui > 0, "vẫn phải giao dịch được, không phải đứng im");
    }

    #[test]
    fn kiem_soat_ton_kho_giu_tran_voi_moi_hat_giong() {
        for hat in [1u64, 7, 23, 42, 2024] {
            let p = sinh_phien_ghi(8_000, hat);
            let tran = 200i64;
            let mut c = TaoLapCoKiemSoat::moi(1, 100, tran, 5);
            let kq = BoPhatLai::moi(MoHinhDoTre::dat_thue_rieng(), TocDoPhat::NhanhNhatCoThe)
                .chay(&p, &mut c);
            assert!(kq.vi_the_cuoi.so_luong.abs() <= tran,
                    "hạt giống {}: vị thế {} vượt trần {}", hat, kq.vi_the_cuoi.so_luong, tran);
        }
    }

    #[test]
    fn so_bi_cheo_thi_chien_luoc_dung_ngoai() {
        let mut s = SoRutGon::default();
        s.them(Chieu::Mua, 8_500, 100);
        s.them(Chieu::Ban, 8_400, 100);
        let mut c = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                    vi_the_toi_da: 500, buoc: 0, moi_n_su_kien: 1 };
        let lenh = c.khi_co_su_kien(&DongHoAo::moi(0), &s, &ViThe::default());
        assert!(lenh.is_empty(), "sổ chéo → phải đứng ngoài, không được coi là cơ hội");
    }

    #[test]
    fn so_rong_thi_chien_luoc_khong_gui_lenh() {
        let mut c = TaoLapDonGian { do_lech_tick: 2, co_lenh: 100,
                                    vi_the_toi_da: 500, buoc: 0, moi_n_su_kien: 1 };
        assert!(c.khi_co_su_kien(&DongHoAo::moi(0), &SoRutGon::default(),
                                 &ViThe::default()).is_empty());
    }

    // ---------- Sinh phiên ----------
    #[test]
    fn sinh_phien_tat_dinh_va_thoi_gian_tang() {
        assert_eq!(sinh_phien_ghi(100, 5), sinh_phien_ghi(100, 5));
        assert_ne!(sinh_phien_ghi(100, 5), sinh_phien_ghi(100, 6));
        let p = sinh_phien_ghi(1_000, 1);
        for w in p.windows(2) {
            assert!(w[1].thoi_diem_ns > w[0].thoi_diem_ns);
        }
    }

    #[test]
    fn phien_sinh_ra_co_du_ba_loai_su_kien() {
        let p = sinh_phien_ghi(5_000, 3);
        let them = p.iter()
            .filter(|k| matches!(k.su_kien, SuKienThiTruong::ThemLenh { .. })).count();
        let huy = p.iter()
            .filter(|k| matches!(k.su_kien, SuKienThiTruong::HuyLenh { .. })).count();
        let khop = p.iter()
            .filter(|k| matches!(k.su_kien, SuKienThiTruong::KhopLenh { .. })).count();
        assert!(them > 0 && huy > 0 && khop > 0, "phiên phải có cả ba loại sự kiện");
        assert_eq!(them + huy + khop, p.len());
        assert!(them > khop, "thực tế: đặt lệnh nhiều hơn khớp lệnh rất nhiều");
    }
}
