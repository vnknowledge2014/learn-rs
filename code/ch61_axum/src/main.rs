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
