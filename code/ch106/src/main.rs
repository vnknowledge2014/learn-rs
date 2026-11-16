//! # Chương 106: An ninh Không dây & Phòng lab OSWP Tổng hợp
//!
//! Crate này mô phỏng RF, beacon 802.11, hệ thống khoá WPA2 và phân tích cấu hình
//! bằng Rust 2024 thuần `std`. Mọi capture, danh tính và mật khẩu đều tổng hợp.
//! Không có interface radio, socket, lệnh hệ điều hành hay dữ liệu mục tiêu thật.

#![forbid(unsafe_code)]
#![allow(dead_code)]

use std::error::Error;
use std::fmt;
use std::str::FromStr;

// ============================================================================
// 1. ĐỊA CHỈ MAC VÀ PHÉP ĐO RF
// ============================================================================

/// Địa chỉ MAC 48-bit của một thiết bị tổng hợp.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    /// Tạo địa chỉ từ đúng sáu octet.
    pub const fn new(octets: [u8; 6]) -> Self {
        Self(octets)
    }

    /// Trả về sáu octet theo thứ tự truyền.
    pub const fn octets(self) -> [u8; 6] {
        self.0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let o = self.0;
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            o[0], o[1], o[2], o[3], o[4], o[5]
        )
    }
}

/// Lỗi phân tích địa chỉ MAC dạng `xx:xx:xx:xx:xx:xx`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MacAddressParseError;

impl fmt::Display for MacAddressParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("địa chỉ MAC phải gồm sáu octet hex, ngăn cách bằng dấu hai chấm")
    }
}

impl Error for MacAddressParseError {}

impl FromStr for MacAddress {
    type Err = MacAddressParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = text.split(':').collect();
        if parts.len() != 6 || parts.iter().any(|part| part.len() != 2) {
            return Err(MacAddressParseError);
        }
        let mut octets = [0u8; 6];
        for (slot, part) in octets.iter_mut().zip(parts) {
            *slot = u8::from_str_radix(part, 16).map_err(|_| MacAddressParseError)?;
        }
        Ok(Self(octets))
    }
}

/// Công suất theo dBm, được kiểm tra là số hữu hạn trong miền biểu diễn của lab.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Dbm(f64);

impl Dbm {
    /// Tạo giá trị dBm hữu hạn từ -150 đến +30 dBm.
    pub fn new(value: f64) -> Result<Self, MeasurementError> {
        if !value.is_finite() {
            return Err(MeasurementError::NonFinite);
        }
        if !(-150.0..=30.0).contains(&value) {
            return Err(MeasurementError::OutOfRange);
        }
        Ok(Self(value))
    }

    /// Đổi công suất mW sang dBm bằng `10 * log10(P_mW)`.
    pub fn from_milliwatts(power_mw: f64) -> Result<Self, MeasurementError> {
        if !power_mw.is_finite() {
            return Err(MeasurementError::NonFinite);
        }
        if power_mw <= 0.0 {
            return Err(MeasurementError::InvalidPower);
        }
        Self::new(10.0 * power_mw.log10())
    }

    /// Giá trị số thực theo dBm.
    pub const fn value(self) -> f64 {
        self.0
    }
}

/// Tỷ số tín hiệu trên nhiễu theo dB.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Snr(f64);

impl Snr {
    /// Giá trị SNR theo dB.
    pub const fn db(self) -> f64 {
        self.0
    }

    /// Tỷ số công suất tuyến tính tương ứng: `10^(SNR_dB / 10)`.
    pub fn linear_ratio(self) -> f64 {
        10_f64.powf(self.0 / 10.0)
    }
}

/// Một mẫu RSSI và noise floor tổng hợp, cả hai cùng đơn vị dBm.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalMeasurement {
    rssi: Dbm,
    noise_floor: Dbm,
}

impl SignalMeasurement {
    /// Kiểm tra phép đo đầu thu Wi-Fi và ràng buộc RSSI không thấp hơn nhiễu nền.
    pub fn new(rssi: Dbm, noise_floor: Dbm) -> Result<Self, MeasurementError> {
        for value in [rssi.value(), noise_floor.value()] {
            if !(-127.0..=0.0).contains(&value) {
                return Err(MeasurementError::OutOfRange);
            }
        }
        if rssi.value() < noise_floor.value() {
            return Err(MeasurementError::SignalBelowNoise);
        }
        Ok(Self { rssi, noise_floor })
    }

    /// Constructor thuận tiện nhận hai số dBm.
    pub fn from_dbm(rssi_dbm: f64, noise_floor_dbm: f64) -> Result<Self, MeasurementError> {
        Self::new(Dbm::new(rssi_dbm)?, Dbm::new(noise_floor_dbm)?)
    }

    /// RSSI đã kiểm tra.
    pub const fn rssi(self) -> Dbm {
        self.rssi
    }

    /// Noise floor đã kiểm tra.
    pub const fn noise_floor(self) -> Dbm {
        self.noise_floor
    }

    /// SNR theo dB: RSSI(dBm) trừ noise floor(dBm).
    pub fn snr(self) -> Snr {
        Snr(self.rssi.value() - self.noise_floor.value())
    }
}

/// Lỗi bất biến của phép đo tín hiệu.
#[derive(Debug, Clone, PartialEq)]
pub enum MeasurementError {
    /// Giá trị NaN hoặc vô cực.
    NonFinite,
    /// Ngoài miền được mô hình hóa.
    OutOfRange,
    /// Công suất mW phải lớn hơn không.
    InvalidPower,
    /// Không thể lấy logarit hoặc so SNR khi tín hiệu thấp hơn nhiễu nền.
    SignalBelowNoise,
}

impl fmt::Display for MeasurementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::NonFinite => "phép đo phải là số hữu hạn",
            Self::OutOfRange => "phép đo nằm ngoài miền cho phép",
            Self::InvalidPower => "công suất mW phải lớn hơn không",
            Self::SignalBelowNoise => "RSSI không được thấp hơn noise floor trong mô hình này",
        };
        f.write_str(text)
    }
}

impl Error for MeasurementError {}

// ============================================================================
// 2. BĂNG TẦN, KÊNH, ĐỘ RỘNG KÊNH VÀ PHY
// ============================================================================

/// Ba băng tần được dùng trong mô hình Wi-Fi hiện đại.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Band {
    /// 2.4 GHz.
    Ghz2_4,
    /// 5 GHz.
    Ghz5,
    /// 6 GHz.
    Ghz6,
}

impl fmt::Display for Band {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ghz2_4 => "2.4 GHz",
            Self::Ghz5 => "5 GHz",
            Self::Ghz6 => "6 GHz",
        })
    }
}

/// Độ rộng phổ của một kênh Wi-Fi.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelWidth {
    /// Một kênh 20 MHz.
    Mhz20,
    /// Hai kênh 20 MHz liên kết.
    Mhz40,
    /// Bốn kênh 20 MHz liên kết.
    Mhz80,
    /// Tám kênh 20 MHz liên kết.
    Mhz160,
}

impl ChannelWidth {
    /// Độ rộng theo MHz.
    pub const fn mhz(self) -> u16 {
        match self {
            Self::Mhz20 => 20,
            Self::Mhz40 => 40,
            Self::Mhz80 => 80,
            Self::Mhz160 => 160,
        }
    }

    /// Số kênh con 20 MHz.
    pub const fn bonded_count(self) -> usize {
        (self.mhz() / 20) as usize
    }
}

/// Chế độ ghép kênh/truy cập phổ của AP.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhyMode {
    /// OFDM: một lần truyền dùng toàn bộ kênh tại một thời điểm.
    Ofdm,
    /// OFDMA: chia kênh thành resource unit cho nhiều client trong cùng lần truyền.
    Ofdma,
}

/// Kênh primary và nhóm kênh liên kết đã được kiểm tra.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Channel {
    band: Band,
    number: u16,
    width: ChannelWidth,
    bonded: Vec<u16>,
}

impl Channel {
    /// Tạo kênh và xác định nhóm bonding. Kênh 2.4 GHz 40 MHz trong lab dùng
    /// primary thấp và secondary cao hơn bốn số kênh để giữ mô hình nhỏ, rõ ràng.
    pub fn new(band: Band, number: u16, width: ChannelWidth) -> Result<Self, ChannelError> {
        let bonded = bonded_channels_for(band, number, width)?;
        Ok(Self {
            band,
            number,
            width,
            bonded,
        })
    }

    /// Băng tần.
    pub const fn band(&self) -> Band {
        self.band
    }

    /// Số kênh primary.
    pub const fn number(&self) -> u16 {
        self.number
    }

    /// Độ rộng kênh.
    pub const fn width(&self) -> ChannelWidth {
        self.width
    }

    /// Các kênh 20 MHz tạo nên kênh hiện tại.
    pub fn bonded_channels(&self) -> &[u16] {
        &self.bonded
    }
}

/// Lỗi cấu hình kênh/độ rộng kênh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    /// Kênh không tồn tại trong băng tần được mô hình hóa.
    UnsupportedChannel { band: Band, number: u16 },
    /// Băng tần không hỗ trợ độ rộng này.
    UnsupportedWidth { band: Band, width: ChannelWidth },
    /// Primary channel không thuộc một nhóm bonding hoàn chỉnh.
    InvalidBonding { number: u16, width: ChannelWidth },
}

impl fmt::Display for ChannelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedChannel { band, number } => {
                write!(f, "kênh {number} không thuộc {band}")
            }
            Self::UnsupportedWidth { band, width } => {
                write!(f, "{band} không hỗ trợ độ rộng {} MHz", width.mhz())
            }
            Self::InvalidBonding { number, width } => write!(
                f,
                "kênh {number} không tạo được nhóm {} MHz hoàn chỉnh",
                width.mhz()
            ),
        }
    }
}

impl Error for ChannelError {}

const BAND5_CHANNELS: &[u16] = &[
    36, 40, 44, 48, 52, 56, 60, 64, 100, 104, 108, 112, 116, 120, 124, 128, 132, 136, 140, 144,
    149, 153, 157, 161, 165,
];
const BAND5_40: &[&[u16]] = &[
    &[36, 40],
    &[44, 48],
    &[52, 56],
    &[60, 64],
    &[100, 104],
    &[108, 112],
    &[116, 120],
    &[124, 128],
    &[132, 136],
    &[140, 144],
    &[149, 153],
    &[157, 161],
];
const BAND5_80: &[&[u16]] = &[
    &[36, 40, 44, 48],
    &[52, 56, 60, 64],
    &[100, 104, 108, 112],
    &[116, 120, 124, 128],
    &[132, 136, 140, 144],
    &[149, 153, 157, 161],
];
const BAND5_160: &[&[u16]] = &[
    &[36, 40, 44, 48, 52, 56, 60, 64],
    &[100, 104, 108, 112, 116, 120, 124, 128],
];

fn band_supports_channel(band: Band, number: u16) -> bool {
    match band {
        Band::Ghz2_4 => (1..=14).contains(&number),
        Band::Ghz5 => BAND5_CHANNELS.contains(&number),
        Band::Ghz6 => (1..=233).contains(&number) && (number - 1).is_multiple_of(4),
    }
}

fn find_band5_group(number: u16, groups: &[&[u16]]) -> Option<Vec<u16>> {
    groups
        .iter()
        .find(|group| group.contains(&number))
        .map(|group| group.to_vec())
}

fn bonded_channels_for(
    band: Band,
    number: u16,
    width: ChannelWidth,
) -> Result<Vec<u16>, ChannelError> {
    if !band_supports_channel(band, number) {
        return Err(ChannelError::UnsupportedChannel { band, number });
    }
    if width == ChannelWidth::Mhz20 {
        return Ok(vec![number]);
    }
    if band == Band::Ghz2_4 && !matches!(width, ChannelWidth::Mhz40) {
        return Err(ChannelError::UnsupportedWidth { band, width });
    }

    match band {
        Band::Ghz2_4 => {
            if number > 9 {
                return Err(ChannelError::InvalidBonding { number, width });
            }
            Ok(vec![number, number + 4])
        }
        Band::Ghz5 => {
            let groups = match width {
                ChannelWidth::Mhz40 => BAND5_40,
                ChannelWidth::Mhz80 => BAND5_80,
                ChannelWidth::Mhz160 => BAND5_160,
                ChannelWidth::Mhz20 => unreachable!("20 MHz đã được xử lý"),
            };
            find_band5_group(number, groups).ok_or(ChannelError::InvalidBonding { number, width })
        }
        Band::Ghz6 => {
            let count = width.bonded_count();
            if (width == ChannelWidth::Mhz40 && number > 229)
                || (matches!(width, ChannelWidth::Mhz80 | ChannelWidth::Mhz160) && number > 221)
            {
                return Err(ChannelError::InvalidBonding { number, width });
            }
            let span = (count * 2) as u16;
            let base = 1 + ((number - 1) / span) * span;
            let group: Vec<u16> = (0..count).map(|index| base + (index as u16) * 4).collect();
            if group.len() == count && group.contains(&number) {
                Ok(group)
            } else {
                Err(ChannelError::InvalidBonding { number, width })
            }
        }
    }
}

// ============================================================================
// 3. PARSER QUẢN LÝ 802.11 CÓ GIỚI HẠN
// ============================================================================

/// Loại frame ở hai bit type của Frame Control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// Management frame.
    Management,
    /// Control frame.
    Control,
    /// Data frame.
    Data,
}

/// Beacon đã được giải mã từ một subset 802.11 rõ ràng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Beacon {
    /// Timestamp tám byte của beacon.
    pub timestamp: u64,
    /// Beacon interval theo time unit; phải khác không khi encode.
    pub interval_tu: u16,
    /// Capability Information thô.
    pub capabilities: u16,
    /// SSID dạng UTF-8; chuỗi rỗng biểu diễn hidden SSID.
    pub ssid: String,
    /// Kênh trong DS Parameter Set của fixture 2.4 GHz.
    pub channel: u8,
    /// BSSID lấy từ Address 3.
    pub bssid: MacAddress,
}

impl Beacon {
    /// Tạo beacon tổng hợp với timestamp cố định để test tái lập.
    pub fn synthetic(ssid: &str, channel: u8, bssid: MacAddress) -> Self {
        Self {
            timestamp: 0x0102_0304_0506_0708,
            interval_tu: 100,
            capabilities: 0x0411,
            ssid: ssid.to_string(),
            channel,
            bssid,
        }
    }

    /// Parse một management frame và chỉ chấp nhận subtype Beacon (8).
    pub fn parse(bytes: &[u8]) -> Result<Self, FrameError> {
        let frame = WirelessFrame::parse(bytes)?;
        match frame.body {
            ManagementBody::Beacon(beacon) => Ok(beacon),
            _ => Err(FrameError::UnexpectedSubtype {
                expected: 8,
                actual: frame.subtype,
            }),
        }
    }

    /// Encode fixture beacon, không phải phát beacon ra radio.
    pub fn encode_synthetic(&self, sequence: u16, retry: bool) -> Result<Vec<u8>, FrameError> {
        if self.ssid.len() > 32 {
            return Err(FrameError::InvalidSsidLength(self.ssid.len()));
        }
        if self.interval_tu == 0 {
            return Err(FrameError::InvalidBeaconInterval);
        }
        if self.channel == 0 {
            return Err(FrameError::InvalidChannelNumber(self.channel));
        }
        if sequence > 0x0fff {
            return Err(FrameError::InvalidSequenceNumber(sequence));
        }

        let mut bytes = Vec::with_capacity(38 + self.ssid.len());
        let frame_control = 0x0080_u16 | if retry { 0x0800 } else { 0 };
        bytes.extend_from_slice(&frame_control.to_le_bytes());
        bytes.extend_from_slice(&0_u16.to_le_bytes());
        bytes.extend_from_slice(&[0xff; 6]);
        bytes.extend_from_slice(&self.bssid.octets());
        bytes.extend_from_slice(&self.bssid.octets());
        bytes.extend_from_slice(&(sequence << 4).to_le_bytes());
        bytes.extend_from_slice(&self.timestamp.to_le_bytes());
        bytes.extend_from_slice(&self.interval_tu.to_le_bytes());
        bytes.extend_from_slice(&self.capabilities.to_le_bytes());
        bytes.push(0);
        bytes.push(self.ssid.len() as u8);
        bytes.extend_from_slice(self.ssid.as_bytes());
        bytes.extend_from_slice(&[3, 1, self.channel]);
        Ok(bytes)
    }
}

/// Phần thân management frame mà parser hiểu tường minh.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ManagementBody {
    /// Beacon với SSID và DS Parameter Set bắt buộc.
    Beacon(Beacon),
    /// Probe Request chỉ chứa SSID IE; `None` là wildcard/hidden SSID.
    ProbeRequest { ssid: Option<String> },
    /// Authentication frame với ba trường cố định.
    Authentication {
        /// 0 là Open System, 1 là Shared Key (lịch sử).
        algorithm: u16,
        /// Số bước xác thực.
        transaction: u16,
        /// Status code.
        status: u16,
    },
    /// Subtype khác được giữ nguyên payload, không suy diễn semantics.
    Other { subtype: u8, payload: Vec<u8> },
}

/// Management frame có header 24 byte và thông tin sequence/retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WirelessFrame {
    /// Luôn là `Management` trong parser có giới hạn này.
    pub frame_type: FrameType,
    /// Subtype bốn bit.
    pub subtype: u8,
    /// Duration/ID thô.
    pub duration: u16,
    /// Address 1: receiver/destination.
    pub destination: MacAddress,
    /// Address 2: transmitter/source.
    pub source: MacAddress,
    /// Address 3: BSSID trong fixture.
    pub bssid: MacAddress,
    /// Sequence number 12 bit.
    pub sequence: u16,
    /// Fragment number 4 bit.
    pub fragment: u8,
    /// Cờ retry trong Frame Control.
    pub retry: bool,
    /// Thân management frame đã phân loại.
    pub body: ManagementBody,
}

impl WirelessFrame {
    /// Parse management frame theo subset được mô tả trong chương. Control/Data
    /// bị từ chối thay vì bị hiểu nhầm là beacon.
    pub fn parse(bytes: &[u8]) -> Result<Self, FrameError> {
        if bytes.len() < 24 {
            return Err(FrameError::Truncated {
                needed: 24,
                actual: bytes.len(),
            });
        }
        let fc = u16::from_le_bytes([bytes[0], bytes[1]]);
        if fc & 0b11 != 0 {
            return Err(FrameError::UnsupportedProtocolVersion((fc & 0b11) as u8));
        }
        let frame_type = match (fc >> 2) & 0b11 {
            0 => FrameType::Management,
            1 => FrameType::Control,
            2 => FrameType::Data,
            value => return Err(FrameError::UnsupportedFrameType(value as u8)),
        };
        if frame_type != FrameType::Management {
            return Err(FrameError::UnsupportedFrameType((fc >> 2) as u8 & 0b11));
        }
        let subtype = ((fc >> 4) & 0x0f) as u8;
        let destination =
            MacAddress::new(bytes[4..10].try_into().map_err(|_| FrameError::Truncated {
                needed: 10,
                actual: bytes.len(),
            })?);
        let source =
            MacAddress::new(
                bytes[10..16]
                    .try_into()
                    .map_err(|_| FrameError::Truncated {
                        needed: 16,
                        actual: bytes.len(),
                    })?,
            );
        let bssid =
            MacAddress::new(
                bytes[16..22]
                    .try_into()
                    .map_err(|_| FrameError::Truncated {
                        needed: 22,
                        actual: bytes.len(),
                    })?,
            );
        let sequence_control = u16::from_le_bytes([bytes[22], bytes[23]]);
        let body_bytes = &bytes[24..];
        let body = match subtype {
            8 => ManagementBody::Beacon(parse_beacon_body(body_bytes, bssid)?),
            4 => ManagementBody::ProbeRequest {
                ssid: parse_probe_ssid(body_bytes)?,
            },
            11 => parse_authentication_body(body_bytes)?,
            _ => ManagementBody::Other {
                subtype,
                payload: body_bytes.to_vec(),
            },
        };
        Ok(Self {
            frame_type,
            subtype,
            duration: u16::from_le_bytes([bytes[2], bytes[3]]),
            destination,
            source,
            bssid,
            sequence: sequence_control >> 4,
            fragment: (sequence_control & 0x0f) as u8,
            retry: fc & 0x0800 != 0,
            body,
        })
    }
}

/// Lỗi parser, luôn trả kết quả thay vì panic trên dữ liệu xấu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    /// Buffer kết thúc trước khi đủ trường.
    Truncated { needed: usize, actual: usize },
    /// Chỉ protocol version 0 được hỗ trợ.
    UnsupportedProtocolVersion(u8),
    /// Parser không nhận Control/Data hoặc type dành riêng.
    UnsupportedFrameType(u8),
    /// `Beacon::parse` nhận subtype khác beacon.
    UnexpectedSubtype { expected: u8, actual: u8 },
    /// IE chưa được subset này hỗ trợ.
    UnsupportedInformationElement(u8),
    /// Thiếu SSID hoặc DS Parameter Set.
    MissingInformationElement(u8),
    /// Lặp một IE chỉ được phép xuất hiện một lần.
    DuplicateInformationElement(u8),
    /// Độ dài IE không đúng.
    InvalidInformationElementLength { tag: u8, length: usize },
    /// SSID vượt 32 byte.
    InvalidSsidLength(usize),
    /// SSID trong subset giáo trình phải là UTF-8.
    InvalidUtf8Ssid,
    /// Beacon interval bằng không.
    InvalidBeaconInterval,
    /// Kênh zero không hợp lệ trong fixture.
    InvalidChannelNumber(u8),
    /// Sequence number của 802.11 chỉ có 12 bit.
    InvalidSequenceNumber(u16),
    /// Còn byte sau khi đã đọc đủ subset; không bỏ qua dữ liệu lạ.
    TrailingBytes(usize),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "frame tổng hợp không hợp lệ: {self:?}")
    }
}

impl Error for FrameError {}

fn read_ie<'a>(bytes: &'a [u8], cursor: &mut usize) -> Result<(u8, &'a [u8]), FrameError> {
    if bytes.len().saturating_sub(*cursor) < 2 {
        return Err(FrameError::Truncated {
            needed: cursor.saturating_add(2),
            actual: bytes.len(),
        });
    }
    let tag = bytes[*cursor];
    let length = bytes[*cursor + 1] as usize;
    let start = *cursor + 2;
    let end = start.checked_add(length).ok_or(FrameError::Truncated {
        needed: usize::MAX,
        actual: bytes.len(),
    })?;
    if end > bytes.len() {
        return Err(FrameError::Truncated {
            needed: end,
            actual: bytes.len(),
        });
    }
    *cursor = end;
    Ok((tag, &bytes[start..end]))
}

fn decode_ssid(bytes: &[u8]) -> Result<String, FrameError> {
    if bytes.len() > 32 {
        return Err(FrameError::InvalidSsidLength(bytes.len()));
    }
    std::str::from_utf8(bytes)
        .map(str::to_owned)
        .map_err(|_| FrameError::InvalidUtf8Ssid)
}

fn parse_beacon_body(bytes: &[u8], bssid: MacAddress) -> Result<Beacon, FrameError> {
    if bytes.len() < 12 {
        return Err(FrameError::Truncated {
            needed: 36,
            actual: bytes.len() + 24,
        });
    }
    let timestamp =
        u64::from_le_bytes(bytes[0..8].try_into().map_err(|_| FrameError::Truncated {
            needed: 32,
            actual: bytes.len() + 24,
        })?);
    let interval_tu = u16::from_le_bytes([bytes[8], bytes[9]]);
    if interval_tu == 0 {
        return Err(FrameError::InvalidBeaconInterval);
    }
    let capabilities = u16::from_le_bytes([bytes[10], bytes[11]]);
    let mut cursor = 12;
    let mut ssid = None;
    let mut channel = None;
    while ssid.is_none() || channel.is_none() {
        if cursor == bytes.len() {
            return Err(FrameError::MissingInformationElement(if ssid.is_none() {
                0
            } else {
                3
            }));
        }
        let (tag, value) = read_ie(bytes, &mut cursor)?;
        match tag {
            0 if ssid.is_some() => return Err(FrameError::DuplicateInformationElement(0)),
            3 if channel.is_some() => return Err(FrameError::DuplicateInformationElement(3)),
            0 => ssid = Some(decode_ssid(value)?),
            3 if value.len() == 1 => {
                if value[0] == 0 {
                    return Err(FrameError::InvalidChannelNumber(0));
                }
                channel = Some(value[0]);
            }
            3 => {
                return Err(FrameError::InvalidInformationElementLength {
                    tag,
                    length: value.len(),
                });
            }
            other => return Err(FrameError::UnsupportedInformationElement(other)),
        }
    }
    if cursor != bytes.len() {
        return Err(FrameError::TrailingBytes(bytes.len() - cursor));
    }
    Ok(Beacon {
        timestamp,
        interval_tu,
        capabilities,
        ssid: ssid.unwrap_or_default(),
        channel: channel.unwrap_or_default(),
        bssid,
    })
}

fn parse_probe_ssid(bytes: &[u8]) -> Result<Option<String>, FrameError> {
    let mut cursor = 0;
    let (tag, value) = read_ie(bytes, &mut cursor)?;
    if tag != 0 {
        return Err(FrameError::MissingInformationElement(0));
    }
    if cursor != bytes.len() {
        return Err(FrameError::TrailingBytes(bytes.len() - cursor));
    }
    let ssid = decode_ssid(value)?;
    Ok(if ssid.is_empty() { None } else { Some(ssid) })
}

fn parse_authentication_body(bytes: &[u8]) -> Result<ManagementBody, FrameError> {
    if bytes.len() < 6 {
        return Err(FrameError::Truncated {
            needed: 30,
            actual: bytes.len() + 24,
        });
    }
    if bytes.len() > 6 {
        return Err(FrameError::TrailingBytes(bytes.len() - 6));
    }
    Ok(ManagementBody::Authentication {
        algorithm: u16::from_le_bytes([bytes[0], bytes[1]]),
        transaction: u16::from_le_bytes([bytes[2], bytes[3]]),
        status: u16::from_le_bytes([bytes[4], bytes[5]]),
    })
}

// ============================================================================
// 4. SHA-1, HMAC, PBKDF2 VÀ HỆ THỐNG KHOÁ WPA2 (CLEAN-ROOM)
// ============================================================================

/// SHA-1 theo FIPS 180-4, dùng cho HMAC-SHA1 của WPA2; không dùng để băm mật khẩu mới.
pub fn sha1(data: &[u8]) -> [u8; 20] {
    let mut state = [
        0x6745_2301_u32,
        0xefcd_ab89,
        0x98ba_dcfe,
        0x1032_5476,
        0xc3d2_e1f0,
    ];
    let bit_length = (data.len() as u64).wrapping_mul(8);
    let mut padded = data.to_vec();
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_length.to_be_bytes());

    for block in padded.as_chunks::<64>().0 {
        let mut words = [0_u32; 80];
        for (index, word) in words.iter_mut().take(16).enumerate() {
            let offset = index * 4;
            *word = u32::from_be_bytes([
                block[offset],
                block[offset + 1],
                block[offset + 2],
                block[offset + 3],
            ]);
        }
        for index in 16..80 {
            words[index] =
                (words[index - 3] ^ words[index - 8] ^ words[index - 14] ^ words[index - 16])
                    .rotate_left(1);
        }
        let [mut a, mut b, mut c, mut d, mut e] = state;
        for (index, word) in words.iter().enumerate() {
            let (function, constant) = match index {
                0..=19 => ((b & c) | ((!b) & d), 0x5a82_7999),
                20..=39 => (b ^ c ^ d, 0x6ed9_eba1),
                40..=59 => ((b & c) | (b & d) | (c & d), 0x8f1b_bcdc),
                _ => (b ^ c ^ d, 0xca62_c1d6),
            };
            let temporary = a
                .rotate_left(5)
                .wrapping_add(function)
                .wrapping_add(e)
                .wrapping_add(constant)
                .wrapping_add(*word);
            e = d;
            d = c;
            c = b.rotate_left(30);
            b = a;
            a = temporary;
        }
        for (slot, value) in state.iter_mut().zip([a, b, c, d, e]) {
            *slot = slot.wrapping_add(value);
        }
    }

    let mut output = [0_u8; 20];
    for (chunk, value) in output.as_chunks_mut::<4>().0.iter_mut().zip(state) {
        chunk.copy_from_slice(&value.to_be_bytes());
    }
    output
}

/// HMAC-SHA1 theo RFC 2104 với block size 64 byte.
pub fn hmac_sha1(key: &[u8], message: &[u8]) -> [u8; 20] {
    let mut normalized = [0_u8; 64];
    if key.len() > 64 {
        normalized[..20].copy_from_slice(&sha1(key));
    } else {
        normalized[..key.len()].copy_from_slice(key);
    }
    let mut inner_pad = [0x36_u8; 64];
    let mut outer_pad = [0x5c_u8; 64];
    for index in 0..64 {
        inner_pad[index] ^= normalized[index];
        outer_pad[index] ^= normalized[index];
    }
    let mut inner = Vec::with_capacity(64 + message.len());
    inner.extend_from_slice(&inner_pad);
    inner.extend_from_slice(message);
    let inner_hash = sha1(&inner);
    let mut outer = Vec::with_capacity(84);
    outer.extend_from_slice(&outer_pad);
    outer.extend_from_slice(&inner_hash);
    sha1(&outer)
}

/// Lỗi tham số KDF hoặc passphrase/SSID WPA2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KdfError {
    /// PBKDF2 không chấp nhận zero vòng lặp.
    ZeroIterations,
    /// Output rỗng không tạo thành khoá.
    EmptyOutput,
    /// Số block vượt giới hạn 32-bit của PBKDF2.
    OutputTooLong,
    /// Passphrase WPA2 phải gồm 8–63 ký tự ASCII in được.
    InvalidPassphrase,
    /// SSID dùng làm salt phải dài 1–32 byte.
    InvalidSsid,
}

impl fmt::Display for KdfError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "tham số KDF không hợp lệ: {self:?}")
    }
}

impl Error for KdfError {}

/// PBKDF2-HMAC-SHA1 theo RFC 8018. Các block `F_i` được GHÉP NỐI;
/// chỉ các giá trị `U_j` trong cùng một block mới được XOR.
pub fn pbkdf2_hmac_sha1(
    password: &[u8],
    salt: &[u8],
    iterations: u32,
    output: &mut [u8],
) -> Result<(), KdfError> {
    if iterations == 0 {
        return Err(KdfError::ZeroIterations);
    }
    if output.is_empty() {
        return Err(KdfError::EmptyOutput);
    }
    let blocks = output.len().div_ceil(20);
    if blocks > u32::MAX as usize {
        return Err(KdfError::OutputTooLong);
    }

    for block_index in 1..=blocks {
        let mut initial_salt = Vec::with_capacity(salt.len() + 4);
        initial_salt.extend_from_slice(salt);
        initial_salt.extend_from_slice(&(block_index as u32).to_be_bytes());
        let mut u = hmac_sha1(password, &initial_salt);
        let mut accumulator = u;
        for _ in 1..iterations {
            u = hmac_sha1(password, &u);
            for (target, value) in accumulator.iter_mut().zip(u) {
                *target ^= value;
            }
        }
        let start = (block_index - 1) * 20;
        let length = (output.len() - start).min(20);
        output[start..start + length].copy_from_slice(&accumulator[..length]);
    }
    Ok(())
}

/// Suy ra PMK WPA2 32 byte: PBKDF2-HMAC-SHA1(password, SSID, 4096, 256 bit).
pub fn derive_pmk(password: &str, ssid: &str) -> Result<[u8; 32], KdfError> {
    if !(8..=63).contains(&password.len())
        || !password.bytes().all(|byte| (0x20..=0x7e).contains(&byte))
    {
        return Err(KdfError::InvalidPassphrase);
    }
    if !(1..=32).contains(&ssid.len()) || !ssid.is_char_boundary(ssid.len()) {
        return Err(KdfError::InvalidSsid);
    }
    let mut pmk = [0_u8; 32];
    pbkdf2_hmac_sha1(password.as_bytes(), ssid.as_bytes(), 4096, &mut pmk)?;
    Ok(pmk)
}

/// PTK 64 byte từ PRF-512; với CCMP chỉ 48 byte đầu (KCK/KEK/TK 16 byte) được dùng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ptk([u8; 64]);

impl Ptk {
    /// Toàn bộ output PRF-512 để quan sát tính tất định.
    pub const fn as_bytes(&self) -> &[u8; 64] {
        &self.0
    }

    /// Key Confirmation Key, dùng tính MIC.
    pub fn kck(&self) -> [u8; 16] {
        self.0[..16].try_into().unwrap_or([0; 16])
    }

    /// Key Encryption Key, bảo vệ phân phối khoá nhóm.
    pub fn kek(&self) -> [u8; 16] {
        self.0[16..32].try_into().unwrap_or([0; 16])
    }

    /// Phần temporal-key còn lại; CCMP lấy 16 byte đầu của phần này.
    pub fn temporal_key_material(&self) -> [u8; 32] {
        self.0[32..].try_into().unwrap_or([0; 32])
    }
}

fn prf_sha1(key: &[u8], label: &[u8], data: &[u8], output: &mut [u8]) {
    for (counter, chunk) in output.chunks_mut(20).enumerate() {
        let mut input = Vec::with_capacity(label.len() + data.len() + 2);
        input.extend_from_slice(label);
        input.push(0);
        input.extend_from_slice(data);
        input.push(counter as u8);
        let digest = hmac_sha1(key, &input);
        chunk.copy_from_slice(&digest[..chunk.len()]);
    }
}

/// Suy ra PTK từ PMK, địa chỉ và nonce. Min/Max theo thứ tự byte loại bỏ
/// sự khác biệt giữa hai phía khi cùng tính một khoá.
pub fn derive_ptk(
    pmk: &[u8; 32],
    ap_mac: MacAddress,
    client_mac: MacAddress,
    anonce: [u8; 32],
    snonce: [u8; 32],
) -> Ptk {
    let ap = ap_mac.octets();
    let client = client_mac.octets();
    let mut data = Vec::with_capacity(76);
    data.extend_from_slice(ap.min(client).as_slice());
    data.extend_from_slice(ap.max(client).as_slice());
    data.extend_from_slice(anonce.min(snonce).as_slice());
    data.extend_from_slice(anonce.max(snonce).as_slice());
    let mut bytes = [0_u8; 64];
    prf_sha1(pmk, b"Pairwise key expansion", &data, &mut bytes);
    Ptk(bytes)
}

/// So sánh byte có thời gian phụ thuộc độ dài, không dừng sớm theo nội dung.
pub fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    let mut difference = 0_u8;
    for (&a, &b) in left.iter().zip(right) {
        difference |= a ^ b;
    }
    difference == 0
}

// ============================================================================
// 5. HANDSHAKE, PMKID VÀ ĐÁNH GIÁ WORDLIST TỔNG HỢP
// ============================================================================

/// Bản ghi message 2 của 4-way handshake, với payload hoàn toàn tổng hợp.
/// Trường MIC trong payload được coi là zero trước khi tính toán.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HandshakeCapture {
    /// SSID là salt của PMK.
    pub ssid: String,
    /// Authenticator/AP MAC.
    pub ap_mac: MacAddress,
    /// Supplicant/client MAC.
    pub client_mac: MacAddress,
    /// Nonce của AP trong message 1.
    pub anonce: [u8; 32],
    /// Nonce của client trong message 2.
    pub snonce: [u8; 32],
    /// Replay counter của bản ghi giả.
    pub replay_counter: u64,
    /// Payload giả đã zero trường MIC; không phải capture từ radio.
    pub eapol_message: Vec<u8>,
    /// HMAC-SHA1-128 với KCK của PTK.
    pub mic: [u8; 16],
}

impl HandshakeCapture {
    /// Tạo handshake giả từ passphrase; không chứa hay đọc capture thật.
    pub fn synthetic(password: &str) -> Result<Self, KdfError> {
        Self::synthetic_with_parameters(
            password,
            "IEEE",
            MacAddress::new([0x02, 0, 0, 0, 0, 1]),
            MacAddress::new([0x02, 0, 0, 0, 0, 2]),
            [0x11; 32],
            [0x22; 32],
        )
    }

    /// Constructor tổng hợp có tham số rõ ràng, thuận tiện cho bài tập.
    pub fn synthetic_with_parameters(
        password: &str,
        ssid: &str,
        ap_mac: MacAddress,
        client_mac: MacAddress,
        anonce: [u8; 32],
        snonce: [u8; 32],
    ) -> Result<Self, KdfError> {
        let pmk = derive_pmk(password, ssid)?;
        let mut capture = Self {
            ssid: ssid.to_string(),
            ap_mac,
            client_mac,
            anonce,
            snonce,
            replay_counter: 2,
            eapol_message: b"SYNTHETIC-M2|WPA2-CCMP|MIC-FIELD=ZERO".to_vec(),
            mic: [0; 16],
        };
        capture.mic = capture.mic_with_pmk(&pmk);
        Ok(capture)
    }

    /// PTK tương ứng với một PMK ứng viên.
    pub fn derive_ptk(&self, pmk: &[u8; 32]) -> Ptk {
        derive_ptk(pmk, self.ap_mac, self.client_mac, self.anonce, self.snonce)
    }

    /// Tính MIC trên payload tổng hợp bằng KCK.
    pub fn mic_with_pmk(&self, pmk: &[u8; 32]) -> [u8; 16] {
        let ptk = self.derive_ptk(pmk);
        let digest = hmac_sha1(&ptk.kck(), &self.eapol_message);
        digest[..16].try_into().unwrap_or([0; 16])
    }

    /// Verify MIC bằng PMK đã có.
    pub fn verify_mic(&self, pmk: &[u8; 32]) -> bool {
        constant_time_eq(&self.mic, &self.mic_with_pmk(pmk))
    }

    /// Thử một passphrase. Sai định dạng passphrase hoặc sai MIC đều trả `false`.
    pub fn verify_password(&self, password: &str) -> bool {
        derive_pmk(password, &self.ssid).is_ok_and(|pmk| self.verify_mic(&pmk))
    }
}

/// Bản ghi PMKID, khác loại dữ liệu với 4-way handshake ở cấp kiểu Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PmkidRecord {
    /// SSID là salt của PMK.
    pub ssid: String,
    /// AP MAC.
    pub ap_mac: MacAddress,
    /// Client MAC.
    pub client_mac: MacAddress,
    /// HMAC-SHA1-128(PMK, "PMK Name" || AA || SPA).
    pub pmkid: [u8; 16],
}

impl PmkidRecord {
    /// Tạo PMKID giả từ cùng tham số xác định với handshake fixture.
    pub fn synthetic(password: &str) -> Result<Self, KdfError> {
        let mut record = Self {
            ssid: "IEEE".to_string(),
            ap_mac: MacAddress::new([0x02, 0, 0, 0, 0, 1]),
            client_mac: MacAddress::new([0x02, 0, 0, 0, 0, 2]),
            pmkid: [0; 16],
        };
        let pmk = derive_pmk(password, &record.ssid)?;
        let mut message = Vec::with_capacity(20);
        message.extend_from_slice(b"PMK Name");
        message.extend_from_slice(&record.ap_mac.octets());
        message.extend_from_slice(&record.client_mac.octets());
        let digest = hmac_sha1(&pmk, &message);
        record.pmkid.copy_from_slice(&digest[..16]);
        Ok(record)
    }

    /// Thử passphrase trên PMKID; không cần nonce hay payload handshake.
    pub fn verify_password(&self, password: &str) -> bool {
        let Ok(pmk) = derive_pmk(password, &self.ssid) else {
            return false;
        };
        let mut message = Vec::with_capacity(20);
        message.extend_from_slice(b"PMK Name");
        message.extend_from_slice(&self.ap_mac.octets());
        message.extend_from_slice(&self.client_mac.octets());
        let digest = hmac_sha1(&pmk, &message);
        constant_time_eq(&self.pmkid, &digest[..16])
    }
}

/// Nguồn bằng chứng offline, được giữ riêng thay vì trộn hai công thức.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureKind {
    /// Có nonces, địa chỉ, payload và MIC của handshake.
    FourWayHandshake,
    /// Chỉ có PMKID và hai địa chỉ.
    Pmkid,
}

/// Kết quả duyệt wordlist giả; chỉ trả chỉ số ứng viên, không mật khẩu thật.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfflineAssessment {
    /// Loại bằng chứng đã dùng.
    pub kind: CaptureKind,
    /// Số ứng viên đã thử, kể cả ứng viên đúng cuối cùng.
    pub candidates_tested: usize,
    /// Chỉ số ứng viên khớp, hoặc `None` nếu tất cả đều sai.
    pub matched_index: Option<usize>,
}

impl OfflineAssessment {
    /// Có ứng viên nào vượt qua kiểm tra không.
    pub const fn succeeded(&self) -> bool {
        self.matched_index.is_some()
    }
}

/// Các stage typed thay thế một chuỗi lệnh thu capture/bẻ khoá thật.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureStage {
    /// Input chỉ có thể là fixture trong bộ nhớ.
    SyntheticInput,
    /// Management frame đã được parse và kiểm tra.
    TypedFrames,
    /// Đã hoàn tất một phép đánh giá offline trên dữ liệu giả.
    OfflineAssessmentComplete,
}

// ============================================================================
// 6. MÔ HÌNH AP/CLIENT, EVIL-TWIN INDICATORS VÀ HARDENING
// ============================================================================

/// Các thế hệ cấu hình bảo mật được mô hình hóa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecuritySuite {
    /// Không xác thực/mã hoá.
    Open,
    /// WEP, chỉ còn ý nghĩa lịch sử.
    Wep,
    /// WPA Personal với TKIP/RC4, lỗi thời.
    WpaPersonal,
    /// WPA2 Personal với PSK.
    Wpa2Personal,
    /// WPA2 Enterprise với 802.1X/EAP.
    Wpa2Enterprise,
    /// WPA3 Personal với SAE.
    Wpa3Personal,
    /// WPA3 Enterprise.
    Wpa3Enterprise,
}

impl SecuritySuite {
    fn is_enterprise(self) -> bool {
        matches!(self, Self::Wpa2Enterprise | Self::Wpa3Enterprise)
    }

    fn strength(self) -> u8 {
        match self {
            Self::Open => 0,
            Self::Wep => 1,
            Self::WpaPersonal => 2,
            Self::Wpa2Personal | Self::Wpa2Enterprise => 3,
            Self::Wpa3Personal | Self::Wpa3Enterprise => 4,
        }
    }
}

/// Protected Management Frames: tắt, có hỗ trợ hoặc bắt buộc.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PmfMode {
    /// Không bảo vệ management frame.
    Disabled,
    /// Có thể negociate nhưng không bắt buộc.
    Capable,
    /// Bắt buộc PMF.
    Required,
}

impl PmfMode {
    fn strength(self) -> u8 {
        match self {
            Self::Disabled => 0,
            Self::Capable => 1,
            Self::Required => 2,
        }
    }
}

/// Chính sách EAP tổng hợp mà client phải ghim khi dùng Enterprise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnterprisePolicy {
    /// Tên CA được tin trong fixture, không phải chứng chỉ thật.
    pub trusted_ca: String,
    /// Tên server được mong đợi.
    pub server_name: String,
    /// EAP method được chấp nhận.
    pub eap_method: String,
}

/// Một AP trong snapshot tổng hợp.
#[derive(Debug, Clone, PartialEq)]
pub struct AccessPoint {
    /// SSID quảng bá.
    pub ssid: String,
    /// BSSID/địa chỉ radio.
    pub bssid: MacAddress,
    /// Kênh primary và bonding đã kiểm tra.
    pub channel: Channel,
    /// Security suite.
    pub security: SecuritySuite,
    /// Chính sách PMF.
    pub pmf: PmfMode,
    /// OFDM hoặc OFDMA.
    pub phy_mode: PhyMode,
    /// Chính sách Enterprise nếu có.
    pub enterprise_policy: Option<EnterprisePolicy>,
    /// Lịch sử tín hiệu tổng hợp theo thứ tự thời gian.
    pub signal_history: Vec<SignalMeasurement>,
}

impl AccessPoint {
    /// Constructor cho các trường bắt buộc; kiểm tra SSID 1–32 byte.
    pub fn new(
        ssid: &str,
        bssid: MacAddress,
        channel: Channel,
        security: SecuritySuite,
        pmf: PmfMode,
        phy_mode: PhyMode,
    ) -> Result<Self, AccessPointError> {
        if !(1..=32).contains(&ssid.len()) {
            return Err(AccessPointError::InvalidSsid);
        }
        Ok(Self {
            ssid: ssid.to_string(),
            bssid,
            channel,
            security,
            pmf,
            phy_mode,
            enterprise_policy: None,
            signal_history: Vec::new(),
        })
    }

    /// Phép đo mới nhất nếu có.
    pub fn latest_signal(&self) -> Option<SignalMeasurement> {
        self.signal_history.last().copied()
    }
}

/// Lỗi mô hình AP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccessPointError {
    /// SSID phải dài 1–32 byte.
    InvalidSsid,
}

impl fmt::Display for AccessPointError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SSID của AP phải dài từ 1 đến 32 byte")
    }
}

impl Error for AccessPointError {}

/// Client tổng hợp và các chính sách quyết định có nên tin một BSS hay không.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Client {
    /// Địa chỉ client.
    pub mac: MacAddress,
    /// BSSID đã liên kết trong snapshot, nếu có.
    pub associated_to: Option<MacAddress>,
    /// Client có yêu cầu PMF không.
    pub pmf_required: bool,
    /// Client có hỗ trợ WPA3-SAE không.
    pub supports_wpa3: bool,
    /// Chính sách server/CA phải khớp khi Enterprise.
    pub enterprise_policy: Option<EnterprisePolicy>,
}

/// Một chỉ dấu evil twin; mỗi chỉ dấu riêng lẻ không phải lời kết án.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvilTwinIndicator {
    /// Cùng SSID nhưng khác BSSID: cũng có thể là AP hợp lệ khác trong ESS.
    BssidMismatch,
    /// Kênh/băng tần/độ rộng khác baseline.
    RadioProfileMismatch,
    /// Security suite yếu hơn baseline.
    SecurityDowngrade,
    /// PMF bị hạ mức.
    PmfDowngrade,
    /// CA/server/EAP policy khác hoặc bị bỏ.
    EnterprisePolicyMismatch,
    /// RSSI mới nhất chênh baseline trên 20 dB.
    SignalJump,
}

/// Mức kết luận có giới hạn, không bao giờ gọi là “đã xác nhận” chỉ từ snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvilTwinVerdict {
    /// Không cùng SSID nên không phải phép so sánh evil twin hợp lệ.
    NotComparable,
    /// Không thấy chỉ dấu nào trong các trường đã đo.
    NoStrongIndicators,
    /// Có dữ liệu khác biệt nhưng chưa đủ bằng chứng.
    InsufficientEvidence,
    /// Nên điều tra thêm bằng nguồn độc lập.
    Suspicious,
    /// Nhiều chỉ dấu độc lập cùng xuất hiện; vẫn cần xác minh vận hành.
    HighConfidence,
}

/// Kết quả so sánh AP quan sát với baseline đã được phê duyệt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvilTwinAssessment {
    /// Mức kết luận thận trọng.
    pub verdict: EvilTwinVerdict,
    /// Các chỉ dấu cụ thể theo thứ tự cố định.
    pub indicators: Vec<EvilTwinIndicator>,
    /// Điểm heuristic chỉ để giải thích, không phải xác suất.
    pub score: u8,
    /// Hành động phòng thủ tiếp theo.
    pub recommendation: &'static str,
}

/// Mức nghiêm trọng của một finding cấu hình.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    /// Thông tin.
    Info,
    /// Thấp.
    Low,
    /// Trung bình.
    Medium,
    /// Cao.
    High,
}

/// Finding có evidence, detection và remediation đi cùng nhau.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityFinding {
    /// Mã finding ổn định.
    pub code: &'static str,
    /// Mức nghiêm trọng.
    pub severity: Severity,
    /// Bằng chứng từ chính snapshot tổng hợp.
    pub evidence: String,
    /// Telemetry/hành vi giám sát tương ứng.
    pub detection: &'static str,
    /// Khắc phục tương ứng.
    pub remediation: &'static str,
}

/// Báo cáo cấu hình và các khuyến nghị phòng thủ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityAssessment {
    /// Các finding, không tự suy diễn điều chưa quan sát.
    pub findings: Vec<SecurityFinding>,
    /// Checklist hardening cố định, có thể đưa vào báo cáo.
    pub recommendations: Vec<&'static str>,
}

/// Bộ phân tích tất định trên dữ liệu trong bộ nhớ.
#[derive(Debug, Clone)]
pub struct CaptureAnalyzer {
    stage: CaptureStage,
}

impl Default for CaptureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CaptureAnalyzer {
    /// Bắt đầu pipeline ở `SyntheticInput`.
    pub const fn new() -> Self {
        Self {
            stage: CaptureStage::SyntheticInput,
        }
    }

    /// Stage hiện tại của typed pipeline.
    pub const fn stage(&self) -> CaptureStage {
        self.stage
    }

    /// Parse beacon; chỉ chuyển stage khi toàn bộ input hợp lệ.
    pub fn parse_beacon(&mut self, bytes: &[u8]) -> Result<Beacon, FrameError> {
        let beacon = Beacon::parse(bytes)?;
        self.stage = CaptureStage::TypedFrames;
        Ok(beacon)
    }

    /// Thử wordlist giả trên handshake. Ứng viên sai phải trả `false`.
    pub fn assess_handshake_wordlist(
        &mut self,
        capture: &HandshakeCapture,
        candidates: &[String],
    ) -> OfflineAssessment {
        let mut matched_index = None;
        let mut candidates_tested = 0;
        for (index, candidate) in candidates.iter().enumerate() {
            candidates_tested += 1;
            if capture.verify_password(candidate) {
                matched_index = Some(index);
                break;
            }
        }
        self.stage = CaptureStage::OfflineAssessmentComplete;
        OfflineAssessment {
            kind: CaptureKind::FourWayHandshake,
            candidates_tested,
            matched_index,
        }
    }

    /// Thử wordlist giả trên PMKID bằng công thức riêng của PMKID.
    pub fn assess_pmkid_wordlist(
        &mut self,
        record: &PmkidRecord,
        candidates: &[String],
    ) -> OfflineAssessment {
        let mut matched_index = None;
        let mut candidates_tested = 0;
        for (index, candidate) in candidates.iter().enumerate() {
            candidates_tested += 1;
            if record.verify_password(candidate) {
                matched_index = Some(index);
                break;
            }
        }
        self.stage = CaptureStage::OfflineAssessmentComplete;
        OfflineAssessment {
            kind: CaptureKind::Pmkid,
            candidates_tested,
            matched_index,
        }
    }

    /// So sánh nhiều indicator. Chỉ khác BSSID hoặc chỉ trùng SSID không kết luận evil twin.
    pub fn assess_evil_twin(
        &self,
        baseline: &AccessPoint,
        observed: &AccessPoint,
    ) -> EvilTwinAssessment {
        if baseline.ssid != observed.ssid {
            return EvilTwinAssessment {
                verdict: EvilTwinVerdict::NotComparable,
                indicators: Vec::new(),
                score: 0,
                recommendation: "Không suy diễn quan hệ giữa hai SSID khác nhau.",
            };
        }

        let mut indicators = Vec::new();
        let mut score = 0_u8;
        let mut push = |indicator: EvilTwinIndicator, weight: u8| {
            indicators.push(indicator);
            score = score.saturating_add(weight);
        };
        if baseline.bssid != observed.bssid {
            push(EvilTwinIndicator::BssidMismatch, 1);
        }
        if baseline.channel != observed.channel {
            push(EvilTwinIndicator::RadioProfileMismatch, 1);
        }
        if observed.security.strength() < baseline.security.strength()
            || (baseline.security.is_enterprise() && !observed.security.is_enterprise())
        {
            push(EvilTwinIndicator::SecurityDowngrade, 2);
        }
        if observed.pmf.strength() < baseline.pmf.strength() {
            push(EvilTwinIndicator::PmfDowngrade, 2);
        }
        if baseline.enterprise_policy != observed.enterprise_policy {
            push(EvilTwinIndicator::EnterprisePolicyMismatch, 3);
        }
        if let (Some(before), Some(now)) = (baseline.latest_signal(), observed.latest_signal())
            && (before.rssi().value() - now.rssi().value()).abs() > 20.0
        {
            push(EvilTwinIndicator::SignalJump, 1);
        }

        let verdict = if indicators.is_empty() {
            EvilTwinVerdict::NoStrongIndicators
        } else if score < 2 {
            EvilTwinVerdict::InsufficientEvidence
        } else if score < 4 {
            EvilTwinVerdict::Suspicious
        } else {
            EvilTwinVerdict::HighConfidence
        };
        EvilTwinAssessment {
            verdict,
            indicators,
            score,
            recommendation: "Đối chiếu inventory, CA/server name và lịch sử radio độc lập; cách ly SSID nếu chính sách xác thực bị hạ.",
        }
    }

    /// Đánh giá cấu hình AP và gắn mỗi finding với detection/remediation.
    pub fn assess_security(&self, ap: &AccessPoint) -> SecurityAssessment {
        let mut findings = Vec::new();
        let mut finding = |code: &'static str,
                           severity: Severity,
                           evidence: String,
                           detection: &'static str,
                           remediation: &'static str| {
            findings.push(SecurityFinding {
                code,
                severity,
                evidence,
                detection,
                remediation,
            });
        };
        match ap.security {
            SecuritySuite::Open => finding(
                "OPEN-NETWORK",
                Severity::High,
                format!("SSID {} không có lớp xác thực/mã hoá.", ap.ssid),
                "Cảnh báo khi baseline đổi sang Open hoặc client tự liên kết.",
                "Chuyển sang WPA3-SAE hoặc Enterprise; cô lập mạng guest có chủ đích.",
            ),
            SecuritySuite::Wep => finding(
                "LEGACY-WEP",
                Severity::High,
                "WEP/RC4 và IV ngắn chỉ còn giá trị lịch sử.".to_string(),
                "Quét inventory định kỳ để phát hiện thiết bị legacy.",
                "Thay thiết bị hoặc cô lập tuyệt đối; nâng lên WPA2/WPA3.",
            ),
            SecuritySuite::WpaPersonal => finding(
                "LEGACY-WPA",
                Severity::High,
                "WPA Personal/TKIP đã lỗi thời.".to_string(),
                "Theo dõi security suite trong beacon đã được chuẩn hoá.",
                "Nâng lên WPA2-AES/CCMP hoặc WPA3-SAE.",
            ),
            SecuritySuite::Wpa2Personal | SecuritySuite::Wpa2Enterprise
                if ap.pmf != PmfMode::Required =>
            {
                finding(
                    "PMF-NOT-REQUIRED",
                    Severity::Medium,
                    format!("WPA2 đang có PMF = {:?}.", ap.pmf),
                    "Cảnh báo thay đổi PMF và management-frame anomaly.",
                    "Bật PMF required khi toàn bộ client tương thích; lên kế hoạch WPA3.",
                );
            }
            SecuritySuite::Wpa3Personal | SecuritySuite::Wpa3Enterprise
                if ap.pmf != PmfMode::Required =>
            {
                finding(
                    "WPA3-PMF-NOT-REQUIRED",
                    Severity::High,
                    "Cấu hình WPA3 tổng hợp nhưng PMF chưa required.".to_string(),
                    "Kiểm tra sai lệch cấu hình trong configuration management.",
                    "Đặt PMF required và kiểm tra lại hồ sơ WPA3.",
                );
            }
            _ => {}
        }
        if ap.security.is_enterprise() {
            match &ap.enterprise_policy {
                None => finding(
                    "MISSING-EAP-POLICY",
                    Severity::High,
                    "Enterprise AP thiếu CA/server/EAP policy.".to_string(),
                    "Ghi nhận client chấp nhận server ngoài policy.",
                    "Ghim CA và server name, giới hạn EAP method; tuyệt đối không bỏ kiểm chứng chỉ.",
                ),
                Some(policy)
                    if policy.trusted_ca.is_empty()
                        || policy.server_name.is_empty()
                        || policy.eap_method.is_empty() =>
                {
                    finding(
                        "INCOMPLETE-EAP-POLICY",
                        Severity::High,
                        "Ít nhất một trường CA/server/EAP policy bị rỗng.".to_string(),
                        "Kiểm tra policy version và authentication failure.",
                        "Điền đầy đủ CA, server name và EAP method được duyệt.",
                    );
                }
                _ => {}
            }
        }
        if let Some(signal) = ap.latest_signal()
            && signal.snr().db() < 20.0
        {
            finding(
                "LOW-SNR",
                Severity::Low,
                format!("SNR mới nhất chỉ {:.1} dB.", signal.snr().db()),
                "Theo dõi SNR, retry và interference theo thời gian.",
                "Khảo sát vị trí/kênh, giảm nhiễu; không nhầm nhiễu với tấn công.",
            );
        }
        SecurityAssessment {
            findings,
            recommendations: hardening_recommendations().to_vec(),
        }
    }
}

/// Checklist phòng thủ dùng lại được trong demo và bài tập.
pub const fn hardening_recommendations() -> [&'static str; 6] {
    [
        "Ưu tiên WPA3-SAE/Enterprise và PMF required; loại bỏ Open/WEP/WPA khỏi inventory.",
        "Dùng passphrase dài, duy nhất và quản trị vòng đời khoá; WPA2-PSK không có forward secrecy.",
        "Với Enterprise, ghim CA/server name và EAP policy trên client; kiểm tra chứng chỉ bắt buộc.",
        "Phân đoạn client/guest/IoT, áp least privilege và giới hạn đường đi sau khi liên kết.",
        "Giám sát thay đổi BSSID/security/PMF/kênh/SNR và retry; lưu baseline có provenance.",
        "Có runbook cách ly SSID/AP, thu thập log được phép, thu hồi khoá và retest sau khắc phục.",
    ]
}

// ============================================================================
// 7. FIXTURE MOCKWIFILAB VÀ DEMO TẤT ĐỊNH
// ============================================================================

/// Phòng lab trong bộ nhớ: cùng input, cùng mọi output ở mọi lần chạy.
#[derive(Debug, Clone, PartialEq)]
pub struct MockWiFiLab {
    access_points: Vec<AccessPoint>,
    clients: Vec<Client>,
    handshake: HandshakeCapture,
    pmkid: PmkidRecord,
}

impl Default for MockWiFiLab {
    fn default() -> Self {
        Self::standard()
    }
}

impl MockWiFiLab {
    /// Passphrase tổng hợp, không thuộc một tài khoản hay mạng thật.
    pub const SYNTHETIC_PASSWORD: &'static str = "SYNTHETIC-LAB-PASSPHRASE";

    /// Tạo fixture giống nhau ở mọi lần gọi, không đọc đồng hồ/OS/radio.
    pub fn standard() -> Self {
        let channel = Channel::new(Band::Ghz2_4, 6, ChannelWidth::Mhz20)
            .expect("kênh fixture 2.4 GHz hợp lệ");
        let policy = EnterprisePolicy {
            trusted_ca: "OSWP Synthetic Root CA".to_string(),
            server_name: "radius.lab.example".to_string(),
            eap_method: "PEAP-MSCHAPv2-synthetic".to_string(),
        };
        let mut baseline = AccessPoint::new(
            "OSWP-LAB.example",
            MacAddress::new([0x02, 0x11, 0x22, 0x33, 0x44, 0x55]),
            channel,
            SecuritySuite::Wpa2Enterprise,
            PmfMode::Required,
            PhyMode::Ofdma,
        )
        .expect("SSID fixture hợp lệ");
        baseline.enterprise_policy = Some(policy.clone());
        baseline.signal_history = vec![
            SignalMeasurement::from_dbm(-58.0, -94.0).expect("phép đo fixture hợp lệ"),
            SignalMeasurement::from_dbm(-55.0, -94.0).expect("phép đo fixture hợp lệ"),
        ];

        let mut suspect = baseline.clone();
        suspect.bssid = MacAddress::new([0x02, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]);
        suspect.channel =
            Channel::new(Band::Ghz2_4, 11, ChannelWidth::Mhz20).expect("kênh fixture hợp lệ");
        suspect.security = SecuritySuite::Wpa2Personal;
        suspect.pmf = PmfMode::Disabled;
        suspect.enterprise_policy = None;
        suspect.signal_history =
            vec![SignalMeasurement::from_dbm(-88.0, -94.0).expect("phép đo fixture hợp lệ")];

        let mut open_ap = AccessPoint::new(
            "LAB-OPEN.example",
            MacAddress::new([0x02, 0, 0, 0, 0, 99]),
            Channel::new(Band::Ghz5, 36, ChannelWidth::Mhz80).expect("kênh fixture 5 GHz hợp lệ"),
            SecuritySuite::Open,
            PmfMode::Disabled,
            PhyMode::Ofdm,
        )
        .expect("SSID fixture hợp lệ");
        open_ap.signal_history =
            vec![SignalMeasurement::from_dbm(-80.0, -90.0).expect("phép đo fixture hợp lệ")];

        let handshake = HandshakeCapture::synthetic(Self::SYNTHETIC_PASSWORD)
            .expect("passphrase fixture WPA2 hợp lệ");
        let pmkid = PmkidRecord::synthetic(Self::SYNTHETIC_PASSWORD).expect("PMKID fixture hợp lệ");
        let client = Client {
            mac: MacAddress::new([0x02, 0, 0, 0, 0, 2]),
            associated_to: Some(baseline.bssid),
            pmf_required: true,
            supports_wpa3: true,
            enterprise_policy: Some(policy),
        };
        Self {
            access_points: vec![baseline, suspect, open_ap],
            clients: vec![client],
            handshake,
            pmkid,
        }
    }

    /// Snapshot AP theo thứ tự BSSID, không phụ thuộc HashMap hay môi trường.
    pub fn scan(&self) -> Vec<AccessPoint> {
        let mut aps = self.access_points.clone();
        aps.sort_by(|left, right| {
            left.bssid
                .cmp(&right.bssid)
                .then_with(|| left.ssid.cmp(&right.ssid))
        });
        aps
    }

    /// Baseline Enterprise đã được phê duyệt trong fixture.
    pub fn trusted_ap(&self) -> &AccessPoint {
        &self.access_points[0]
    }

    /// AP quan sát có nhiều chỉ dấu cần điều tra.
    pub fn suspect_ap(&self) -> &AccessPoint {
        &self.access_points[1]
    }

    /// Danh sách client tổng hợp.
    pub fn clients(&self) -> &[Client] {
        &self.clients
    }

    /// Handshake tổng hợp.
    pub const fn handshake(&self) -> &HandshakeCapture {
        &self.handshake
    }

    /// PMKID tổng hợp.
    pub const fn pmkid(&self) -> &PmkidRecord {
        &self.pmkid
    }

    /// Wordlist nhỏ, tất định và chỉ dành cho fixture.
    pub fn synthetic_wordlist() -> Vec<String> {
        vec![
            "wrong-lab-password".to_string(),
            Self::SYNTHETIC_PASSWORD.to_string(),
        ]
    }

    /// Beacon 2.4 GHz tổng hợp tương ứng baseline.
    pub fn beacon_bytes(&self) -> Result<Vec<u8>, FrameError> {
        let ap = self.trusted_ap();
        Beacon::synthetic(&ap.ssid, ap.channel.number() as u8, ap.bssid).encode_synthetic(7, true)
    }
}

/// Encode hex cho test vector và báo cáo học tập.
pub fn hex_encode(bytes: &[u8]) -> String {
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        text.push_str(&format!("{byte:02x}"));
    }
    text
}

fn main() -> Result<(), Box<dyn Error>> {
    let lab = MockWiFiLab::standard();
    let mut analyzer = CaptureAnalyzer::new();
    let beacon = analyzer.parse_beacon(&lab.beacon_bytes()?)?;

    println!("MockWiFiLab — toàn bộ dữ liệu đều tổng hợp");
    println!("scan: {} AP (thứ tự BSSID)", lab.scan().len());
    println!(
        "beacon: ssid={}, channel={}, retry-fixture=true",
        beacon.ssid, beacon.channel
    );
    println!(
        "RF: RSSI={:.1} dBm, SNR={:.1} dB",
        lab.trusted_ap()
            .latest_signal()
            .map(|s| s.rssi().value())
            .unwrap_or(0.0),
        lab.trusted_ap()
            .latest_signal()
            .map(|s| s.snr().db())
            .unwrap_or(0.0)
    );

    let handshake_result =
        analyzer.assess_handshake_wordlist(lab.handshake(), &MockWiFiLab::synthetic_wordlist());
    let pmkid_result =
        analyzer.assess_pmkid_wordlist(lab.pmkid(), &MockWiFiLab::synthetic_wordlist());
    println!(
        "handshake: matched={:?}, wrong-password={}",
        handshake_result.matched_index,
        lab.handshake().verify_password("definitely-wrong")
    );
    println!(
        "PMKID: kind={:?}, matched={:?}",
        pmkid_result.kind, pmkid_result.matched_index
    );

    let twin = analyzer.assess_evil_twin(lab.trusted_ap(), lab.suspect_ap());
    println!(
        "evil-twin assessment: {:?}, indicators={}, score={}",
        twin.verdict,
        twin.indicators.len(),
        twin.score
    );
    let security = analyzer.assess_security(lab.suspect_ap());
    println!(
        "security: findings={}, recommendations={}",
        security.findings.len(),
        security.recommendations.len()
    );
    Ok(())
}

// ============================================================================
// 8. UNIT TESTS: VECTOR CHUẨN, MALFORMED INPUT VÀ BẤT BIẾN PHÒNG THỦ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn pmk_reference() -> [u8; 32] {
        derive_pmk("password", "IEEE").expect("vector PMK hợp lệ")
    }

    #[test]
    fn mac_address_round_trip_and_rejects_bad_text() {
        let mac: MacAddress = "02:00:00:ab:cd:ef".parse().expect("MAC hợp lệ");
        assert_eq!(mac.octets(), [2, 0, 0, 0xab, 0xcd, 0xef]);
        assert_eq!(mac.to_string(), "02:00:00:ab:cd:ef");
        for invalid in [
            "",
            "02:00:00:00:00",
            "02:00:00:00:00:00:00",
            "gg:00:00:00:00:00",
            "2:00:00:00:00:00",
        ] {
            assert!(
                invalid.parse::<MacAddress>().is_err(),
                "phải từ chối {invalid}"
            );
        }
    }

    #[test]
    fn dbm_formula_and_boundaries() {
        assert!((Dbm::from_milliwatts(1.0).expect("1 mW").value() - 0.0).abs() < 1e-12);
        assert!((Dbm::from_milliwatts(100.0).expect("100 mW").value() - 20.0).abs() < 1e-12);
        assert!(Dbm::new(-150.0).is_ok());
        assert!(Dbm::new(30.0).is_ok());
        assert_eq!(Dbm::new(f64::NAN), Err(MeasurementError::NonFinite));
        assert_eq!(
            Dbm::from_milliwatts(0.0),
            Err(MeasurementError::InvalidPower)
        );
        assert_eq!(Dbm::new(31.0), Err(MeasurementError::OutOfRange));
    }

    #[test]
    fn snr_uses_difference_of_dbm_and_converts_to_linear() {
        let signal = SignalMeasurement::from_dbm(-52.0, -92.0).expect("phép đo hợp lệ");
        assert!((signal.snr().db() - 40.0).abs() < 1e-12);
        assert!((signal.snr().linear_ratio() - 10_000.0).abs() < 1e-8);
        assert_eq!(
            SignalMeasurement::from_dbm(-95.0, -90.0),
            Err(MeasurementError::SignalBelowNoise)
        );
        assert_eq!(
            SignalMeasurement::from_dbm(-128.0, -94.0),
            Err(MeasurementError::OutOfRange)
        );
    }

    #[test]
    fn channels_validate_all_three_bands_and_widths() {
        let c24 = Channel::new(Band::Ghz2_4, 1, ChannelWidth::Mhz40).expect("40 MHz 2.4");
        assert_eq!(c24.bonded_channels(), &[1, 5]);
        let c5 = Channel::new(Band::Ghz5, 52, ChannelWidth::Mhz80).expect("80 MHz 5");
        assert_eq!(c5.bonded_channels(), &[52, 56, 60, 64]);
        let c6 = Channel::new(Band::Ghz6, 5, ChannelWidth::Mhz160).expect("160 MHz 6");
        assert_eq!(c6.bonded_channels(), &[1, 5, 9, 13, 17, 21, 25, 29]);
        assert_eq!(c6.width().mhz(), 160);
    }

    #[test]
    fn invalid_channel_width_and_bonding_are_errors() {
        assert!(matches!(
            Channel::new(Band::Ghz2_4, 15, ChannelWidth::Mhz20),
            Err(ChannelError::UnsupportedChannel { .. })
        ));
        assert!(matches!(
            Channel::new(Band::Ghz2_4, 6, ChannelWidth::Mhz80),
            Err(ChannelError::UnsupportedWidth { .. })
        ));
        assert!(matches!(
            Channel::new(Band::Ghz5, 165, ChannelWidth::Mhz40),
            Err(ChannelError::InvalidBonding { .. })
        ));
        assert!(matches!(
            Channel::new(Band::Ghz6, 233, ChannelWidth::Mhz80),
            Err(ChannelError::InvalidBonding { .. })
        ));
        assert!(Channel::new(Band::Ghz6, 2, ChannelWidth::Mhz20).is_err());
    }

    #[test]
    fn beacon_round_trip_has_typed_header_sequence_and_retry() {
        let mac = MacAddress::new([2, 1, 2, 3, 4, 5]);
        let beacon = Beacon::synthetic("OSWP-LAB", 6, mac);
        let bytes = beacon.encode_synthetic(0x0abc, true).expect("encode");
        let frame = WirelessFrame::parse(&bytes).expect("parse frame");
        assert_eq!(frame.frame_type, FrameType::Management);
        assert_eq!(frame.subtype, 8);
        assert_eq!(frame.sequence, 0x0abc);
        assert_eq!(frame.fragment, 0);
        assert!(frame.retry);
        assert_eq!(frame.bssid, mac);
        assert_eq!(Beacon::parse(&bytes).expect("parse beacon"), beacon);
    }

    #[test]
    fn beacon_sequence_enforces_the_twelve_bit_boundary() {
        let beacon = Beacon::synthetic("OSWP-LAB", 6, MacAddress::new([2, 1, 2, 3, 4, 5]));
        let bytes = beacon
            .encode_synthetic(0x0fff, false)
            .expect("giá trị biên hợp lệ");
        assert_eq!(
            WirelessFrame::parse(&bytes).expect("parse").sequence,
            0x0fff
        );
        assert_eq!(
            beacon.encode_synthetic(0x1000, false),
            Err(FrameError::InvalidSequenceNumber(0x1000))
        );
    }

    #[test]
    fn every_truncated_beacon_prefix_returns_error_without_panic() {
        let bytes = Beacon::synthetic("OSWP-LAB", 6, MacAddress::new([2, 0, 0, 0, 0, 1]))
            .encode_synthetic(1, false)
            .expect("encode");
        for length in 0..bytes.len() {
            assert!(
                Beacon::parse(&bytes[..length]).is_err(),
                "prefix {length} phải bị từ chối"
            );
        }
    }

    #[test]
    fn beacon_rejects_trailing_bytes_and_bad_ie_length() {
        let mut bytes = Beacon::synthetic("LAB", 6, MacAddress::new([2, 0, 0, 0, 0, 1]))
            .encode_synthetic(1, false)
            .expect("encode");
        bytes.push(0);
        assert_eq!(Beacon::parse(&bytes), Err(FrameError::TrailingBytes(1)));

        let mut malformed = Beacon::synthetic("LAB", 6, MacAddress::new([2, 0, 0, 0, 0, 1]))
            .encode_synthetic(1, false)
            .expect("encode");
        let ds_length_index = malformed.len() - 2;
        malformed[ds_length_index] = 2;
        assert!(matches!(
            Beacon::parse(&malformed),
            Err(FrameError::Truncated { .. })
        ));
    }

    #[test]
    fn beacon_rejects_missing_duplicate_unknown_and_non_utf8_fields() {
        let bssid = MacAddress::new([2, 0, 0, 0, 0, 1]);
        let fixed = Beacon::synthetic("LAB", 6, bssid)
            .encode_synthetic(1, false)
            .expect("encode");
        let ds_start = 36 + 2 + "LAB".len();
        let missing_ssid = [&fixed[..36], &fixed[ds_start..]].concat();
        assert_eq!(
            Beacon::parse(&missing_ssid),
            Err(FrameError::MissingInformationElement(0))
        );

        let mut duplicate = fixed[..36].to_vec();
        duplicate.extend_from_slice(&[0, 1, b'X']);
        duplicate.extend_from_slice(&fixed[36..]);
        assert_eq!(
            Beacon::parse(&duplicate),
            Err(FrameError::DuplicateInformationElement(0))
        );

        let mut unknown_before_required = fixed[..36].to_vec();
        unknown_before_required.extend_from_slice(&[1, 1, 0]);
        unknown_before_required.extend_from_slice(&fixed[36..]);
        assert_eq!(
            Beacon::parse(&unknown_before_required),
            Err(FrameError::UnsupportedInformationElement(1))
        );

        let mut non_utf8 = Beacon::synthetic("L", 6, bssid)
            .encode_synthetic(1, false)
            .expect("encode");
        non_utf8[38] = 0xff;
        assert_eq!(Beacon::parse(&non_utf8), Err(FrameError::InvalidUtf8Ssid));
    }

    #[test]
    fn parser_rejects_wrong_type_version_and_subtype() {
        let good = Beacon::synthetic("LAB", 6, MacAddress::new([2, 0, 0, 0, 0, 1]))
            .encode_synthetic(1, false)
            .expect("encode");
        let mut data = good.clone();
        data[0] = 0x08;
        assert_eq!(
            WirelessFrame::parse(&data),
            Err(FrameError::UnsupportedFrameType(2))
        );
        let mut version = good.clone();
        version[0] |= 1;
        assert_eq!(
            WirelessFrame::parse(&version),
            Err(FrameError::UnsupportedProtocolVersion(1))
        );
        let mut probe_response = good;
        probe_response[0] = 0x50;
        assert_eq!(
            Beacon::parse(&probe_response),
            Err(FrameError::UnexpectedSubtype {
                expected: 8,
                actual: 5
            })
        );
    }

    #[test]
    fn probe_and_authentication_have_typed_bodies() {
        let mut probe = vec![0x40, 0, 0, 0];
        probe.extend_from_slice(&[0xff; 6]);
        probe.extend_from_slice(&[2, 0, 0, 0, 0, 2]);
        probe.extend_from_slice(&[0; 6]);
        probe.extend_from_slice(&[0; 2]);
        probe.extend_from_slice(&[0, 3, b'L', b'A', b'B']);
        let frame = WirelessFrame::parse(&probe).expect("probe");
        assert_eq!(
            frame.body,
            ManagementBody::ProbeRequest {
                ssid: Some("LAB".to_string())
            }
        );

        let mut auth = probe[..24].to_vec();
        auth[0] = 0xb0;
        auth.extend_from_slice(&0_u16.to_le_bytes());
        auth.extend_from_slice(&2_u16.to_le_bytes());
        auth.extend_from_slice(&0_u16.to_le_bytes());
        assert!(matches!(
            WirelessFrame::parse(&auth).expect("auth").body,
            ManagementBody::Authentication { transaction: 2, .. }
        ));
    }

    #[test]
    fn arbitrary_short_buffers_never_panic() {
        for byte in 0_u8..=255 {
            for length in 0..40 {
                let bytes = vec![byte; length];
                assert!(WirelessFrame::parse(&bytes).is_err() || length >= 24);
            }
        }
    }

    #[test]
    fn sha1_and_hmac_match_standard_vectors() {
        assert_eq!(
            hex_encode(&sha1(b"")),
            "da39a3ee5e6b4b0d3255bfef95601890afd80709"
        );
        assert_eq!(
            hex_encode(&sha1(b"abc")),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            hex_encode(&hmac_sha1(&[0x0b; 20], b"Hi There")),
            "b617318655057264e28bc0b6fb378c8ef146be00"
        );
    }

    #[test]
    fn pbkdf2_matches_rfc6070_and_rejects_zero_iterations() {
        let mut one = [0_u8; 20];
        pbkdf2_hmac_sha1(b"password", b"salt", 1, &mut one).expect("PBKDF2");
        assert_eq!(hex_encode(&one), "0c60c80f961f0e71f3a9b524af6012062fe037a6");
        let mut many = [0_u8; 20];
        pbkdf2_hmac_sha1(b"password", b"salt", 4096, &mut many).expect("PBKDF2");
        assert_eq!(
            hex_encode(&many),
            "4b007901b765489abead49d926f721d065a429c1"
        );
        assert_eq!(
            pbkdf2_hmac_sha1(b"password", b"salt", 0, &mut many),
            Err(KdfError::ZeroIterations)
        );
        assert_eq!(
            pbkdf2_hmac_sha1(b"password", b"salt", 1, &mut []),
            Err(KdfError::EmptyOutput)
        );
    }

    #[test]
    fn wpa2_pmk_matches_independent_4096_iteration_vector() {
        assert_eq!(
            hex_encode(&pmk_reference()),
            "f42c6fc52df0ebef9ebb4b90b38a5f902e83fe1b135a70e23aed762e9710a12e"
        );
        assert_eq!(
            derive_pmk("short", "IEEE"),
            Err(KdfError::InvalidPassphrase)
        );
        assert_eq!(derive_pmk("password", ""), Err(KdfError::InvalidSsid));
        assert!(derive_pmk("password", &"X".repeat(33)).is_err());
    }

    #[test]
    fn ptk_matches_independent_prf512_vector() {
        let ptk = derive_ptk(
            &pmk_reference(),
            MacAddress::new([2, 0, 0, 0, 0, 1]),
            MacAddress::new([2, 0, 0, 0, 0, 2]),
            [0x11; 32],
            [0x22; 32],
        );
        assert_eq!(
            hex_encode(ptk.as_bytes()),
            "e9cdf3afcaced664d70bc43ecc94ba970e89568bd85de99f263ae72134d994685b88694a8268c05bfe4c53de87b97d81bf57fe713477dc6adec2492d7cfd62b2"
        );
        assert_eq!(ptk.kck(), ptk.as_bytes()[..16]);
        assert_eq!(ptk.temporal_key_material().len(), 32);
    }

    #[test]
    fn synthetic_handshake_accepts_right_password_and_rejects_wrong_one() {
        let capture = HandshakeCapture::synthetic("password").expect("handshake");
        assert!(capture.verify_password("password"));
        assert!(!capture.verify_password("wrong-password"));
        assert_eq!(hex_encode(&capture.mic), "a0d2b9f9c39fd71b9e661ae8c17d10db");
    }

    #[test]
    fn mic_fails_with_wrong_key_message_nonce_or_mac() {
        let capture = HandshakeCapture::synthetic("password").expect("handshake");
        let pmk = pmk_reference();
        assert!(capture.verify_mic(&pmk));
        let mut wrong_pmk = pmk;
        wrong_pmk[0] ^= 1;
        assert!(!capture.verify_mic(&wrong_pmk));

        let mut changed = capture.clone();
        changed.eapol_message[0] ^= 1;
        assert!(!changed.verify_mic(&pmk));
        let mut changed = capture.clone();
        changed.anonce[0] ^= 1;
        assert!(!changed.verify_mic(&pmk));
        let mut changed = capture;
        changed.ap_mac = MacAddress::new([2, 0, 0, 0, 0, 9]);
        assert!(!changed.verify_mic(&pmk));
    }

    #[test]
    fn constant_time_comparison_checks_content_and_length() {
        assert!(constant_time_eq(&[1, 2, 3], &[1, 2, 3]));
        assert!(!constant_time_eq(&[1, 2, 3], &[1, 2, 4]));
        assert!(!constant_time_eq(&[1, 2], &[1, 2, 0]));
    }

    #[test]
    fn pmkid_has_known_vector_and_is_distinct_from_handshake() {
        let record = PmkidRecord::synthetic("password").expect("PMKID");
        assert_eq!(
            hex_encode(&record.pmkid),
            "3e07270d0746b999db1bec32482841f1"
        );
        assert!(record.verify_password("password"));
        assert!(!record.verify_password("wrong-password"));
        let mut analyzer = CaptureAnalyzer::new();
        let result = analyzer.assess_pmkid_wordlist(&record, &["password".to_string()]);
        assert_eq!(result.kind, CaptureKind::Pmkid);
        assert_ne!(result.kind, CaptureKind::FourWayHandshake);
    }

    #[test]
    fn offline_wordlist_success_failure_empty_and_attempt_count() {
        let capture = HandshakeCapture::synthetic("password").expect("handshake");
        let mut analyzer = CaptureAnalyzer::new();
        let words = vec![
            "wrong-one".to_string(),
            "password".to_string(),
            "unused-pass".to_string(),
        ];
        let result = analyzer.assess_handshake_wordlist(&capture, &words);
        assert_eq!(result.matched_index, Some(1));
        assert_eq!(result.candidates_tested, 2);
        assert_eq!(analyzer.stage(), CaptureStage::OfflineAssessmentComplete);

        let wrong = analyzer.assess_handshake_wordlist(&capture, &["not-the-password".to_string()]);
        assert!(!wrong.succeeded());
        assert_eq!(wrong.candidates_tested, 1);
        let empty = analyzer.assess_handshake_wordlist(&capture, &[]);
        assert_eq!(empty.candidates_tested, 0);
        assert!(!empty.succeeded());
    }

    #[test]
    fn parse_failure_does_not_advance_typed_capture_stage() {
        let mut analyzer = CaptureAnalyzer::new();
        assert!(analyzer.parse_beacon(&[0; 3]).is_err());
        assert_eq!(analyzer.stage(), CaptureStage::SyntheticInput);
    }

    #[test]
    fn ssid_or_bssid_difference_alone_does_not_prove_evil_twin() {
        let lab = MockWiFiLab::standard();
        let analyzer = CaptureAnalyzer::new();
        let mut only_bssid = lab.trusted_ap().clone();
        only_bssid.bssid = MacAddress::new([2, 9, 9, 9, 9, 9]);
        let assessment = analyzer.assess_evil_twin(lab.trusted_ap(), &only_bssid);
        assert_eq!(assessment.verdict, EvilTwinVerdict::InsufficientEvidence);
        assert_eq!(
            assessment.indicators,
            vec![EvilTwinIndicator::BssidMismatch]
        );

        let identical = analyzer.assess_evil_twin(lab.trusted_ap(), lab.trusted_ap());
        assert_eq!(identical.verdict, EvilTwinVerdict::NoStrongIndicators);
        let mut different_ssid = only_bssid;
        different_ssid.ssid = "ANOTHER.example".to_string();
        assert_eq!(
            analyzer
                .assess_evil_twin(lab.trusted_ap(), &different_ssid)
                .verdict,
            EvilTwinVerdict::NotComparable
        );
    }

    #[test]
    fn multiple_independent_indicators_raise_confidence_without_claiming_certainty() {
        let lab = MockWiFiLab::standard();
        let assessment =
            CaptureAnalyzer::new().assess_evil_twin(lab.trusted_ap(), lab.suspect_ap());
        assert_eq!(assessment.verdict, EvilTwinVerdict::HighConfidence);
        assert!(
            assessment
                .indicators
                .contains(&EvilTwinIndicator::SecurityDowngrade)
        );
        assert!(
            assessment
                .indicators
                .contains(&EvilTwinIndicator::PmfDowngrade)
        );
        assert!(
            assessment
                .indicators
                .contains(&EvilTwinIndicator::EnterprisePolicyMismatch)
        );
        assert!(
            assessment
                .indicators
                .contains(&EvilTwinIndicator::SignalJump)
        );
    }

    #[test]
    fn security_assessment_links_findings_to_defense_and_remediation() {
        let lab = MockWiFiLab::standard();
        let assessment = CaptureAnalyzer::new().assess_security(lab.suspect_ap());
        assert!(
            assessment
                .findings
                .iter()
                .any(|finding| finding.code == "PMF-NOT-REQUIRED")
        );
        assert!(
            assessment
                .findings
                .iter()
                .any(|finding| finding.code == "LOW-SNR")
        );
        assert_eq!(assessment.recommendations.len(), 6);
        for finding in assessment.findings {
            assert!(!finding.evidence.is_empty());
            assert!(!finding.detection.is_empty());
            assert!(!finding.remediation.is_empty());
        }
    }

    #[test]
    fn open_and_incomplete_enterprise_configurations_are_flagged() {
        let lab = MockWiFiLab::standard();
        let analyzer = CaptureAnalyzer::new();
        let open = lab
            .scan()
            .into_iter()
            .find(|ap| ap.security == SecuritySuite::Open)
            .expect("open fixture");
        assert!(
            analyzer
                .assess_security(&open)
                .findings
                .iter()
                .any(|finding| finding.code == "OPEN-NETWORK")
        );

        let mut enterprise = lab.trusted_ap().clone();
        enterprise.enterprise_policy = None;
        assert!(
            analyzer
                .assess_security(&enterprise)
                .findings
                .iter()
                .any(|finding| finding.code == "MISSING-EAP-POLICY")
        );
    }

    #[test]
    fn access_point_and_beacon_validate_input_boundaries() {
        let channel = Channel::new(Band::Ghz2_4, 1, ChannelWidth::Mhz20).expect("channel");
        assert!(
            AccessPoint::new(
                "",
                MacAddress::new([0; 6]),
                channel.clone(),
                SecuritySuite::Open,
                PmfMode::Disabled,
                PhyMode::Ofdm
            )
            .is_err()
        );
        assert!(
            AccessPoint::new(
                &"X".repeat(33),
                MacAddress::new([0; 6]),
                channel,
                SecuritySuite::Open,
                PmfMode::Disabled,
                PhyMode::Ofdm
            )
            .is_err()
        );
        let mut beacon = Beacon::synthetic("LAB", 6, MacAddress::new([0; 6]));
        beacon.interval_tu = 0;
        assert_eq!(
            beacon.encode_synthetic(0, false),
            Err(FrameError::InvalidBeaconInterval)
        );
        let beacon = Beacon::synthetic(&"X".repeat(33), 6, MacAddress::new([0; 6]));
        assert!(matches!(
            beacon.encode_synthetic(0, false),
            Err(FrameError::InvalidSsidLength(33))
        ));
    }

    #[test]
    fn standard_lab_and_scan_are_deterministic_and_sorted() {
        let first = MockWiFiLab::standard();
        let second = MockWiFiLab::standard();
        assert_eq!(first, second);
        let scan = first.scan();
        assert!(scan.windows(2).all(|pair| pair[0].bssid <= pair[1].bssid));
        assert_eq!(first.clients().len(), 1);
        assert!(first.clients()[0].pmf_required);
        assert_eq!(first.trusted_ap().phy_mode, PhyMode::Ofdma);
    }
}
