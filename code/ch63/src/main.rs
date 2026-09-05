#![allow(dead_code, unused_variables)]
//! Chương 63 — Ứng dụng Desktop & Đa nền tảng: kiến trúc trạng thái (Elm/Redux),
//! cầu IPC frontend↔backend (như Tauri command). Lõi thuần túy, kiểm thử được.

use std::collections::HashMap;

// ============================================================================
// 1. KIẾN TRÚC TRẠNG THÁI (The Elm Architecture) — Model · Message · update
// ============================================================================
// Đây là mô hình quản lý trạng thái mà Redux, Elm, và gpui (của Zed) đều dùng:
// mọi thay đổi đi qua MỘT hàm `update` thuần túy. Không sửa trạng thái lung tung.

#[derive(Debug, Clone, PartialEq)]
pub struct MoHinh {
    pub cong_viec: Vec<CongViec>,
    pub bo_loc: BoLoc,
    pub id_ke_tiep: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CongViec {
    pub id: u64,
    pub tieu_de: String,
    pub xong: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BoLoc { TatCa, ChuaXong, DaXong }

/// Mọi thứ CÓ THỂ xảy ra trong ứng dụng, liệt kê bằng enum (kiểu tổng, Chương 20).
/// Không có hành động nào ngoài danh sách này — trạng thái thay đổi có kiểm soát.
#[derive(Debug, Clone, PartialEq)]
pub enum ThongDiep {
    ThemViec(String),
    BatTat(u64),
    Xoa(u64),
    DoiBoLoc(BoLoc),
    XoaDaXong,
}

impl MoHinh {
    pub fn moi() -> Self {
        MoHinh { cong_viec: Vec::new(), bo_loc: BoLoc::TatCa, id_ke_tiep: 1 }
    }

    /// HÀM `update` THUẦN TÚY: (trạng thái cũ, thông điệp) -> trạng thái mới.
    /// Đây là trái tim của kiến trúc: mọi thay đổi phải đi qua đây, nên dễ
    /// suy luận, dễ kiểm thử, dễ ghi lại (undo/redo, ghi nhật ký, phát lại).
    pub fn update(mut self, td: ThongDiep) -> Self {
        match td {
            ThongDiep::ThemViec(tieu_de) => {
                let t = tieu_de.trim();
                if !t.is_empty() {
                    self.cong_viec.push(CongViec {
                        id: self.id_ke_tiep, tieu_de: t.to_string(), xong: false,
                    });
                    self.id_ke_tiep += 1;
                }
            }
            ThongDiep::BatTat(id) => {
                if let Some(cv) = self.cong_viec.iter_mut().find(|c| c.id == id) {
                    cv.xong = !cv.xong;
                }
            }
            ThongDiep::Xoa(id) => {
                self.cong_viec.retain(|c| c.id != id);
            }
            ThongDiep::DoiBoLoc(bl) => {
                self.bo_loc = bl;
            }
            ThongDiep::XoaDaXong => {
                self.cong_viec.retain(|c| !c.xong);
            }
        }
        self
    }

    /// Dẫn xuất: danh sách hiển thị theo bộ lọc hiện tại (view thuần túy).
    pub fn hien_thi(&self) -> Vec<&CongViec> {
        self.cong_viec.iter().filter(|c| match self.bo_loc {
            BoLoc::TatCa => true,
            BoLoc::ChuaXong => !c.xong,
            BoLoc::DaXong => c.xong,
        }).collect()
    }

    pub fn so_chua_xong(&self) -> usize {
        self.cong_viec.iter().filter(|c| !c.xong).count()
    }
}

// ============================================================================
// 2. CẦU IPC — frontend gọi backend (như Tauri command)
// ============================================================================
// Trong Tauri, giao diện (JS/Svelte) gọi hàm Rust qua `invoke("ten", tham_so)`.
// Ta mô phỏng cầu đó: một bộ điều phối nhận tên lệnh + tham số, trả kết quả JSON.

#[derive(Debug, PartialEq)]
pub enum KetQuaLenh {
    Ok(String),
    Loi(String),
}

pub trait LenhBackend {
    fn ten(&self) -> &str;
    fn chay(&self, tham_so: &HashMap<String, String>) -> KetQuaLenh;
}

/// Ví dụ lệnh: đọc thông tin hệ thống (backend làm việc mà webview không làm được).
pub struct LenhThongTinHeThong;
impl LenhBackend for LenhThongTinHeThong {
    fn ten(&self) -> &str { "thong_tin_he_thong" }
    fn chay(&self, _: &HashMap<String, String>) -> KetQuaLenh {
        KetQuaLenh::Ok("os=cross-platform;kien_truc=x86_64".to_string())
    }
}

/// Ví dụ lệnh: lưu tệp (thao tác hệ thống — chỉ backend được phép, vì bảo mật).
pub struct LenhLuuTep;
impl LenhBackend for LenhLuuTep {
    fn ten(&self) -> &str { "luu_tep" }
    fn chay(&self, tham_so: &HashMap<String, String>) -> KetQuaLenh {
        let ten = match tham_so.get("ten") {
            Some(t) if !t.is_empty() => t,
            _ => return KetQuaLenh::Loi("thiếu tên tệp".into()),
        };
        // Chặn path traversal (Chương 57) — webview không được ghi ra ngoài thư mục app!
        if ten.contains("..") || ten.starts_with('/') {
            return KetQuaLenh::Loi("đường dẫn không an toàn".into());
        }
        KetQuaLenh::Ok(format!("đã lưu {}", ten))
    }
}

/// Cầu IPC: đăng ký lệnh và điều phối lời gọi từ frontend.
pub struct CauIPC {
    lenh: Vec<Box<dyn LenhBackend>>,
}
impl CauIPC {
    pub fn moi() -> Self { CauIPC { lenh: Vec::new() } }
    pub fn dang_ky(mut self, l: Box<dyn LenhBackend>) -> Self {
        self.lenh.push(l);
        self
    }
    /// invoke(ten, tham_so) — y hệt `invoke` của Tauri.
    pub fn invoke(&self, ten: &str, tham_so: HashMap<String, String>) -> KetQuaLenh {
        match self.lenh.iter().find(|l| l.ten() == ten) {
            Some(l) => l.chay(&tham_so),
            None => KetQuaLenh::Loi(format!("lệnh {:?} không được đăng ký", ten)),
        }
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   DESKTOP & ĐA NỀN TẢNG: KIẾN TRÚC TRẠNG THÁI + CẦU IPC        ");
    println!("═══════════════════════════════════════════════════════════════");

    println!("\n1. KIẾN TRÚC TRẠNG THÁI (Elm/Redux) — mọi thay đổi qua `update`");
    let m = MoHinh::moi()
        .update(ThongDiep::ThemViec("Học Tauri".into()))
        .update(ThongDiep::ThemViec("Viết ứng dụng".into()))
        .update(ThongDiep::ThemViec("Đóng gói đa nền tảng".into()))
        .update(ThongDiep::BatTat(1)); // đánh dấu việc #1 xong

    println!("   Tổng công việc: {}, chưa xong: {}", m.cong_viec.len(), m.so_chua_xong());
    let m = m.update(ThongDiep::DoiBoLoc(BoLoc::ChuaXong));
    println!("   Lọc 'chưa xong': {:?}", m.hien_thi().iter().map(|c| &c.tieu_de).collect::<Vec<_>>());

    println!("\n2. CẦU IPC — frontend (Svelte/JS) gọi backend (Rust)");
    let cau = CauIPC::moi()
        .dang_ky(Box::new(LenhThongTinHeThong))
        .dang_ky(Box::new(LenhLuuTep));

    println!("   invoke('thong_tin_he_thong'): {:?}", cau.invoke("thong_tin_he_thong", HashMap::new()));
    let mut ts = HashMap::new();
    ts.insert("ten".to_string(), "ghi_chu.txt".to_string());
    println!("   invoke('luu_tep', {{ten: 'ghi_chu.txt'}}): {:?}", cau.invoke("luu_tep", ts.clone()));
    ts.insert("ten".to_string(), "../../etc/passwd".to_string());
    println!("   invoke('luu_tep', {{ten: '../../etc/passwd'}}): {:?}", cau.invoke("luu_tep", ts));
    println!("   invoke('lenh_la'): {:?}", cau.invoke("lenh_la", HashMap::new()));

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   MỘT LÕI RUST · NHIỀU NỀN TẢNG · GIAO DIỆN WEB HAY NATIVE      ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    #[test]
    fn them_viec_va_tang_id() {
        let m = MoHinh::moi()
            .update(ThongDiep::ThemViec("A".into()))
            .update(ThongDiep::ThemViec("B".into()));
        assert_eq!(m.cong_viec.len(), 2);
        assert_eq!(m.cong_viec[0].id, 1);
        assert_eq!(m.cong_viec[1].id, 2);
    }

    #[test]
    fn them_viec_rong_bi_bo_qua() {
        let m = MoHinh::moi()
            .update(ThongDiep::ThemViec("   ".into()))
            .update(ThongDiep::ThemViec("".into()));
        assert_eq!(m.cong_viec.len(), 0);
    }

    #[test]
    fn bat_tat_trang_thai() {
        let m = MoHinh::moi().update(ThongDiep::ThemViec("X".into()));
        assert!(!m.cong_viec[0].xong);
        let m = m.update(ThongDiep::BatTat(1));
        assert!(m.cong_viec[0].xong);
        let m = m.update(ThongDiep::BatTat(1)); // bật lại
        assert!(!m.cong_viec[0].xong);
    }

    #[test]
    fn xoa_va_xoa_da_xong() {
        let m = MoHinh::moi()
            .update(ThongDiep::ThemViec("A".into()))
            .update(ThongDiep::ThemViec("B".into()))
            .update(ThongDiep::ThemViec("C".into()))
            .update(ThongDiep::BatTat(1))
            .update(ThongDiep::BatTat(3));
        // Xóa 1 việc cụ thể
        let m2 = m.clone().update(ThongDiep::Xoa(2));
        assert_eq!(m2.cong_viec.len(), 2);
        // Xóa mọi việc đã xong (1 và 3)
        let m3 = m.update(ThongDiep::XoaDaXong);
        assert_eq!(m3.cong_viec.len(), 1);
        assert_eq!(m3.cong_viec[0].tieu_de, "B");
    }

    #[test]
    fn bo_loc_hien_thi_dung() {
        let m = MoHinh::moi()
            .update(ThongDiep::ThemViec("A".into()))
            .update(ThongDiep::ThemViec("B".into()))
            .update(ThongDiep::BatTat(1)); // A xong
        assert_eq!(m.clone().update(ThongDiep::DoiBoLoc(BoLoc::TatCa)).hien_thi().len(), 2);
        assert_eq!(m.clone().update(ThongDiep::DoiBoLoc(BoLoc::DaXong)).hien_thi().len(), 1);
        assert_eq!(m.update(ThongDiep::DoiBoLoc(BoLoc::ChuaXong)).hien_thi().len(), 1);
    }

    #[test]
    fn update_thuan_tuy_cho_phep_phat_lai() {
        // Vì update thuần túy, ta có thể PHÁT LẠI một chuỗi thông điệp để dựng
        // lại đúng trạng thái — nền của undo/redo và event sourcing (Chương 54).
        let lich_su = vec![
            ThongDiep::ThemViec("A".into()),
            ThongDiep::ThemViec("B".into()),
            ThongDiep::BatTat(1),
        ];
        let dung = |ds: &[ThongDiep]| ds.iter().cloned().fold(MoHinh::moi(), |m, td| m.update(td));
        // Phát lại hai lần cho CÙNG kết quả (tất định)
        assert_eq!(dung(&lich_su), dung(&lich_su));
    }

    #[test]
    fn ipc_dieu_phoi_lenh() {
        let cau = CauIPC::moi()
            .dang_ky(Box::new(LenhThongTinHeThong))
            .dang_ky(Box::new(LenhLuuTep));
        assert!(matches!(cau.invoke("thong_tin_he_thong", HashMap::new()), KetQuaLenh::Ok(_)));
        assert!(matches!(cau.invoke("lenh_khong_co", HashMap::new()), KetQuaLenh::Loi(_)));
    }

    #[test]
    fn ipc_chan_path_traversal() {
        let cau = CauIPC::moi().dang_ky(Box::new(LenhLuuTep));
        let mut ok = HashMap::new();
        ok.insert("ten".into(), "note.txt".to_string());
        assert!(matches!(cau.invoke("luu_tep", ok), KetQuaLenh::Ok(_)));

        let mut xau = HashMap::new();
        xau.insert("ten".into(), "../../../etc/passwd".to_string());
        // Cầu IPC chặn — webview KHÔNG được ghi ra ngoài thư mục app (bảo mật)
        assert!(matches!(cau.invoke("luu_tep", xau), KetQuaLenh::Loi(_)));
    }
}
