# Chương 61: Phát triển Backend Web với Axum — Định tuyến, Bộ trích xuất, Trạng thái & Xử lý lỗi (Backend Web Development)

## Giới thiệu & Mục tiêu học tập

Chương 51 đã giới thiệu Axum và gRPC ở mức tổng quan. Chương này đi sâu vào **xây dựng một dịch vụ web hoàn chỉnh**: định tuyến, trích xuất dữ liệu từ yêu cầu, quản lý trạng thái chia sẻ, xử lý lỗi, và — quan trọng nhất — **cách tổ chức mã để kiểm thử được mà không cần chạy server**.

[Axum](https://github.com/tokio-rs/axum) là framework web của nhóm Tokio, được yêu thích nhờ một triết lý: **dùng hệ thống kiểu của Rust làm giao diện**. Bạn khai báo handler nhận `Path<u32>`, `Json<T>`, `State<S>` — và Axum tự động trích xuất, kiểm tra, ép kiểu. Sai kiểu là lỗi biên dịch hoặc `422` tự động, không phải lỗi runtime mơ hồ.

Để bạn hiểu *cơ chế bên dưới* Axum, chương này xây một **mini-router** từ đầu — khớp phương thức + mẫu đường dẫn, trích tham số động, gọi handler — rồi mới chỉ ra mã Axum thật tương ứng. Mini-router chạy offline và có test đầy đủ, minh họa nguyên tắc vàng: **tách lõi nghiệp vụ thuần túy khỏi khung web**, để test không cần mạng.

Mục tiêu học tập:
- Hiểu vòng đời một yêu cầu HTTP: **định tuyến → trích xuất → xử lý → phản hồi**.
- Cài **bộ định tuyến** khớp mẫu đường dẫn động (`/san-pham/:id`).
- Quản lý **trạng thái chia sẻ** an toàn đa luồng bằng `Arc<Mutex<...>>` (Chương 27, 46).
- Xử lý lỗi bằng **mã trạng thái HTTP đúng ngữ nghĩa**: 200/201/404/422.
- Viết handler như **lõi thuần túy kiểm thử được** (Chương 20, 55) — test API không cần server.
- Đọc và viết mã **Axum thật**, hiểu bộ trích xuất (extractor) và `IntoResponse`.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│         HÌNH TƯỢNG: MỘT TÒA SOẠN BÁO NHẬN VÀ XỬ LÝ THƯ ĐỘC GIẢ                    │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│   Thư đến (YÊU CẦU HTTP)                                                          │
│        │                                                                         │
│        ▼                                                                         │
│   [1. NHÂN VIÊN PHÂN LOẠI]  = BỘ ĐỊNH TUYẾN (Router)                             │
│        Đọc địa chỉ trên phong bì ("mục Thể thao", "mục Kinh tế"):                │
│        GET /san-pham/7 → chuyển tới đúng biên tập viên phụ trách.                │
│        Không có mục nào khớp → trả lại "404 địa chỉ không tồn tại".              │
│        │                                                                         │
│        ▼                                                                         │
│   [2. THƯ KÝ BÓC THƯ]  = BỘ TRÍCH XUẤT (Extractor)                               │
│        Mở phong bì, lấy ra: số báo (:id=7), nội dung (JSON body).                │
│        Nội dung sai định dạng → "422 thư không đọc được".                        │
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

Một tuyến gồm ba phần: **phương thức** (GET/POST...), **mẫu đường dẫn** (có thể chứa tham số động `:id`), và **bộ xử lý**. Bộ định tuyến duyệt các tuyến, tìm cái đầu tiên khớp cả phương thức lẫn hình dạng đường dẫn.

Điểm tinh tế: `GET /san-pham/:id` khớp `/san-pham/7` và trích `id = "7"`, nhưng KHÔNG khớp `/san-pham` (khác số đoạn) hay `POST /san-pham/7` (khác phương thức). Cùng một đường dẫn với phương thức khác là *tuyến khác* — đó là cách REST phân biệt "xem" (GET) với "xóa" (DELETE) cùng một tài nguyên.

### 2. Bộ trích xuất (Extractor) — hệ thống kiểu làm giao diện

Đây là điều làm Axum khác biệt. Trong nhiều framework, bạn tự lấy dữ liệu từ đối tượng request thô rồi tự ép kiểu — dễ sai runtime. Axum đảo ngược: bạn **khai báo kiểu bạn muốn** trong chữ ký handler, và framework tự trích xuất:

```rust
async fn view(Path(id): Path<u32>, State(store): State<Arc<Store>>) -> impl IntoResponse
//           └── trích :id từ URL, ép sang u32   └── lấy trạng thái chia sẻ
```

Nếu URL có `:id` không phải số, Axum trả `400` tự động — handler của bạn *không bao giờ được gọi* với dữ liệu sai. Đây là "phân tích, đừng xác thực" (Chương 20) ở tầng HTTP: sau khi qua extractor, `id` đã là `u32` hợp lệ, không cần kiểm tra lại.

### 3. Trạng thái chia sẻ — an toàn đa luồng

Một máy chủ web xử lý nhiều yêu cầu *đồng thời* trên nhiều luồng (Chương 49). Trạng thái chung (kho sản phẩm, kết nối cơ sở dữ liệu) phải chia sẻ an toàn. Mẫu chuẩn: `Arc<Mutex<T>>` — `Arc` để nhiều luồng cùng sở hữu (Chương 27), `Mutex` để chỉ một luồng sửa tại một thời điểm (Chương 46).

> Trong sản phẩm thật, thay `Mutex<HashMap>` bằng một *pool kết nối cơ sở dữ liệu* (như `sqlx::PgPool`) — bản thân nó đã an toàn đa luồng, và bạn để cơ sở dữ liệu lo chuyện đồng thời (Chương 35, MVCC).

### 4. Xử lý lỗi — mã trạng thái đúng ngữ nghĩa

Mỗi loại lỗi có một mã HTTP đúng:

| Tình huống | Mã | Ý nghĩa |
|---|---|---|
| Thành công | 200 | OK |
| Tạo mới thành công | 201 | Created |
| Không tìm thấy tài nguyên | 404 | Not Found |
| Dữ liệu client gửi sai | 422 / 400 | Unprocessable / Bad Request |
| Chưa đăng nhập | 401 | Unauthorized |
| Không đủ quyền | 403 | Forbidden (nhớ IDOR ở Chương 57!) |
| Lỗi phía máy chủ | 500 | Internal Server Error |

Trong Axum, bạn định nghĩa một kiểu lỗi và cài `IntoResponse` cho nó — mỗi biến thể lỗi ánh xạ tới một mã. Đây chính là *Bifunctor* của `Result` (Chương 19): `map` cho nhánh thành công, `map_err` cho nhánh lỗi thành mã HTTP.

### 5. Vì sao tách lõi thuần túy khỏi khung web

Mã trong chương này để toàn bộ logic nghiệp vụ trong các hàm `xu_ly_*` **thuần túy** (nhận yêu cầu + trạng thái, trả phản hồi), và bộ định tuyến chỉ là lớp mỏng điều phối. Nhờ vậy:
- **Test không cần server**: gọi `app.handle(yeu_cau, state)` trực tiếp và kiểm phản hồi (xem module test).
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
    let field = analyze_than(&yc.than);
    let name = match field.get("ten") {
        Some(t) if !t.is_empty() => t.clone(),
        _ => return Response::data_sai("thiếu tên sản phẩm"),
    };
    let price: u64 = match field.get("gia").and_then(|g| g.parse().ok()) {
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
```

---

## Chuyển sang Axum thật

Cùng một API viết bằng Axum thật chỉ khác ở *lớp vỏ*. Đây là mã Axum tương đương (cần các crate `axum`, `tokio`, `serde` trong `Cargo.toml`):

```rust
// Cargo.toml:
//   axum = "0.8"
//   tokio = { version = "1", features = ["full"] }
//   serde = { version = "1", features = ["derive"] }
//   serde_json = "1"

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize)]
struct SanPham { id: u64, name: String, price: u64 }

#[derive(Deserialize)]
struct TaoSanPham { name: String, price: u64 }

struct State {
    store: Mutex<HashMap<u64, SanPham>>,
    next_id: Mutex<u64>,
}

// Handler nhận State và Json ĐÃ ĐƯỢC TRÍCH XUẤT + KIỂM KIỂU tự động.
async fn tao(
    State(tt): State<Arc<State>>,
    Json(input): Json<TaoSanPham>,          // JSON sai -> Axum tự trả 422
) -> (StatusCode, Json<SanPham>) {
    let mut id_ke = tt.next_id.lock().unwrap();
    let id = *id_ke; *id_ke += 1;
    let sp = SanPham { id, name: input.name, price: input.price };
    tt.store.lock().unwrap().insert(id, sp.clone());
    (StatusCode::CREATED, Json(sp))
}

async fn view(
    State(tt): State<Arc<State>>,
    Path(id): Path<u64>,                      // :id không phải số -> Axum tự trả 400
) -> Result<Json<SanPham>, StatusCode> {
    tt.store.lock().unwrap().get(&id)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)          // map_err = Bifunctor (Chương 19)
}

#[tokio::main]
async fn main() {
    let tt = Arc::new(State {
        store: Mutex::new(HashMap::new()),
        next_id: Mutex::new(1),
    });

    let app = Router::new()
        .route("/san-pham", post(tao))
        .route("/san-pham/{id}", get(view))
        .with_state(tt);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

So sánh mã Axum với mini-router: **cùng một kiến trúc** — định tuyến, trích xuất, trạng thái chia sẻ, mã lỗi. Axum chỉ thêm phần bất đồng bộ (`async`/`await`, Chương 49) và tự động hóa việc trích xuất/serialize. Hiểu mini-router là hiểu Axum.

> **Kiểm thử Axum thật**: dùng `tower::ServiceExt::oneshot` để gửi yêu cầu giả vào router mà **không mở cổng mạng** — đúng tầng integration test ở Chương 55. Toàn bộ triết lý "test không cần server" vẫn áp dụng.

---

## Bảng tra cứu lỗi biên dịch thường gặp

| Lỗi | Nguyên nhân trong chương này | Cách sửa |
|---|---|---|
| `E0308: expected Response, found ()` | Nhánh xử lý quên trả về giá trị | Mọi nhánh `match` định tuyến phải trả `Response`; bỏ dấu `;` ở biểu thức cuối |
| `E0596: cannot borrow data in an Arc as mutable` | Sửa trạng thái dùng chung qua `Arc` | `Arc<Mutex<T>>`, rồi `.lock().unwrap()` trước khi ghi |
| `E0597: borrowed value does not live long enough` | Trả tham chiếu tới dữ liệu bên trong khoá | Sao chép ra khỏi vùng khoá rồi mới trả; đừng để `MutexGuard` thoát ra ngoài |
| `E0382: use of moved value` | Dùng lại `Request` sau khi đã chuyển vào bộ xử lý | Truyền `&Request`, hoặc `clone()` nếu thật cần sở hữu |
| Định tuyến trả 404 cho đường dẫn đúng | So khớp trước khi tách tham số động | Tách đoạn đường dẫn rồi mới so; xem `RouteMatcher` |

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Vòng đời yêu cầu**: định tuyến → trích xuất → xử lý → phản hồi. Mỗi tầng có một trách nhiệm.
2. **Axum dùng hệ thống kiểu làm giao diện.** `Path<u32>`, `Json<T>` tự trích xuất và kiểm kiểu — sai kiểu là 400/422 tự động, không phải lỗi runtime.
3. **Trạng thái chia sẻ dùng `Arc<Mutex<T>>`** (hoặc pool cơ sở dữ liệu). Mã lỗi phải đúng ngữ nghĩa HTTP.
4. **Tách lõi nghiệp vụ khỏi khung web** để test không cần server. Đổi framework không đổi logic — đây là "lõi thuần túy, vỏ mệnh lệnh" ở tầng web.

### Bài tập rèn luyện tự giải:

**Bài tập 1 (Thêm tuyến PUT cập nhật)**
Thêm handler `xu_ly_cap_nhat` cho `PUT /san-pham/:id` cập nhật tên và giá. Trả 404 nếu không tồn tại, 422 nếu dữ liệu sai. Viết test.

<details>
<summary><b>Lời giải</b></summary>

```rust
pub fn handle_update(yc: &Request, tt: &State) -> Response {
    let id: u64 = match yc.path_param.get("id").and_then(|s| s.parse().ok()) {
        Some(x) => x, None => return Response::data_sai("id không hợp lệ"),
    };
    let field = analyze_than(&yc.than);
    let price: u64 = match field.get("gia").and_then(|g| g.parse().ok()) {
        Some(g) => g, None => return Response::data_sai("giá phải là số"),
    };
    let name = match field.get("ten") { Some(t) if !t.is_empty() => t.clone(),
        _ => return Response::data_sai("thiếu tên") };
    let mut store = tt.store.lock().unwrap();
    match store.get_mut(&id) {
        Some(sp) => { sp.name = name; sp.price = price; Response::ok(format!("Đã cập nhật #{}", id)) }
        None => Response::not_seen(),
    }
}
// đăng ký: .them(Method::PUT, "/san-pham/:id", Arc::new(handle_update))
```
</details>

**Bài tập 2 (Middleware ghi nhật ký)**
Trong Axum, middleware bọc quanh handler. Mô phỏng: viết hàm `voi_nhat_ky(app, yc, tt)` gọi router rồi ghi lại `phương_thức đường_dẫn -> mã`. Đây là mẫu *Decorator* (hàm bậc cao, Chương 17).

<details>
<summary><b>Gợi ý</b></summary>

Middleware chính là *hàm bậc cao bọc handler* — nhận yêu cầu, làm gì đó trước, gọi handler, làm gì đó sau, trả phản hồi. Đo thời gian, ghi log, kiểm xác thực đều là middleware. Nhớ hàm `measure_exec_time` ở Chương 17.
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
