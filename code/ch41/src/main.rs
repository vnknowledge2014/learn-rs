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
        return Err("Kích thước gói tin quá ngắn để chứa IPv4 Header hợp lệ!");
    }

    // Byte 0: 4-bit Version và 4-bit IHL
    let version = (raw_bytes[0] >> 4) & 0x0F;
    let ihl = (raw_bytes[0] & 0x0F) as usize;
    let header_length_bytes = ihl * 4;

    if version != 4 {
        return Err("Đây không phải gói tin định dạng IPv4!");
    }

    // IHL nhỏ hơn 5 (20 byte) là gói dị dạng: nếu chấp nhận, "payload" sẽ chồng lên
    // chính các trường tiêu đề (TTL, IP nguồn/đích...) mà ta vừa đọc.
    if ihl < 5 {
        return Err("IHL nhỏ hơn 5: tiêu đề IPv4 tối thiểu phải dài 20 byte!");
    }

    // Byte 2..4: Total Length (Big-Endian) = tiêu đề + dữ liệu, KHÔNG gồm phần đệm
    // mà tầng liên kết (Ethernet) có thể chèn vào cuối khung.
    let total_length = u16::from_be_bytes([raw_bytes[2], raw_bytes[3]]) as usize;
    if total_length < header_length_bytes {
        return Err("Total Length nhỏ hơn độ dài tiêu đề khai báo!");
    }
    if raw_bytes.len() < total_length {
        return Err("Gói tin bị cắt cụt: ngắn hơn Total Length khai báo trong tiêu đề!");
    }

    let ttl = raw_bytes[8];
    let protocol = raw_bytes[9];

    let mut source_ip = [0u8; 4];
    source_ip.copy_from_slice(&raw_bytes[12..16]);

    let mut dest_ip = [0u8; 4];
    dest_ip.copy_from_slice(&raw_bytes[16..20]);

    // Trích xuất payload mà không tốn một lần cấp phát Heap nào (Zero-Copy).
    // Cắt tới total_length để loại phần đệm ở cuối khung (nếu có).
    let payload = &raw_bytes[header_length_bytes..total_length];

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
    pub machine: &'static str,
    pub entry_point_address: u64,
}

/// Đọc số nguyên N byte tại `offset` theo đúng thứ tự byte mà tệp khai báo
fn read_u16(data: &[u8], offset: usize, big_endian: bool) -> u16 {
    let b = [data[offset], data[offset + 1]];
    if big_endian {
        u16::from_be_bytes(b)
    } else {
        u16::from_le_bytes(b)
    }
}

fn read_u32(data: &[u8], offset: usize, big_endian: bool) -> u32 {
    let b: [u8; 4] = data[offset..offset + 4].try_into().unwrap();
    if big_endian {
        u32::from_be_bytes(b)
    } else {
        u32::from_le_bytes(b)
    }
}

fn read_u64(data: &[u8], offset: usize, big_endian: bool) -> u64 {
    let b: [u8; 8] = data[offset..offset + 8].try_into().unwrap();
    if big_endian {
        u64::from_be_bytes(b)
    } else {
        u64::from_le_bytes(b)
    }
}

/// Trình giải mã tiêu đề tệp ELF Linux (cả ELF32 lẫn ELF64, cả LE lẫn BE)
pub fn parse_elf_header(binary_data: &[u8]) -> Result<ParsedElfHeader, &'static str> {
    if binary_data.len() < 16 {
        return Err("Tập tin quá nhỏ để chứa phần định danh ELF (e_ident 16 byte)!");
    }

    // Kiểm tra 4 Magic Bytes: 0x7F, 'E', 'L', 'F'
    if binary_data[0..4] != [0x7F, b'E', b'L', b'F'] {
        return Err("Dấu hiệu nhận dạng Magic Bytes không khớp với định dạng ELF!");
    }

    // Byte 4: EI_CLASS (1 = 32-bit, 2 = 64-bit) quyết định KÍCH THƯỚC các trường địa chỉ
    let (bit_architecture, header_size, is_64) = match binary_data[4] {
        1 => ("32-bit (ELFCLASS32)", 52, false),
        2 => ("64-bit (ELFCLASS64)", 64, true),
        _ => return Err("EI_CLASS không hợp lệ (phải là 1 hoặc 2)!"),
    };

    // Byte 5: EI_DATA (1 = Little Endian, 2 = Big Endian) quyết định THỨ TỰ BYTE của mọi trường sau
    let (endianness, big_endian) = match binary_data[5] {
        1 => ("Little-Endian (ELFDATA2LSB)", false),
        2 => ("Big-Endian (ELFDATA2MSB)", true),
        _ => return Err("EI_DATA không hợp lệ (phải là 1 hoặc 2)!"),
    };

    if binary_data.len() < header_size {
        return Err("Tập tin bị cắt cụt: ngắn hơn ELF Header của lớp đã khai báo!");
    }

    // Byte 18..20: e_machine — kiến trúc CPU đích
    let machine = match read_u16(binary_data, 18, big_endian) {
        0x03 => "x86 (i386)",
        0x08 => "MIPS",
        0x14 => "PowerPC",
        0x28 => "ARM (32-bit)",
        0x3E => "x86-64 (AMD64)",
        0xB7 => "AArch64 (ARM64)",
        0xF3 => "RISC-V",
        _ => "Kiến trúc khác",
    };

    // Byte 24..: e_entry — 4 byte trên ELF32, 8 byte trên ELF64, theo đúng EI_DATA
    let entry_point_address = if is_64 {
        read_u64(binary_data, 24, big_endian)
    } else {
        read_u32(binary_data, 24, big_endian) as u64
    };

    Ok(ParsedElfHeader {
        is_valid_elf: true,
        bit_architecture,
        endianness,
        machine,
        entry_point_address,
    })
}

fn main() {
    println!("==================================================================");
    println!("   PHÂN TÍCH GÓI TIN ZERO-COPY & GIẢI MÃ TỆP NHỊ PHÂN ELF RUST   ");
    println!("==================================================================");

    // -------------------------------------------------------------
    // 1. THỬ NGHIỆM GIẢI MÃ GÓI TIN MẠNG IPV4 ZERO-COPY
    // -------------------------------------------------------------
    println!("\n[1] Giải mã gói tin mạng IPv4 mô phỏng:");

    // Dựng mảng byte gói tin mẫu (Tiêu đề 20 bytes + Payload 4 bytes)
    let sample_packet: [u8; 24] = [
        0x45, 0x00, 0x00, 0x18, // Ver=4, IHL=5, Total Len=24
        0x1C, 0x7B, 0x40, 0x00, // ID, Flags, Fragment Offset
        0x40, 0x06, 0x00, 0x00, // TTL=64, Protocol=6 (TCP), Checksum
        192, 168, 1, 100, // Source IP: 192.168.1.100
        10, 0, 0, 1, // Dest IP: 10.0.0.1
        0xDE, 0xAD, 0xBE, 0xEF, // Payload dữ liệu
    ];

    match parse_ipv4_packet(&sample_packet) {
        Ok(parsed) => {
            println!("    - Phiên bản IP      : IPv{}", parsed.version);
            println!(
                "    - Độ dài Tiêu đề    : {} bytes",
                parsed.header_length_bytes
            );
            println!("    - Thời gian sống TTL: {}", parsed.ttl);
            println!("    - Giao thức tầng 4  : {} (TCP)", parsed.protocol);
            println!(
                "    - Địa chỉ IP Nguồn  : {}.{}.{}.{}",
                parsed.source_ip[0], parsed.source_ip[1], parsed.source_ip[2], parsed.source_ip[3]
            );
            println!(
                "    - Địa chỉ IP Đích   : {}.{}.{}.{}",
                parsed.dest_ip[0], parsed.dest_ip[1], parsed.dest_ip[2], parsed.dest_ip[3]
            );
            println!("    - Payload Data (Hex): {:X?}", parsed.payload);
            println!("    => Zero-Copy: Payload là lát cắt &[u8] trỏ thẳng vào mảng gốc!");
        }
        Err(err) => println!("    [!] Phân tích thất bại: {}", err),
    }

    // -------------------------------------------------------------
    // 2. THỬ NGHIỆM GIẢI MÃ TIÊU ĐỀ TỆP NHỊ PHÂN LINUX ELF
    // -------------------------------------------------------------
    println!("\n[2] Giải mã tiêu đề tệp thực thi ELF Linux mô phỏng:");

    // Tạo mảng 64 bytes mô phỏng phần đầu ELF64
    let mut mock_elf_data = [0u8; 64];
    mock_elf_data[0] = 0x7F;
    mock_elf_data[1] = b'E';
    mock_elf_data[2] = b'L';
    mock_elf_data[3] = b'F';
    mock_elf_data[4] = 2; // ELFCLASS64
    mock_elf_data[5] = 1; // ELFDATA2LSB (Little Endian)
    mock_elf_data[6] = 1; // EV_CURRENT
    mock_elf_data[18..20].copy_from_slice(&0x3Eu16.to_le_bytes()); // e_machine = x86-64

    // Đặt địa chỉ Entry Point giả lập: 0x0000000000401000
    let entry_addr: u64 = 0x00401000;
    mock_elf_data[24..32].copy_from_slice(&entry_addr.to_le_bytes());

    match parse_elf_header(&mock_elf_data) {
        Ok(elf) => {
            println!("    - Magic Bytes Valid : {}", elf.is_valid_elf);
            println!("    - Lớp địa chỉ       : {}", elf.bit_architecture);
            println!("    - Kiến trúc CPU     : {}", elf.machine);
            println!("    - Thứ tự Byte Endian: {}", elf.endianness);
            println!(
                "    - Địa chỉ khởi chạy : 0x{:012X}",
                elf.entry_point_address
            );
            println!("    => Nhận dạng tệp nhị phân thành công chỉ với 64 bytes đầu!");
        }
        Err(err) => println!("    [!] Phân tích ELF thất bại: {}", err),
    }

    println!("\n==================================================================");
    println!("   HOÀN TẤT: TỐC ĐỘ PHÂN TÍCH TỐI ĐA - KHÔNG CẤP PHÁT HEAP!     ");
    println!("==================================================================");
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ipv4(ihl: u8, total_len: u16, payload: &[u8]) -> Vec<u8> {
        let mut p = vec![0u8; 20];
        p[0] = 0x40 | ihl;
        p[2..4].copy_from_slice(&total_len.to_be_bytes());
        p[8] = 64;
        p[9] = 17;
        p[12..16].copy_from_slice(&[10, 0, 0, 1]);
        p[16..20].copy_from_slice(&[10, 0, 0, 2]);
        p.extend_from_slice(payload);
        p
    }

    #[test]
    fn parses_minimal_packet() {
        let pkt = ipv4(5, 23, b"abc");
        let h = parse_ipv4_packet(&pkt).unwrap();
        assert_eq!(h.header_length_bytes, 20);
        assert_eq!(h.protocol, 17);
        assert_eq!(h.source_ip, [10, 0, 0, 1]);
        assert_eq!(h.payload, b"abc");
    }

    #[test]
    fn rejects_ihl_below_five() {
        // Lỗi cũ: IHL = 2 được chấp nhận và "payload" bắt đầu từ byte 8 — chồng lên tiêu đề
        for ihl in 0..5 {
            assert!(
                parse_ipv4_packet(&ipv4(ihl, 24, b"data")).is_err(),
                "IHL = {ihl}"
            );
        }
    }

    #[test]
    fn rejects_ihl_longer_than_packet_and_truncated_packets() {
        assert!(parse_ipv4_packet(&ipv4(15, 60, b"")).is_err()); // khai 60 byte, chỉ có 20
        assert!(parse_ipv4_packet(&ipv4(5, 100, b"abc")).is_err()); // Total Length > thực tế
        assert!(parse_ipv4_packet(&ipv4(5, 10, b"abc")).is_err()); // Total Length < tiêu đề
    }

    #[test]
    fn payload_excludes_link_layer_padding() {
        // Khung Ethernet tối thiểu 60 byte: gói IP 23 byte được đệm thêm số 0 ở cuối
        let mut pkt = ipv4(5, 23, b"abc");
        pkt.resize(46, 0);
        assert_eq!(parse_ipv4_packet(&pkt).unwrap().payload, b"abc");
    }

    fn elf(class: u8, data: u8, len: usize) -> Vec<u8> {
        let mut v = vec![0u8; len];
        v[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
        v[4] = class;
        v[5] = data;
        v[6] = 1;
        v
    }

    #[test]
    fn elf64_little_endian_entry() {
        let mut v = elf(2, 1, 64);
        v[18..20].copy_from_slice(&0xB7u16.to_le_bytes());
        v[24..32].copy_from_slice(&0x0040_1000u64.to_le_bytes());
        let h = parse_elf_header(&v).unwrap();
        assert_eq!(h.entry_point_address, 0x0040_1000);
        assert_eq!(h.machine, "AArch64 (ARM64)");
    }

    #[test]
    fn elf32_reads_four_byte_entry() {
        // Lỗi cũ: luôn đọc 8 byte ở offset 24 -> với ELF32 lẫn cả e_phoff vào e_entry
        let mut v = elf(1, 1, 52);
        v[18..20].copy_from_slice(&0x03u16.to_le_bytes());
        v[24..28].copy_from_slice(&0x0804_8000u32.to_le_bytes());
        v[28..32].copy_from_slice(&0x34u32.to_le_bytes()); // e_phoff = 52
        let h = parse_elf_header(&v).unwrap();
        assert_eq!(h.entry_point_address, 0x0804_8000);
        assert_eq!(h.machine, "x86 (i386)");
    }

    #[test]
    fn elf_big_endian_is_honoured() {
        // Lỗi cũ: luôn from_le_bytes kể cả khi EI_DATA = 2 (Big-Endian)
        let mut v = elf(2, 2, 64);
        v[18..20].copy_from_slice(&0x14u16.to_be_bytes());
        v[24..32].copy_from_slice(&0x1000_0000u64.to_be_bytes());
        let h = parse_elf_header(&v).unwrap();
        assert_eq!(h.entry_point_address, 0x1000_0000);
        assert_eq!(h.machine, "PowerPC");
    }

    #[test]
    fn elf_rejects_bad_class_data_and_truncation() {
        assert!(parse_elf_header(&elf(3, 1, 64)).is_err());
        assert!(parse_elf_header(&elf(2, 0, 64)).is_err());
        assert!(parse_elf_header(&elf(2, 1, 40)).is_err()); // ELF64 cần 64 byte
        assert!(parse_elf_header(b"MZ not an elf file at all").is_err());
    }
}
