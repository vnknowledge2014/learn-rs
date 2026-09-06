#![allow(dead_code, unused_variables)]
//! Chương 61 — Backend Web: kiến trúc một dịch vụ HTTP. Lõi định tuyến + xử lý
//! nghiệp vụ thuần túy (kiểm thử được KHÔNG cần server), phản chiếu cách Axum hoạt động.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// 1. MÔ HÌNH HTTP — Request / Response / Method
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Method { GET, POST, PUT, DELETE }

#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub than: String, // body (JSON dạng chuỗi cho đơn giản)
    pub path_param: HashMap<String, String>, // /user/:id -> {id: "7"}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub id: u16, // 200, 201, 404, 422...
    pub than: String,
}

impl Response {
    pub fn ok(than: impl Into<String>) -> Self { Response { id: 200, than: than.into() } }
    pub fn tao(than: impl Into<String>) -> Self { Response { id: 201, than: than.into() } }
    pub fn not_seen() -> Self { Response { id: 404, than: "Không tìm thấy".into() } }
    pub fn data_sai(ly_do: impl Into<String>) -> Self { Response { id: 422, than: ly_do.into() } }
}

// ============================================================================
// 2. BỘ ĐỊNH TUYẾN (Router) — khớp phương thức + mẫu đường dẫn
// ============================================================================

pub type UnitHandle = Arc<dyn Fn(&Request, &State) -> Response + Send + Sync>;

pub struct Route {
    method: Method,
    mau: Vec<String>, // ["user", ":id", "profile"]
    handle: UnitHandle,
}

pub struct RouteMatcher {
    route: Vec<Route>,
}

impl RouteMatcher {
    pub fn new() -> Self { RouteMatcher { route: Vec::new() } }

    pub fn them(mut self, pt: Method, mau: &str, handle: UnitHandle) -> Self {
        self.route.push(Route {
            method: pt,
            mau: mau.trim_matches('/').split('/').map(|s| s.to_string()).collect(),
            handle,
        });
        self
    }

    /// Khớp một yêu cầu với tuyến. Trả về (bộ xử lý, tham số đường dẫn) nếu khớp.
    fn fill<'a>(&'a self, yc: &Request) -> Option<(&'a UnitHandle, HashMap<String, String>)> {
        let part: Vec<&str> = yc.path.trim_matches('/').split('/').collect();
        for t in &self.route {
            if t.method != yc.method || t.mau.len() != part.len() {
                continue;
            }
            let mut param = HashMap::new();
            let mut fill = true;
            for (mau, thuc) in t.mau.iter().zip(part.iter()) {
                if let Some(name) = mau.strip_prefix(':') {
                    param.insert(name.to_string(), thuc.to_string()); // tham số động
                } else if mau != thuc {
                    fill = false;
                    break;
                }
            }
            if fill {
                return Some((&t.handle, param));
            }
        }
        None
    }

    /// Xử lý một yêu cầu: khớp tuyến, gọi bộ xử lý, hoặc trả 404.
    pub fn handle(&self, mut yc: Request, tt: &State) -> Response {
        match self.fill(&yc) {
            Some((handle, param)) => {
                yc.path_param = param;
                handle(&yc, tt)
            }
            None => Response::not_seen(),
        }
    }
}

// ============================================================================
// 3. TRẠNG THÁI CHIA SẺ (Shared State) — như State<Arc<AppState>> của Axum
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct SanPham {
    pub id: u64,
    pub name: String,
    pub price: u64,
}

pub struct State {
    pub store: Mutex<HashMap<u64, SanPham>>,
    pub next_id: Mutex<u64>,
}

impl State {
    pub fn new() -> Arc<Self> {
        Arc::new(State {
            store: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        })
    }
}

// ============================================================================
// 4. BỘ XỬ LÝ (Handlers) — LÕI THUẦN TÚY nghiệp vụ, kiểm thử được
// ============================================================================

/// Phân tích JSON thô rất đơn giản: "ten=X;gia=Y" (thay cho serde để chạy offline).
fn analyze_than(than: &str) -> HashMap<String, String> {
    than.split(';')
        .filter_map(|c| c.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

pub fn xu_ly_liet_ke(_yc: &Request, tt: &State) -> Response {
    let store = tt.store.lock().unwrap();
    let mut list: Vec<&SanPham> = store.values().collect();
    list.sort_by_key(|s| s.id);
    let than = list.iter().map(|s| format!("{}:{}:{}", s.id, s.name, s.price))
        .collect::<Vec<_>>().join(",");
    Response::ok(than)
}

pub fn handle_view_one(yc: &Request, tt: &State) -> Response {
    let id: u64 = match yc.path_param.get("id").and_then(|s| s.parse().ok()) {
        Some(x) => x,
        None => return Response::data_sai("id không hợp lệ"),
    };
    match tt.store.lock().unwrap().get(&id) {
        Some(sp) => Response::ok(format!("{}:{}:{}", sp.id, sp.name, sp.price)),
        None => Response::not_seen(),
    }
}

pub fn handle_make(yc: &Request, tt: &State) -> Response {
    let truong = analyze_than(&yc.than);
    let name = match truong.get("ten") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return Response::data_sai("thiếu tên sản phẩm"),
    };
    let price: u64 = match truong.get("gia").and_then(|g| g.parse().ok()) {
        Some(g) => g,
        None => return Response::data_sai("giá phải là số nguyên"),
    };
    let mut id_ke = tt.next_id.lock().unwrap();
    let id = *id_ke;
    *id_ke += 1;
    tt.store.lock().unwrap().insert(id, SanPham { id, name, price });
    Response::tao(format!("Đã tạo sản phẩm #{}", id))
}

pub fn handle_remove(yc: &Request, tt: &State) -> Response {
    let id: u64 = match yc.path_param.get("id").and_then(|s| s.parse().ok()) {
        Some(x) => x,
        None => return Response::data_sai("id không hợp lệ"),
    };
    if tt.store.lock().unwrap().remove(&id).is_some() {
        Response::ok(format!("Đã xóa #{}", id))
    } else {
        Response::not_seen()
    }
}

/// Dựng bộ định tuyến — tương đương `Router::new().route(...)` của Axum.
pub fn use_resp_use() -> RouteMatcher {
    RouteMatcher::new()
        .them(Method::GET, "/san-pham", Arc::new(xu_ly_liet_ke))
        .them(Method::GET, "/san-pham/:id", Arc::new(handle_view_one))
        .them(Method::POST, "/san-pham", Arc::new(handle_make))
        .them(Method::DELETE, "/san-pham/:id", Arc::new(handle_remove))
}

fn yc(pt: Method, dd: &str, than: &str) -> Request {
    Request { method: pt, path: dd.into(), than: than.into(), path_param: HashMap::new() }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   BACKEND WEB: BỘ ĐỊNH TUYẾN · TRẠNG THÁI · BỘ XỬ LÝ (như Axum) ");
    println!("═══════════════════════════════════════════════════════════════");

    let app = use_resp_use();
    let tt = State::new();

    let goi = |pt, dd: &str, than: &str| {
        let r = app.handle(yc(pt, dd, than), &tt);
        println!("   {:>6} {:<18} -> {} {}", format!("{:?}", &r.id)[0..3].to_string(), dd, r.id, r.than);
        r
    };

    println!("\nMô phỏng các lời gọi API:");
    goi(Method::POST, "/san-pham", "ten=Bàn phím;gia=1200000");
    goi(Method::POST, "/san-pham", "ten=Chuột;gia=350000");
    goi(Method::GET, "/san-pham", "");
    goi(Method::GET, "/san-pham/1", "");
    goi(Method::GET, "/san-pham/99", "");         // 404
    goi(Method::POST, "/san-pham", "gia=xyz");     // 422 thiếu tên
    goi(Method::DELETE, "/san-pham/2", "");
    goi(Method::GET, "/khong-co-tuyen", "");       // 404

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   LÕI NGHIỆP VỤ THUẦN TÚY = KIỂM THỬ ĐƯỢC KHÔNG CẦN CHẠY SERVER ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn environment() -> (RouteMatcher, Arc<State>) {
        (use_resp_use(), State::new())
    }

    #[test]
    fn create_and_read_product() {
        let (app, tt) = environment();
        let r = app.handle(yc(Method::POST, "/san-pham", "ten=Sách;gia=45000"), &tt);
        assert_eq!(r.id, 201);
        let r = app.handle(yc(Method::GET, "/san-pham/1", ""), &tt);
        assert_eq!(r.id, 200);
        assert_eq!(r.than, "1:Sách:45000");
    }

    #[test]
    fn unknown_route_returns_404() {
        let (app, tt) = environment();
        assert_eq!(app.handle(yc(Method::GET, "/bat-ky", ""), &tt).id, 404);
    }

    #[test]
    fn wrong_method_returns_404() {
        let (app, tt) = environment();
        // Có tuyến GET /san-pham/:id nhưng không có PUT -> 404
        assert_eq!(app.handle(yc(Method::PUT, "/san-pham/1", ""), &tt).id, 404);
    }

    #[test]
    fn invalid_payload_returns_422() {
        let (app, tt) = environment();
        // Thiếu tên
        assert_eq!(app.handle(yc(Method::POST, "/san-pham", "gia=100"), &tt).id, 422);
        // Giá không phải số
        assert_eq!(app.handle(yc(Method::POST, "/san-pham", "ten=X;gia=abc"), &tt).id, 422);
    }

    #[test]
    fn dynamic_path_params() {
        let (app, tt) = environment();
        app.handle(yc(Method::POST, "/san-pham", "ten=A;gia=1"), &tt);
        app.handle(yc(Method::POST, "/san-pham", "ten=B;gia=2"), &tt);
        // :id được trích đúng
        assert_eq!(app.handle(yc(Method::GET, "/san-pham/2", ""), &tt).than, "2:B:2");
    }

    #[test]
    fn delete_product() {
        let (app, tt) = environment();
        app.handle(yc(Method::POST, "/san-pham", "ten=A;gia=1"), &tt);
        assert_eq!(app.handle(yc(Method::DELETE, "/san-pham/1", ""), &tt).id, 200);
        assert_eq!(app.handle(yc(Method::GET, "/san-pham/1", ""), &tt).id, 404); // đã xóa
        assert_eq!(app.handle(yc(Method::DELETE, "/san-pham/1", ""), &tt).id, 404); // xóa lại
    }

    #[test]
    fn list_is_sorted_by_id() {
        let (app, tt) = environment();
        for i in 1..=3 { app.handle(yc(Method::POST, "/san-pham", &format!("ten=SP{};gia={}", i, i)), &tt); }
        let r = app.handle(yc(Method::GET, "/san-pham", ""), &tt);
        assert_eq!(r.than, "1:SP1:1,2:SP2:2,3:SP3:3");
    }
}
