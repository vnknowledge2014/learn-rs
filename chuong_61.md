# Chương 61: Phát triển Backend Web với Axum — Định tuyến, Bộ trích xuất, Trạng thái & Xử lý lỗi (Backend Web Development)

## Giới thiệu & Mục tiêu học tập

Chương 51 đã giới thiệu Axum và gRPC ở mức tổng quan. Chương này đi sâu vào **xây dựng một dịch vụ web hoàn chỉnh**: định tuyến, trích xuất dữ liệu từ yêu cầu, quản lý trạng thái chia sẻ, xử lý lỗi, và — quan trọng nhất — **cách tổ chức mã để kiểm thử được mà không cần chạy server**.

[Axum](https://github.com/tokio-rs/axum) là framework web của nhóm Tokio, được yêu thích nhờ một triết lý: **dùng hệ thống kiểu của Rust làm giao diện**. Bạn khai báo handler nhận `Path<u32>`, `Json<T>`, `State<S>` — và Axum tự động trích xuất, kiểm tra, ép kiểu. Dữ liệu sai là phản hồi `400`/`422` tự động (hoặc lỗi biên dịch nếu bạn dùng sai kiểu trong mã), không phải lỗi runtime mơ hồ.

Để bạn hiểu *cơ chế bên dưới* Axum, chương này xây một **mini-router** từ đầu — khớp phương thức + mẫu đường dẫn, trích tham số động, gọi handler — rồi mới chỉ ra mã Axum thật tương ứng. Mini-router chạy offline và có test đầy đủ, minh họa nguyên tắc vàng: **tách lõi nghiệp vụ thuần túy khỏi khung web**, để test không cần mạng.

Mục tiêu học tập:
- Hiểu vòng đời một yêu cầu HTTP: **định tuyến → trích xuất → xử lý → phản hồi**.
- Cài **bộ định tuyến** khớp mẫu đường dẫn động (`/products/{id}`, cú pháp của Axum 0.8).
- Quản lý **trạng thái chia sẻ** an toàn đa luồng bằng `Arc<Mutex<...>>` (Chương 27, 40, 50).
- Xử lý lỗi bằng **mã trạng thái HTTP đúng ngữ nghĩa**: 200/201/400/404/405/422.
- Viết handler như **lõi thuần túy kiểm thử được** (Chương 20, 55) — test API không cần server.
- Đọc và viết mã **Axum thật**, hiểu bộ trích xuất (extractor) và `IntoResponse`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: MỘT TÒA SOẠN BÁO NHẬN VÀ XỬ LÝ THƯ ĐỘC GIẢ                   │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Thư đến (YÊU CẦU HTTP)                                                         │
│        │                                                                         │
│        ▼                                                                         │
│   [1. NHÂN VIÊN PHÂN LOẠI]  = BỘ ĐỊNH TUYẾN (Router)                             │
│        Đọc địa chỉ trên phong bì ("mục Thể thao", "mục Kinh tế"):                │
│        GET /products/7 → chuyển tới đúng biên tập viên phụ trách.                │
│        Không có mục nào khớp → trả lại "404 địa chỉ không tồn tại".              │
│        │                                                                         │
│        ▼                                                                         │
│   [2. THƯ KÝ BÓC THƯ]  = BỘ TRÍCH XUẤT (Extractor)                               │
│        Mở phong bì, lấy ra: số báo ({id}=7), nội dung (JSON body).               │
│        Thư không đọc được (JSON hỏng) → "400"; đọc được nhưng                    │
│        thiếu/sai mục (sai kiểu, thiếu trường) → "422".                           │
│        │                                                                         │
│        ▼                                                                         │
│   [3. BIÊN TẬP VIÊN]  = BỘ XỬ LÝ (Handler) — LÕI THUẦN TÚY                       │
│        Xử lý nghiệp vụ: tra sản phẩm #7 trong KHO (trạng thái chia sẻ).          │
│        Trả về nội dung, hoặc "404 không có sản phẩm này".                        │
│        │                                                                         │
│        ▼                                                                         │
│   Thư trả lời (PHẢN HỒI HTTP) với mã trạng thái đúng.                            │
└──────────────────────────────────────────────────────────────────────────────────┘
```

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Định tuyến — khớp phương thức và mẫu đường dẫn

Một tuyến gồm ba phần: **phương thức** (GET/POST...), **mẫu đường dẫn** (có thể chứa tham số động `{id}` — Axum 0.8 dùng ngoặc nhọn; bản 0.7 trở về trước dùng `:id`), và **bộ xử lý**. Bộ định tuyến duyệt các tuyến, tìm cái đầu tiên khớp cả phương thức lẫn hình dạng đường dẫn.

Điểm tinh tế: `GET /products/{id}` khớp `/products/7` và trích `id = "7"`, nhưng KHÔNG khớp `/products` (khác số đoạn) hay `POST /products/7` (khác phương thức). Cùng một đường dẫn với phương thức khác là *tuyến khác* — đó là cách REST phân biệt "xem" (GET) với "xóa" (DELETE) cùng một tài nguyên. Khi đường dẫn khớp mà phương thức không khớp tuyến nào, phản hồi đúng là `405 Method Not Allowed` chứ không phải `404` — cả Axum lẫn mini-router của chương đều làm vậy.

### 2. Bộ trích xuất (Extractor) — hệ thống kiểu làm giao diện

Đây là điều làm Axum khác biệt. Trong nhiều framework, bạn tự lấy dữ liệu từ đối tượng request thô rồi tự ép kiểu — dễ sai runtime. Axum đảo ngược: bạn **khai báo kiểu bạn muốn** trong chữ ký handler, và framework tự trích xuất:

```rust
async fn view(Path(id): Path<u32>, State(store): State<Arc<Store>>) -> impl IntoResponse
//           └── trích {id} từ URL, ép sang u32  └── lấy trạng thái chia sẻ
```

Nếu đoạn `{id}` trong URL không phải số, Axum trả `400` tự động — handler của bạn *không bao giờ được gọi* với dữ liệu sai. Đây là "phân tích, đừng xác thực" (Chương 20) ở tầng HTTP: sau khi qua extractor, `id` đã là `u32` hợp lệ, không cần kiểm tra lại.

### 3. Trạng thái chia sẻ — an toàn đa luồng

Một máy chủ web xử lý nhiều yêu cầu *đồng thời* trên nhiều luồng (Chương 49). Trạng thái chung (kho sản phẩm, kết nối cơ sở dữ liệu) phải chia sẻ an toàn. Mẫu chuẩn: `Arc<Mutex<T>>` — `Arc` để nhiều luồng cùng sở hữu (Chương 27), `Mutex` để chỉ một luồng sửa tại một thời điểm (Chương 40, 50).

> Trong sản phẩm thật, thay `Mutex<HashMap>` bằng một *pool kết nối cơ sở dữ liệu* (như `sqlx::PgPool`) — bản thân nó đã an toàn đa luồng, và bạn để cơ sở dữ liệu lo chuyện đồng thời (Chương 35, MVCC).

### 4. Xử lý lỗi — mã trạng thái đúng ngữ nghĩa

Mỗi loại lỗi có một mã HTTP đúng:

| Tình huống | Mã | Ý nghĩa |
|---|---|---|
| Thành công | 200 | OK |
| Tạo mới thành công | 201 | Created |
| Không tìm thấy tài nguyên | 404 | Not Found |
| Yêu cầu sai hình thức (JSON hỏng cú pháp, tham số đường dẫn không phải số) | 400 | Bad Request |
| Đúng hình thức nhưng dữ liệu sai (sai kiểu, thiếu trường, vi phạm ràng buộc) | 422 | Unprocessable Content |
| Có tuyến nhưng sai phương thức | 405 | Method Not Allowed |
| Chưa đăng nhập | 401 | Unauthorized |
| Không đủ quyền | 403 | Forbidden (nhớ IDOR ở Chương 57!) |
| Lỗi phía máy chủ | 500 | Internal Server Error |

Trong Axum, bạn định nghĩa một kiểu lỗi và cài `IntoResponse` cho nó — mỗi biến thể lỗi ánh xạ tới một mã. Đây chính là *Bifunctor* của `Result` (Chương 19): `map` cho nhánh thành công, `map_err` cho nhánh lỗi thành mã HTTP.

Với bộ trích xuất `Json<T>` của Axum 0.8, ranh giới 400/422 rất rõ: thân không phải JSON hợp lệ (`{name:`) → **400**; JSON hợp lệ nhưng không khớp kiểu `T` (`"price": "abc"`, thiếu trường) → **422**; thiếu header `Content-Type: application/json` → **415**. Crate `ch61_axum` có test cho cả ba trường hợp.

### 5. Vì sao tách lõi thuần túy khỏi khung web

Mã trong chương này để toàn bộ logic nghiệp vụ trong các hàm `handle_*` **thuần túy** (nhận yêu cầu + trạng thái, trả phản hồi), và bộ định tuyến chỉ là lớp mỏng điều phối. Nhờ vậy:
- **Test không cần server**: gọi `app.handle(request, &state)` trực tiếp và kiểm phản hồi (xem module test).
- **Đổi framework không đổi logic**: chuyển từ router tự viết sang Axum, hay sang một framework khác, phần lõi giữ nguyên.

Đây là kiến trúc "lõi thuần túy, vỏ mệnh lệnh" (Chương 20) áp dụng vào web: Axum là *vỏ*, các hàm nghiệp vụ là *lõi*.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Lõi định tuyến + nghiệp vụ, chạy offline, kiểm thử đầy đủ:

```bash
cd code
cargo run  -p ch61
cargo test -p ch61
```

```rust
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
```

---

## Chuyển sang Axum thật

Cùng một API viết bằng Axum thật chỉ khác ở *lớp vỏ*. Mã dưới đây là crate `code/ch61_axum` (Axum 0.8 + Tokio), biên dịch và kiểm thử trong workspace:

```bash
cd code
cargo test -p ch61_axum          # test bằng tower::ServiceExt::oneshot, không mở cổng
cargo run  -p ch61_axum          # chạy server thật tại 127.0.0.1:3000
```

```toml
# code/ch61_axum/Cargo.toml
[package]
name = "ch61_axum"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = "0.8"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

Để ý hai chi tiết: kiểu trạng thái tên là `AppState` — nếu đặt tên `State` nó sẽ đè lên bộ trích xuất `axum::extract::State` và gây lỗi khó hiểu; và `Json<T>` luôn đứng **cuối** danh sách tham số vì nó tiêu thụ thân yêu cầu.

```rust
//! Chương 61 — cùng API sản phẩm như `ch61`, nhưng viết bằng Axum 0.8 + Tokio thật.
//! Lõi nghiệp vụ (`AppState`) vẫn tách khỏi lớp vỏ HTTP; test gửi yêu cầu giả vào
//! router bằng `tower::ServiceExt::oneshot` — không mở cổng mạng.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// 1. MÔ HÌNH DỮ LIỆU — serde lo việc (giải) tuần tự hoá JSON
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Product {
    pub id: u64,
    pub name: String,
    pub price: u64,
}

/// Thân yêu cầu `POST /products` và `PUT /products/{id}`.
#[derive(Debug, Deserialize)]
pub struct NewProduct {
    pub name: String,
    pub price: u64,
}

// ============================================================================
// 2. TRẠNG THÁI CHIA SẺ — đặt tên `AppState` để KHÔNG đè `axum::extract::State`
// ============================================================================

#[derive(Default)]
pub struct AppState {
    inner: Mutex<Store>,
}

#[derive(Default)]
struct Store {
    products: HashMap<u64, Product>,
    next_id: u64,
}

pub type SharedState = Arc<AppState>;

// ============================================================================
// 3. KIỂU LỖI + IntoResponse — mỗi biến thể ánh xạ tới một mã HTTP
// ============================================================================

#[derive(Debug, PartialEq)]
pub enum AppError {
    NotFound,
    Validation(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::NotFound => (StatusCode::NOT_FOUND, "Không tìm thấy").into_response(),
            AppError::Validation(reason) => {
                (StatusCode::UNPROCESSABLE_ENTITY, reason).into_response()
            }
        }
    }
}

// ============================================================================
// 4. LÕI NGHIỆP VỤ — hàm đồng bộ thuần, không biết gì về HTTP
// ============================================================================

impl AppState {
    pub fn list(&self) -> Vec<Product> {
        let store = self.inner.lock().unwrap();
        let mut list: Vec<Product> = store.products.values().cloned().collect();
        list.sort_by_key(|p| p.id);
        list
    }

    pub fn get(&self, id: u64) -> Result<Product, AppError> {
        let store = self.inner.lock().unwrap();
        // Sao chép ra ngoài trước khi khoá được nhả — không để MutexGuard thoát ra.
        store.products.get(&id).cloned().ok_or(AppError::NotFound)
    }

    pub fn create(&self, input: NewProduct) -> Result<Product, AppError> {
        validate(&input)?;
        // Một khoá duy nhất bao cả bộ đếm lẫn kho: cấp mã và chèn là một bước nguyên tử.
        let mut store = self.inner.lock().unwrap();
        store.next_id += 1;
        let id = store.next_id;
        let product = Product {
            id,
            name: input.name,
            price: input.price,
        };
        store.products.insert(id, product.clone());
        Ok(product)
    }

    pub fn update(&self, id: u64, input: NewProduct) -> Result<Product, AppError> {
        validate(&input)?;
        let mut store = self.inner.lock().unwrap();
        let product = store.products.get_mut(&id).ok_or(AppError::NotFound)?;
        product.name = input.name;
        product.price = input.price;
        Ok(product.clone())
    }

    pub fn delete(&self, id: u64) -> Result<(), AppError> {
        let mut store = self.inner.lock().unwrap();
        store
            .products
            .remove(&id)
            .map(|_| ())
            .ok_or(AppError::NotFound)
    }
}

/// Kiểm tra *nghiệp vụ* (JSON đã đúng cú pháp và đúng kiểu, nhưng giá trị vô nghĩa).
fn validate(input: &NewProduct) -> Result<(), AppError> {
    if input.name.trim().is_empty() {
        return Err(AppError::Validation("thiếu tên sản phẩm".into()));
    }
    Ok(())
}

// ============================================================================
// 5. VỎ HTTP — handler mỏng: nhận dữ liệu ĐÃ trích xuất, gọi lõi, trả phản hồi
// ============================================================================

async fn list_products(State(state): State<SharedState>) -> Json<Vec<Product>> {
    Json(state.list())
}

async fn get_product(
    State(state): State<SharedState>,
    Path(id): Path<u64>, // {id} không phải số -> Axum tự trả 400, handler không được gọi
) -> Result<Json<Product>, AppError> {
    state.get(id).map(Json) // map cho nhánh Ok; AppError tự thành mã HTTP
}

async fn create_product(
    State(state): State<SharedState>,
    // JSON hỏng cú pháp -> 400; đúng cú pháp nhưng sai kiểu/thiếu trường -> 422;
    // thiếu header `Content-Type: application/json` -> 415. Tất cả do Axum tự trả.
    Json(input): Json<NewProduct>,
) -> Result<(StatusCode, Json<Product>), AppError> {
    let product = state.create(input)?;
    Ok((StatusCode::CREATED, Json(product)))
}

async fn update_product(
    State(state): State<SharedState>,
    Path(id): Path<u64>,
    Json(input): Json<NewProduct>, // Json phải là bộ trích xuất CUỐI (nó tiêu thụ thân)
) -> Result<Json<Product>, AppError> {
    state.update(id, input).map(Json)
}

async fn delete_product(
    State(state): State<SharedState>,
    Path(id): Path<u64>,
) -> Result<StatusCode, AppError> {
    state.delete(id)?;
    Ok(StatusCode::NO_CONTENT)
}

/// Dựng router. Axum 0.8 dùng cú pháp `{id}` cho tham số đường dẫn (bản ≤0.7 dùng `:id`).
pub fn app(state: SharedState) -> Router {
    Router::new()
        .route("/products", get(list_products).post(create_product))
        .route(
            "/products/{id}",
            get(get_product).put(update_product).delete(delete_product),
        )
        .with_state(state)
}

#[tokio::main]
async fn main() {
    let state = SharedState::default();
    let addr = std::env::var("ADDR").unwrap_or_else(|_| "127.0.0.1:3000".into());
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Đang lắng nghe tại http://{addr}  (Ctrl+C để dừng)");
    println!("Thử: curl -X POST -H 'content-type: application/json' \\");
    println!("        -d '{{\"name\":\"Bàn phím\",\"price\":1200000}}' http://{addr}/products");
    axum::serve(listener, app(state)).await.unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, header};
    use http_body_util::BodyExt;
    use tower::ServiceExt; // cho `.oneshot(...)`

    /// Gửi một yêu cầu vào router (không qua mạng), trả (mã, thân dạng chuỗi).
    async fn send(
        app: &Router,
        method: &str,
        uri: &str,
        json: Option<&str>,
    ) -> (StatusCode, String) {
        let mut builder = Request::builder().method(method).uri(uri);
        let body = match json {
            Some(j) => {
                builder = builder.header(header::CONTENT_TYPE, "application/json");
                Body::from(j.to_owned())
            }
            None => Body::empty(),
        };
        // Router là Clone và rẻ để clone (Arc bên trong); oneshot tiêu thụ service.
        let response = app
            .clone()
            .oneshot(builder.body(body).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    fn new_app() -> Router {
        app(SharedState::default())
    }

    #[tokio::test]
    async fn create_then_get() {
        let app = new_app();
        let (status, body) = send(
            &app,
            "POST",
            "/products",
            Some(r#"{"name":"Sách","price":45000}"#),
        )
        .await;
        assert_eq!(status, StatusCode::CREATED);
        let created: Product = serde_json::from_str(&body).unwrap();
        assert_eq!(created.id, 1);

        let (status, body) = send(&app, "GET", "/products/1", None).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(serde_json::from_str::<Product>(&body).unwrap(), created);
    }

    #[tokio::test]
    async fn missing_product_is_404() {
        let (status, _) = send(&new_app(), "GET", "/products/99", None).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn non_numeric_path_param_is_400() {
        let (status, _) = send(&new_app(), "GET", "/products/abc", None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn malformed_json_is_400() {
        let (status, _) = send(&new_app(), "POST", "/products", Some("{name:")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn wrong_json_type_is_422() {
        // Cú pháp JSON đúng, nhưng price là chuỗi -> lỗi dữ liệu -> 422
        let (status, _) = send(
            &new_app(),
            "POST",
            "/products",
            Some(r#"{"name":"A","price":"abc"}"#),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn empty_name_is_422_from_our_validation() {
        let (status, body) = send(
            &new_app(),
            "POST",
            "/products",
            Some(r#"{"name":"  ","price":1}"#),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body, "thiếu tên sản phẩm");
    }

    #[tokio::test]
    async fn missing_content_type_is_415() {
        let app = new_app();
        let req = Request::builder()
            .method("POST")
            .uri("/products")
            .body(Body::from(r#"{"name":"A","price":1}"#))
            .unwrap();
        let response = app.oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn wrong_method_is_405() {
        // Axum phân biệt "không có tuyến" (404) với "có tuyến, sai phương thức" (405)
        let (status, _) = send(&new_app(), "PATCH", "/products/1", None).await;
        assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test]
    async fn update_and_delete() {
        let app = new_app();
        send(&app, "POST", "/products", Some(r#"{"name":"A","price":1}"#)).await;
        let (status, body) = send(
            &app,
            "PUT",
            "/products/1",
            Some(r#"{"name":"B","price":2}"#),
        )
        .await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(serde_json::from_str::<Product>(&body).unwrap().name, "B");

        assert_eq!(
            send(&app, "DELETE", "/products/1", None).await.0,
            StatusCode::NO_CONTENT
        );
        assert_eq!(
            send(&app, "DELETE", "/products/1", None).await.0,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn list_is_sorted_by_id() {
        let app = new_app();
        for i in 1..=3 {
            let json = format!(r#"{{"name":"P{i}","price":{i}}}"#);
            send(&app, "POST", "/products", Some(&json)).await;
        }
        let (_, body) = send(&app, "GET", "/products", None).await;
        let ids: Vec<u64> = serde_json::from_str::<Vec<Product>>(&body)
            .unwrap()
            .iter()
            .map(|p| p.id)
            .collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }
}
```

So sánh mã Axum với mini-router: **cùng một kiến trúc** — định tuyến, trích xuất, trạng thái chia sẻ, mã lỗi. Axum chỉ thêm phần bất đồng bộ (`async`/`await`, Chương 49) và tự động hóa việc trích xuất/serialize. Hiểu mini-router là hiểu Axum.

> **Kiểm thử Axum thật**: module test ở trên dùng `tower::ServiceExt::oneshot` để gửi yêu cầu giả vào router mà **không mở cổng mạng** — đúng tầng integration test ở Chương 55. Toàn bộ triết lý "test không cần server" vẫn áp dụng.

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0308: mismatched types` (expected `Response`, found `()`) | Nhánh xử lý quên trả về giá trị | Mọi nhánh `match` định tuyến phải trả `Response`; bỏ dấu `;` ở biểu thức cuối |
| `E0596: cannot borrow data in an Arc as mutable` | Sửa trạng thái dùng chung qua `Arc` | `Arc<Mutex<T>>`, rồi `.lock().unwrap()` trước khi ghi |
| `E0515: cannot return value referencing temporary value` | Trả tham chiếu tới dữ liệu bên trong khoá (`s.store.lock().unwrap().get(&id)`) — `MutexGuard` tạm bị huỷ cuối câu lệnh | Sao chép ra (`.cloned()`) rồi mới trả; đừng để tham chiếu sống lâu hơn `MutexGuard` |
| `E0255`: the name `State` is defined multiple times | Tự đặt `struct State` trong khi đã `use axum::extract::State` | Đặt tên khác (`AppState`), như crate `ch61_axum` |
| `E0382: borrow of moved value` | Dùng lại `Request` sau khi đã chuyển vào bộ xử lý | Truyền `&Request`, hoặc `clone()` nếu thật cần sở hữu |
| Định tuyến trả 404 cho đường dẫn đúng (lỗi logic, không phải lỗi biên dịch) | So khớp cả chuỗi trước khi tách tham số động | Tách đoạn đường dẫn rồi mới so từng đoạn; xem `Router::match_path` |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Vòng đời yêu cầu**: định tuyến → trích xuất → xử lý → phản hồi. Mỗi tầng có một trách nhiệm.
2. **Axum dùng hệ thống kiểu làm giao diện.** `Path<u32>`, `Json<T>` tự trích xuất và kiểm kiểu — yêu cầu hỏng là 400, dữ liệu sai kiểu là 422, tự động, handler không bị gọi.
3. **Trạng thái chia sẻ dùng `Arc<Mutex<T>>`** (hoặc pool cơ sở dữ liệu). Mã lỗi phải đúng ngữ nghĩa HTTP.
4. **Tách lõi nghiệp vụ khỏi khung web** để test không cần server. Đổi framework không đổi logic — đây là "lõi thuần túy, vỏ mệnh lệnh" ở tầng web.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Thêm tuyến PUT cập nhật)**
Thêm handler `handle_update` cho `PUT /products/{id}` cập nhật tên và giá. Trả 404 nếu không tồn tại, 400 nếu mã không phải số, 422 nếu dữ liệu sai. Viết test. (Bản Axum đã có sẵn `update_product` trong `ch61_axum` để đối chiếu.)

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn handle_update(req: &Request, state: &AppState) -> Response {
    let id = match path_id(req) {
        Ok(id) => id,
        Err(resp) => return resp, // 400
    };
    let fields = parse_body(&req.body);
    let name = match fields.get("name") {
        Some(n) if !n.is_empty() => n.clone(),
        _ => return Response::unprocessable("thiếu tên sản phẩm"),
    };
    let price: u64 = match fields.get("price").and_then(|g| g.parse().ok()) {
        Some(g) => g,
        None => return Response::unprocessable("giá phải là số nguyên"),
    };
    let mut store = state.store.lock().unwrap();
    match store.get_mut(&id) {
        Some(p) => {
            p.name = name;
            p.price = price;
            Response::ok(format!("Đã cập nhật #{}", id))
        }
        None => Response::not_found(),
    }
}
// đăng ký trong build_router():
//     .route(Method::Put, "/products/{id}", Arc::new(handle_update))

#[test]
fn update_product() {
    let app = build_router().route(Method::Put, "/products/{id}", Arc::new(handle_update));
    let state = AppState::new();
    app.handle(request(Method::Post, "/products", "name=A;price=1"), &state);
    let put = |path: &str, body: &str| app.handle(request(Method::Put, path, body), &state);
    assert_eq!(put("/products/1", "name=B;price=2").status, 200);
    assert_eq!(put("/products/9", "name=B;price=2").status, 404);
    assert_eq!(put("/products/x", "name=B;price=2").status, 400);
    assert_eq!(put("/products/1", "price=2").status, 422);
    let r = app.handle(request(Method::Get, "/products/1", ""), &state);
    assert_eq!(r.body, "1:B:2");
}
```
</details>

**Bài tập 2 (Middleware ghi nhật ký)**
Trong Axum, middleware bọc quanh handler. Mô phỏng: viết hàm `with_logging(app, req, state)` gọi router rồi ghi lại `phương_thức đường_dẫn -> mã`. Đây là mẫu *Decorator* (hàm bậc cao, Chương 17).

<details>
<summary><b>Gợi ý</b></summary>

Middleware chính là *hàm bậc cao bọc handler* — nhận yêu cầu, làm gì đó trước, gọi handler, làm gì đó sau, trả phản hồi. Đo thời gian, ghi log, kiểm xác thực đều là middleware. Nhớ hàm `measure_exec_time` ở Chương 17.
</details>

<details>
<summary><b>Lời giải</b></summary>

```rust
fn method_name(m: Method) -> &'static str {
    match m {
        Method::Get => "GET",
        Method::Post => "POST",
        Method::Put => "PUT",
        Method::Delete => "DELETE",
    }
}

/// Middleware ghi nhật ký: làm gì đó TRƯỚC (ghi lại phương thức + đường dẫn,
/// vì `req` sắp bị chuyển vào router), gọi router, làm gì đó SAU (ghi mã trạng
/// thái), rồi trả nguyên phản hồi. Router và handler không hề biết mình bị bọc.
pub fn with_logging(
    app: &Router,
    req: Request,
    state: &AppState,
    log: &mut Vec<String>,
) -> Response {
    let line = format!("{} {}", method_name(req.method), req.path); // TRƯỚC
    let response = app.handle(req, state);
    log.push(format!("{} -> {}", line, response.status)); // SAU
    response
}

#[cfg(test)]
mod exercise_2 {
    use super::*;
    #[test]
    fn logging_middleware_records_every_request() {
        let app = build_router();
        let state = AppState::new();
        let mut log = Vec::new();
        let created = with_logging(
            &app,
            request(Method::Post, "/products", "name=A;price=1"),
            &state,
            &mut log,
        );
        assert_eq!(created.status, 201); // phản hồi đi qua NGUYÊN VẸN
        with_logging(
            &app,
            request(Method::Get, "/products/9", ""),
            &state,
            &mut log,
        );
        with_logging(
            &app,
            request(Method::Put, "/products/1", ""),
            &state,
            &mut log,
        );
        assert_eq!(
            log,
            [
                "POST /products -> 201",
                "GET /products/9 -> 404",
                "PUT /products/1 -> 405"
            ]
        );
    }
}
```

Chú ý thứ tự: phải chép `method`/`path` ra *trước* khi gọi `app.handle(req, ...)`, vì `req` bị chuyển quyền sở hữu vào router (dùng lại sau đó là `E0382`). Trong Axum, cùng ý tưởng này là `axum::middleware::from_fn` hoặc lớp `TraceLayer` của `tower-http` — một hàm bọc quanh dịch vụ bên trong, nhận yêu cầu, gọi `next.run(req).await`, rồi xem phản hồi. Đo thời gian (như `measure_exec_time` ở Chương 17) hay kiểm xác thực đều viết theo đúng khuôn này.
</details>

**Bài tập 3 (Tư duy: chọn mã trạng thái)**
Với mỗi tình huống, chọn mã HTTP đúng:
1. Người dùng gửi form đăng ký với email đã tồn tại.
2. Người dùng chưa đăng nhập gọi API cần đăng nhập.
3. Người dùng #1 cố xóa bài viết của người #2.
4. Cơ sở dữ liệu mất kết nối giữa chừng.

<details>
<summary><b>Lời giải tham khảo</b></summary>

1. **409 Conflict** (email trùng là xung đột trạng thái), hoặc 422 nếu coi là lỗi validation.
2. **401 Unauthorized** (chưa xác thực danh tính).
3. **403 Forbidden** (đã đăng nhập nhưng không đủ quyền — đây là IDOR ở Chương 57, đừng nhầm với 401).
4. **500 Internal Server Error** (lỗi phía máy chủ, không phải lỗi của client).

Quy tắc: **4xx là lỗi của client** (gửi sai), **5xx là lỗi của server** (xử lý hỏng). Phân biệt 401 (chưa xác thực) với 403 (đã xác thực, thiếu quyền) là câu hỏi phỏng vấn kinh điển.
</details>
