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

impl Default for SharedAppState {
    fn default() -> Self {
        Self::new()
    }
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

/// Lỗi giải mã gói tin nhị phân
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    Truncated,     // gói bị cắt cụt giữa chừng
    VarintTooLong, // varint dài quá 10 byte
    UnsupportedWireType(u8),
    InvalidUtf8,
}

/// Bộ mã hóa nhị phân theo ĐÚNG định dạng dây (wire format) của Protocol Buffers
/// cho thông điệp:
///   message Product { uint64 id = 1; uint64 price_cents = 2; bool in_stock = 3; string name = 4; }
/// Mỗi trường là cặp (khóa, giá trị); khóa = (số_trường << 3) | kiểu_dây, cũng mã bằng varint.
pub struct ProtobufWireCodec;

impl ProtobufWireCodec {
    const WIRE_VARINT: u8 = 0;
    const WIRE_LEN: u8 = 2;

    /// Varint: mỗi byte chứa 7 bit dữ liệu, bit cao = "còn byte nữa".
    /// Số nhỏ tốn ít byte: 101 -> 1 byte, 550_000 -> 3 byte (thay vì 8 byte cố định).
    fn put_varint(mut n: u64, out: &mut Vec<u8>) {
        while n >= 0x80 {
            out.push((n as u8) | 0x80);
            n >>= 7;
        }
        out.push(n as u8);
    }

    fn get_varint(bytes: &[u8], pos: &mut usize) -> Result<u64, DecodeError> {
        let mut result = 0u64;
        for shift in (0..70).step_by(7) {
            let byte = *bytes.get(*pos).ok_or(DecodeError::Truncated)?;
            *pos += 1;
            result |= u64::from(byte & 0x7F) << shift;
            if byte & 0x80 == 0 {
                return Ok(result);
            }
        }
        Err(DecodeError::VarintTooLong)
    }

    fn put_key(field: u64, wire_type: u8, out: &mut Vec<u8>) {
        Self::put_varint((field << 3) | u64::from(wire_type), out);
    }

    /// Mã hóa sản phẩm thành chuỗi byte nhị phân
    pub fn encode_product(product: &ProductEntity) -> Vec<u8> {
        let mut bytes = Vec::new();

        Self::put_key(1, Self::WIRE_VARINT, &mut bytes); // 0x08
        Self::put_varint(product.id, &mut bytes);

        Self::put_key(2, Self::WIRE_VARINT, &mut bytes); // 0x10
        Self::put_varint(product.price_cents, &mut bytes);

        Self::put_key(3, Self::WIRE_VARINT, &mut bytes); // 0x18
        Self::put_varint(u64::from(product.in_stock), &mut bytes);

        // Chuỗi: kiểu "length-delimited" = độ dài (varint, không giới hạn 255) + các byte
        Self::put_key(4, Self::WIRE_LEN, &mut bytes); // 0x22
        Self::put_varint(product.name.len() as u64, &mut bytes);
        bytes.extend_from_slice(product.name.as_bytes());

        bytes
    }

    /// Giải mã gói tin. Mọi chỗ đọc đều kiểm biên: gói hỏng/cắt cụt trả `Err`,
    /// KHÔNG bao giờ panic (dữ liệu từ mạng là dữ liệu không tin được).
    /// Trường lạ được bỏ qua — đúng tinh thần tương thích xuôi/ngược của Protobuf.
    pub fn decode_product(bytes: &[u8]) -> Result<ProductEntity, DecodeError> {
        let mut product = ProductEntity {
            id: 0,
            name: String::new(),
            price_cents: 0,
            in_stock: false,
        };

        let mut pos = 0;
        while pos < bytes.len() {
            let key = Self::get_varint(bytes, &mut pos)?;
            let (field, wire_type) = (key >> 3, (key & 0x7) as u8);
            match wire_type {
                Self::WIRE_VARINT => {
                    let value = Self::get_varint(bytes, &mut pos)?;
                    match field {
                        1 => product.id = value,
                        2 => product.price_cents = value,
                        3 => product.in_stock = value != 0,
                        _ => {} // trường lạ: bỏ qua
                    }
                }
                Self::WIRE_LEN => {
                    let len = Self::get_varint(bytes, &mut pos)? as usize;
                    let end = pos.checked_add(len).ok_or(DecodeError::Truncated)?;
                    let payload = bytes.get(pos..end).ok_or(DecodeError::Truncated)?;
                    pos = end;
                    if field == 4 {
                        product.name = std::str::from_utf8(payload)
                            .map_err(|_| DecodeError::InvalidUtf8)?
                            .to_string();
                    }
                }
                other => return Err(DecodeError::UnsupportedWireType(other)),
            }
        }

        Ok(product)
    }
}

/// Thoát ký tự đặc biệt khi nhúng chuỗi vào JSON (nếu không, tên chứa `"` sẽ phá JSON)
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
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
            // Giả lập trả về chuỗi định dạng JSON (thực tế: serde_json)
            Ok(format!(
                r#"{{"id":{},"name":"{}","price_cents":{},"in_stock":{}}}"#,
                prod.id,
                json_escape(&prod.name),
                prod.price_cents,
                prod.in_stock
            ))
        } else {
            Err("404 Not Found: Không tìm thấy sản phẩm")
        }
    }

    /// Xử lý yêu cầu dạng gRPC nhị phân
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
    println!("   DỊCH VỤ THÔNG LƯỢNG CAO: AXUM REST & TONIC GRPC TỐI ƯU RUST    ");
    println!("==================================================================");

    // 1. Khởi tạo trạng thái dùng chung được bọc trong con trỏ Arc
    let shared_state = Arc::new(SharedAppState::new());
    let router = TypeSafeServiceRouter::new(shared_state);

    // 2. Thử nghiệm gọi cổng REST API (JSON Payload)
    println!("\n[1] Xử lý qua cổng REST API (JSON Text Format):");
    let rest_response = router.handle_rest_get_product(101).unwrap();
    println!("    - Payload REST JSON nhận được: {}", rest_response);
    println!(
        "    - Dung lượng payload JSON    : {} bytes",
        rest_response.len()
    );

    // 3. Thử nghiệm gọi cổng gRPC (Protocol Buffers Binary Format)
    println!("\n[2] Xử lý qua cổng gRPC nội bộ (Protobuf Binary Format):");
    let grpc_binary = router.handle_grpc_get_product(101).unwrap();
    println!(
        "    - Payload gRPC Binary nhận được (Hex): {:02X?}",
        grpc_binary
    );
    println!(
        "    - Dung lượng payload gRPC             : {} bytes",
        grpc_binary.len()
    );

    // So sánh kích thước truyền tải
    let savings = ((rest_response.len() as f64 - grpc_binary.len() as f64)
        / rest_response.len() as f64)
        * 100.0;
    println!(
        "    ==> Protobuf nhỏ hơn JSON {:.1}% với bản ghi này (tên trường không đi theo gói tin)",
        savings
    );

    // 4. Giải mã ngược gói tin gRPC
    println!("\n[3] Phục hồi thực thể từ gói tin nhị phân gRPC:");
    let decoded = ProtobufWireCodec::decode_product(&grpc_binary).unwrap();
    println!("    - ID sản phẩm : {}", decoded.id);
    println!("    - Tên sản phẩm: {}", decoded.name);
    println!("    - Giá tiền    : {}đ", decoded.price_cents);
    println!("    - Còn hàng    : {}", decoded.in_stock);
    assert_eq!(decoded.id, 101);

    // 5. Gói tin hỏng không làm sập dịch vụ
    let truncated = &grpc_binary[..grpc_binary.len() - 3];
    println!(
        "\n[4] Gói tin bị cắt cụt -> {:?}",
        ProtobufWireCodec::decode_product(truncated)
    );

    println!("\n==================================================================");
    println!("   XÁC NHẬN: MÔ HÌNH HYBRID AXUM & TONIC SẴN SÀNG VẬN HÀNH!     ");
    println!("==================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(name: &str) -> ProductEntity {
        ProductEntity {
            id: 150,
            name: name.to_string(),
            price_cents: 550_000,
            in_stock: true,
        }
    }

    #[test]
    fn matches_official_protobuf_bytes() {
        // Ví dụ kinh điển trong tài liệu Protobuf: trường 1 = 150 -> 08 96 01
        let bytes = ProtobufWireCodec::encode_product(&sample("ab"));
        assert_eq!(&bytes[..3], &[0x08, 0x96, 0x01]);
        assert_eq!(&bytes[bytes.len() - 4..], &[0x22, 0x02, b'a', b'b']);
    }

    #[test]
    fn roundtrip_including_long_names() {
        // Tên > 255 byte: bản cũ ghi độ dài bằng 1 byte `len as u8` nên hỏng gói tin
        for name in ["Bàn phím cơ", &"x".repeat(300)] {
            let p = sample(name);
            let bytes = ProtobufWireCodec::encode_product(&p);
            assert_eq!(ProtobufWireCodec::decode_product(&bytes), Ok(p));
        }
    }

    #[test]
    fn corrupted_input_is_an_error_not_a_panic() {
        let bytes = ProtobufWireCodec::encode_product(&sample("hello"));
        for cut in 0..bytes.len() {
            // Mọi tiền tố đều không được panic
            let _ = ProtobufWireCodec::decode_product(&bytes[..cut]);
        }
        assert_eq!(
            ProtobufWireCodec::decode_product(&bytes[..bytes.len() - 1]),
            Err(DecodeError::Truncated)
        );
        assert_eq!(
            ProtobufWireCodec::decode_product(&[0x0D]), // kiểu dây 5 (fixed32)
            Err(DecodeError::UnsupportedWireType(5))
        );
    }

    #[test]
    fn json_name_is_escaped() {
        let state = Arc::new(SharedAppState::new());
        state
            .catalog
            .lock()
            .unwrap()
            .insert(7, sample(r#"Sách "Rust""#));
        let router = TypeSafeServiceRouter::new(state);
        let json = router.handle_rest_get_product(7).unwrap();
        assert!(json.contains(r#""name":"Sách \"Rust\"""#));
    }
}
