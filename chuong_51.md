# Chương 51: Dịch vụ REST & gRPC thông lượng cao với Axum & Tonic (High-Throughput REST & gRPC Services with Axum & Tonic)

## Giới thiệu & Mục tiêu học tập

Trong kỷ nguyên của các ứng dụng quy mô toàn cầu, một hệ thống backend không chỉ đơn thuần là nhận một yêu cầu và trả về kết quả. Một máy chủ hiện đại phải xử lý từ hàng chục ngàn đến hàng triệu yêu cầu mỗi giây (RPS - Requests Per Second) với độ trễ phản hồi tính bằng mili-giây.

Để đạt được kỳ tích này, hệ sinh thái Rust đã sản sinh ra hai "vũ khí tối thượng" định hình lại tiêu chuẩn của ngành công nghiệp:
1. **Axum**: Web framework thế hệ mới được bảo trợ chính thức bởi đội ngũ phát triển Tokio, xây dựng trên nền tảng trừu tượng hóa cực mạnh của thư viện `Tower`. Axum mang tới sự kết hợp hoàn hảo giữa độ an toàn kiểu dữ liệu tuyệt đối (Type-Safe Routing) và tốc độ phục vụ REST API thuộc top đầu thế giới.
2. **Tonic**: Hiện thực hóa chuẩn mực giao thức **gRPC** (Google Remote Procedure Call) trên nền HTTP/2 và định dạng nhị phân Protocol Buffers (Protobuf). gRPC với Tonic là "huyết mạch" kết nối siêu tốc giữa các microservice nội bộ, giúp tăng thông lượng truyền tải từ 7 đến 10 lần so với chuẩn REST/JSON truyền thống.

Mục tiêu học tập của bạn:
- Nắm vững kiến trúc cốt lõi của Axum: Bộ định tuyến Router, các Bộ trích xuất (Extractor) dữ liệu an toàn (Extractors: `Json`, `Path`, `State`), và tầng Middleware với Tower.
- Hiểu thấu cơ chế hoạt động của gRPC và Protocol Buffers: Đa dồn kênh nhiều luồng (Multiplexing) trên 1 kết nối TCP duy nhất của HTTP/2, và sự vượt trội của định dạng nhị phân so với văn bản JSON.
- Xây dựng mô hình kiến trúc lai (Hybrid Architecture): Cổng ngoài đón khách (API Gateway) dùng Axum REST/JSON, trong khi mạng nội bộ giao tiếp bằng gRPC Tonic siêu tốc.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu tại sao các tập đoàn lớn lại chuyển từ REST/JSON sang gRPC/Protobuf trong giao tiếp nội bộ, hãy quan sát trạm thu phí cao tốc:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│              HÌNH TƯỢNG HÓA: LÀN THU PHÍ TIỀN MẶT VS LÀN THU PHÍ TỰ ĐỘNG VETC    │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. REST API VỚI JSON: LÀN THU PHÍ DỪNG XE ĐẾM TIỀN MẶT]                         │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Tài xế dừng hẳn xe lại ──► Kéo cửa kính xuống:                       │         │
│ │ - Đưa tờ giấy viết tay dài dòng bằng tiếng Việt (Chuỗi văn bản JSON).│         │
│ │ - Nhân viên trạm soi đèn pin đọc từng chữ, đếm tiền thối lại...      │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│   ===> Tốn 30 giây mỗi xe, hàng trăm ô tô nối đuôi nhau ùn tắc kéo dài!          │
│                                                                                  │
│ [2. gRPC VỚI PROTOCOL BUFFERS: LÀN THU PHÍ TỰ ĐỘNG KHÔNG DỪNG ETC (HTTP/2)]      │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Xe dán tem mã vạch thông minh (Định dạng nhị phân Protobuf siêu nén).│         │
│ │ Xe phóng qua trạm với tốc độ 80km/h:                                 │         │
│ │ - Máy quét laser rọi qua trong 1 phần nghìn giây (Zero-Copy Parse).  │         │
│ │ - Barie tự động nâng lên, tiền tự động trừ trong tích tắc!           │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ HTTP/2 Multiplexing: 10 làn xe chạy song song trên cùng 1 cây cầu!   │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│   ===> THÔNG LƯỢNG GẤP 10 LẦN, XE QUA TRẠM VÙN VỤT KHÔNG HỀ CÓ ĐỘ TRỄ!          │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Viết thư tay tiếng Việt (REST API với JSON)
- JSON rất thân thiện với con người: Bạn mở tệp tin ra là đọc được ngay: `{"ten": "Alice", "tuoi": 25}`.
- Nhưng đối với máy tính, việc phân tích (parse) chuỗi JSON là một cực hình: Máy tính phải quét từng ký tự xem dấu ngoặc kép ở đâu, dấu hai chấm ở đâu, chuyển chuỗi `"25"` thành số nguyên 4 bytes. Nó giống như người thủ kho phải đọc bức thư dài dòng mới biết cần xuất bao nhiêu bao gạo.

### 2. Mã Morse của thuyền trưởng (gRPC với Protocol Buffers)
- Protobuf loại bỏ toàn bộ các ký tự rườm rà (dấu ngoặc, tên trường). Thay vào đó, nó gán mỗi trường một mã số nhị phân siêu ngắn (Field Tag).
- Trường `ten` là mã số 1, `tuoi` là mã số 2. Dữ liệu được nén thành chuỗi byte nhị phân ngắn bằng $1/5$ chuỗi JSON.
- Khi máy tính nhận được gói tin, nó chỉ việc nhảy thẳng tới vị trí byte đó và đọc giá trị tức thì, không cần dò tìm ký tự. Kết hợp với đường cao tốc HTTP/2 (cho phép gửi hàng trăm yêu cầu cùng lúc trên 1 sợi cáp mạng duy nhất mà không bị nghẽn đầu làn), gRPC mang lại tốc độ không đối thủ!

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Kiến trúc Bộ Trích Xuất (Extractors Architecture) trong Axum

Khác với các web framework truyền thống trong Python hay JavaScript (nơi lập trình viên phải tự lấy dữ liệu từ `req.body`, `req.params` rồi tự ép kiểu dễ sinh lỗi runtime), Axum vận hành hoàn toàn dựa trên hệ thống kiểm tra kiểu dữ liệu tĩnh của Rust:
- Mọi tham số truyền vào hàm xử lý (Handler) đều phải triển khai trait `FromRequest` hoặc `FromRequestParts`:
```rust
// Axum tự động xác thực và giải mã kiểu dữ liệu ngay từ chữ ký hàm!
async fn tao_san_pham(
    State(app_state): State<Arc<AppState>>, // Trích xuất trạng thái chia sẻ
    Path(category_id): Path<u32>,          // Trích xuất tham số trên URL
    Json(payload): Json<CreateProductReq>, // Tự động kiểm tra cú pháp và parse JSON body
) -> Result<Json<ProductResponse>, AppError> { ... }
```
- Nếu client gửi lên một chuỗi JSON sai kiểu dữ liệu (ví dụ trường `price` cần số nguyên nhưng client gửi chuỗi ký tự), Axum sẽ tự động từ chối yêu cầu với mã lỗi `422 Unprocessable Entity` ngay lập tức mà hàm handler của bạn không hề bị gọi, bảo vệ hệ thống tuyệt đối!

### 2. Sự Tiến hóa từ HTTP/1.1 lên HTTP/2 trong gRPC

```
HTTP/1.1 (Tuần tự - Head-of-Line Blocking):
Kết nối TCP: [Yêu cầu 1 ──►] [Đợi Phản hồi 1 ◄──] [Yêu cầu 2 ──►] ...

HTTP/2 trong gRPC (Đa dồn kênh - Multiplexing):
Kết nối TCP: ──[Stream 1: Req]──[Stream 2: Req]──[Stream 1: Res]──[Stream 3: Req]──►
```
- **HTTP/1.1**: Mỗi yêu cầu phải chờ yêu cầu trước đó nhận được phản hồi xong mới được gửi tiếp trên cùng 1 kết nối (hiện tượng Head-of-Line Blocking). Để gửi nhiều yêu cầu, trình duyệt phải mở từ 6 đến 8 kết nối TCP song song, gây lãng phí bộ đệm (buffer) và bắt tay TCP tốn kém.
- **HTTP/2**: Toàn bộ các cuộc gọi RPC đều được phân chia thành các khung nhị phân (Binary Frames) có đánh số Stream ID, cùng lúc bay trên **duy nhất 1 kết nối TCP**. Dịch vụ A có thể gửi 10,000 lệnh gọi tới Dịch vụ B đồng thời mà không hề bị nghẽn!

### 3. Mô hình Kiến trúc Lai Hiện đại (Hybrid Architecture)

Trong các tập đoàn công nghệ lớn:
- **Cổng API Gateway phía ngoài (Public Gateway)**: Sử dụng **Axum** đón các yêu cầu từ Web Browser và Mobile App bằng chuẩn REST/JSON thân thiện.
- **Mạng lưới Dịch vụ nội bộ (Internal Service Mesh)**: Toàn bộ việc trao đổi giữa Service A, Service B, Service C được thực hiện qua **gRPC Tonic** nhị phân siêu tốc, giúp giảm tới 80% độ trễ mạng nội bộ.

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là mã nguồn Rust hoàn chỉnh hiện thực hóa một **Dịch vụ Điều phối API Thông lượng cao (High-Throughput API Gateway Dispatcher)**: Tự tay cài đặt cơ chế định tuyến Type-Safe theo triết lý của Axum, tích hợp chia sẻ trạng thái an toàn đa luồng `Arc`, kết hợp bộ mã hóa nhị phân mô phỏng chuẩn gRPC Protocol Buffers siêu tốc:

```rust
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

/// Trạng thái dùng shared toàn dịch vụ (Shared Application State)
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
            return Err("Kich thuoc byte protobuf qua short!");
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
            Err("404 Not Found: Low tim thay san pham")
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

    // 1. Khởi tạo trạng thái dùng shared được bọc trong con trỏ Arc
    let shared_state = Arc::new(SharedAppState::new());
    let router = TypeSafeServiceRouter::new(shared_state);

    // 2. Thử nghiệm gọi cổng REST API (JSON Payload)
    println!("\n[1] Xu ly qua cong REST API (JSON Text Format):");
    let rest_response = router.handle_rest_get_product(101).unwrap();
    println!("    - Payload REST JSON nhan duoc: {}", rest_response);
    println!("    - Dung luong payload JSON    : {} bytes", rest_response.len());

    // 3. Thử nghiệm gọi cổng gRPC (Protocol Buffers Binary Format)
    println!("\n[2] Xu ly qua cong gRPC noi bo (Protobuf Binary Format):");
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
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi lập trình REST API và gRPC với Axum và Tonic trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0277** | `the trait 'FromRequest' is not implemented for 'MyCustomType'` | Sử dụng một kiểu dữ liệu tùy chỉnh làm tham số trong hàm handler của Axum mà chưa triển khai trait trích xuất. | Bọc kiểu dữ liệu trong `Json<MyCustomType>` hoặc tự viết `impl<S> FromRequest<S> for MyCustomType`. |
| **E0599** | `no method named 'into_response' found for type 'MyError'` | Hàm handler trả về một kiểu lỗi tùy chỉnh chưa triển khai trait `IntoResponse` của Axum. | Triển khai trait `IntoResponse` để quy định mã HTTP Status và thông báo JSON trả về khi có lỗi. |
| **E0277** | `the trait 'Send' is not implemented for 'AppState'` | Trạng thái chia sẻ `State(state)` chứa các cấu trúc không an toàn đa luồng. | Đảm bảo `AppState` chỉ chứa các trường thỏa mãn ràng buộc `Send + Sync`, dùng `Mutex` hoặc `RwLock`. |
| **E0382** | `use of moved value: 'payload'` | Bạn di chuyển quyền sở hữu (ownership) của `payload` nhiều lần bên trong hàm xử lý. | Sử dụng tham chiếu mượn (borrow) hoặc tạo bản sao độc lập trước khi tái sử dụng. |

### Ví dụ phân tích lỗi `E0599` khi thiếu triển khai IntoResponse:

```rust
// Đoạn mã lỗi minh họa E0599:
struct SystemError {
    greeting: String,
}

// Hàm handler trả về SystemError nhưng chưa có IntoResponse
// async fn error_handler() -> Result<&'static str, SystemError> {
//     Err(SystemError { greeting: "Lỗi nội bộ".into() }) // LỖI E0599!
// }

// Cách sửa chữa đúng chuẩn: Tự quy định cách chuyển đổi sang HTTP Response
struct StdError {
    chi_tiet: &'static str,
}

impl StdError {
    fn to_http_status(&self) -> (u16, &'static str) {
        (500, self.chi_tiet)
    }
}

fn check_error() {
    let err = StdError { chi_tiet: "Lỗi kết nối database" };
    let (code, msg) = err.to_http_status();
    println!("Mã lỗi HTTP: {} - Nội dung: {}", code, msg);
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Sức mạnh Type-Safe của Axum**: Tận dụng triệt để hệ thống trích xuất (Extractors) để loại bỏ toàn bộ lỗi ép kiểu dữ liệu ngay từ cổng vào API.
2. **Ưu thế tuyệt đối của gRPC & Tonic**: Hoạt động trên HTTP/2 Multiplexing với định dạng nhị phân Protocol Buffers, tiết kiệm băng thông và tăng tốc độ xử lý gấp 7-10 lần so với JSON.
3. **Kiến trúc Lai (Hybrid Architecture)**: Sử dụng Axum RESTful cho giao diện công cộng bên ngoài và Tonic gRPC cho hệ thống giao tiếp vi dịch vụ nội bộ.
4. **Tối ưu hóa Băng thông & Bộ nhớ**: Kết hợp hài hòa giữa quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) để bảo đảm thông lượng tối đa mà không gây rò rỉ bộ nhớ.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Bổ sung Trường Timestamp và Checksum vào Protobuf)**:  
   Mở rộng `ProtobufWireCodec` thêm trường số 5 chứa dấu mốc thời gian `created_at: u64` và mã kiểm tra tính toàn vẹn CRC32. Cập nhật hàm giải mã để tự động kiểm tra xem gói tin có bị can thiệp trên đường truyền hay không.
2. **Bài tập 2 (Xây dựng Middleware Giới hạn Tần suất - Rate-Limiting Tower Layer)**:  
   Thiết kế một lớp trung gian Middleware đếm số lượng yêu cầu của một Client IP. Nếu client gửi quá 100 yêu cầu trong vòng 1 giây, lập tức trả về mã lỗi HTTP `429 Too Many Requests`.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Tại sao gRPC chưa thay thế hoàn toàn REST?)**:  
   Mặc dù gRPC vượt trội hoàn toàn về mặt tốc độ, tại sao các tập đoàn công nghệ lớn vẫn duy trì cổng REST/JSON cho người dùng đầu cuối (Client-facing)? Hãy phân tích các khía cạnh về: Tính tương thích của trình duyệt web (Browser Compatibility), khả năng debug thủ công qua `curl`, và tính thân thiện với các nhà phát triển thứ ba.

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Protobuf mã hóa mỗi trường là cặp (thẻ, giá trị), số nguyên dùng varint. Thêm trường 5 = timestamp, rồi bọc CRC32 quanh toàn gói để bên nhận phát hiện gói bị sửa.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
/// Mã hóa varint (số nguyên độ dài thay đổi) — nền tảng của Protobuf.
/// Mỗi byte giữ 7 bit dữ liệu + 1 bit "còn nữa" ở đầu.
fn encode_varint(mut n: u64, out: &mut Vec<u8>) {
    loop {
        let mut byte = (n & 0x7F) as u8;
        n >>= 7;
        if n != 0 { byte |= 0x80; } // bit cao = "còn byte nữa"
        out.push(byte);
        if n == 0 { break; }
    }
}
fn decode_varint(data: &[u8], pos: &mut usize) -> Option<u64> {
    let (mut result, mut shift) = (0u64, 0);
    while *pos < data.len() {
        let byte = data[*pos]; *pos += 1;
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 { return Some(result); } // hết chuỗi varint
        shift += 7;
    }
    None
}

/// CRC32 (đa thức IEEE) để kiểm gói có bị sửa trên đường truyền không.
fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// Đóng gói: trường 5 (created_at) + CRC32 bọc quanh toàn bộ phần dữ liệu.
fn encode_with_timestamp_crc(created_at: u64) -> Vec<u8> {
    let mut body = Vec::new();
    encode_varint(5 << 3, &mut body);      // thẻ trường 5, kiểu varint
    encode_varint(created_at, &mut body);  // giá trị
    let checksum = crc32(&body);
    let mut packet = body;
    packet.extend_from_slice(&checksum.to_le_bytes()); // 4 byte CRC ở cuối
    packet
}

/// Giải mã + KIỂM CRC: Err nếu gói bị can thiệp trên đường truyền.
fn decode_and_verify(packet: &[u8]) -> Result<u64, &'static str> {
    if packet.len() < 4 { return Err("gói quá ngắn"); }
    let (body, crc_bytes) = packet.split_at(packet.len() - 4);
    let stored = u32::from_le_bytes(crc_bytes.try_into().unwrap());
    if crc32(body) != stored {
        return Err("CRC không khớp — gói tin đã bị sửa trên đường truyền!");
    }
    let mut pos = 0;
    let _tag = decode_varint(body, &mut pos).ok_or("thiếu thẻ")?;
    decode_varint(body, &mut pos).ok_or("thiếu giá trị created_at")
}

#[test]
fn timestamp_va_crc_phat_hien_sua_doi() {
    let packet = encode_with_timestamp_crc(1_700_000_000);
    assert_eq!(decode_and_verify(&packet), Ok(1_700_000_000));

    // Sửa một byte giữa gói -> CRC lệch -> bị phát hiện.
    let mut hong = packet.clone();
    hong[1] ^= 0xFF;
    assert!(decode_and_verify(&hong).is_err());
}
```

Hai kỹ thuật nền tảng ở đây: **varint** làm cho số nhỏ tốn ít byte (số 5 chỉ 1 byte thay vì 8) — đó là lý do Protobuf gọn hơn JSON nhiều lần với dữ liệu số. Và **CRC32 bọc quanh gói** cho bên nhận phát hiện *bất kỳ* thay đổi nào trên đường truyền: đổi một bit là CRC lệch hẳn. Lưu ý CRC chỉ chống **hỏng ngẫu nhiên** (nhiễu đường truyền), *không* chống kẻ tấn công cố ý — kẻ tấn công sửa gói thì tính lại CRC mới được; chống sửa đổi có chủ đích cần chữ ký mật mã (HMAC), không phải CRC.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Middleware đếm số yêu cầu mỗi IP trong một cửa sổ trượt 1 giây. Vượt 100 thì trả 429 thay vì cho qua. Đây là cùng ý tưởng rate-limiter, đặt ở tầng trung gian.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Lớp trung gian giới hạn tần suất: mỗi IP tối đa 100 yêu cầu / 1 giây.
pub struct RateLimitLayer {
    // ip -> (số đếm trong cửa sổ, thời điểm bắt đầu cửa sổ)
    counters: HashMap<String, (u32, Instant)>,
    gioi_han: u32,
    cua_so: Duration,
}

impl RateLimitLayer {
    pub fn new() -> Self {
        Self { counters: HashMap::new(), gioi_han: 100, cua_so: Duration::from_secs(1) }
    }

    /// Trả Ok(()) nếu cho qua, Err(429) nếu vượt giới hạn.
    pub fn check(&mut self, ip: &str, now: Instant) -> Result<(), u16> {
        let e = self.counters.entry(ip.to_string()).or_insert((0, now));
        // Cửa sổ 1 giây đã trôi qua -> đặt lại bộ đếm.
        if now.duration_since(e.1) >= self.cua_so {
            *e = (0, now);
        }
        e.0 += 1;
        if e.0 > self.gioi_han {
            Err(429) // HTTP 429 Too Many Requests
        } else {
            Ok(())
        }
    }
}

#[test]
fn chan_khi_vuot_100_moi_giay() {
    let mut rl = RateLimitLayer::new();
    let t0 = Instant::now();
    // 100 yêu cầu đầu trong cùng cửa sổ: cho qua.
    for _ in 0..100 { assert_eq!(rl.check("1.2.3.4", t0), Ok(())); }
    // Yêu cầu thứ 101: bị chặn với 429.
    assert_eq!(rl.check("1.2.3.4", t0), Err(429));
    // Sang cửa sổ mới (qua 1 giây) -> bộ đếm reset, lại cho qua.
    assert_eq!(rl.check("1.2.3.4", t0 + Duration::from_millis(1100)), Ok(()));
    // IP khác không bị ảnh hưởng.
    assert_eq!(rl.check("9.9.9.9", t0), Ok(()));
}
```

Đặt giới hạn tần suất thành **một lớp trung gian (middleware/layer)** là mẫu kiến trúc quan trọng: nó tách *mối quan tâm ngang* (cross-cutting concern) — chống lạm dụng — ra khỏi logic nghiệp vụ. Mọi yêu cầu đi qua lớp này *trước khi* chạm tới trình xử lý thật; vượt ngưỡng thì bị chặn ngay tại cửa với mã `429`, không tốn tài nguyên xử lý. Đây chính là cách Tower (hệ sinh thái của Tokio) tổ chức: xếp chồng các lớp (rate-limit, xác thực, ghi log, nén...) quanh dịch vụ lõi, mỗi lớp làm một việc. Bản ở đây dùng **cửa sổ cố định** (đơn giản); dịch vụ thật thường dùng *cửa sổ trượt* hoặc *thùng thẻ bài* (token bucket) để tránh hiện tượng dồn cục ở ranh giới cửa sổ.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Câu hỏi so sánh: gRPC nhanh hơn, nhưng REST/JSON thắng ở đâu khiến các hãng vẫn giữ nó cho người dùng cuối? Nghĩ về trình duyệt, công cụ gỡ lỗi, và rào cản với nhà phát triển bên thứ ba.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**gRPC nhanh hơn nhưng chưa thay REST cho giao diện hướng người dùng cuối**, vì tốc độ không phải yếu tố duy nhất — và ở lớp tiếp xúc bên ngoài, những thứ REST mạnh lại quan trọng hơn.

**Ba khía cạnh REST/JSON vẫn thắng:**

**1. Tương thích trình duyệt.** gRPC chạy trên HTTP/2 với khung nhị phân và yêu cầu kiểm soát mức thấp (trailer, luồng) mà **JavaScript trong trình duyệt không truy cập được trực tiếp**. Muốn gọi gRPC từ trình duyệt phải qua một lớp cầu (gRPC-Web) kèm proxy trung gian. REST/JSON thì gọi thẳng bằng `fetch()` — *mọi* trình duyệt hỗ trợ sẵn, không cần proxy. Với ứng dụng web hướng người dùng, đây gần như là yếu tố quyết định.

**2. Gỡ lỗi thủ công qua `curl`.** REST/JSON là **văn bản người đọc được**: gõ `curl https://api/user/42` là thấy ngay phản hồi JSON, đọc hiểu tức thì, sửa thử tại chỗ. gRPC là **nhị phân** — không `curl` đọc được, phải có công cụ chuyên dụng (`grpcurl`) và tệp định nghĩa `.proto` mới giải mã nổi. Khả năng "mở terminal gõ một dòng là xem được" cực kỳ giá trị khi vận hành và gỡ lỗi lúc 3 giờ sáng.

**3. Thân thiện với nhà phát triển bên thứ ba.** Một API công khai cho hàng nghìn lập trình viên bên ngoài tích hợp cần **rào cản gia nhập thấp nhất có thể**. REST/JSON: đọc tài liệu, gọi thử bằng bất cứ ngôn ngữ nào, không cần công cụ đặc biệt. gRPC: phải lấy tệp `.proto`, chạy trình sinh mã cho đúng ngôn ngữ, học mô hình streaming. Ma sát cao hơn hẳn — chấp nhận được cho *nội bộ*, nhưng cản trở cho một API mở.

**Vì sao vẫn dùng gRPC — và dùng ở đâu:** gRPC tỏa sáng ở **giao tiếp *giữa các dịch vụ nội bộ*** (service-to-service, đông-tây): nơi cả hai đầu do bạn kiểm soát, hiệu năng và hợp đồng kiểu chặt (typed contract) qua `.proto` đáng giá hơn khả năng đọc bằng mắt. Kiến trúc phổ biến nhất hiện nay là **lai**: gRPC cho lõi nội bộ (dịch vụ gọi dịch vụ), và một **cổng REST/JSON ở rìa** (API gateway) để trình duyệt và bên thứ ba nói chuyện với hệ thống. Mỗi giao thức đặt đúng chỗ mạnh của nó — không phải cái này *thay* cái kia, mà là *phân vai*.
</details>
