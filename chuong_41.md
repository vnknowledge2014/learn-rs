# Chương 41: Phân tích gói tin mạng không sao chép & Giải mã tệp nhị phân ELF/PE (Zero-Copy Network Packet Inspection & ELF/PE Parsing)

## Giới thiệu & Mục tiêu học tập

Trong các hệ thống phòng thủ không gian mạng hiện đại như Tường lửa thế hệ mới (Next-Gen Firewall), Hệ thống phát hiện xâm nhập (IDS - Intrusion Detection System như Suricata hay Snort), cũng như trong các công cụ dịch ngược mã độc (Malware Reverse Engineering), có hai kỹ năng cấp thấp tối quan trọng:
1. **Phân tích gói tin mạng tốc độ cao (High-Speed Packet Inspection)**: Hàng chục triệu gói tin (packets) ập vào máy chủ mỗi giây; nếu mỗi gói tin đều phải sao chép qua lại trên RAM thì CPU sẽ bốc khói ngay lập tức. Kỹ thuật **Không sao chép (Zero-Copy Parsing)** của Rust là chìa khóa vàng để đạt thông lượng 100Gbps.
2. **Giải mã định dạng tệp thực thi nhị phân (Binary Parsing: ELF & PE)**: Khi một tệp tin lạ được tải về máy chủ, làm thế nào chuyên gia an ninh biết đó là mã độc viết cho Linux hay Windows, nhắm vào chip ARM hay x86_64 trước khi nó kịp chạy?

Mục tiêu học tập của bạn:
- Nắm vững kiến trúc khung truyền dữ liệu mạng: Khung Ethernet (Layer 2), Gói tin IPv4 (Layer 3), và Tiêu đề TCP/UDP (Layer 4).
- Làm chủ kỹ thuật trích xuất dữ liệu không sao chép (Zero-Copy) dựa trên lát cắt byte `&[u8]` và chuyển đổi thứ tự byte mạng (Network Byte Order - Big-Endian) sang thứ tự CPU (Host Byte Order - Little-Endian).
- Khám phá giải phẫu cấu trúc tệp nhị phân chuẩn mực: Tệp ELF (trên Linux) và tệp PE (trên Windows), nhận diện các "Byte ma thuật" (Magic Bytes) nhận dạng tệp.
- Tự tay lập trình công cụ kiểm tra tệp nhị phân và phân tích gói tin mạng chuẩn mực bằng Rust với hiệu năng tối đa và an toàn bộ nhớ tuyệt đối.

---

## Hình tượng hóa đời sống (Intuitive Everyday Analogy)

Để hiểu sâu sắc kỹ thuật Zero-Copy và Magic Bytes, hãy quan sát hai hình tượng đời sống quen thuộc sau:

```
┌──────────────────────────────────────────────────────────────────────────────────┐
│             HÌNH TƯỢNG HÓA: SOI THẺ HÀNH LÝ & CON DẤU HỘ CHIẾU QUỐC TẾ           │
├──────────────────────────────────────────────────────────────────────────────────┤
│                                                                                  │
│ [1. KỸ THUẬT ZERO-COPY: SOI THẺ HÀNH LÝ BẰNG MÁY QUÉT MÃ VẠCH]                   │
│ ┌──────────────────────────────────────────────────────────────────────┐         │
│ │ Cách sao chép cũ (Copy Overhead):                                    │         │
│ │   Nhân viên mở từng vali ra, bốc hết quần áo bỏ sang một thùng mới,  │         │
│ │   đếm từng chiếc áo rồi lại nhét ngược lại (Chậm chạp, tốn công)!    │         │
│ ├──────────────────────────────────────────────────────────────────────┤         │
│ │ Cách Zero-Copy của Rust:                                             │         │
│ │   Vali vẫn nằm nguyên trên băng chuyền. Nhân viên hải quan chỉ liếc  │         │
│ │   mắt đọc tấm thẻ hành lý trong suốt dán ngoài vỏ (Lát cắt mượn &[u8])│        │
│ │   ===> Tốc độ ánh sáng, tốn 0 đồng chi phí sao chép bộ nhớ!         │         │
│ └──────────────────────────────────────────────────────────────────────┘         │
│                                                                                  │
│ [2. MAGIC BYTES: CON DẤU QUỐC HUY TRÊN BÌA CUỐN HỘ CHIẾU]                        │
│ Khi một hành khách bước vào cửa hải quan, nhân viên không cần phỏng vấn 1 tiếng: │
│ - Nhìn 4 ký tự đầu bìa sổ: 0x7F 'E' 'L' 'F' ──► Công dân hệ điều hành Linux!     │
│ - Nhìn 2 ký tự đầu bìa sổ: 'M' 'Z'          ──► Công dân hệ điều hành Windows!   │
│ - Nhìn trang 5 (EI_CLASS): Byte 0x02        ──► Kiến trúc 64-bit hiện đại!       │
└──────────────────────────────────────────────────────────────────────────────────┘
```

### 1. Soi thẻ hành lý sân bay (Zero-Copy Packet Inspection)
- Hãy tưởng tượng băng chuyền hành lý tại sân bay quốc tế vận chuyển 100,000 kiện hàng mỗi giờ (tương đương dòng gói tin mạng ào ạt đổ về card mạng NIC).
- **Cách tiếp cận sao chép dữ liệu (Deep Copying)**: Mỗi khi có kiện hàng tới, nhân viên hải quan bưng kiện hàng xuống đất, mở khóa, bốc toàn bộ đồ đạc sang một chiếc vali mới toanh của sân bay, dán nhãn rồi mới đẩy đi tiếp. Thao tác này làm nghẽn toàn bộ sân bay và tốn hàng núi bộ nhớ đệm (buffer)!
- **Cách tiếp cận Zero-Copy của Rust**: Kiện hàng vẫn nằm nguyên trên băng chuyền. Nhân viên hải quan chỉ cần cầm máy quét tia hồng ngoại rọi vào tấm thẻ hành lý gắn ngoài vỏ vali (mượn tham chiếu `&[u8]`). Nhìn vào đúng vị trí byte thứ 12 để đọc địa chỉ người gửi, byte thứ 16 để đọc người nhận. Không một hạt bụi nào bị di chuyển, không tốn 1 byte RAM cấp phát mới nào!

### 2. Con dấu quốc huy trên hộ chiếu (ELF/PE Magic Bytes)
- Bất kể một người mang quốc tịch nào, trang bìa cuốn hộ chiếu luôn có con dấu biểu trưng độc nhất không thể làm giả.
- Trong thế giới nhị phân, mọi định dạng tệp tin đều bắt đầu bằng một chuỗi **Magic Bytes** cố định ở offset 0:
  - Nếu bạn đổi tên tệp mã độc từ `virus.exe` thành `hinh_anh_dep.jpg`, người dùng Windows có thể bị lừa.
  - Nhưng một bộ phân tích nhị phân (Binary Parser) viết bằng Rust sẽ đọc ngay 2 byte đầu tiên: Nếu thấy ký tự `MZ` (`0x4D 0x5A` - tên viết tắt của kỹ sư Mark Zbikowski, tác giả kiến trúc MS-DOS), chương trình sẽ gióng chuông báo động ngay: *"Đây là tệp thực thi Windows trá hình, không phải ảnh JPEG!"*.

---

## Khái niệm & Cơ chế kỹ thuật chuyên sâu (In-Depth Technical Mechanics)

### 1. Cấu trúc Gói tin Mạng Lồng ghép (Encapsulated Packet Headers)

Một gói tin truyền qua cáp quang hay sóng Wi-Fi là một chuỗi byte tuần tự được đóng gói theo các tầng giao thức:

```
┌─────────────────────────┬─────────────────────────┬─────────────────────────┬──────────────────┐
│ Layer 2: Ethernet (14B) │ Layer 3: IPv4 (20B)     │ Layer 4: TCP/UDP (20B)  │ Dữ liệu Payload  │
│ MAC Đích | MAC Nguồn... │ IP Nguồn | IP Đích...   │ Cổng Nguồn | Cổng Đích..│ HTTP, DNS, v.v.  │
└─────────────────────────┴─────────────────────────┴─────────────────────────┴──────────────────┘
```

#### Giải mã Tiêu đề IPv4 (20 bytes chuẩn):
- **Byte 0**: 4 bits đầu là Phiên bản (`Version = 4`), 4 bits sau là Độ dài tiêu đề (`IHL - Internet Header Length`, thường là 5 từ 32-bit = 20 bytes).
- **Byte 8**: Thời gian sống (`TTL - Time to Live`): Số trạm trung chuyển (router) tối đa gói tin được đi qua trước khi bị hủy để chống lặp vòng.
- **Byte 9**: Giao thức tầng trên (`Protocol`): `1` là ICMP (Ping), `6` là TCP, `17` là UDP.
- **Bytes 12..15**: Địa chỉ IP Nguồn (Source IP Address).
- **Bytes 16..19**: Địa chỉ IP Đích (Destination IP Address).

### 2. Thứ tự Byte Mạng (Network Byte Order - Big-Endian)

- Các kiến trúc vi xử lý hiện đại (x86_64, ARM64) lưu trữ số nguyên nhiều byte theo quy ước **Little-Endian** (byte có trọng số nhỏ nhất nằm ở địa chỉ thấp).
- Tuy nhiên, quy ước quốc tế của mạng Internet quy định toàn bộ số nguyên truyền qua dây cáp mạng phải tuân theo **Big-Endian** (byte có trọng số lớn nhất đi trước).
- Do đó, khi đọc các trường như số cổng (16-bit) hay độ dài gói tin, lập trình viên Rust bắt buộc phải sử dụng hàm chuyển đổi: `u16::from_be_bytes(bytes)` để đảm bảo CPU hiểu đúng con số.

### 3. Giải mã Tệp Nhị phân Linux ELF (Executable and Linkable Format)

Mọi tệp thực thi, thư viện chia sẻ (`.so`), và tệp mã đối tượng (`.o`) trên Linux đều tuân thủ cấu trúc ELF Header:

```
┌──────────────────────┬─────────────────┬─────────────────┬────────────────┬─────────────────────┐
│ Magic Bytes (4B)     │ Class (1B)      │ Endianness (1B) │ Version (1B)   │ Entry Point (8B)    │
│ 0x7F 'E' 'L' 'F'     │ 1 = 32b, 2 = 64b│ 1 = LE, 2 = BE  │ Luôn bằng 1    │ Địa chỉ bắt đầu chạy│
└──────────────────────┴─────────────────┴─────────────────┴────────────────┴─────────────────────┘
```
- **Byte 0..3**: Chuỗi ma thuật định danh: `0x7F`, `0x45` ('E'), `0x4C` ('L'), `0x46` ('F').
- **Byte 4 (`EI_CLASS`)**: Phân biệt kiến trúc: `1` là 32-bit, `2` là 64-bit.
- **Byte 5 (`EI_DATA`)**: Phân biệt mã hóa số: `1` là Little-Endian (Intel/AMD), `2` là Big-Endian (IBM PowerPC).
- **Bytes 24..31** (trên hệ 64-bit): Điểm nhập cuộc (`Entry Point Virtual Address`) — địa chỉ câu lệnh máy đầu tiên mà CPU sẽ nhảy tới khi tiến trình khởi động!

---

## Mã nguồn minh họa thực chiến (Idiomatic Runnable Rust Blueprint)

Dưới đây là chương trình Rust hoàn chỉnh thể hiện kỹ thuật **Zero-Copy Parser**: Giải mã trực tiếp gói tin mạng IPv4 và bóc tách tiêu đề tệp thực thi nhị phân Linux ELF chỉ từ một mảng byte thô `&[u8]`, không cấp phát thêm bất kỳ ô nhớ Heap nào:

```rust
#![allow(dead_code, unused_variables, unused_imports)]
/// Thông tin tiêu đề gói tin IPv4 sau khi giải mã Zero-Copy
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedIpv4Header<'a> {
    pub version: u8,
    pub header_length_bytes: usize,
    pub ttl: u8,
    pub protocol: u8,
    pub source_ip: [u8; 4],
    pub dest_ip: [u8; 4],
    pub payload: &'a [u8], // Lát cắt mượn trực tiếp từ gói tin gốc (Zero-Copy!)
}

/// Trình phân tích tiêu đề gói tin mạng IPv4
pub fn parse_ipv4_packet(raw_bytes: &[u8]) -> Result<ParsedIpv4Header<'_>, &'static str> {
    if raw_bytes.len() < 20 {
        return Err("Kich thuoc goi tin qua short de chua IPv4 Header hop le!");
    }

    // Byte 0: 4-bit Version và 4-bit IHL
    let version = (raw_bytes[0] >> 4) & 0x0F;
    let ihl = (raw_bytes[0] & 0x0F) as usize;
    let header_length_bytes = ihl * 4;

    if version != 4 {
        return Err("Day khong phai goi tin dinh dang IPv4!");
    }

    if raw_bytes.len() < header_length_bytes {
        return Err("Do dai goi tin thuc te nho hon IHL khai bao trong tieu de!");
    }

    let ttl = raw_bytes[8];
    let protocol = raw_bytes[9];

    let mut source_ip = [0u8; 4];
    source_ip.copy_from_slice(&raw_bytes[12..16]);

    let mut dest_ip = [0u8; 4];
    dest_ip.copy_from_slice(&raw_bytes[16..20]);

    // Trích xuất payload mà không tốn một lần cấp phát Heap nào (Zero-Copy)
    let payload = &raw_bytes[header_length_bytes..];

    Ok(ParsedIpv4Header {
        version,
        header_length_bytes,
        ttl,
        protocol,
        source_ip,
        dest_ip,
        payload,
    })
}

/// Thông tin tiêu đề tệp thực thi nhị phân Linux ELF
#[derive(Debug, PartialEq, Eq)]
pub struct ParsedElfHeader {
    pub is_valid_elf: bool,
    pub bit_architecture: &'static str,
    pub endianness: &'static str,
    pub entry_point_address: u64,
}

/// Trình giải mã tiêu đề tệp ELF Linux
pub fn parse_elf_header(binary_data: &[u8]) -> Result<ParsedElfHeader, &'static str> {
    if binary_data.len() < 32 {
        return Err("Tap tin qua nho de chua ELF Header hop le!");
    }

    // Kiểm tra 4 Magic Bytes: 0x7F, 'E', 'L', 'F'
    if binary_data[0..4] != [0x7F, b'E', b'L', b'F'] {
        return Err("Dau hieu nhan dang Magic Bytes khong khop voi dinh dang ELF!");
    }

    // Byte 4: EI_CLASS (1 = 32-bit, 2 = 64-bit)
    let bit_architecture = match binary_data[4] {
        1 => "32-bit (x86 / ARM32)",
        2 => "64-bit (x86_64 / AArch64)",
        _ => "Kien truc khong xac dinh",
    };

    // Byte 5: EI_DATA (1 = Little Endian, 2 = Big Endian)
    let endianness = match binary_data[5] {
        1 => "Little-Endian (Intel/AMD)",
        2 => "Big-Endian (Network/MIPS)",
        _ => "Dinh dang endian khong xac dinh",
    };

    // Trích xuất địa chỉ Entry Point (Byte 24..32 cho tệp 64-bit Little-Endian)
    let mut entry_bytes = [0u8; 8];
    entry_bytes.copy_from_slice(&binary_data[24..32]);
    let entry_point_address = u64::from_le_bytes(entry_bytes);

    Ok(ParsedElfHeader {
        is_valid_elf: true,
        bit_architecture,
        endianness,
        entry_point_address,
    })
}

fn main() {
    println!("==================================================================");
    println!("   PHAN TICH GOI TIN ZERO-COPY & GIAI MA TEP NHI PHAN ELF RUST   ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM GIẢI MÃ GÓI TIN MẠNG IPV4 ZERO-COPY
    // -------------------------------------------------------------
    println!("\n[1] Giai ma goi tin mang IPv4 mo phong:");

    // Dựng mảng byte gói tin mẫu (Tiêu đề 20 bytes + Payload 4 bytes)
    let sample_packet: [u8; 24] = [
        0x45, 0x00, 0x00, 0x18, // Ver=4, IHL=5, Total Len=24
        0x1C, 0x7B, 0x40, 0x00, // ID, Flags, Fragment Offset
        0x40, 0x06, 0x00, 0x00, // TTL=64, Protocol=6 (TCP), Checksum
        192, 168, 1, 100,       // Source IP: 192.168.1.100
        10, 0, 0, 1,            // Dest IP: 10.0.0.1
        0xDE, 0xAD, 0xBE, 0xEF, // Payload du lieu
    ];

    match parse_ipv4_packet(&sample_packet) {
        Ok(parsed) => {
            println!("    - Phien ban IP      : IPv{}", parsed.version);
            println!("    - Do dai Tieu de   : {} bytes", parsed.header_length_bytes);
            println!("    - Thoi gian song TTL: {}", parsed.ttl);
            println!("    - Giao thuc tang 4  : {} (TCP)", parsed.protocol);
            println!(
                "    - Dia chi IP Nguon : {}.{}.{}.{}",
                parsed.source_ip[0], parsed.source_ip[1], parsed.source_ip[2], parsed.source_ip[3]
            );
            println!(
                "    - Dia chi IP Dich  : {}.{}.{}.{}",
                parsed.dest_ip[0], parsed.dest_ip[1], parsed.dest_ip[2], parsed.dest_ip[3]
            );
            println!("    - Payload Data (Hex): {:X?}", parsed.payload);
            println!("    => Zero-Copy: Payload la lat cat &[u8] tro thang vao mang goc!");
        }
        Err(err) => println!("    [!] Failed phan products: {}", err),
    }

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM GIẢI MÃ TIÊU ĐỀ TỆP NHỊ PHÂN LINUX ELF
    // -------------------------------------------------------------
    println!("\n[2] Giai ma tieu de tep thuc thi ELF Linux mo phong:");

    // Tạo mảng 64 bytes mô phỏng phần đầu ELF64
    let mut mock_elf_data = [0u8; 64];
    mock_elf_data[0] = 0x7F;
    mock_elf_data[1] = b'E';
    mock_elf_data[2] = b'L';
    mock_elf_data[3] = b'F';
    mock_elf_data[4] = 2; // ELFCLASS64
    mock_elf_data[5] = 1; // ELFDATA2LSB (Little Endian)
    mock_elf_data[6] = 1; // EV_CURRENT

    // Đặt địa chỉ Entry Point giả lập: 0x0000000000401000
    let entry_addr: u64 = 0x00401000;
    mock_elf_data[24..32].copy_from_slice(&entry_addr.to_le_bytes());

    match parse_elf_header(&mock_elf_data) {
        Ok(elf) => {
            println!("    - Magic Bytes Valid : {}", elf.is_valid_elf);
            println!("    - Kien truc Chip CPU: {}", elf.bit_architecture);
            println!("    - Thu tu Byte Endian: {}", elf.endianness);
            println!("    - Dia chi khoi chay : 0x{:012X}", elf.entry_point_address);
            println!("    => Nhan dang tep nhi phan thanh cong chi voi 64 bytes dau!");
        }
        Err(err) => println!("    [!] Failed phan products ELF: {}", err),
    }

    println!("\n==================================================================");
    println!("   HOAN TAT: TOC DO PHAN TICH TOI DA - KHONG CAP PHAT HEAP!     ");
    println!("==================================================================");
}
```

---

## Bảng tra cứu lỗi biên dịch & Cách khắc phục (Compiler Error Guide)

Dưới đây là các lỗi biên dịch thường gặp nhất khi lập trình phân tích nhị phân và lát cắt byte trong Rust:

| Mã lỗi | Thông báo mẫu từ trình biên dịch | Nguyên nhân cốt lõi | Cách khắc phục nhanh |
|---|---|---|---|
| **E0507** | `cannot move out of a shared reference` | Cố gắng di chuyển quyền sở hữu (ownership) của một trường nằm bên trong lát cắt mượn `&[u8]`. | Sử dụng phép sao chép dữ liệu nhỏ qua `.copy_from_slice()` hoặc chỉ mượn (borrow) tham chiếu con. |
| **E0277** | `the trait 'From<[u8]>' is not implemented` | Cố gắng chuyển đổi mảng slice `&[u8]` thành mảng có kích thước cố định `[u8; 4]` mà không qua phương thức `try_into`. | Sử dụng `slice[..4].try_into().unwrap()` hoặc hàm sao chép byte chuyên dụng. |
| **E0597** | `'raw_bytes' does not live long enough` | Cấu trúc chứa trường lát cắt `&'a [u8]` cố gắng sống lâu hơn biến mảng byte gốc mà nó đang tham chiếu. | Đảm bảo mảng gốc có thời gian sống (lifetime) bao trùm toàn bộ phạm vi sử dụng của cấu trúc phân tích. |
| **E0308** | `mismatched types: expected array '[u8; 4]', found slice '&[u8]'` | Nhầm lẫn giữa mảng cố định nằm trên Stack và lát cắt mượn động trên bộ nhớ đệm (buffer). | Khai báo rõ ràng mảng cố định `let mut arr = [0u8; 4];` rồi gọi `.copy_from_slice()`. |

### Ví dụ phân tích lỗi `E0507` khi trích xuất mảng con từ lát cắt:

```rust
// Đoạn mã lỗi minh họa E0507:
fn e0507_broken(slice: &[u8]) {
    // let mang_bon_byte: [u8; 4] = slice[0..4]; // LỖI E0507: Không thể move dữ liệu từ slice mượn!
}

// Cách sửa chữa đúng chuẩn: Dùng con trỏ mượn hoặc sao chép byte
fn vi_du_dung_e0507(slice: &[u8]) -> [u8; 4] {
    let mut mang = [0u8; 4];
    mang.copy_from_slice(&slice[0..4]); // Sao chép an toàn 4 bytes
    mang
}
```

---

## Tóm tắt chương & Bài tập rèn luyện (Summary & Exercises)

### 4 Điểm cốt lõi cần ghi nhớ:
1. **Triết lý Zero-Copy**: Tận dụng triệt để lát cắt byte `&[u8]` để phân tích dữ liệu mạng và tệp nhị phân mà không tiêu tốn tài nguyên cấp phát Heap hay sao chép bộ đệm.
2. **Quy ước Byte Mạng**: Dữ liệu mạng luôn lưu theo chuẩn Big-Endian, yêu cầu chuyển đổi tường minh sang Little-Endian của CPU khi xử lý số nguyên.
3. **Dấu vân tay Magic Bytes**: Kiểm tra các byte đầu tiên của tệp tin là bước đầu tiên và tin cậy nhất để nhận diện bản chất nhị phân của tệp (như `0x7F 'E' 'L' 'F'` của Linux hay `MZ` của Windows).
4. **Hệ thống Type-Safe vững chắc**: Cơ chế quyền sở hữu (ownership), mượn (borrow), thời gian sống (lifetime), con trỏ thông minh (smart pointer) và bộ nhớ đệm (buffer) kết hợp hoàn hảo bảo đảm quá trình bóc tách byte không bao giờ bị tràn biên hay sập tiến trình.

### Bài tập rèn luyện tự giải:
1. **Bài tập 1 (Phân tích Tiêu đề gói tin UDP)**:  
   Mở rộng chương trình để phân tích tiêu đề gói tin UDP (8 bytes gồm: Source Port 2B, Dest Port 2B, Length 2B, Checksum 2B). Đọc các giá trị số cổng bằng hàm `u16::from_be_bytes` và in ra thông tin định tuyến.
2. **Bài tập 2 (Nhận diện Tệp Thực thi Windows PE)**:  
   Viết hàm `parse_pe_header(data: &[u8]) -> bool`. Kiểm tra xem 2 byte đầu có phải là `0x4D, 0x5A` ('M', 'Z') hay không. Đọc độ lệch tại byte `0x3C` (e_lfanew) để nhảy tới vị trí tiêu đề PE Signature và xác nhận chuỗi `PE\0\0`.
3. **Bài tập 3 (Suy ngẫm kiến trúc: Denial of Service trong Packet Parser)**:  
   Nếu kẻ tấn công gửi một gói tin có trường `IHL = 15` (khai báo tiêu đề dài 60 bytes) nhưng toàn bộ gói tin gửi qua mạng chỉ dài 20 bytes, chuyện gì sẽ xảy ra nếu trình phân tích không kiểm tra kích thước? Tại sao Rust giúp ngăn chặn triệt để lỗi khai thác này?

---

### Gợi ý & Lời giải

<details>
<summary><b>Bài tập 1 — Gợi ý</b></summary>

Tiêu đề UDP đúng 8 byte, bốn trường u16 liên tiếp. `u16::from_be_bytes` đọc 2 byte theo thứ tự mạng (big-endian). Nhớ kiểm độ dài trước khi đọc.
</details>

<details>
<summary><b>Bài tập 1 — Lời giải</b></summary>

```rust
#[derive(Debug, PartialEq, Eq)]
pub struct UdpHeader {
    pub source_port: u16,
    pub dest_port: u16,
    pub length: u16,
    pub checksum: u16,
}

/// Phân tích 8 byte tiêu đề UDP. Trả Err nếu không đủ 8 byte.
pub fn parse_udp_header(data: &[u8]) -> Result<UdpHeader, &'static str> {
    if data.len() < 8 {
        return Err("Goi tin qua ngan cho UDP header (can it nhat 8 byte)!");
    }
    // Mạng dùng big-endian (network byte order) -> from_be_bytes.
    Ok(UdpHeader {
        source_port: u16::from_be_bytes([data[0], data[1]]),
        dest_port:   u16::from_be_bytes([data[2], data[3]]),
        length:      u16::from_be_bytes([data[4], data[5]]),
        checksum:    u16::from_be_bytes([data[6], data[7]]),
    })
}

#[test]
fn phan_tich_udp_dns() {
    // Cổng nguồn 0x0035 = 53 (DNS), cổng đích 0x1F90 = 8080, length 0x0020 = 32
    let goi = [0x00, 0x35, 0x1F, 0x90, 0x00, 0x20, 0xAB, 0xCD];
    let h = parse_udp_header(&goi).unwrap();
    assert_eq!(h.source_port, 53);
    assert_eq!(h.dest_port, 8080);
    assert_eq!(h.length, 32);
    assert_eq!(h.checksum, 0xABCD);
    // Thiếu byte -> Err, không đọc tràn.
    assert!(parse_udp_header(&[0, 1, 2]).is_err());
}
```

Hai điều đáng khắc: **big-endian là quy ước bắt buộc của mạng** — mọi số nhiều byte trên dây được gửi byte cao trước, nên phải `from_be_bytes` (không phải `from_le_bytes` của máy x86). Và **kiểm `len() < 8` trước khi chạm `data[7]`** — bỏ bước này thì một gói cụt sẽ gây hoảng loạn truy cập ngoài biên; chính là mầm mống bài tập DoS phía dưới.
</details>

<details>
<summary><b>Bài tập 2 — Gợi ý</b></summary>

Định dạng PE có hai mốc: 2 byte đầu là 'MZ' (0x4D 0x5A), và một con trỏ 4 byte tại offset 0x3C chỉ tới chữ ký 'PE\0\0'. Mọi lần đọc đều phải kiểm biên trước.
</details>

<details>
<summary><b>Bài tập 2 — Lời giải</b></summary>

```rust
/// Nhận diện tệp thực thi Windows PE. Kiểm mọi biên trước khi đọc để KHÔNG hoảng loạn.
pub fn parse_pe_header(data: &[u8]) -> bool {
    // 1. Chữ ký MZ ở 2 byte đầu (di sản từ MS-DOS).
    if data.len() < 2 || data[0] != 0x4D || data[1] != 0x5A {
        return false;
    }
    // 2. Tại offset 0x3C là con trỏ 4 byte (e_lfanew) trỏ tới tiêu đề PE.
    if data.len() < 0x40 {
        return false;
    }
    let e_lfanew = u32::from_le_bytes([data[0x3C], data[0x3D], data[0x3E], data[0x3F]]) as usize;
    // 3. Nhảy tới đó và xác nhận chữ ký "PE  " — NHỚ kiểm đủ 4 byte tại vị trí nhảy.
    if e_lfanew + 4 > data.len() {
        return false;
    }
    &data[e_lfanew..e_lfanew + 4] == b"PE\0\0"
}

#[test]
fn nhan_dien_pe() {
    // Dựng một PE tối thiểu: MZ ở đầu, e_lfanew = 0x40, "PE  " tại 0x40.
    let mut data = vec![0u8; 0x44];
    data[0] = 0x4D; data[1] = 0x5A;                 // "MZ"
    data[0x3C..0x40].copy_from_slice(&0x40u32.to_le_bytes()); // e_lfanew = 0x40
    data[0x40..0x44].copy_from_slice(b"PE\0\0");     // chữ ký PE
    assert!(parse_pe_header(&data));

    // Không phải PE / dữ liệu cụt -> false, không sập.
    assert!(!parse_pe_header(b"MZ"));            // có MZ nhưng quá ngắn
    assert!(!parse_pe_header(b"hello world"));   // sai chữ ký ngay từ đầu
    assert!(!parse_pe_header(&[]));              // rỗng
}
```

Điểm cốt lõi của phân tích định dạng nhị phân: **con trỏ trong dữ liệu là dữ liệu không tin được.** `e_lfanew` là một số *đọc từ tệp* — kẻ tấn công đặt nó bằng 0xFFFFFFFF thì `data[e_lfanew..]` sẽ tràn. Nên trước mỗi lần nhảy phải kiểm `e_lfanew + 4 > data.len()`. Cấu trúc "MZ đầu tệp, con trỏ tới PE ở 0x3C" là di sản lịch sử: mọi tệp .exe Windows vẫn mở đầu bằng một chương trình MS-DOS cổ in ra dòng chữ quen thuộc khi chạy ở chế độ DOS.
</details>

<details>
<summary><b>Bài tập 3 — Gợi ý</b></summary>

Kịch bản tấn công: IHL khai báo tiêu đề dài 60 byte nhưng gói chỉ có 20 byte. Nếu parser tin lời khai mà không kiểm, nó đọc ra ngoài vùng nhớ gói tin.
</details>

<details>
<summary><b>Bài tập 3 — Lời giải</b></summary>

**Chuyện xảy ra nếu parser không kiểm kích thước:**

Trường IHL = 15 nghĩa là "tiêu đề dài 15 × 4 = 60 byte". Nếu parser tin con số này rồi làm `payload = &raw_bytes[60..]` trong khi gói tin thật chỉ có 20 byte, thì nó **đọc ra ngoài vùng nhớ của gói** — 40 byte không thuộc về nó.

Trong **C/C++**, đây là thảm họa kinh điển: con trỏ chạy quá cuối bộ đệm, đọc trúng vùng nhớ bên cạnh. Hậu quả tùy nội dung vùng nhớ đó:
- Đọc trúng dữ liệu của phiên khác -> **rò rỉ thông tin** (đúng cơ chế lỗ hổng Heartbleed năm 2014).
- Con trỏ trỏ vào vùng chưa ánh xạ -> **sập tiến trình** (từ chối dịch vụ - DoS).
- Kẻ tấn công gửi hàng loạt gói dị dạng như vậy -> **DoS quy mô lớn** với chi phí gần như bằng 0 cho bên tấn công.

**Vì sao Rust chặn triệt để lỗi này:**

1. **Kiểm biên tự động lúc chạy.** Nếu bạn *quên* kiểm và cứ `&raw_bytes[60..]` trên gói 20 byte, Rust **hoảng loạn (panic) và dừng an toàn** — không bao giờ đọc lén vùng nhớ kế bên. Tệ nhất là một cú sập *có kiểm soát*, không phải rò rỉ dữ liệu âm thầm.

2. **Lát cắt (slice) mang theo độ dài.** `&[u8]` là con trỏ béo: luôn biết vùng nó bao phủ dài bao nhiêu. Không có "con trỏ trần" chạy tự do như C — mọi truy cập đều đối chiếu với độ dài đó.

3. **Kiểu buộc bạn xử lý ca lỗi.** Chính hàm `parse_ipv4_packet` của chương này trả `Result` và có dòng `if raw_bytes.len() < header_length_bytes { return Err(...) }`. Cách *đúng* là biến gói dị dạng thành một `Err` tường minh mà người gọi phải xử lý — thay vì để nó âm thầm phá bộ nhớ.

Nói gọn: trong C, lỗi này là **đọc lén vùng nhớ người khác** (bí mật, nguy hiểm); trong Rust, tệ nhất nó là **một panic có kiểm soát**, và nếu viết đúng thì chỉ là một `Err`. Rust không xóa được lỗi *logic* (quên kiểm), nhưng nó biến hậu quả từ "lỗ hổng bảo mật" thành "dừng an toàn".
</details>
