#![allow(dead_code, unused_variables)]
//! Chương 61 — Backend Web: kiến trúc một dịch vụ HTTP. Lõi định tuyến + xử lý
//! nghiệp vụ thuần túy (kiểm thử được KHÔNG cần server), phản chiếu cách Axum hoạt động.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// 1. MÔ HÌNH HTTP — Request / Response / Method
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum PhuongThuc { GET, POST, PUT, DELETE }

#[derive(Debug, Clone)]
pub struct YeuCau {
    pub phuong_thuc: PhuongThuc,
    pub duong_dan: String,
    pub than: String, // body (JSON dạng chuỗi cho đơn giản)
    pub tham_so_duong_dan: HashMap<String, String>, // /user/:id -> {id: "7"}
}

#[derive(Debug, Clone, PartialEq)]
pub struct PhanHoi {
    pub ma: u16, // 200, 201, 404, 422...
    pub than: String,
}

impl PhanHoi {
    pub fn ok(than: impl Into<String>) -> Self { PhanHoi { ma: 200, than: than.into() } }
    pub fn tao(than: impl Into<String>) -> Self { PhanHoi { ma: 201, than: than.into() } }
    pub fn khong_thay() -> Self { PhanHoi { ma: 404, than: "Không tìm thấy".into() } }
    pub fn du_lieu_sai(ly_do: impl Into<String>) -> Self { PhanHoi { ma: 422, than: ly_do.into() } }
}

// ============================================================================
// 2. BỘ ĐỊNH TUYẾN (Router) — khớp phương thức + mẫu đường dẫn
// ============================================================================

pub type BoXuLy = Arc<dyn Fn(&YeuCau, &TrangThai) -> PhanHoi + Send + Sync>;

pub struct Tuyen {
    phuong_thuc: PhuongThuc,
    mau: Vec<String>, // ["user", ":id", "profile"]
    xu_ly: BoXuLy,
}

pub struct BoDinhTuyen {
    tuyen: Vec<Tuyen>,
}

impl BoDinhTuyen {
    pub fn moi() -> Self { BoDinhTuyen { tuyen: Vec::new() } }

    pub fn them(mut self, pt: PhuongThuc, mau: &str, xu_ly: BoXuLy) -> Self {
        self.tuyen.push(Tuyen {
            phuong_thuc: pt,
            mau: mau.trim_matches('/').split('/').map(|s| s.to_string()).collect(),
            xu_ly,
        });
        self
    }

    /// Khớp một yêu cầu với tuyến. Trả về (bộ xử lý, tham số đường dẫn) nếu khớp.
    fn khop<'a>(&'a self, yc: &YeuCau) -> Option<(&'a BoXuLy, HashMap<String, String>)> {
        let phan: Vec<&str> = yc.duong_dan.trim_matches('/').split('/').collect();
        for t in &self.tuyen {
            if t.phuong_thuc != yc.phuong_thuc || t.mau.len() != phan.len() {
                continue;
            }
            let mut tham_so = HashMap::new();
            let mut khop = true;
            for (mau, thuc) in t.mau.iter().zip(phan.iter()) {
                if let Some(ten) = mau.strip_prefix(':') {
                    tham_so.insert(ten.to_string(), thuc.to_string()); // tham số động
                } else if mau != thuc {
                    khop = false;
                    break;
                }
            }
            if khop {
                return Some((&t.xu_ly, tham_so));
            }
        }
        None
    }

    /// Xử lý một yêu cầu: khớp tuyến, gọi bộ xử lý, hoặc trả 404.
    pub fn xu_ly(&self, mut yc: YeuCau, tt: &TrangThai) -> PhanHoi {
        match self.khop(&yc) {
            Some((xu_ly, tham_so)) => {
                yc.tham_so_duong_dan = tham_so;
                xu_ly(&yc, tt)
            }
            None => PhanHoi::khong_thay(),
        }
    }
}

// ============================================================================
// 3. TRẠNG THÁI CHIA SẺ (Shared State) — như State<Arc<AppState>> của Axum
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct SanPham {
    pub id: u64,
    pub ten: String,
    pub gia: u64,
}

pub struct TrangThai {
    pub kho: Mutex<HashMap<u64, SanPham>>,
    pub id_ke_tiep: Mutex<u64>,
}

impl TrangThai {
    pub fn moi() -> Arc<Self> {
        Arc::new(TrangThai {
            kho: Mutex::new(HashMap::new()),
            id_ke_tiep: Mutex::new(1),
        })
    }
}

// ============================================================================
// 4. BỘ XỬ LÝ (Handlers) — LÕI THUẦN TÚY nghiệp vụ, kiểm thử được
// ============================================================================

/// Phân tích JSON thô rất đơn giản: "ten=X;gia=Y" (thay cho serde để chạy offline).
fn phan_tich_than(than: &str) -> HashMap<String, String> {
    than.split(';')
        .filter_map(|c| c.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

pub fn xu_ly_liet_ke(_yc: &YeuCau, tt: &TrangThai) -> PhanHoi {
    let kho = tt.kho.lock().unwrap();
    let mut ds: Vec<&SanPham> = kho.values().collect();
    ds.sort_by_key(|s| s.id);
    let than = ds.iter().map(|s| format!("{}:{}:{}", s.id, s.ten, s.gia))
        .collect::<Vec<_>>().join(",");
    PhanHoi::ok(than)
}

pub fn xu_ly_xem_mot(yc: &YeuCau, tt: &TrangThai) -> PhanHoi {
    let id: u64 = match yc.tham_so_duong_dan.get("id").and_then(|s| s.parse().ok()) {
        Some(x) => x,
        None => return PhanHoi::du_lieu_sai("id không hợp lệ"),
    };
    match tt.kho.lock().unwrap().get(&id) {
        Some(sp) => PhanHoi::ok(format!("{}:{}:{}", sp.id, sp.ten, sp.gia)),
        None => PhanHoi::khong_thay(),
    }
}

pub fn xu_ly_tao(yc: &YeuCau, tt: &TrangThai) -> PhanHoi {
    let truong = phan_tich_than(&yc.than);
    let ten = match truong.get("ten") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return PhanHoi::du_lieu_sai("thiếu tên sản phẩm"),
    };
    let gia: u64 = match truong.get("gia").and_then(|g| g.parse().ok()) {
        Some(g) => g,
        None => return PhanHoi::du_lieu_sai("giá phải là số nguyên"),
    };
    let mut id_ke = tt.id_ke_tiep.lock().unwrap();
    let id = *id_ke;
    *id_ke += 1;
    tt.kho.lock().unwrap().insert(id, SanPham { id, ten, gia });
    PhanHoi::tao(format!("Đã tạo sản phẩm #{}", id))
}

pub fn xu_ly_xoa(yc: &YeuCau, tt: &TrangThai) -> PhanHoi {
    let id: u64 = match yc.tham_so_duong_dan.get("id").and_then(|s| s.parse().ok()) {
        Some(x) => x,
        None => return PhanHoi::du_lieu_sai("id không hợp lệ"),
    };
    if tt.kho.lock().unwrap().remove(&id).is_some() {
        PhanHoi::ok(format!("Đã xóa #{}", id))
    } else {
        PhanHoi::khong_thay()
    }
}

/// Dựng bộ định tuyến — tương đương `Router::new().route(...)` của Axum.
pub fn dung_ung_dung() -> BoDinhTuyen {
    BoDinhTuyen::moi()
        .them(PhuongThuc::GET, "/san-pham", Arc::new(xu_ly_liet_ke))
        .them(PhuongThuc::GET, "/san-pham/:id", Arc::new(xu_ly_xem_mot))
        .them(PhuongThuc::POST, "/san-pham", Arc::new(xu_ly_tao))
        .them(PhuongThuc::DELETE, "/san-pham/:id", Arc::new(xu_ly_xoa))
}

fn yc(pt: PhuongThuc, dd: &str, than: &str) -> YeuCau {
    YeuCau { phuong_thuc: pt, duong_dan: dd.into(), than: than.into(), tham_so_duong_dan: HashMap::new() }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   BACKEND WEB: BỘ ĐỊNH TUYẾN · TRẠNG THÁI · BỘ XỬ LÝ (như Axum) ");
    println!("═══════════════════════════════════════════════════════════════");

    let app = dung_ung_dung();
    let tt = TrangThai::moi();

    let goi = |pt, dd: &str, than: &str| {
        let r = app.xu_ly(yc(pt, dd, than), &tt);
        println!("   {:>6} {:<18} -> {} {}", format!("{:?}", &r.ma)[0..3].to_string(), dd, r.ma, r.than);
        r
    };

    println!("\nMô phỏng các lời gọi API:");
    goi(PhuongThuc::POST, "/san-pham", "ten=Bàn phím;gia=1200000");
    goi(PhuongThuc::POST, "/san-pham", "ten=Chuột;gia=350000");
    goi(PhuongThuc::GET, "/san-pham", "");
    goi(PhuongThuc::GET, "/san-pham/1", "");
    goi(PhuongThuc::GET, "/san-pham/99", "");         // 404
    goi(PhuongThuc::POST, "/san-pham", "gia=xyz");     // 422 thiếu tên
    goi(PhuongThuc::DELETE, "/san-pham/2", "");
    goi(PhuongThuc::GET, "/khong-co-tuyen", "");       // 404

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   LÕI NGHIỆP VỤ THUẦN TÚY = KIỂM THỬ ĐƯỢC KHÔNG CẦN CHẠY SERVER ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod kiem_thu {
    use super::*;

    fn moi_truong() -> (BoDinhTuyen, Arc<TrangThai>) {
        (dung_ung_dung(), TrangThai::moi())
    }

    #[test]
    fn tao_va_xem_san_pham() {
        let (app, tt) = moi_truong();
        let r = app.xu_ly(yc(PhuongThuc::POST, "/san-pham", "ten=Sách;gia=45000"), &tt);
        assert_eq!(r.ma, 201);
        let r = app.xu_ly(yc(PhuongThuc::GET, "/san-pham/1", ""), &tt);
        assert_eq!(r.ma, 200);
        assert_eq!(r.than, "1:Sách:45000");
    }

    #[test]
    fn tuyen_khong_ton_tai_tra_404() {
        let (app, tt) = moi_truong();
        assert_eq!(app.xu_ly(yc(PhuongThuc::GET, "/bat-ky", ""), &tt).ma, 404);
    }

    #[test]
    fn sai_phuong_thuc_tra_404() {
        let (app, tt) = moi_truong();
        // Có tuyến GET /san-pham/:id nhưng không có PUT -> 404
        assert_eq!(app.xu_ly(yc(PhuongThuc::PUT, "/san-pham/1", ""), &tt).ma, 404);
    }

    #[test]
    fn du_lieu_sai_tra_422() {
        let (app, tt) = moi_truong();
        // Thiếu tên
        assert_eq!(app.xu_ly(yc(PhuongThuc::POST, "/san-pham", "gia=100"), &tt).ma, 422);
        // Giá không phải số
        assert_eq!(app.xu_ly(yc(PhuongThuc::POST, "/san-pham", "ten=X;gia=abc"), &tt).ma, 422);
    }

    #[test]
    fn tham_so_duong_dan_dong() {
        let (app, tt) = moi_truong();
        app.xu_ly(yc(PhuongThuc::POST, "/san-pham", "ten=A;gia=1"), &tt);
        app.xu_ly(yc(PhuongThuc::POST, "/san-pham", "ten=B;gia=2"), &tt);
        // :id được trích đúng
        assert_eq!(app.xu_ly(yc(PhuongThuc::GET, "/san-pham/2", ""), &tt).than, "2:B:2");
    }

    #[test]
    fn xoa_san_pham() {
        let (app, tt) = moi_truong();
        app.xu_ly(yc(PhuongThuc::POST, "/san-pham", "ten=A;gia=1"), &tt);
        assert_eq!(app.xu_ly(yc(PhuongThuc::DELETE, "/san-pham/1", ""), &tt).ma, 200);
        assert_eq!(app.xu_ly(yc(PhuongThuc::GET, "/san-pham/1", ""), &tt).ma, 404); // đã xóa
        assert_eq!(app.xu_ly(yc(PhuongThuc::DELETE, "/san-pham/1", ""), &tt).ma, 404); // xóa lại
    }

    #[test]
    fn liet_ke_sap_theo_id() {
        let (app, tt) = moi_truong();
        for i in 1..=3 { app.xu_ly(yc(PhuongThuc::POST, "/san-pham", &format!("ten=SP{};gia={}", i, i)), &tt); }
        let r = app.xu_ly(yc(PhuongThuc::GET, "/san-pham", ""), &tt);
        assert_eq!(r.than, "1:SP1:1,2:SP2:2,3:SP3:3");
    }
}
