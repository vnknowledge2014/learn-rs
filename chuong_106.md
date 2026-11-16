# Chương 106: An ninh Không dây & Phòng lab OSWP Tổng hợp — RF, 802.11, WPA2/WPA3, Handshake, PMKID & Evil Twin (Wireless Security & OSWP Lab)

> **Phạm vi:** chương này phục vụ học tập trong lab có ủy quyền. Mọi capture, địa chỉ, mật khẩu và nhật ký đều là dữ liệu tổng hợp. Crate không mở socket, không nhận tên interface, không bật monitor mode, không phát frame và không có chức năng deauthentication. Đây là mô hình clean-room từ các chuẩn công khai, không phải công cụ tấn công mạng thật.

## 1. Từ một cuộc gọi nhiễu tới một bài kiểm tra có bằng chứng

Hãy tưởng tượng bạn đang nói chuyện trong một hội trường. Giọng người đối diện là **tín hiệu**, tiếng điều hòa và hàng trăm cuộc trò chuyện khác là **nhiễu**. Đứng gần nhau giúp nghe rõ hơn, nhưng một loa cùng kênh vẫn có thể lấn át. Đổi sang phòng rộng hơn giống tăng độ rộng kênh: có thể truyền nhanh hơn, nhưng cũng gom nhiều nhiễu hơn.

Một buổi kiểm tra wireless có trách nhiệm cũng cần những câu hỏi giống vậy:

1. Ta đang nhìn **sóng radio**, **frame**, hay **cấu hình xác thực**?
2. Dữ liệu có đủ để kết luận không, hay chỉ là một dấu hiệu?
3. Nếu có finding, log nào phát hiện nó và khắc phục ra sao?
4. Phép thử có nằm trong ủy quyền và có cleanup không?

```text
Sóng RF → frame 802.11 → BSS/SSID → xác thực → khoá phiên → telemetry
   │           │             │             │             │
 dBm/SNR   beacon/retry   BSSID/ESS     PSK/EAP/SAE   MIC/replay   detection
```

### Mục tiêu học tập

Sau chương này, bạn có thể:

- Tính dBm và SNR, phân biệt kênh primary với channel bonding.
- Đọc một beacon trong subset 802.11 và giải thích các bước gia nhập BSS.
- Phân biệt WPA2-PSK, Enterprise, WPA3-SAE và PMF.
- Suy ra PMK bằng PBKDF2-HMAC-SHA1, mô tả PTK/GTK và bốn message của handshake.
- Giải thích PMKID khác 4-way handshake ở chỗ nào.
- Chạy một offline assessment trên wordlist giả, trong đó mật khẩu sai phải thất bại.
- Đánh giá evil-twin indicators mà không kết luận chỉ từ SSID trùng nhau.
- Viết finding có evidence, detection, remediation và cleanup.

Tiên quyết nên có: kiến thức byte và mạng ở Chương 65, băm ở Chương 70 và PBKDF2 ở Chương 101. Crate này vẫn tự cài SHA-1/HMAC/PBKDF2 để test độc lập.

---

## 2. RF và PHY — đừng đo Wi-Fi chỉ bằng “số vạch”

### 2.1 dBm là logarit, không phải phần trăm pin

dBm là mức công suất đối chiếu 1 mW:

```text
P_dBm = 10 × log10(P_mW)
```

0 dBm tương ứng 1 mW; 20 dBm tương ứng 100 mW. RSSI thường là số âm: −45 dBm mạnh hơn −80 dBm. Giảm 10 dB tương ứng công suất giảm mười lần. “RSSI cao” không đồng nghĩa mọi phiên đều nhanh: băng thông còn phụ thuộc nhiễu, retry, PHY và tải.

Crate dùng `Dbm` thay vì truyền `f64` tự do để từ chối NaN/vô cực và giữ miền mô phỏng rõ ràng. Đây là giới hạn của lab, không phải định mức pháp lý của thiết bị.

### 2.2 SNR dùng phép trừ khi hai đại lượng cùng dBm

```text
SNR_dB = RSSI_dBm − noise_floor_dBm
```

Với RSSI −52 dBm và noise floor −92 dBm, SNR là 40 dB. Không lấy −52 chia cho −92: đại lượng logarit phải được trừ. Khi cần tỷ số tuyến tính, dùng `10^(SNR_dB / 10)`; 40 dB tương ứng 10.000.

```rust
#[derive(Debug)]
struct Measurement {
    rssi_dbm: f64,
    noise_dbm: f64,
}

impl Measurement {
    fn snr_db(&self) -> f64 {
        self.rssi_dbm - self.noise_dbm
    }
}

fn main() {
    let sample = Measurement { rssi_dbm: -52.0, noise_dbm: -92.0 };
    assert_eq!(sample.snr_db(), 40.0);
    assert_eq!(10_f64.powf(sample.snr_db() / 10.0), 10_000.0);
}
```

Trong crate, `SignalMeasurement::from_dbm` còn kiểm tra miền fixture và bất biến tín hiệu không thấp hơn noise floor. Một receiver thật có thể báo các giá trị ngoài mô hình đơn giản này; không mang bất biến của mock áp đặt lên mọi driver.

### 2.3 Suy giảm và nhiễu

- **Attenuation:** tường, khoảng cách và vật hấp thụ làm giảm công suất.
- **Interference:** thiết bị cùng/chồng kênh làm tăng tranh chấp và retry.
- **Multipath:** phản xạ có thể gây triệt tiêu hoặc được MIMO tận dụng, tùy hệ thống.
- **Noise floor:** mức nền để so tín hiệu; không phải “số người đang dùng Wi-Fi”.

Một AP phát mạnh hơn không tự động tốt hơn: client có công suất phát thấp hơn AP và đường lên vẫn có thể kém. Đánh giá phải nhìn cả hai chiều và chuỗi đo theo thời gian.

### 2.4 Kênh không đồng nghĩa độ rộng kênh

| Băng | Vai trò trong lab | Lưu ý |
|---|---|---|
| 2.4 GHz | Kênh cách nhau 5 MHz, ví dụ 1/6/11 | Vùng phổ hẹp, dễ chồng lấn; quy định kênh tùy quốc gia |
| 5 GHz | Nhiều nhóm 20/40/80/160 MHz | Một số kênh chịu DFS và quy định địa phương |
| 6 GHz | Wi-Fi 6E/7, kênh 20 MHz đánh số cách 4 | Thiết bị và vùng điều tiết phải hỗ trợ |

Channel bonding ghép các kênh 20 MHz: 40 MHz gồm hai, 80 MHz gồm bốn, 160 MHz gồm tám. Kênh primary vẫn mang ý nghĩa điều khiển; không phải “đổi kênh 36 thành kênh 80”.

`Channel::new` kiểm tra primary có thuộc nhóm bonding hoàn chỉnh không. Danh sách kênh trong crate là **subset tất định**, không thay thế cơ sở dữ liệu điều tiết hoặc country code.

### 2.5 OFDM và OFDMA/Wi-Fi 6

OFDM chia kênh thành nhiều sóng mang con trực giao. Ở mô hình của chương, một lần truyền dùng toàn kênh cho một phiên. Wi-Fi 6 thêm OFDMA: chia kênh thành các resource unit để phục vụ nhiều client trong cùng cơ hội truyền. OFDMA không phải “nhiều mật khẩu Wi-Fi” và không tự giải quyết xác thực; nó là cơ chế PHY/MAC phục vụ hiệu quả và độ trễ.

---

## 3. 802.11 MAC — đọc frame trước khi bàn về bảo mật

### 3.1 SSID, BSSID và BSS

- **SSID:** tên mạng, tối đa 32 byte; không phải định danh duy nhất toàn cầu.
- **BSSID:** định danh một BSS, trong infrastructure thường gắn địa chỉ radio của AP.
- **ESS:** nhiều BSS có thể cùng SSID để roaming. Vì vậy cùng SSID nhưng khác BSSID chưa chứng minh evil twin.
- **MAC:** địa chỉ 48-bit; fixture dùng bit locally administered và miền `.example`.

### 3.2 Ba loại frame

| Type | Ví dụ | Vai trò |
|---|---|---|
| Management | beacon, probe, authentication, association | Khám phá và quản lý liên kết |
| Control | ACK, RTS/CTS | Hỗ trợ điều khiển truy cập môi trường |
| Data | MSDU được đóng gói | Mang dữ liệu người dùng |

Parser của crate chỉ nhận management frame có header 24 byte, protocol version 0. Nó hiểu beacon, probe request và authentication; các subtype khác được giữ thành `ManagementBody::Other`. Control/Data bị từ chối thay vì đoán sai.

Frame Control là hai byte little-endian. Hai bit đầu là version, hai bit tiếp theo là type, bốn bit tiếp theo là subtype. Sequence Control tách thành fragment number 4 bit thấp và sequence number 12 bit cao. Cờ retry nằm ở bit 11 của Frame Control.

```rust
#[derive(Debug, PartialEq, Eq)]
struct FrameControl(u16);

impl FrameControl {
    fn version(self) -> u16 { self.0 & 0b11 }
    fn frame_type(self) -> u16 { (self.0 >> 2) & 0b11 }
    fn subtype(self) -> u16 { (self.0 >> 4) & 0b1111 }
    fn retry(self) -> bool { self.0 & 0x0800 != 0 }
}

fn main() {
    let beacon_retry = FrameControl(0x0880);
    assert_eq!(beacon_retry.version(), 0);
    assert_eq!(beacon_retry.frame_type(), 0);
    assert_eq!(beacon_retry.subtype(), 8);
    assert!(beacon_retry.retry());
}
```

### 3.3 Beacon có fixed fields và Information Elements

Subset beacon của lab gồm:

```text
24-byte management header
8-byte timestamp | 2-byte beacon interval | 2-byte capabilities
SSID IE: tag 0 | length | 0..32 byte UTF-8
DS Parameter Set IE: tag 3 | length 1 | channel
```

Parser trả lỗi cho prefix bị cắt, IE tràn buffer, thiếu/lặp trường, SSID quá dài, UTF-8 không hợp lệ, kênh zero và trailing bytes. Nó **không phải parser 802.11 đầy đủ**: RSN IE, vendor IE, HT/VHT/HE/EHT, FCS và nhiều biến thể header không nằm trong subset. Một frame dùng IE hợp lệ ngoài subset sẽ bị từ chối có chủ đích, không bị âm thầm bỏ qua.

### 3.4 Client gia nhập BSS như thế nào

```text
AP phát beacon
      ↓
Client có thể gửi probe request ←→ probe response
      ↓
802.11 authentication (thường Open System ở bước MAC)
      ↓
Association request/response
      ↓
Với WPA2-Personal: 4-way handshake tạo/xác nhận khoá
Với Enterprise: 802.1X/EAP xác thực trước khi có khoá phiên
```

“Open System authentication” ở bước 802.11 không có nghĩa mạng bỏ WPA2. Ngược lại, một association thành công cũng chưa chứng minh client có khoá hợp lệ. Phải tách từng stage thay vì gộp thành “đã kết nối Wi-Fi”.

### 3.5 Vai trò của Kismet và Wireshark trong lab được cấp phép

- **Adapter/driver:** quyết định có nhận được management/radio metadata không; hỗ trợ khác nhau theo OS và chipset.
- **Kismet:** inventory thiết bị, kênh, SSID/BSSID và lịch sử quan sát.
- **Wireshark:** đọc capture, kiểm tra field và chuỗi frame.

Display filter như `wlan.fc.type_subtype == 0x0008` chọn beacon; `eapol` chọn khung EAPOL đã decode. Filter là công cụ đọc bằng chứng, không phải bằng chứng tự nó. Chương không phát lệnh radio hay hướng dẫn thu capture ngoài phạm vi lab.

---

## 4. Từ WEP tới WPA3 — thay đổi cơ chế, không chỉ đổi tên

| Cấu hình | Điểm chính | Đánh giá hiện nay |
|---|---|---|
| WEP | RC4, IV ngắn và quản lý IV yếu | Chỉ còn ý nghĩa lịch sử; loại khỏi mạng thật |
| WPA/TKIP | Cơ chế chuyển tiếp từ WEP | Lỗi thời |
| WPA2-Personal | PSK dùng chung, thường CCMP/AES | Phụ thuộc mạnh vào passphrase và PMF |
| WPA2-Enterprise | 802.1X/EAP, máy chủ xác thực | Phải kiểm tra CA/server và EAP policy |
| WPA3-Personal | SAE thay PSK handshake truyền thống | Chống offline guessing từ SAE exchange tốt hơn và có forward secrecy |
| WPA3-Enterprise | Enterprise với yêu cầu bảo mật hiện đại | Cần cấu hình và client tương thích |

WPA2-PSK không có forward secrecy: nếu passphrase bị lộ và capture phù hợp còn lưu, dữ liệu cũ có thể bị đánh giá lại theo điều kiện giao thức. SAE dùng giao thức khác, không suy ra PMK bằng PBKDF2 theo cách WPA2-PSK; đừng áp công thức chương này cho WPA3.

**PMF (Protected Management Frames)** bảo vệ các management frame được hỗ trợ khỏi giả mạo/sửa đổi trái phép. `Capable` cho phép negotiate; `Required` bắt buộc. WPA3 yêu cầu PMF trong hồ sơ tương ứng; mock báo `WPA3-PMF-NOT-REQUIRED` khi fixture mâu thuẫn.

RC4 trong phần lịch sử là **stream cipher đồng bộ**: hai phía phải dùng đúng vị trí keystream. Nó không phải stream cipher tự đồng bộ.

---

## 5. WPA2 key hierarchy — từ passphrase tới khoá phiên

### 5.1 PMK — 32 byte từ PBKDF2

Với WPA2-Personal:

```text
PMK = PBKDF2-HMAC-SHA1(password, SSID, 4096, 32 byte)
```

SSID là salt, 4096 là số vòng và output là 256 bit. SHA-1 ở đây nằm trong HMAC theo giao thức WPA2; không suy ra rằng SHA-1 phù hợp để thiết kế hệ thống mới.

Với PBKDF2, trong mỗi block:

```text
U1 = PRF(password, salt || INT32_BE(i))
Uj = PRF(password, Uj−1)
Fi = U1 XOR U2 XOR ... XOR Uc
```

Các block `Fi` được **ghép nối**, rồi cắt theo độ dài output. Không XOR các block với nhau. Test crate dùng vector RFC 6070 và vector PMK độc lập:

```text
password = password
SSID     = IEEE
PMK      = f42c6fc52df0ebef9ebb4b90b38a5f902e83fe1b135a70e23aed762e9710a12e
```

### 5.2 PTK — gắn khoá với hai địa chỉ và hai nonce

AP tạo ANonce, client tạo SNonce. Hai bên sắp xếp địa chỉ và nonce theo min/max byte để cùng một input:

```text
data = min(AA, SPA) || max(AA, SPA) || min(ANonce, SNonce) || max(ANonce, SNonce)
PTK material = PRF-SHA1(PMK, "Pairwise key expansion", data)
```

Với CCMP, PTK là 48 byte: KCK 16 byte, KEK 16 byte, TK 16 byte. Crate gọi PRF-512 và giữ 64 byte trong `Ptk` để minh họa nhiều output block; **PTK-CCMP chỉ dùng 48 byte đầu**. Không lấy 64 byte này làm temporal key.

- **KCK:** xác nhận khoá, dùng MIC của EAPOL trong mô hình WPA2 này.
- **KEK:** bảo vệ phân phối key data, ví dụ GTK.
- **TK:** khoá dữ liệu unicast theo cipher suite.

### 5.3 GTK và MIC

GTK bảo vệ lưu lượng group/broadcast/multicast trong BSS và được phân phối có bảo vệ, không suy ra công khai từ SSID. PMK/PTK/GTK có vai trò khác nhau; gọi tất cả là “mật khẩu Wi-Fi đã mã hóa” là sai mô hình.

MIC giúp phát hiện message bị sửa khi biết KCK. Replay counter và nonce phục vụ tính mới/chống replay; một MIC đúng trên message cũ không tự biến message cũ thành phiên mới. Crate kiểm tra thay đổi payload, nonce, MAC hoặc PMK đều làm verification thất bại.

---

## 6. 4-way handshake và PMKID — hai loại bằng chứng khác nhau

### 6.1 Bốn message

| Message | Vai trò khái niệm | Điểm cần kiểm tra |
|---|---|---|
| M1: AP → client | Gửi ANonce và replay counter | Nonce mới, đúng phiên |
| M2: client → AP | Gửi SNonce và MIC sau khi suy PTK | MIC trên EAPOL với trường MIC zero khi tính |
| M3: AP → client | Xác nhận và có thể phân phối GTK được bảo vệ | Replay/flags/key data nhất quán |
| M4: client → AP | Xác nhận hoàn tất | Đúng phiên và replay counter |

Để mô phỏng offline assessment của WPA2-PSK, ta cần SSID, hai địa chỉ, hai nonce, payload EAPOL và MIC tương ứng. Crate tạo một `HandshakeCapture` tổng hợp từ chính passphrase lab. Nó không đọc `.pcap`, không decode EAPOL thật và không khẳng định đã thu handshake từ radio.

### 6.2 PMKID không phải 4-way handshake

Với PMKID trong WPA2-PSK:

```text
PMKID = HMAC-SHA1-128(PMK, "PMK Name" || AA || SPA)
```

Nó là một identifier trong cơ chế roaming/PMKSA. PMKID không chứa ANonce/SNonce và không phải bốn message. Offline candidate check vẫn cần SSID để suy PMK và hai địa chỉ để tính lại PMKID.

| | 4-way capture | PMKID record |
|---|---|---|
| Nonce | Có ANonce/SNonce | Không có trong công thức PMKID |
| Bằng chứng xác nhận | MIC trên EAPOL | PMKID 16 byte |
| API crate | `HandshakeCapture` | `PmkidRecord` |
| Kết luận khi khớp | Candidate khớp MIC tổng hợp | Candidate khớp PMKID tổng hợp |

Vector fixture với `password`, SSID `IEEE`, AP `02:00:00:00:00:01`, client `02:00:00:00:00:02`:

```text
MIC    = a0d2b9f9c39fd71b9e661ae8c17d10db
PMKID  = 3e07270d0746b999db1bec32482841f1
```

MIC thuộc payload tổng hợp của crate; nó không phải MIC của một capture bên ngoài.

### 6.3 Offline assessment có giới hạn

Pipeline thay thế mô tả airodump/aircrack bằng typed stages:

```text
SyntheticInput → TypedFrames → OfflineAssessmentComplete
       │               │
       └─ parse lỗi: giữ nguyên stage, trả FrameError
```

`CaptureAnalyzer` duyệt một wordlist nhỏ trong bộ nhớ và trả `matched_index` cùng `candidates_tested`. Passphrase sai hoặc sai định dạng trả `false`; wordlist rỗng không tạo finding thành công. Không có tối ưu GPU, sinh wordlist thật, kết nối dịch vụ hay đọc credential trên máy.

Một candidate khớp trong mock chứng minh **fixture nhất quán với công thức**, không chứng minh một mạng thật đã bị xâm nhập.

---

## 7. Evil twin — tổng hợp chỉ dấu thay vì phán đoán theo tên mạng

Một AP giả có thể quảng bá SSID quen thuộc, nhưng SSID trùng cũng xuất hiện trong ESS hợp pháp. `CaptureAnalyzer::assess_evil_twin` xem:

1. Có cùng SSID để so sánh không?
2. BSSID có khác baseline không?
3. Kênh/băng/width có thay đổi không?
4. Security suite có hạ từ Enterprise xuống Personal/Open không?
5. PMF có bị hạ mức không?
6. CA, server name hoặc EAP method có khác/mất không?
7. RSSI có thay đổi bất thường so với lịch sử không?

Điểm heuristic chỉ giúp sắp xếp điều tra, **không phải xác suất tấn công**. Chỉ khác BSSID cho `InsufficientEvidence`. Nhiều chỉ dấu độc lập có thể cho `HighConfidence`, nhưng vẫn cần inventory, log và xác minh vận hành. Crate không tạo captive portal và không thu credential.

### Finding phải nối được sang phòng thủ

| Kỹ thuật/rủi ro | Evidence trong mock | Detection | Remediation |
|---|---|---|---|
| Open/WEP/WPA legacy | `security` của AP | Configuration drift, wireless inventory | Nâng WPA2/WPA3, thay thiết bị, cô lập legacy |
| WPA2 thiếu PMF required | `pmf` | Baseline PMF, management anomaly | Bật PMF khi tương thích, lên kế hoạch WPA3 |
| WPA3 nhưng PMF không required | Suite và PMF mâu thuẫn | Configuration validation | Sửa profile và retest |
| Enterprise thiếu policy | CA/server/EAP rỗng hoặc vắng | Authentication failure, policy version | Ghim CA/server, giới hạn EAP method |
| Evil-twin nghi ngờ | Nhiều indicator có nguồn | BSSID/security/PMF/SNR history | Điều tra độc lập, cách ly theo runbook |
| SNR thấp | RSSI/noise floor | SNR/retry/interference theo thời gian | Khảo sát RF; không mặc định là tấn công |

Hardening tổng thể gồm passphrase duy nhất và đủ mạnh cho WPA2 còn tồn tại, WPA3/PMF, Enterprise policy, segmentation, monitoring và quy trình thu hồi khoá. Sau remediation phải retest cùng fixture để chứng minh finding biến mất mà không xoá evidence lịch sử.

---

## 8. Crate `MockWiFiLab` — chạy, kiểm tra, tái lập

Crate `code/ch106` dùng Rust 2024 thuần `std` và `#![forbid(unsafe_code)]`.

| Nhóm | API chính |
|---|---|
| RF | `Dbm`, `Snr`, `SignalMeasurement` |
| PHY | `Band`, `ChannelWidth`, `Channel`, `PhyMode` |
| 802.11 | `WirelessFrame`, `Beacon`, `ManagementBody`, `FrameError` |
| KDF | `sha1`, `hmac_sha1`, `pbkdf2_hmac_sha1`, `derive_pmk`, `derive_ptk` |
| Capture | `HandshakeCapture`, `PmkidRecord`, `CaptureAnalyzer` |
| Đánh giá | `AccessPoint`, `Client`, `SecurityAssessment`, `EvilTwinAssessment` |
| Fixture | `MockWiFiLab::standard()` |

Chạy demo:

```bash
cargo run --manifest-path code/Cargo.toml -p ch106 --locked
```

Output tất định:

```text
MockWiFiLab — toàn bộ dữ liệu đều tổng hợp
scan: 3 AP (thứ tự BSSID)
beacon: ssid=OSWP-LAB.example, channel=6, retry-fixture=true
RF: RSSI=-55.0 dBm, SNR=39.0 dB
handshake: matched=Some(1), wrong-password=false
PMKID: kind=Pmkid, matched=Some(1)
evil-twin assessment: HighConfidence, indicators=6, score=10
security: findings=2, recommendations=6
```

29 unit tests bao phủ vector SHA-1/HMAC/PBKDF2/PMK/PTK/MIC/PMKID, wrong key/password, mọi prefix beacon bị cắt, malformed IE, channel/width, chu trình stage và tính deterministic của scan. Test không dùng thời gian, mạng, filesystem, radio hay thứ tự hash map.

```bash
cargo test --manifest-path code/Cargo.toml -p ch106 --locked
cargo clippy --manifest-path code/Cargo.toml -p ch106 --all-targets --locked -- -D warnings
```

---

## 9. Lỗi biên dịch và lỗi mô hình thường gặp

| Triệu chứng | Nguyên nhân | Cách sửa |
|---|---|---|
| `E0596: cannot borrow as mutable` | Tạo `PmkidRecord` rồi gán `record.pmkid` nhưng thiếu `mut` | Khai báo `let mut record`; không sửa bằng `unsafe` |
| `E0308: expected an array with a fixed size` | Truyền `Vec<u8>` vào tham số `[u8; 32]` | Giữ kiểm tra độ dài rồi `try_into()`; không cắt nonce tùy ý |
| `E0502` quanh closure ghi findings | Giữ hai mutable borrow tới cùng state | Gom mutation vào một method/closure rồi kết thúc borrow |
| Test PTK báo expected dài hơn actual | Ghép thừa byte vào chuỗi hex | Đối chiếu độ dài: output 64 byte phải có 128 ký tự hex |
| Parser báo `TrailingBytes` | Thêm IE sau khi subset đã đọc đủ | Hiểu giới hạn parser; mở rộng schema có test thay vì bỏ byte lặng lẽ |
| `Wrong password` nhưng MIC vẫn pass | Tái sử dụng PMK cũ thay vì suy lại từ candidate | Suy PMK từ candidate và chính SSID của record |
| Không thấy `SecurityDowngrade` khi Enterprise → Personal | Chỉ so thế hệ WPA2, bỏ mất thay đổi xác thực | Kiểm tra riêng Enterprise → Personal |
| Kết luận evil twin chỉ từ SSID | Thiếu mô hình ESS/baseline | Thu thập nhiều indicator và giữ kết luận có giới hạn |

---

## 10. Bài tập

### Bài 1 — Tính SNR và tỷ số tuyến tính

Tạo phép đo RSSI −61 dBm, noise floor −91 dBm. Xác nhận SNR 30 dB và tỷ số tuyến tính 1.000. Kiểm tra một phép đo có RSSI thấp hơn noise floor bị mock từ chối.

<details><summary>Gợi ý</summary>

Dùng `SignalMeasurement::from_dbm`; lấy `snr().db()` và `snr().linear_ratio()`. So số thực bằng sai số nhỏ.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_snr() {
    let sample = SignalMeasurement::from_dbm(-61.0, -91.0).expect("phép đo hợp lệ");
    assert!((sample.snr().db() - 30.0).abs() < 1e-12);
    assert!((sample.snr().linear_ratio() - 1_000.0).abs() < 1e-9);
    assert_eq!(
        SignalMeasurement::from_dbm(-95.0, -90.0),
        Err(MeasurementError::SignalBelowNoise)
    );
}
```

</details>

### Bài 2 — Parse beacon và kiểm tra malformed input

Lấy beacon từ `MockWiFiLab`, xác nhận SSID, kênh 6, sequence 7 và retry. Sau đó chứng minh mọi prefix chưa đủ độ dài đều trả lỗi.

<details><summary>Gợi ý</summary>

`beacon_bytes()` trả `Vec<u8>`. Dùng `WirelessFrame::parse` khi cần header và `Beacon::parse` khi chỉ cần beacon.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_beacon() -> Result<(), FrameError> {
    let lab = MockWiFiLab::standard();
    let bytes = lab.beacon_bytes()?;
    let frame = WirelessFrame::parse(&bytes)?;
    assert_eq!(frame.sequence, 7);
    assert!(frame.retry);
    let beacon = Beacon::parse(&bytes)?;
    assert_eq!(beacon.ssid, "OSWP-LAB.example");
    assert_eq!(beacon.channel, 6);
    for length in 0..bytes.len() {
        assert!(Beacon::parse(&bytes[..length]).is_err());
    }
    Ok(())
}
```

</details>

### Bài 3 — Tái lập PTK và MIC trên vector tổng hợp

Dựng handshake với passphrase `password`, SSID `IEEE`, AP/client `02:00:00:00:00:01` và `02:00:00:00:00:02`, ANonce gồm byte `0x11`, SNonce gồm `0x22`. Xác nhận PMK, PTK và MIC theo test vector; candidate sai phải thất bại.

<details><summary>Gợi ý</summary>

Dùng `HandshakeCapture::synthetic_with_parameters`, sau đó `derive_pmk`, `derive_ptk` và `verify_mic`. `hex_encode` giúp đối chiếu bytes.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_ptk_mic() -> Result<(), KdfError> {
    let ap = MacAddress::new([0x02, 0, 0, 0, 0, 1]);
    let client = MacAddress::new([0x02, 0, 0, 0, 0, 2]);
    let capture = HandshakeCapture::synthetic_with_parameters(
        "password", "IEEE", ap, client, [0x11; 32], [0x22; 32],
    )?;
    let pmk = derive_pmk("password", "IEEE")?;
    assert_eq!(
        hex_encode(&pmk),
        "f42c6fc52df0ebef9ebb4b90b38a5f902e83fe1b135a70e23aed762e9710a12e"
    );
    let ptk = capture.derive_ptk(&pmk);
    assert_eq!(hex_encode(&ptk.kck()), "e9cdf3afcaced664d70bc43ecc94ba97");
    assert!(capture.verify_mic(&pmk));
    assert_eq!(hex_encode(&capture.mic), "a0d2b9f9c39fd71b9e661ae8c17d10db");
    assert!(!capture.verify_password("wrong-password"));
    Ok(())
}
```

</details>

### Bài 4 — Phân loại AP nghi evil twin

Chứng minh chỉ đổi BSSID chưa đủ kết luận, còn fixture có Enterprise → Personal, PMF disabled và policy bị xoá phải có nhiều indicator. Kết quả vẫn phải giữ recommendation điều tra.

<details><summary>Gợi ý</summary>

Clone `trusted_ap()` cho trường hợp chỉ đổi BSSID. Với `suspect_ap()`, tìm `SecurityDowngrade`, `PmfDowngrade` và `EnterprisePolicyMismatch`.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_evil_twin() {
    let lab = MockWiFiLab::standard();
    let analyzer = CaptureAnalyzer::new();
    let mut roaming_candidate = lab.trusted_ap().clone();
    roaming_candidate.bssid = MacAddress::new([0x02, 9, 9, 9, 9, 9]);
    let weak = analyzer.assess_evil_twin(lab.trusted_ap(), &roaming_candidate);
    assert_eq!(weak.verdict, EvilTwinVerdict::InsufficientEvidence);
    assert_eq!(weak.indicators, vec![EvilTwinIndicator::BssidMismatch]);

    let suspect = analyzer.assess_evil_twin(lab.trusted_ap(), lab.suspect_ap());
    assert_eq!(suspect.verdict, EvilTwinVerdict::HighConfidence);
    for indicator in [
        EvilTwinIndicator::SecurityDowngrade,
        EvilTwinIndicator::PmfDowngrade,
        EvilTwinIndicator::EnterprisePolicyMismatch,
    ] {
        assert!(suspect.indicators.contains(&indicator));
    }
    assert!(!suspect.recommendation.is_empty());
}
```

</details>

### Bài 5 — Viết hardening report từ assessment

Tạo một bản WPA3-Personal nhưng PMF chỉ `Capable`. Yêu cầu report có finding `WPA3-PMF-NOT-REQUIRED`, evidence/detection/remediation không rỗng và sáu khuyến nghị hardening. Sau khi đặt PMF `Required`, finding đó phải biến mất.

<details><summary>Gợi ý</summary>

Clone AP tin cậy, đổi `security` và `pmf`. `assess_security` trả struct sở hữu findings và recommendations.

</details>

<details><summary>Lời giải</summary>

```rust
pub fn exercise_hardening_report() {
    let lab = MockWiFiLab::standard();
    let analyzer = CaptureAnalyzer::new();
    let mut ap = lab.trusted_ap().clone();
    ap.security = SecuritySuite::Wpa3Personal;
    ap.pmf = PmfMode::Capable;
    ap.enterprise_policy = None;
    let before = analyzer.assess_security(&ap);
    assert!(before.findings.iter().any(|finding| {
        finding.code == "WPA3-PMF-NOT-REQUIRED"
            && !finding.evidence.is_empty()
            && !finding.detection.is_empty()
            && !finding.remediation.is_empty()
    }));
    assert_eq!(before.recommendations.len(), 6);

    ap.pmf = PmfMode::Required;
    let after = analyzer.assess_security(&ap);
    assert!(!after.findings.iter().any(|finding| {
        finding.code == "WPA3-PMF-NOT-REQUIRED"
    }));
}
```

</details>

---

## 11. Tóm tắt và checklist lab

- dBm là logarit; SNR bằng hiệu hai giá trị dBm.
- Kênh primary khác channel width; bonding cần nhóm kênh hợp lệ.
- SSID không định danh duy nhất một AP; BSSID và ESS phải đi cùng mô hình.
- WPA2-PSK dùng PBKDF2-HMAC-SHA1 4096 vòng để tạo PMK 32 byte.
- PTK gắn PMK với địa chỉ và nonce; GTK có vai trò nhóm; MIC không thay replay protection.
- PMKID khác 4-way handshake và phải có kiểu dữ liệu riêng.
- Offline assessment trong crate chỉ kiểm tra fixture tổng hợp.
- Evil-twin assessment phải giữ uncertainty, provenance và bước xác minh.
- Mỗi finding cần evidence, detection, remediation và retest.

Checklist khi chuyển kiến thức vào **lab riêng được ủy quyền**:

- [ ] Có văn bản ủy quyền, phạm vi SSID/khu vực/thời gian và điều kiện dừng.
- [ ] Chỉ dùng adapter/capture thuộc lab; không tác động mạng lân cận.
- [ ] Ghi provenance cho capture và phiên bản cấu hình.
- [ ] Không đưa passphrase/credential thật vào source, log hoặc báo cáo.
- [ ] Tách quan sát, suy luận và điều chưa biết.
- [ ] Có detection/remediation cho từng kỹ thuật được mô phỏng.
- [ ] Cleanup theo RoE, retest và lưu bằng chứng đã redaction.

### Nguồn chuẩn và đọc tiếp

- [FIPS 180-4 — Secure Hash Standard](https://nvlpubs.nist.gov/nistpubs/FIPS/NIST.FIPS.180-4.pdf): SHA-1 dùng trong test vector.
- [RFC 2104 — HMAC](https://www.rfc-editor.org/rfc/rfc2104) và [RFC 8018 — PKCS 5](https://www.rfc-editor.org/rfc/rfc8018): HMAC/PBKDF2.
- [IEEE 802.11](https://standards.ieee.org/ieee/802.11/7028/): MAC/PHY và key hierarchy; cần phiên bản chuẩn phù hợp khi đối chiếu deployment.
- [Wi-Fi Alliance — WPA3](https://www.wi-fi.org/discover-wi-fi/security): tổng quan WPA3 và PMF.
- [OffSec PEN-210/OSWP](https://www.offsec.com/courses/pen-210/): định hướng lab wireless; chương này không sao chép giáo trình có bản quyền.

Chương trước: [Chương 105 — Tác tử Pentest Tự hành](chuong_105.md). Tiếp tục theo [ROADMAP](ROADMAP.md) và outline Phần II; không có liên kết tới chương chưa được publish.
