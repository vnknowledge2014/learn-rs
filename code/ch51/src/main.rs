#![allow(dead_code, unused_variables, unused_imports)]
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Mô hình Thực thể Sản phẩm trong hệ thống
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductEntity {
    pub id: u64,
    pub name: String,
    pub price_cents: u64,
    pub in_stock: bool,
}

/// Trạng thái dùng chung toàn dịch vụ (Shared Application State)
pub struct SharedAppState {
    pub catalog: Mutex<HashMap<u64, ProductEntity>>,
}

impl SharedAppState {
    pub fn new() -> Self {
        let mut catalog = HashMap::new();
        catalog.insert(
            101,
            ProductEntity {
                id: 101,
                name: "Rust Masterclass Hardcover".to_string(),
                price_cents: 550_000,
                in_stock: true,
            },
        );
        catalog.insert(
            102,
            ProductEntity {
                id: 102,
                name: "Mechanical Keyboard 68-Key".to_string(),
                price_cents: 1_200_000,
                in_stock: false,
            },
        );
        Self {
            catalog: Mutex::new(catalog),
        }
    }
}

/// Mô phỏng Bộ mã hóa nhị phân Protocol Buffers chuẩn gRPC (gRPC Binary Wire Encoding)
pub struct ProtobufWireCodec;

impl ProtobufWireCodec {
    /// Mã hóa sản phẩm thành chuỗi byte nhị phân siêu nén
    /// Tag 1: ID (8B) | Tag 2: Price (8B) | Tag 3: InStock (1B) | Tag 4: Name (Length + Bytes)
    pub fn encode_product(product: &ProductEntity) -> Vec<u8> {
        let mut bytes = Vec::new();

        // Field 1: ID
        bytes.push(0x08); // Tag 1, Type: Varint
        bytes.extend_from_slice(&product.id.to_le_bytes());

        // Field 2: Price Cents
        bytes.push(0x10); // Tag 2, Type: Varint
        bytes.extend_from_slice(&product.price_cents.to_le_bytes());

        // Field 3: In Stock
        bytes.push(0x18); // Tag 3, Type: Varint
        bytes.push(if product.in_stock { 1 } else { 0 });

        // Field 4: Name String
        bytes.push(0x22); // Tag 4, Type: Length-delimited
        let name_bytes = product.name.as_bytes();
        bytes.push(name_bytes.len() as u8);
        bytes.extend_from_slice(name_bytes);

        bytes
    }

    /// Giải mã nhị phân không sao chép từ chuỗi byte gRPC
    pub fn decode_product(bytes: &[u8]) -> Result<ProductEntity, &'static str> {
        if bytes.len() < 20 {
            return Err("Kich thuoc byte protobuf qua ngan!");
        }

        let mut id = 0u64;
        let mut price = 0u64;
        let mut in_stock = false;
        let mut name = String::new();

        let mut idx = 0;
        while idx < bytes.len() {
            let tag = bytes[idx];
            idx += 1;

            match tag {
                0x08 => {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&bytes[idx..idx + 8]);
                    id = u64::from_le_bytes(b);
                    idx += 8;
                }
                0x10 => {
                    let mut b = [0u8; 8];
                    b.copy_from_slice(&bytes[idx..idx + 8]);
                    price = u64::from_le_bytes(b);
                    idx += 8;
                }
                0x18 => {
                    in_stock = bytes[idx] == 1;
                    idx += 1;
                }
                0x22 => {
                    let len = bytes[idx] as usize;
                    idx += 1;
                    name = String::from_utf8_lossy(&bytes[idx..idx + len]).to_string();
                    idx += len;
                }
                _ => break,
            }
        }

        Ok(ProductEntity {
            id,
            name,
            price_cents: price,
            in_stock,
        })
    }
}

/// Trình điều phối dịch vụ mô phỏng cách Axum Router định tuyến Type-Safe
pub struct TypeSafeServiceRouter {
    state: Arc<SharedAppState>,
}

impl TypeSafeServiceRouter {
    pub fn new(state: Arc<SharedAppState>) -> Self {
        Self { state }
    }

    /// Xử lý yêu cầu dạng REST/JSON
    pub fn handle_rest_get_product(&self, product_id: u64) -> Result<String, &'static str> {
        let catalog = self.state.catalog.lock().unwrap();
        if let Some(prod) = catalog.get(&product_id) {
            // Giả lập trả về chuỗi định dạng JSON
            Ok(format!(
                r#"{{"id":{},"name":"{}","price_cents":{},"in_stock":{}}}"#,
                prod.id, prod.name, prod.price_cents, prod.in_stock
            ))
        } else {
            Err("404 Not Found: Khong tim thay san pham")
        }
    }

    /// Xử lý yêu cầu dạng gRPC nhị phân siêu tốc
    pub fn handle_grpc_get_product(&self, product_id: u64) -> Result<Vec<u8>, &'static str> {
        let catalog = self.state.catalog.lock().unwrap();
        if let Some(prod) = catalog.get(&product_id) {
            // Trả về gói tin nhị phân Protobuf nén gọn
            Ok(ProtobufWireCodec::encode_product(prod))
        } else {
            Err("gRPC Status: NOT_FOUND (Code 5)")
        }
    }
}

fn main() {
    println!("==================================================================");
    println!("   DICH VU THONG LUONG CAO: AXUM REST & TONIC GRPC TOI UU RUST    ");
    println!("==================================================================");

    // 1. Khởi tạo trạng thái dùng chung được bọc trong con trỏ Arc
    let shared_state = Arc::new(SharedAppState::new());
    let router = TypeSafeServiceRouter::new(shared_state);

    // 2. Thử nghiệm gọi cổng REST API (JSON Payload)
    println!("\n[1] Xu ly qua cong REST API (JSON Text Format):");
    let rest_response = router.handle_rest_get_product(101).unwrap();
    println!("    - Payload REST JSON nhan duoc: {}", rest_response);
    println!("    - Dung luong payload JSON    : {} bytes", rest_response.len());

    // 3. Thử nghiệm gọi cổng gRPC (Protocol Buffers Binary Format)
    println!("\n[2] Xu ly qua cong gRPC concat bo (Protobuf Binary Format):");
    let grpc_binary = router.handle_grpc_get_product(101).unwrap();
    println!("    - Payload gRPC Binary nhan duoc (Hex): {:02X?}", grpc_binary);
    println!("    - Dung luong payload gRPC             : {} bytes", grpc_binary.len());

    // So sánh kích thước truyền tải
    let savings = ((rest_response.len() as f64 - grpc_binary.len() as f64)
        / rest_response.len() as f64)
        * 100.0;
    println!(
        "    ==> gRPC Protobuf tiet kiem duoc: {:.1}% bang thong mang!",
        savings
    );

    // 4. Giải mã ngược gói tin gRPC (Zero-Copy Validation)
    println!("\n[3] Phuc hoi thuc the tu goi tin nhi phan gRPC:");
    let decoded = ProtobufWireCodec::decode_product(&grpc_binary).unwrap();
    println!("    - ID San pham : {}", decoded.id);
    println!("    - Ten San pham: {}", decoded.name);
    println!("    - Gia tien    : {}d", decoded.price_cents);
    println!("    - Con hang    : {}", decoded.in_stock);
    assert_eq!(decoded.id, 101);

    println!("\n==================================================================");
    println!("   XAC NHAN: MO HINH HYBRID AXUM & TONIC SAN SANG VAN HANH!     ");
    println!("==================================================================");
}
