//! Chương 61 — Backend Web: kiến trúc một dịch vụ HTTP. Lõi định tuyến + xử lý
//! nghiệp vụ thuần túy (kiểm thử được KHÔNG cần server), phản chiếu cách Axum hoạt động.
//! Bản Axum 0.8 thật của cùng API nằm ở crate `ch61_axum`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// 1. MÔ HÌNH HTTP — Request / Response / Method
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub path: String,
    pub body: String, // thân yêu cầu (dạng "k=v;k=v" cho đơn giản)
    pub path_params: HashMap<String, String>, // /users/{id} -> {id: "7"}
}

#[derive(Debug, Clone, PartialEq)]
pub struct Response {
    pub status: u16, // 200, 201, 400, 404, 405, 422...
    pub body: String,
}

impl Response {
    pub fn ok(body: impl Into<String>) -> Self {
        Response {
            status: 200,
            body: body.into(),
        }
    }
    pub fn created(body: impl Into<String>) -> Self {
        Response {
            status: 201,
            body: body.into(),
        }
    }
    /// Yêu cầu sai hình thức (vd tham số đường dẫn không phải số) — như Axum trả 400.
    pub fn bad_request(reason: impl Into<String>) -> Self {
        Response {
            status: 400,
            body: reason.into(),
        }
    }
    pub fn not_found() -> Self {
        Response {
            status: 404,
            body: "Không tìm thấy".into(),
        }
    }
    pub fn method_not_allowed() -> Self {
        Response {
            status: 405,
            body: "Phương thức không được hỗ trợ".into(),
        }
    }
    /// Đúng hình thức nhưng dữ liệu không hợp lệ — 422.
    pub fn unprocessable(reason: impl Into<String>) -> Self {
        Response {
            status: 422,
            body: reason.into(),
        }
    }
}

// ============================================================================
// 2. BỘ ĐỊNH TUYẾN (Router) — khớp phương thức + mẫu đường dẫn
// ============================================================================

pub type Handler = Arc<dyn Fn(&Request, &AppState) -> Response + Send + Sync>;

pub struct Route {
    method: Method,
    pattern: Vec<String>, // ["users", "{id}", "profile"]
    handler: Handler,
}

/// Kết quả khớp tuyến.
enum RouteMatch<'a> {
    Found(&'a Handler, HashMap<String, String>),
    /// Có tuyến khớp đường dẫn nhưng không khớp phương thức.
    WrongMethod,
    NotFound,
}

#[derive(Default)]
pub struct Router {
    routes: Vec<Route>,
}

impl Router {
    pub fn new() -> Self {
        Self::default()
    }

    /// Đăng ký một tuyến. Tham số động viết `{ten}` như Axum 0.8.
    pub fn route(mut self, method: Method, pattern: &str, handler: Handler) -> Self {
        self.routes.push(Route {
            method,
            pattern: pattern
                .trim_matches('/')
                .split('/')
                .map(|s| s.to_string())
                .collect(),
            handler,
        });
        self
    }

    /// Khớp đường dẫn với một mẫu; trả tham số động nếu khớp.
    fn match_path(pattern: &[String], segments: &[&str]) -> Option<HashMap<String, String>> {
        if pattern.len() != segments.len() {
            return None;
        }
        let mut params = HashMap::new();
        for (pat, actual) in pattern.iter().zip(segments) {
            if let Some(name) = pat.strip_prefix('{').and_then(|p| p.strip_suffix('}')) {
                params.insert(name.to_string(), actual.to_string()); // tham số động
            } else if pat != actual {
                return None;
            }
        }
        Some(params)
    }

    /// Khớp một yêu cầu với tuyến: tách đoạn đường dẫn TRƯỚC, rồi mới so từng đoạn.
    fn find(&self, req: &Request) -> RouteMatch<'_> {
        let segments: Vec<&str> = req.path.trim_matches('/').split('/').collect();
        let mut path_matched = false;
        for route in &self.routes {
            if let Some(params) = Self::match_path(&route.pattern, &segments) {
                if route.method == req.method {
                    return RouteMatch::Found(&route.handler, params);
                }
                path_matched = true;
            }
        }
        if path_matched {
            RouteMatch::WrongMethod
        } else {
            RouteMatch::NotFound
        }
    }

    /// Xử lý một yêu cầu: khớp tuyến, gọi bộ xử lý, hoặc trả 404/405.
    pub fn handle(&self, mut req: Request, state: &AppState) -> Response {
        match self.find(&req) {
            RouteMatch::Found(handler, params) => {
                req.path_params = params;
                handler(&req, state)
            }
            RouteMatch::WrongMethod => Response::method_not_allowed(),
            RouteMatch::NotFound => Response::not_found(),
        }
    }
}

// ============================================================================
// 3. TRẠNG THÁI CHIA SẺ (Shared State) — như State<Arc<AppState>> của Axum
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub price: u64,
}

/// Đặt tên `AppState` (không phải `State`) để không đè `axum::extract::State`.
pub struct AppState {
    pub store: Mutex<HashMap<u64, Product>>,
    pub next_id: Mutex<u64>,
}

impl AppState {
    pub fn new() -> Arc<Self> {
        Arc::new(AppState {
            store: Mutex::new(HashMap::new()),
            next_id: Mutex::new(1),
        })
    }
}

// ============================================================================
// 4. BỘ XỬ LÝ (Handlers) — LÕI THUẦN TÚY nghiệp vụ, kiểm thử được
// ============================================================================

/// Phân tích thân yêu cầu rất đơn giản: "name=X;price=Y" (thay cho serde để chạy offline).
fn parse_body(body: &str) -> HashMap<String, String> {
    body.split(';')
        .filter_map(|c| c.split_once('='))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

/// Trích `{id}` thành u64. Sai -> 400, giống `Path<u64>` của Axum.
fn path_id(req: &Request) -> Result<u64, Response> {
    req.path_params
        .get("id")
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| Response::bad_request("mã sản phẩm phải là số nguyên"))
}

fn format_product(p: &Product) -> String {
    format!("{}:{}:{}", p.id, p.name, p.price)
}

pub fn handle_list(_req: &Request, state: &AppState) -> Response {
    let store = state.store.lock().unwrap();
    let mut list: Vec<&Product> = store.values().collect();
    list.sort_by_key(|p| p.id);
    let body = list
        .iter()
        .map(|p| format_product(p))
        .collect::<Vec<_>>()
        .join(",");
    Response::ok(body)
}

pub fn handle_get_one(req: &Request, state: &AppState) -> Response {
    let id = match path_id(req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    match state.store.lock().unwrap().get(&id) {
        Some(p) => Response::ok(format_product(p)),
        None => Response::not_found(),
    }
}

pub fn handle_create(req: &Request, state: &AppState) -> Response {
    let fields = parse_body(&req.body);
    let name = match fields.get("name") {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return Response::unprocessable("thiếu tên sản phẩm"),
    };
    let price: u64 = match fields.get("price").and_then(|g| g.parse().ok()) {
        Some(g) => g,
        None => return Response::unprocessable("giá phải là số nguyên"),
    };
    let mut next_id = state.next_id.lock().unwrap();
    let id = *next_id;
    *next_id += 1;
    state
        .store
        .lock()
        .unwrap()
        .insert(id, Product { id, name, price });
    Response::created(format!("Đã tạo sản phẩm #{}", id))
}

pub fn handle_delete(req: &Request, state: &AppState) -> Response {
    let id = match path_id(req) {
        Ok(id) => id,
        Err(resp) => return resp,
    };
    if state.store.lock().unwrap().remove(&id).is_some() {
        Response::ok(format!("Đã xóa #{}", id))
    } else {
        Response::not_found()
    }
}

/// Dựng bộ định tuyến — tương đương `Router::new().route(...)` của Axum.
pub fn build_router() -> Router {
    Router::new()
        .route(Method::Get, "/products", Arc::new(handle_list))
        .route(Method::Get, "/products/{id}", Arc::new(handle_get_one))
        .route(Method::Post, "/products", Arc::new(handle_create))
        .route(Method::Delete, "/products/{id}", Arc::new(handle_delete))
}

fn request(method: Method, path: &str, body: &str) -> Request {
    Request {
        method,
        path: path.into(),
        body: body.into(),
        path_params: HashMap::new(),
    }
}

fn main() {
    println!("═══════════════════════════════════════════════════════════════");
    println!("   BACKEND WEB: BỘ ĐỊNH TUYẾN · TRẠNG THÁI · BỘ XỬ LÝ (như Axum) ");
    println!("═══════════════════════════════════════════════════════════════");

    let app = build_router();
    let state = AppState::new();

    let call = |method: Method, path: &str, body: &str| {
        let r = app.handle(request(method, path, body), &state);
        println!(
            "   {:<7} {:<16} -> {} {}",
            format!("{method:?}").to_uppercase(),
            path,
            r.status,
            r.body
        );
        r
    };

    println!("\nMô phỏng các lời gọi API:");
    call(Method::Post, "/products", "name=Bàn phím;price=1200000");
    call(Method::Post, "/products", "name=Chuột;price=350000");
    call(Method::Get, "/products", "");
    call(Method::Get, "/products/1", "");
    call(Method::Get, "/products/99", ""); // 404
    call(Method::Get, "/products/abc", ""); // 400 mã không phải số
    call(Method::Post, "/products", "price=xyz"); // 422 thiếu tên
    call(Method::Put, "/products/1", ""); // 405 có tuyến nhưng sai phương thức
    call(Method::Delete, "/products/2", "");
    call(Method::Get, "/no-such-route", ""); // 404

    println!("\n═══════════════════════════════════════════════════════════════");
    println!("   LÕI NGHIỆP VỤ THUẦN TÚY = KIỂM THỬ ĐƯỢC KHÔNG CẦN CHẠY SERVER ");
    println!("═══════════════════════════════════════════════════════════════");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Router, Arc<AppState>) {
        (build_router(), AppState::new())
    }

    #[test]
    fn create_and_read_product() {
        let (app, state) = setup();
        let r = app.handle(
            request(Method::Post, "/products", "name=Sách;price=45000"),
            &state,
        );
        assert_eq!(r.status, 201);
        let r = app.handle(request(Method::Get, "/products/1", ""), &state);
        assert_eq!(r.status, 200);
        assert_eq!(r.body, "1:Sách:45000");
    }

    #[test]
    fn unknown_route_returns_404() {
        let (app, state) = setup();
        let r = app.handle(request(Method::Get, "/anything", ""), &state);
        assert_eq!(r.status, 404);
    }

    #[test]
    fn wrong_method_returns_405() {
        let (app, state) = setup();
        // Có tuyến GET /products/{id} nhưng không có PUT -> 405 (như Axum)
        let r = app.handle(request(Method::Put, "/products/1", ""), &state);
        assert_eq!(r.status, 405);
    }

    #[test]
    fn non_numeric_id_returns_400() {
        let (app, state) = setup();
        let r = app.handle(request(Method::Get, "/products/abc", ""), &state);
        assert_eq!(r.status, 400);
    }

    #[test]
    fn invalid_payload_returns_422() {
        let (app, state) = setup();
        // Thiếu tên
        let r = app.handle(request(Method::Post, "/products", "price=100"), &state);
        assert_eq!(r.status, 422);
        // Giá không phải số
        let r = app.handle(
            request(Method::Post, "/products", "name=X;price=abc"),
            &state,
        );
        assert_eq!(r.status, 422);
    }

    #[test]
    fn dynamic_path_params() {
        let (app, state) = setup();
        app.handle(request(Method::Post, "/products", "name=A;price=1"), &state);
        app.handle(request(Method::Post, "/products", "name=B;price=2"), &state);
        // {id} được trích đúng
        let r = app.handle(request(Method::Get, "/products/2", ""), &state);
        assert_eq!(r.body, "2:B:2");
    }

    #[test]
    fn delete_product() {
        let (app, state) = setup();
        app.handle(request(Method::Post, "/products", "name=A;price=1"), &state);
        let del = || app.handle(request(Method::Delete, "/products/1", ""), &state);
        assert_eq!(del().status, 200);
        let r = app.handle(request(Method::Get, "/products/1", ""), &state);
        assert_eq!(r.status, 404); // đã xóa
        assert_eq!(del().status, 404); // xóa lại
    }

    #[test]
    fn list_is_sorted_by_id() {
        let (app, state) = setup();
        for i in 1..=3 {
            let body = format!("name=P{i};price={i}");
            app.handle(request(Method::Post, "/products", &body), &state);
        }
        let r = app.handle(request(Method::Get, "/products", ""), &state);
        assert_eq!(r.body, "1:P1:1,2:P2:2,3:P3:3");
    }
}
