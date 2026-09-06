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
        return Err("Kich thuoc goi tin qua ngan de chua IPv4 Header hop le!");
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
        Err(err) => println!("    [!] Failed phan tich: {}", err),
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
        Err(err) => println!("    [!] Failed phan tich ELF: {}", err),
    }

    println!("\n==================================================================");
    println!("   HOAN TAT: TOC DO PHAN TICH TOI DA - KHONG CAP PHAT HEAP!     ");
    println!("==================================================================");
}
