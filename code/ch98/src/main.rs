//! # Chương 98: Phòng thí nghiệm Khai thác Nhị phân — Mã Dính Lỗi, Mẫu Tuần hoàn, Mã hoá Shellcode & Chuỗi ROP
//! (Binary Exploit Lab: Vulnerable Binaries, Cyclic Patterns, Shellcode Encoding & ROP Chains)
//!
//! Crate `ch98` là **phòng thí nghiệm (lab)** mô phỏng trọn vẹn quy trình khai thác nhị phân
//! (binary exploitation) được viết bằng **100% Rust 2024 thuần `std`**, `[dependencies]` rỗng tuyệt đối.
//! Đây là chương mở đầu **Chủ đề 28 — An ninh Ứng dụng & Khai thác Nhị phân**, nền tảng cho các
//! phòng thí nghiệm tiếp theo (mạng, web, cloud) của giáo trình.
//!
//! **Cam kết đạo đức (ethics-first)**: toàn bộ mã trong crate này chỉ thao tác trên dữ liệu *tổng hợp
//! tất định* — một khung ngăn xếp giả lập trong bộ nhớ, một "kênh truyền" mô phỏng, một vỏ hệ điều
//! hành giả (`MockShell`) và các binary giả được chính crate này dựng lên. Không có đoạn mã nào
//! mở socket, quét cổng, ghi file hệ thống hay gọi `execve`. Những "exploit" trong chương chỉ
//! là phép tính toán thuần túy trên `Vec<u8>`.
//!
//! Các thành phần chính:
//!
//! - [`VulnerableBuffer`]: khung ngăn xếp giả lập có canary (stack cookie), khung con trỏ cơ sở
//!   (saved RBP) và địa chỉ trả về — nơi diễn ra "tràn bộ đệm" (buffer overflow).
//! - [`CyclicPatternGenerator`]: bộ sinh mẫu tuần hoàn kiểu De Bruijn để **định vị chính xác** offset
//!   bị ghi đè mà không cần dò thử mù mẫn.
//! - [`BadCharDetector`]: dò byte "xấu" bị kênh truyền lọc mất hoặc biến đổi (XOR/ADD).
//! - [`ShellcodeEncoder`]: mã hoá shellcode bằng XOR xoay vòng, kèm biến đổi bit `Not/Rol/Ror`.
//! - [`NopSledBuilder`]: dựng "đường trượt NOP" (NOP sled) để tăng biên an toàn của địa chỉ nhảy.
//! - [`ExploitAssembler`]: lắp ráp payload cuối cùng (padding → return address → NOP sled → shellcode).
//! - [`RopGadgetFinder`] & [`RopChainBuilder`]: tìm gadget trong binary giả và xây chuỗi ROP
//!   kiểu `ret2libc` (mô phỏng hoàn toàn trong bộ nhớ).
//! - [`MockShell`]: vỏ hệ điều hành giả để minh hoạ hậu quả của chuỗi ROP.
//!
//! Toàn bộ crate gồm **9 thành phần + 40 bài kiểm thử tự động** chứng minh các bất biến:
//! phát hiện canary, định vị offset, phát hiện byte xấu, tính nghịch đảo của mã hoá XOR, tính hợp
//! lệ của NOP sled, tính đúng đắn của lắp ráp payload và mô phỏng chuỗi ROP `ret2libc`.

#![allow(dead_code)]

use std::collections::HashMap;
use std::ptr;

// ============================================================================
// 1. KHUNG NGĂN XẾP GIẢ LẬP — `VULNERABLEBUFFER`
// ============================================================================

/// Kích thước bộ đệm người dùng điều khiển được nằm trong khung ngăn xếp giả lập.
///
/// Giá trị này quyết định vị trí của canary: canary nằm **ngay sau** bộ đệm, nên một payload dài
/// hơn `BUFFER_SIZE` byte sẽ bắt đầu ghi đè canary.
pub const BUFFER_SIZE: usize = 64;

/// Giá trị canary cố định (stack cookie) dùng cho khung ngăn xếp giả lập.
///
/// Trên hệ thống thật, canary là một giá trị ngẫu nhiên 64-bit lấy từ `/dev/urandom` mỗi lần gọi
/// hàm; ở đây ta cố định nó để mọi bài kiểm thử đều **tất định** (deterministic) và có thể tái lập.
pub const CANARY_MAGIC: u64 = 0x00DE_BEEF_CAFE_1234;

/// Địa chỉ trả về mặc định của khung ngăn xếp giả lập (địa chỉ "hợp lệ" của hàm gọi).
pub const DEFAULT_RETURN_ADDRESS: u64 = 0x0040_1000;

/// Địa chỉ khung ngăn xếp của hàm gọi (saved RBP) — con trỏ khung ngăn xếp trước đó.
pub const DEFAULT_SAVED_RBP: u64 = 0x0000_7FFF_0000_0000;

/// Tổng chiều dài của khung ngăn xếp giả lập: bộ đệm + canary + saved RBP + địa chỉ trả về.
pub const FRAME_SIZE: usize = BUFFER_SIZE + 8 + 8 + 8;

/// Khung ngăn xếp giả lập của một hàm "dễ bị khai thác".
///
/// Bốn trường công khai mô tả đúng trật tự bộ nhớ của một khung ngăn xếp trên kiến trúc x86-64:
///
/// ```text
///  địa chỉ cao hơn  ┌───────────────────┐  offset 80
///                  │  return_address   │  (8 byte) — đích của lệnh `ret`
///                  ├───────────────────┤  offset 72
///                  │    saved_rbp      │  (8 byte) — khung ngăn xếp của hàm gọi
///                  ├───────────────────┤  offset 64
///                  │    canary_value   │  (8 byte) — stack cookie
///                  ├───────────────────┤  offset 0
///                  │      buffer       │  (64 byte) — bộ đệm người dùng điều khiển
///  địa chỉ thấp hơn └───────────────────┘
/// ```
///
/// Bản thân struct **không** bị ghi vượt biên: toàn bộ hiện tượng "tràn bộ đệm" được mô phỏng
/// bằng một bản sao byte của khung ngăn xếp, nên không có hành vi không xác định (UB).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VulnerableBuffer {
    /// Bộ đệm người dùng điều khiển được (64 byte).
    pub buffer: [u8; BUFFER_SIZE],
    /// Giá trị canary hiện tại trong khung ngăn xếp.
    pub canary_value: u64,
    /// Con trỏ khung ngăn xếp được lưu lại (saved RBP).
    pub saved_rbp: u64,
    /// Địa chỉ trả về — giá trị mà lệnh `ret` sẽ nhảy tới.
    pub return_address: u64,
}

impl Default for VulnerableBuffer {
    /// Tạo khung ngăn xếp với canary và địa chỉ trả về mặc định.
    fn default() -> Self {
        Self::new()
    }
}

impl VulnerableBuffer {
    /// Khởi tạo một khung ngăn xếp "sạch".
    ///
    /// * `buffer` toàn bộ 0 byte;
    /// * `canary_value` = [`CANARY_MAGIC`];
    /// * `saved_rbp` = [`DEFAULT_SAVED_RBP`];
    /// * `return_address` = [`DEFAULT_RETURN_ADDRESS`].
    pub fn new() -> Self {
        Self {
            buffer: [0u8; BUFFER_SIZE],
            canary_value: CANARY_MAGIC,
            saved_rbp: DEFAULT_SAVED_RBP,
            return_address: DEFAULT_RETURN_ADDRESS,
        }
    }

    /// Ghi dữ liệu người dùng vào bộ đệm, **mô phỏng** hành vi của một `strcpy`/`read` không kiểm tra biên.
    ///
    /// Hàm trả về `true` khi dữ liệu **vừa khít** trong bộ đệm (`data.len() <= BUFFER_SIZE`) và
    /// `false` khi dữ liệu tràn ra ngoài. Khi tràn, phần dữ liệu vượt quá `BUFFER_SIZE` sẽ ghi đè
    /// lần lượt lên canary, `saved_rbp` và `return_address` — đúng như một lỗi tràn bộ đệm thật.
    ///
    /// Để giữ tính an toàn của Rust, phần tràn **không** ghi vào vùng nhớ Rust thật; thay vào đó
    /// ta dựng một bản sao byte của 24 byte đuôi khung ngăn xếp rồi ghi đè lên đó.
    pub fn write_input(&mut self, data: &[u8]) -> bool {
        let in_len = data.len().min(BUFFER_SIZE);
        if in_len > 0 {
            // SAFETY: `in_len <= BUFFER_SIZE == self.buffer.len()` nên vùng đích nằm trọn trong
            // `self.buffer`; `data` là một lát byte hợp lệ có ít nhất `in_len` byte nên vùng nguồn
            // cũng hợp lệ. `data` được mượn bất biến từ bên ngoài còn `self.buffer` thuộc sở hữu
            // của `self`, do đó hai vùng không thể chồng lấn — điều kiện của `copy_nonoverlapping`
            // được thoả mãn hoàn toàn.
            unsafe {
                ptr::copy_nonoverlapping(data.as_ptr(), self.buffer.as_mut_ptr(), in_len);
            }
        }

        let spill_len = data.len().saturating_sub(BUFFER_SIZE);
        if spill_len > 0 {
            // Mô phỏng phần tràn xuống 24 byte đuôi khung ngăn xếp (canary + rbp + return address).
            let mut tail = [0u8; 24];
            tail[0..8].copy_from_slice(&self.canary_value.to_le_bytes());
            tail[8..16].copy_from_slice(&self.saved_rbp.to_le_bytes());
            tail[16..24].copy_from_slice(&self.return_address.to_le_bytes());

            let copy_len = spill_len.min(tail.len());
            // SAFETY: `copy_len <= tail.len()` nên vùng đích hợp lệ; `data.len() >= BUFFER_SIZE +
            // copy_len` nên vùng nguồn (`data.as_ptr().add(BUFFER_SIZE)`) cũng hợp lệ. `tail` là mảng
            // cục bộ nằm trên ngăn xếp của lời gọi này, không thể chồng lấn với lát `data` đang mượn.
            unsafe {
                ptr::copy_nonoverlapping(
                    data.as_ptr().add(BUFFER_SIZE),
                    tail.as_mut_ptr(),
                    copy_len,
                );
            }

            self.canary_value = u64::from_le_bytes(tail[0..8].try_into().expect("8 byte canary"));
            self.saved_rbp = u64::from_le_bytes(tail[8..16].try_into().expect("8 byte rbp"));
            self.return_address = u64::from_le_bytes(tail[16..24].try_into().expect("8 byte ret"));
        }

        data.len() <= BUFFER_SIZE
    }

    /// Kiểm tra canary còn nguyên vẹn hay không.
    ///
    /// Nếu `false`, chương trình đã bị phát hiện tràn bộ đệm — đây chính là cơ chế bảo vệ của
    /// stack protector.
    pub fn is_canary_intact(&self) -> bool {
        self.canary_value == CANARY_MAGIC
    }

    /// Kiểm tra địa chỉ trả về đã bị người tấn công ghi đè hay chưa.
    pub fn did_overwrite_return_address(&self) -> bool {
        self.return_address != DEFAULT_RETURN_ADDRESS
    }

    /// Mô phỏng lệnh `ret`: trả về địa chỉ mà chương trình **sẽ** nhảy tới.
    ///
    /// Hàm này chỉ *trả về giá trị*, tuyệt đối không thực hiện nhảy nhánh thật.
    pub fn trigger(&self) -> u64 {
        // SAFETY: `trigger` KHÔNG thực hiện `ret` thật và không dereference bất kỳ con trỏ nào —
        // nó chỉ đọc một giá trị `u64` đã lưu trong struct rồi trả về. Vì vậy dù giá trị trả về
        // là địa chỉ do kẻ tấn công kiểm soát, hành vi của hàm vẫn là hành vi *đã định nghĩa*
        // (well-defined) và không thể gây ra undefined behaviour.
        self.return_address
    }

    /// Vị trí (offset) của canary trong "khung ngăn xếp", tính từ đầu bộ đệm.
    pub fn canary_position(&self) -> usize {
        BUFFER_SIZE
    }

    /// Mô tả bố cục khung ngăn xếp dưới dạng `(tên, offset, kích thước)`.
    ///
    /// Đây là hàm **associated** (không cần `self`) vì bố cục là thuộc tính cố định của kiểu dữ liệu.
    pub fn stack_frame_layout() -> Vec<(&'static str, usize, usize)> {
        vec![
            ("buffer", 0, BUFFER_SIZE),
            ("canary", BUFFER_SIZE, 8),
            ("saved_rbp", BUFFER_SIZE + 8, 8),
            ("return_address", BUFFER_SIZE + 16, 8),
        ]
    }

    /// Dựng lại toàn bộ khung ngăn xếp dưới dạng byte thô (88 byte), theo thứ tự tăng dần địa chỉ.
    pub fn frame_bytes(&self) -> Vec<u8> {
        let mut frame = Vec::with_capacity(FRAME_SIZE);
        frame.extend_from_slice(&self.buffer);
        frame.extend_from_slice(&self.canary_value.to_le_bytes());
        frame.extend_from_slice(&self.saved_rbp.to_le_bytes());
        frame.extend_from_slice(&self.return_address.to_le_bytes());
        frame
    }

    /// Số byte bị tràn ra ngoài bộ đệm nếu gửi vào `data_len` byte dữ liệu.
    pub fn overflow_amount(&self, data_len: usize) -> usize {
        data_len.saturating_sub(BUFFER_SIZE)
    }

    /// Đọc 8 byte địa chỉ trả về trong khung ngăn xếp dưới dạng byte thô.
    pub fn return_address_bytes(&self) -> [u8; 8] {
        self.return_address.to_le_bytes()
    }

    /// Khôi phục khung ngăn xếp về trạng thái ban đầu.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Sinh bản mô tả dạng văn bản của trạng thái khung ngăn xếp (dùng để in minh hoạ).
    pub fn describe(&self) -> String {
        format!(
            "buffer[0..64] | canary=0x{:016X} ({}) | rbp=0x{:016X} | ret=0x{:016X} ({})",
            self.canary_value,
            if self.is_canary_intact() {
                "NGUYÊN VẸN"
            } else {
                "ĐÃ BỊ GHI ĐÈ"
            },
            self.saved_rbp,
            self.return_address,
            if self.did_overwrite_return_address() {
                "ĐÃ KHIỂM SOÁT"
            } else {
                "MẶC ĐỊNH"
            }
        )
    }
}

// ============================================================================
// 2. BỘ SINH MẪU TUẦN HOÀN — `CYCLICPATTERNGENERATOR`
// ============================================================================

/// Số ký tự trong một nhãn định vị (chunk label).
///
/// Mỗi nhãn gồm 4 byte in được, đủ để **định vị duy nhất** một offset trong mẫu.
pub const CHUNK_LABEL_LEN: usize = 4;

/// Số ký tự của bảng chữ cái dùng làm mẫu: `A-Z` (26) + `a-z` (26) + `0-9` (10) = 62 ký tự.
pub const ALPHABET_LEN: usize = 62;

/// Bảng ký tự dùng ở các vị trí chẵn (0, 2) của nhãn 4 ký tự: chữ hoa `A-Z` và chữ số `0-9`.
const EVEN_CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

/// Bảng ký tự dùng ở các vị trí lẻ (1, 3) của nhãn 4 ký tự: chữ thường `a-z`.
///
/// Việc chia bảng chữ cái theo **parity** (chẵn/lẻ) đảm bảo rằng một cửa sổ 4 byte chỉ có thể
/// khớp ở đúng một phân lớp offset — nhờ đó `find_offset` luôn trả về kết quả **duy nhất**.
const ODD_CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyz";

/// Kích thước tối thiểu của "chuỗi hay" (haystack) dùng để quét tìm nhãn.
///
/// Cửa sổ 2048 byte đủ rộng để định vị **mọi** nhãn có offset nhỏ hơn 2044 byte — vượt xa mọi
/// kích thước khung ngăn xếp thực tế — đồng thời nhỏ hơn vị trí tái xuất hiện thứ hai của nhãn
/// đầu tiên (3742 byte), nên kết quả tìm kiếm luôn **duy nhất**.
pub const MIN_SEARCH_WINDOW: usize = 2048;

/// Bộ sinh mẫu tuần hoàn kiểu De Bruijn dùng để định vị offset bị ghi đè.
///
/// Mỗi nhãn 4 ký tự là một số đếm cơ sở hỗn (mixed-radix) theo thứ tự
/// `36 (chữ hoa/số) → 26 (chữ thường) → 36 (chữ hoa/số) → 26 (chữ thường)`, nên:
///
/// * nhãn đầu tiên là `"AaAa"`;
/// * hai nhãn bất kỳ **không bao giờ trùng nhau** trong phạm vi `36·26·36·26 = 876 096` nhãn;
/// * mọi byte sinh ra đều thuộc bảng chữ cái 62 ký tự `A-Za-z0-9` (thân thiện với kênh văn bản).
///
/// Nhờ tính duy nhất này, ta có thể **xác định tuyệt đối** offset đã ghi đè mà không cần dò thử
/// mù mẫn — đây chính là điểm mạnh của mẫu kiểu De Bruijn so với mẫu "lặp bảng chữ cái" cổ điển.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CyclicPatternGenerator {
    /// Bảng chữ cái 62 ký tự được dùng để sinh mẫu.
    pub alphabet: Vec<u8>,
    /// Độ dài chu kỳ: 3 lần trải qua bảng chữ cái 62 ký tự (`62 × 3 = 186` byte).
    pub cycle_len: usize,
}

impl Default for CyclicPatternGenerator {
    /// Tạo bộ sinh với bảng chữ cái chuẩn `A-Za-z0-9` và chu kỳ 186 byte.
    fn default() -> Self {
        Self::new()
    }
}

impl CyclicPatternGenerator {
    /// Khởi tạo bộ sinh mẫu với bảng chữ cái `A-Za-z0-9` và `cycle_len = 62 × 3 = 186`.
    pub fn new() -> Self {
        let mut alphabet = Vec::with_capacity(ALPHABET_LEN);
        alphabet.extend_from_slice(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ");
        alphabet.extend_from_slice(b"abcdefghijklmnopqrstuvwxyz");
        alphabet.extend_from_slice(b"0123456789");
        Self {
            alphabet,
            cycle_len: ALPHABET_LEN * 3,
        }
    }

    /// Sinh `length` byte mẫu.
    ///
    /// Mẫu được chia thành các nhãn 4 byte liên tiếp; nhãn thứ `n` (đánh số từ 0) được mã hoá
    /// bằng hệ cơ sở hỗn nên **mọi nhãn đều khác nhau**.
    pub fn generate(&self, length: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(length);
        for index in 0..length {
            let chunk = (index / CHUNK_LABEL_LEN) as u64;
            let position = index % CHUNK_LABEL_LEN;
            out.push(Self::chunk_bytes(chunk)[position]);
        }
        out
    }

    /// Mã hoá chỉ số nhãn `chunk_index` thành 4 byte theo cơ sở hỗn 36·26·36·26.
    ///
    /// Thứ tự "chạy nhanh nhất" của các chữ số là **ngược lại** với thứ tự byte: chữ số hệ 36
    /// (ký tự ở vị trí chẵn) thay đổi nhanh nhất, rồi mới tới chữ số hệ 26 (ký tự ở vị trí lẻ).
    ///
    /// Nhờ thứ tự này, hai nhãn liền kề **không** dùng chung hai byte đầu, nên một cửa sổ 4 byte
    /// "trượt" qua ranh giới giữa hai nhãn không thể trùng với bất kỳ nhãn hợp lệ nào — đảm bảo
    /// nhãn là duy nhất (xem `find_offset`).
    fn chunk_bytes(chunk_index: u64) -> [u8; CHUNK_LABEL_LEN] {
        let d0 = (chunk_index % 36) as usize;
        let rest = chunk_index / 36;
        let d1 = (rest % 26) as usize;
        let rest = rest / 26;
        let d2 = (rest % 36) as usize;
        let d3 = ((rest / 36) % 26) as usize;
        [EVEN_CHARS[d0], ODD_CHARS[d1], EVEN_CHARS[d2], ODD_CHARS[d3]]
    }

    /// Tìm vị trí bắt đầu của chuỗi `sequence` trong mẫu tuần hoàn.
    ///
    /// Hàm dựng một "chuỗi hay" (haystack) rộng ít nhất [`MIN_SEARCH_WINDOW`] byte rồi quét
    /// tìm cửa sổ khớp đầu tiên. Với nhãn 4 byte sinh từ [`CyclicPatternGenerator::generate`],
    /// kết quả là **duy nhất** và chính xác bằng offset gốc.
    pub fn find_offset(&self, sequence: &[u8]) -> Option<usize> {
        if sequence.is_empty() {
            return None;
        }
        let window = (sequence.len() + self.cycle_len + CHUNK_LABEL_LEN).max(MIN_SEARCH_WINDOW);
        let haystack = self.generate(window);
        haystack
            .windows(sequence.len())
            .position(|window| window == sequence)
    }

    /// Số byte tối thiểu cần gửi để payload **bao phủ trọn** nhãn 4 ký tự nằm tại `offset`.
    ///
    /// Nếu chỉ quan tâm tới byte đầu tiên tại `offset` thì cần `offset + 1` byte; bản đảm bảo
    /// nguyên nhãn giúp ta đọc được "chữ ký" 4 byte của vùng bị ghi đè trong một lần.
    pub fn pattern_size_for_offset(&self, offset: usize) -> usize {
        offset + CHUNK_LABEL_LEN
    }

    /// Kiểm tra xem chuỗi `observed` quan sát được có khớp với mẫu tại `offset` hay không.
    pub fn verify_offset(&self, offset: usize, observed: &[u8]) -> bool {
        if observed.is_empty() {
            return false;
        }
        let stream = self.generate(offset + observed.len());
        stream[offset..].starts_with(observed)
    }

    /// Trả về nhãn 4 ký tự tại `offset` (dạng chuỗi ASCII in được).
    ///
    /// Nhãn là **định danh duy nhất** của offset khi offset nằm ở ranh giới nhãn (chia hết cho 4).
    pub fn chunk_label(&self, offset: usize) -> String {
        self.generate(offset + CHUNK_LABEL_LEN)[offset..]
            .iter()
            .map(|&byte| char::from(byte))
            .collect()
    }

    /// Chỉ số nhãn chứa offset `offset`.
    pub fn chunk_index_of(&self, offset: usize) -> usize {
        offset / CHUNK_LABEL_LEN
    }

    /// Kiểm tra offset có nằm đúng ranh giới nhãn 4 byte hay không.
    pub fn is_chunk_aligned(&self, offset: usize) -> bool {
        offset.is_multiple_of(CHUNK_LABEL_LEN)
    }

    /// Kiểm tra một byte có nằm trong bảng chữ cái của bộ sinh hay không.
    pub fn is_alphabet_byte(&self, byte: u8) -> bool {
        self.alphabet.contains(&byte)
    }

    /// Sinh `count` nhãn liên tiếp (dùng để kiểm tra tính duy nhất).
    pub fn labels(&self, count: usize) -> Vec<String> {
        (0..count)
            .map(|index| self.chunk_label(index * CHUNK_LABEL_LEN))
            .collect()
    }
}

// ============================================================================
// 3. DÒ BYTE "XẤU" — `BADCHARDETECTOR`
// ============================================================================

/// Kênh truyền dữ liệu giả lập — mô phỏng một kênh (socket/pipe) bị lọc bỏ vài byte.
///
/// Kênh thật trong bài lab này **không** mở socket; nó chỉ là một hàm xác định ánh xạ
/// `byte → Option<Vec<u8>>` (không có byte nào bị loại bỏ trừ khi được cấu hình như vậy).
#[derive(Debug, Clone, Default)]
pub struct MockChannel {
    /// Danh sách các byte đã bị kênh loại bỏ trong lần truyền gần nhất.
    pub dropped: Vec<u8>,
}

impl MockChannel {
    /// Tạo kênh truyền mặc định (chưa loại bỏ byte nào).
    pub fn new() -> Self {
        Self {
            dropped: Vec::new(),
        }
    }

    /// Truyền một byte qua kênh.
    ///
    /// Trả về `None` nếu byte bị loại bỏ (mặc định loại `0x00` — byte kết thúc chuỗi C — và
    /// `0x0A` — ký tự xuống dòng — vì đây là hai byte "xấu kinh điển" trong mọi tận công).
    pub fn transmit(&mut self, byte: u8) -> Option<Vec<u8>> {
        if byte == 0x00 || byte == 0x0A {
            self.dropped.push(byte);
            None
        } else {
            Some(vec![byte])
        }
    }
}

/// Bộ dò byte "xấu" (bad character) — byte bị kênh truyền loại bỏ hoặc biến đổi.
///
/// Chiến lược kinh điển: gửi lần lượt từng byte `0x00..=0xFF` qua kênh, so byte nhận về với
/// byte gửi đi. Byte nào **không quay lại nguyên vẹn** là byte xấu và tuyệt đối không được
/// xuất hiện trong payload cuối cùng.
#[derive(Debug, Clone, Default)]
pub struct BadCharDetector {
    /// Các byte đã biết là xấu (có thể do người dùng khai báo thủ công).
    pub bad_chars: Vec<u8>,
    /// Lịch sử từng lần thử: `(byte gửi đi, byte nhận về)`.
    pub responses: Vec<(u8, Vec<u8>)>,
}

impl BadCharDetector {
    /// Tạo bộ dò trống (chưa có byte xấu nào, chưa có lần thử nào).
    pub fn new() -> Self {
        Self {
            bad_chars: Vec::new(),
            responses: Vec::new(),
        }
    }

    /// Ghi nhận một lần thử: đã gửi `sent`, nhận về `received`.
    pub fn record(&mut self, sent: u8, received: Vec<u8>) {
        self.responses.push((sent, received));
    }

    /// Phát hiện các byte bị loại bỏ hoặc bị thay thế.
    ///
    /// Quy tắc: byte gửi `sent` bị coi là **xấu** nếu
    ///
    /// * kênh không trả về byte nào (`received` rỗng) — byte bị lọc mất; hoặc
    /// * byte đầu tiên nhận về khác `sent` — byte bị thay thế hoặc biến đổi.
    pub fn detect(&self) -> Vec<u8> {
        let mut bad = self.bad_chars.clone();
        for (sent, received) in &self.responses {
            let corrupted = match received.first() {
                None => true,
                Some(&first) => first != *sent,
            };
            if corrupted && !bad.contains(sent) {
                bad.push(*sent);
            }
        }
        bad.sort_unstable();
        bad.dedup();
        bad
    }

    /// Kiểm tra một byte có phải byte xấu hay không.
    pub fn is_bad(&self, byte: u8) -> bool {
        self.detect().contains(&byte)
    }

    /// Loại bỏ mọi byte xấu khỏi `payload`.
    pub fn filter_payload(&self, payload: &[u8]) -> Vec<u8> {
        let bad = self.detect();
        payload
            .iter()
            .copied()
            .filter(|byte| !bad.contains(byte))
            .collect()
    }

    /// Ghi nhận một phép biến đổi (XOR/ADD) mà kênh truyền áp dụng lên byte.
    ///
    /// Hàm so `received[0] ^ sent` (độ lệch XOR) và `received[0] - sent` (độ lệch cộng).
    /// Nếu cả hai đều bằng `0` thì byte được giữ nguyên; ngược lại byte bị biến đổi và bị
    /// ghi vào danh sách byte xấu.
    pub fn detect_from_transformation(&mut self, sent: u8, received: Vec<u8>) {
        let xor_delta = received.first().map(|&first| first ^ sent).unwrap_or(0);
        let add_delta = received
            .first()
            .map(|&first| first.wrapping_sub(sent))
            .unwrap_or(0);
        let transformed = received.is_empty() || xor_delta != 0 || add_delta != 0;

        self.responses.push((sent, received));
        if transformed && !self.bad_chars.contains(&sent) {
            self.bad_chars.push(sent);
            self.bad_chars.sort_unstable();
        }
    }

    /// Dò toàn bộ 256 byte qua kênh `channel` và ghi nhận kết quả.
    ///
    /// Hàm trả về danh sách byte xấu tìm được.
    pub fn probe_all_bytes(&mut self, channel: &mut MockChannel) -> Vec<u8> {
        for byte in 0u16..=255 {
            let sent = byte as u8;
            let received = channel.transmit(sent).unwrap_or_default();
            self.record(sent, received);
        }
        self.detect()
    }

    /// Số lượng byte xấu đã biết.
    pub fn bad_char_count(&self) -> usize {
        self.detect().len()
    }

    /// Sinh bản mô tả văn bản của tập byte xấu (dùng để in minh hoạ).
    pub fn summary(&self) -> String {
        let bad = self.detect();
        if bad.is_empty() {
            return "không có byte xấu nào".to_string();
        }
        let items: Vec<String> = bad.iter().map(|byte| format!("0x{byte:02X}")).collect();
        format!("{} byte xấu: {}", bad.len(), items.join(", "))
    }
}

// ============================================================================
// 4. MÃ HOÁ SHELLCODE — `SHELLCODEENCODER`
// ============================================================================

/// Phép biến đổi bit áp dụng lên từng byte của shellcode.
///
/// Ngoài phép biến đổi đơn giản, ta còn cần các phép **tự nghịch đảo** (involutive) để giải mã
/// được mà không cần biết thứ tự phép biến đổi đã áp dụng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BitwiseTransform {
    /// Giữ nguyên byte.
    Identity,
    /// Đảo bit (NOT) — tự nghịch đảo.
    Not,
    /// Xoay vòng trái `n` bit — nghịch đảo là xoay vòng phải `n` bit.
    Rol(u8),
    /// Xoay vòng phải `n` bit — nghịch đảo là xoay vòng trái `n` bit.
    Ror(u8),
}

impl BitwiseTransform {
    /// Áp dụng phép biến đổi lên một byte đơn lẻ.
    pub fn apply(&self, byte: u8) -> u8 {
        match self {
            BitwiseTransform::Identity => byte,
            BitwiseTransform::Not => !byte,
            BitwiseTransform::Rol(bits) => byte.rotate_left(u32::from(*bits % 8)),
            BitwiseTransform::Ror(bits) => byte.rotate_right(u32::from(*bits % 8)),
        }
    }

    /// Trả về phép biến đổi **nghịch đảo** của `self`.
    pub fn inverse(&self) -> BitwiseTransform {
        match self {
            BitwiseTransform::Identity => BitwiseTransform::Identity,
            BitwiseTransform::Not => BitwiseTransform::Not,
            BitwiseTransform::Rol(bits) => BitwiseTransform::Ror(*bits),
            BitwiseTransform::Ror(bits) => BitwiseTransform::Rol(*bits),
        }
    }

    /// Tên hiển thị của phép biến đổi.
    pub fn name(&self) -> &'static str {
        match self {
            BitwiseTransform::Identity => "Identity",
            BitwiseTransform::Not => "Not",
            BitwiseTransform::Rol(bits) => match bits {
                0 => "Rol(0)",
                1 => "Rol(1)",
                2 => "Rol(2)",
                3 => "Rol(3)",
                4 => "Rol(4)",
                5 => "Rol(5)",
                6 => "Rol(6)",
                _ => "Rol(7)",
            },
            BitwiseTransform::Ror(bits) => match bits {
                0 => "Ror(0)",
                1 => "Ror(1)",
                2 => "Ror(2)",
                3 => "Ror(3)",
                4 => "Ror(4)",
                5 => "Ror(5)",
                6 => "Ror(6)",
                _ => "Ror(7)",
            },
        }
    }
}

/// Byte đánh dấu đầu của "stub giải mã" mô phỏng (`0xEB` = lệnh `JMP short` của x86).
pub const STUB_MAGIC: u8 = 0xEB;

/// Tổng chiều dài phần đầu của stub giải mã mô phỏng (magic + filler + 2 trường 32-bit).
pub const STUB_HEADER_LEN: usize = 10;

/// Bộ mã hoá shellcode bằng XOR xoay vòng khoá.
///
/// Phép XOR là **tự nghịch đảo**: $\text{dec}(\text{enc}(x)) = x$ vì
/// $\left(x \oplus k\right) \oplus k = x$. Đây là nền tảng của cả "mã hoá" trong bài lab lẫn
/// kỹ thuật "decoder stub" (mã giải mã chạy trong bộ nhớ) ngoài đời.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellcodeEncoder {
    /// Khoá XOR (xoay vòng). Rỗng nghĩa là "không mã hoá".
    pub key: Vec<u8>,
}

impl ShellcodeEncoder {
    /// Tạo bộ mã hoá với khoá cho trước.
    pub fn new(key: Vec<u8>) -> Self {
        Self { key }
    }

    /// Mã hoá shellcode bằng XOR xoay vòng với khoá.
    ///
    /// Công thức: $\;e_i = s_i \oplus k_{i \bmod |k|}\;$.
    pub fn xor_encode(&self, shellcode: &[u8]) -> Vec<u8> {
        if self.key.is_empty() {
            return shellcode.to_vec();
        }
        shellcode
            .iter()
            .enumerate()
            .map(|(index, &byte)| byte ^ self.key[index % self.key.len()])
            .collect()
    }

    /// Giải mã dữ liệu đã mã hoá XOR.
    ///
    /// Vì XOR tự nghịch đảo nên hàm này **giống hệt** [`ShellcodeEncoder::xor_encode`], nhưng
    /// được giữ riêng để mã nguồn nói rõ ý định.
    pub fn xor_decode(&self, data: &[u8]) -> Vec<u8> {
        if self.key.is_empty() {
            return data.to_vec();
        }
        data.iter()
            .enumerate()
            .map(|(index, &byte)| byte ^ self.key[index % self.key.len()])
            .collect()
    }

    /// Dựng **stub giải mã mô phỏng** cho `encoded_len` byte payload đã mã hoá.
    ///
    /// ⚠️ **Không phải shellcode thật** — đây là một *mô tả* (descriptor) có cấu trúc để ta có thể
    /// kiểm chứng việc giải mã bằng [`ShellcodeEncoder::simulate_decode`]:
    ///
    /// ```text
    ///  offset 0 : 0xEB              — marker "JMP short", dấu hiệu stub mô phỏng
    ///  offset 1 : 0x00              — byte dự phòng
    ///  offset 2 : key_len    (u32 LE)
    ///  offset 6 : encoded_len(u32 LE)
    ///  offset 10: key bytes (key_len byte)
    ///  offset 10+key_len: vùng payload giả lấp bằng 0x90 (encoded_len byte)
    /// ```
    pub fn build_decoder_stub(&self, encoded_len: usize) -> Vec<u8> {
        let key_len = self.key.len().max(1);
        let mut stub = Vec::with_capacity(STUB_HEADER_LEN + key_len + encoded_len);
        stub.push(STUB_MAGIC);
        stub.push(0x00);
        stub.extend_from_slice(&(key_len as u32).to_le_bytes());
        stub.extend_from_slice(&(encoded_len as u32).to_le_bytes());
        stub.extend_from_slice(&self.key);
        stub.extend(std::iter::repeat_n(0x90u8, encoded_len));
        stub
    }

    /// Mô phỏng quá trình chạy stub giải mã: trả về payload đã giải mã.
    ///
    /// Hàm kiểm tra tính nhất quán của stub (magic, độ dài khoá, khoá nhúng có khớp khoá thật)
    /// rồi giải mã bằng XOR. Nếu stub không hợp lệ, trả về vector rỗng.
    pub fn simulate_decode(stub: &[u8], encoded: &[u8], key: &[u8]) -> Vec<u8> {
        if stub.len() < STUB_HEADER_LEN || stub[0] != STUB_MAGIC {
            return Vec::new();
        }
        let key_len = u32::from_le_bytes(
            stub[2..6]
                .try_into()
                .expect("trường key_len dài 4 byte trong stub"),
        ) as usize;
        let encoded_len = u32::from_le_bytes(
            stub[6..10]
                .try_into()
                .expect("trường encoded_len dài 4 byte trong stub"),
        ) as usize;

        if stub.len() < STUB_HEADER_LEN + key_len || key.is_empty() {
            return Vec::new();
        }
        let stub_key = &stub[STUB_HEADER_LEN..STUB_HEADER_LEN + key_len];
        if stub_key != key || encoded.len() < encoded_len {
            return Vec::new();
        }

        encoded[..encoded_len]
            .iter()
            .enumerate()
            .map(|(index, &byte)| byte ^ key[index % key.len()])
            .collect()
    }

    /// Mã hoá kèm phép biến đổi bit.
    ///
    /// Quy trình: `enc = T(xor_encode(s))` — biến đổi bit được áp **sau** phép XOR.
    pub fn encode_with_transform(&self, shellcode: &[u8], transform: BitwiseTransform) -> Vec<u8> {
        self.xor_encode(shellcode)
            .into_iter()
            .map(|byte| transform.apply(byte))
            .collect()
    }

    /// Giải mã ngược lại phép biến đổi bit rồi tới phép XOR.
    ///
    /// Quy trình: `dec = xor_decode(T⁻¹(data))`.
    pub fn decode_with_transform(&self, data: &[u8], transform: BitwiseTransform) -> Vec<u8> {
        let inverse = transform.inverse();
        let restored: Vec<u8> = data.iter().map(|&byte| inverse.apply(byte)).collect();
        self.xor_decode(&restored)
    }

    /// Tìm một khoá XOR **1 byte** sao cho payload mã hoá không chứa byte xấu nào.
    ///
    /// Thử lần lượt 256 khoá; trả về `None` nếu không tồn tại khoá phù hợp (ví dụ khi tập byte
    /// xấu bao phủ toàn bộ 256 giá trị).
    pub fn find_single_byte_key(&self, shellcode: &[u8], bad_chars: &[u8]) -> Option<u8> {
        for candidate in 0u16..=255 {
            let key = candidate as u8;
            let encoded: Vec<u8> = shellcode.iter().map(|&byte| byte ^ key).collect();
            if !encoded.iter().any(|byte| bad_chars.contains(byte)) {
                return Some(key);
            }
        }
        None
    }

    /// Mã hoá "tối ưu": trả về `(payload đã mã hoá, khoá đã dùng)`.
    ///
    /// Thứ tự ưu tiên:
    ///
    /// 1. khoá 1 byte (nhỏ gọn nhất, dễ đoán);
    /// 2. khoá nhiều byte đảo bit (`NOT`) — vẫn giải mã được bằng cùng thuật toán;
    /// 3. khoá nhiều byte gốc — phương án cuối cùng.
    pub fn encode_best(&self, shellcode: &[u8], bad_chars: &[u8]) -> (Vec<u8>, Vec<u8>) {
        if let Some(key) = self.find_single_byte_key(shellcode, bad_chars) {
            let encoded: Vec<u8> = shellcode.iter().map(|&byte| byte ^ key).collect();
            return (encoded, vec![key]);
        }

        let inverted_key: Vec<u8> = self.key.iter().map(|byte| !byte).collect();
        let inverted = ShellcodeEncoder::new(inverted_key.clone());
        let encoded = inverted.xor_encode(shellcode);
        if !encoded.iter().any(|byte| bad_chars.contains(byte)) {
            return (encoded, inverted_key);
        }

        (self.xor_encode(shellcode), self.key.clone())
    }

    /// Độ dài khoá hiện tại.
    pub fn key_len(&self) -> usize {
        self.key.len()
    }

    /// Vân tay FNV-1a 64-bit của khoá — giúp so sánh hai khoá mà không lộ nội dung.
    pub fn key_fingerprint(&self) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for &byte in &self.key {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        hash
    }
}

// ============================================================================
// 5. ĐƯỜNG TRƯỢT NOP — `NOPSLEDBUILDER`
// ============================================================================

/// Mã lệnh NOP một byte của x86-64 (`0x90`).
pub const NOP_BYTE: u8 = 0x90;

/// Mẫu NOP 7 byte: 3 lệnh NOP một byte + lệnh `nop dword [rax+0x0]` (4 byte).
pub const MULTIBYTE_NOP_7: [u8; 7] = [0x90, 0x90, 0x90, 0x0F, 0x1F, 0x40, 0x00];

/// Mẫu NOP 2 byte: `xchg ax, ax` (`0x66 0x90`).
pub const MULTIBYTE_NOP_2: [u8; 2] = [0x66, 0x90];

/// Mẫu NOP 1 byte dùng khi phần chừa của sled chỉ còn đúng 1 byte.
pub const MULTIBYTE_NOP_1: [u8; 1] = [0x90];

/// Bộ dựng đường trượt NOP (NOP sled).
///
/// NOP sled là một vùng toàn NOP nằm **trước** shellcode: khi địa chỉ trả về bị trượt (off-by-one)
/// vào giữa vùng này, CPU vẫn "chạy" qua các lệnh NOP vô hại cho tới khi tới shellcode. Nhờ đó
/// ta không cần biết chính xác vị trí đích, chỉ cần nằm trong khoảng độ rộng của sled.
///
/// Bản "đa byte" dùng các mẫu NOP dài hơn (2 hoặc 7 byte) — cần thiết khi kênh truyền **lọc**
/// byte `0x90` (một số bộ lọc WAF/IDS chặn chuỗi NOP lặp liên tiếp).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NopSledBuilder {
    /// Tổng chiều dài sled (byte).
    pub length: usize,
    /// Có dùng mẫu NOP đa byte hay không.
    pub use_multibyte: bool,
    /// Hạt giống cho bộ sinh ngẫu nhiên tất định chọn mẫu NOP.
    pub seed: u64,
}

impl NopSledBuilder {
    /// Tạo sled dài `length` byte dùng NOP một byte (`0x90`).
    pub fn new(length: usize) -> Self {
        Self {
            length,
            use_multibyte: false,
            seed: 0x0000_5EED_0BAD_F00D,
        }
    }

    /// Bật chế độ NOP đa byte với hạt giống `seed` cho trước (dùng builder pattern).
    pub fn multibyte(mut self, seed: u64) -> Self {
        self.use_multibyte = true;
        self.seed = seed;
        self
    }

    /// Dựng sled.
    ///
    /// Sled được ghép từ các mẫu NOP 7 byte và 2 byte theo thứ tự do PRNG tất định quyết định.
    /// Bộ dựng **không bao giờ cắt ngang** một lệnh NOP: nếu phần chừa không đủ cho mẫu được
    /// chọn, ta dùng mẫu nhỏ hơn vừa khít — nhờ đó sled luôn phân rã được trọn vẹn thành các
    /// lệnh NOP hợp lệ.
    pub fn build(&self) -> Vec<u8> {
        if !self.use_multibyte {
            return vec![NOP_BYTE; self.length];
        }

        let mut out = Vec::with_capacity(self.length);
        let mut state = self.seed | 1;
        while out.len() < self.length {
            state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = state;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;

            let remaining = self.length - out.len();
            let pattern: &[u8] = if z.is_multiple_of(2) {
                Self::fit_pattern(&MULTIBYTE_NOP_7, &MULTIBYTE_NOP_2, remaining)
            } else {
                Self::fit_pattern(&MULTIBYTE_NOP_2, &MULTIBYTE_NOP_7, remaining)
            };
            out.extend_from_slice(pattern);
        }
        out
    }

    /// Chọn mẫu NOP vừa khít với `remaining` byte còn trống (không cắt ngang lệnh).
    fn fit_pattern(
        preferred: &'static [u8],
        fallback: &'static [u8],
        remaining: usize,
    ) -> &'static [u8] {
        if preferred.len() <= remaining {
            return preferred;
        }
        if fallback.len() <= remaining {
            return fallback;
        }
        // Chỉ còn đúng 1 byte: dùng lệnh NOP một byte để giữ tính hợp lệ của sled.
        &MULTIBYTE_NOP_1
    }

    /// Đo phần đầu sled được phủ bởi các lệnh NOP liên tiếp (tính bằng **byte**).
    ///
    /// Hàm dừng lại ngay trước byte đầu tiên không thuộc lệnh NOP nào, nên nếu sled bị cắt
    /// giữa chừng một lệnh NOP dài thì phần đuôi bị cắt **không** được tính.
    pub fn coverage(&self, sled: &[u8]) -> usize {
        let mut covered = 0usize;
        let mut offset = 0usize;
        while offset < sled.len() {
            match Self::nop_instruction_len(&sled[offset..]) {
                Some(length) if offset + length <= sled.len() => {
                    covered += length;
                    offset += length;
                }
                _ => break,
            }
        }
        covered
    }

    /// Kiểm tra tại `offset` có bắt đầu một lệnh NOP hợp lệ hay không.
    pub fn is_nop_at(&self, sled: &[u8], offset: usize) -> bool {
        if offset >= sled.len() {
            return false;
        }
        Self::nop_instruction_len(&sled[offset..]).is_some()
    }

    /// Kiểm tra toàn bộ sled có **phân rã trọn vẹn** thành các lệnh NOP hợp lệ hay không.
    ///
    /// Đây là bất biến quan trọng: nếu sled chứa một byte không thuộc lệnh NOP nào, CPU sẽ thực
    /// thi nhầm dữ liệu và chương trình sẽ sập trước khi tới shellcode.
    pub fn is_valid_nop_sled(&self, sled: &[u8]) -> bool {
        let mut offset = 0usize;
        while offset < sled.len() {
            match Self::nop_instruction_len(&sled[offset..]) {
                Some(length) => offset += length,
                None => return false,
            }
        }
        true
    }

    /// Số lệnh NOP liên tiếp tính từ đầu sled.
    pub fn instruction_count(&self, sled: &[u8]) -> usize {
        let mut count = 0usize;
        let mut offset = 0usize;
        while offset < sled.len() {
            match Self::nop_instruction_len(&sled[offset..]) {
                Some(length) if offset + length <= sled.len() => {
                    count += 1;
                    offset += length;
                }
                _ => break,
            }
        }
        count
    }

    /// Nhận diện chiều dài của lệnh NOP bắt đầu tại `bytes` (nếu có).
    fn nop_instruction_len(bytes: &[u8]) -> Option<usize> {
        match bytes {
            [0x66, 0x90, ..] => Some(2),
            [0x0F, 0x1F, 0x40, 0x00, ..] => Some(4),
            [0x90, ..] => Some(1),
            _ => None,
        }
    }

    /// Sinh bản mô tả văn bản của sled (dùng để in minh hoạ).
    pub fn describe(&self, sled: &[u8]) -> String {
        format!(
            "len={} | multibyte={} | lệnh NOP={} | phủ được {} byte",
            sled.len(),
            self.use_multibyte,
            self.instruction_count(sled),
            self.coverage(sled)
        )
    }
}

// ============================================================================
// 6. LẮP RÁP PAYLOAD — `EXPLOITASSEMBLER`
// ============================================================================

/// Bộ lắp ráp payload khai thác cuối cùng.
///
/// Bố cục payload chuẩn:
///
/// ```text
///  ┌──────────────┬────────────────┬──────────────┬──────────────────┐
///  │   padding    │ return_address │  NOP sled    │ encoded_shellcode│
///  │  (A, A, ..)  │   (8 byte LE)  │ (NOP/NOP2/7) │    (đã mã hoá)   │
///  └──────────────┴────────────────┴──────────────┴──────────────────┘
///   offset 0        offset P          offset P+8      offset P+8+N
/// ```
///
/// `padding` đẩy con trỏ ngăn xếp tới đúng ô địa chỉ trả về; `return_address` ghi đè ô đó
/// bằng địa chỉ đích (trỏ vào đầu NOP sled); NOP sled tạo vùng đệm cho địa chỉ bị trượt;
/// cuối cùng là shellcode đã mã hoá.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploitAssembler {
    /// Vùng đệm (padding) đặt trước địa chỉ trả về.
    pub padding: Vec<u8>,
    /// Địa chỉ trả về sẽ ghi đè.
    pub return_address: u64,
    /// Đường trượt NOP.
    pub nop_sled: Vec<u8>,
    /// Shellcode đã mã hoá.
    pub encoded_shellcode: Vec<u8>,
}

impl ExploitAssembler {
    /// Tạo bộ lắp ráp với `padding_len` byte đệm và địa chỉ trả về `return_address`.
    pub fn new(padding_len: usize, return_address: u64) -> Self {
        Self {
            padding: vec![b'A'; padding_len],
            return_address,
            nop_sled: Vec::new(),
            encoded_shellcode: Vec::new(),
        }
    }

    /// Gắn đường trượt NOP (builder pattern).
    pub fn with_nop_sled(mut self, sled: Vec<u8>) -> Self {
        self.nop_sled = sled;
        self
    }

    /// Gắn shellcode đã mã hoá (builder pattern).
    pub fn with_shellcode(mut self, encoded: Vec<u8>) -> Self {
        self.encoded_shellcode = encoded;
        self
    }

    /// Lắp ráp payload cuối cùng.
    pub fn assemble(&self) -> Vec<u8> {
        let mut payload = Vec::with_capacity(self.total_len());
        payload.extend_from_slice(&self.padding);
        payload.extend_from_slice(&self.return_address.to_le_bytes());
        payload.extend_from_slice(&self.nop_sled);
        payload.extend_from_slice(&self.encoded_shellcode);
        payload
    }

    /// Mô tả các đoạn của payload dưới dạng `(tên, offset, độ dài)`.
    pub fn segment_offsets(&self) -> Vec<(&'static str, usize, usize)> {
        let padding_end = self.padding.len();
        let address_end = padding_end + 8;
        let sled_end = address_end + self.nop_sled.len();
        vec![
            ("padding", 0, self.padding.len()),
            ("return_address", padding_end, 8),
            ("nop_sled", address_end, self.nop_sled.len()),
            ("encoded_shellcode", sled_end, self.encoded_shellcode.len()),
        ]
    }

    /// Tổng chiều dài payload sau khi lắp ráp.
    pub fn total_len(&self) -> usize {
        self.padding.len() + 8 + self.nop_sled.len() + self.encoded_shellcode.len()
    }

    /// Offset của ô địa chỉ trả về trong payload.
    pub fn return_address_offset(&self) -> usize {
        self.padding.len()
    }

    /// Offset bắt đầu của NOP sled.
    pub fn nop_sled_offset(&self) -> usize {
        self.padding.len() + 8
    }

    /// Offset bắt đầu của shellcode đã mã hoá.
    pub fn shellcode_offset(&self) -> usize {
        self.padding.len() + 8 + self.nop_sled.len()
    }

    /// Sinh bản mô tả văn bản của payload (dùng để in minh hoạ).
    pub fn describe(&self) -> String {
        let segments: Vec<String> = self
            .segment_offsets()
            .iter()
            .map(|(name, offset, len)| format!("{name}@{offset}({len})"))
            .collect();
        format!(
            "tổng {} byte | {} | ret=0x{:016X}",
            self.total_len(),
            segments.join(" → "),
            self.return_address
        )
    }
}

// ============================================================================
// 7. TÌM GADGET ROP — `ROPGADGETFINDER`
// ============================================================================

/// Loại gadget ROP (mảnh mã kết thúc bằng `ret` hoặc lệnh nhảy).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GadgetType {
    /// `ret` — quay lại.
    Ret,
    /// `pop rdi; ret` — nạp tham số thứ nhất của hàm (System V AMD64: `rdi`).
    PopRdi,
    /// `pop rsi; ret` — nạp tham số thứ hai (`rsi`).
    PopRsi,
    /// `pop rdx; ret` — nạp tham số thứ ba (`rdx`).
    PopRdx,
    /// `pop rax; ret` — nạp giá trị trả về (`rax`).
    PopRax,
    /// `jmp rsp` — nhảy tới con trỏ ngăn xếp.
    JmpRsp,
    /// `leave; ret` — dọn khung ngăn xếp rồi quay lại.
    LeaveRet,
    /// `nop` — lệnh vô hiệu hoá.
    Nop,
}

impl GadgetType {
    /// Tên hiển thị của gadget.
    pub fn name(&self) -> &'static str {
        match self {
            GadgetType::Ret => "ret",
            GadgetType::PopRdi => "pop rdi ; ret",
            GadgetType::PopRsi => "pop rsi ; ret",
            GadgetType::PopRdx => "pop rdx ; ret",
            GadgetType::PopRax => "pop rax ; ret",
            GadgetType::JmpRsp => "jmp rsp",
            GadgetType::LeaveRet => "leave ; ret",
            GadgetType::Nop => "nop",
        }
    }

    /// Tên thanh ghi mà gadget nạp giá trị vào (nếu có).
    pub fn target_register(&self) -> Option<&'static str> {
        match self {
            GadgetType::PopRdi => Some("rdi"),
            GadgetType::PopRsi => Some("rsi"),
            GadgetType::PopRdx => Some("rdx"),
            GadgetType::PopRax => Some("rax"),
            _ => None,
        }
    }
}

/// Một gadget ROP tìm được trong binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RopGadget {
    /// Offset của gadget trong binary.
    pub offset: usize,
    /// Loại gadget.
    pub kind: GadgetType,
    /// Byte thô của gadget.
    pub bytes: Vec<u8>,
}

/// Bộ quét binary tìm gadget ROP.
///
/// ROP (Return-Oriented Programming) không cần mã máy tùy ý: chỉ cần chuỗi gadget *kết thúc bằng
/// `ret`*. Khi `ret` chạy, nó lấy địa chỉ kế tiếp trên ngăn xếp làm đích nhảy — vì vậy chỉ cần
/// "viết" ngăn xếp, không cần thực thi mã tuỳ ý nào. Đây là lý do ROP vượt qua cơ chế NX/DEP.
#[derive(Debug, Clone)]
pub struct RopGadgetFinder {
    /// Nội dung binary (byte thô) sẽ quét.
    pub binary: Vec<u8>,
}

impl RopGadgetFinder {
    /// Tạo bộ quét từ nội dung binary.
    pub fn new(binary: Vec<u8>) -> Self {
        Self { binary }
    }

    /// Quét toàn bộ binary và trả về mọi gadget tìm được, sắp xếp theo offset tăng dần.
    pub fn find_all(&self) -> Vec<RopGadget> {
        let mut gadgets: Vec<RopGadget> = Vec::new();
        for offset in 0..self.binary.len() {
            for (kind, signature) in Self::signatures() {
                if self.binary[offset..].starts_with(signature) {
                    gadgets.push(RopGadget {
                        offset,
                        kind,
                        bytes: signature.to_vec(),
                    });
                }
            }
        }
        gadgets
    }

    /// Lọc các gadget theo loại.
    pub fn find_by_kind(&self, kind: GadgetType) -> Vec<RopGadget> {
        self.find_all()
            .into_iter()
            .filter(|gadget| gadget.kind == kind)
            .collect()
    }

    /// Tổng số gadget tìm được.
    pub fn gadget_count(&self) -> usize {
        self.find_all().len()
    }

    /// Địa chỉ ( tuyệt đối) của gadget đầu tiên có loại `kind`, tính từ `base_address`.
    pub fn address_of(&self, base_address: u64, kind: GadgetType) -> Option<u64> {
        self.find_by_kind(kind)
            .first()
            .map(|gadget| base_address + gadget.offset as u64)
    }

    /// Bảng chữ ký byte của từng loại gadget.
    fn signatures() -> Vec<(GadgetType, &'static [u8])> {
        vec![
            (GadgetType::PopRdi, &[0x5F, 0xC3]),
            (GadgetType::PopRsi, &[0x5E, 0xC3]),
            (GadgetType::PopRdx, &[0x5A, 0xC3]),
            (GadgetType::PopRax, &[0x58, 0xC3]),
            (GadgetType::LeaveRet, &[0xC9, 0xC3]),
            (GadgetType::JmpRsp, &[0xFF, 0xE4]),
            (GadgetType::Ret, &[0xC3]),
            (GadgetType::Nop, &[0x90]),
        ]
    }

    /// Dựng một **binary giả** chứa sẵn các gadget để minh hoạ.
    ///
    /// Binary này do chính crate tạo ra; nó **không phải** tệp thực thi thật và không bao giờ
    /// được nạp vào bộ nhớ thật.
    pub fn demo_binary() -> Vec<u8> {
        let mut binary = vec![0xCCu8; 256];
        binary[0..4].copy_from_slice(&[0x7F, b'E', b'L', b'F']);
        binary[4..16].copy_from_slice(b"\x02\x01\x01\x00MOCK-BIN");
        binary[64..66].copy_from_slice(&[0x5F, 0xC3]); // pop rdi ; ret
        binary[80..82].copy_from_slice(&[0x5E, 0xC3]); // pop rsi ; ret
        binary[96..98].copy_from_slice(&[0x5A, 0xC3]); // pop rdx ; ret
        binary[112..114].copy_from_slice(&[0x58, 0xC3]); // pop rax ; ret
        binary[128..129].copy_from_slice(&[0xC3]); // ret
        binary[136..138].copy_from_slice(&[0xC9, 0xC3]); // leave ; ret
        binary[144..146].copy_from_slice(&[0xFF, 0xE4]); // jmp rsp
        binary[152..153].copy_from_slice(&[0x90]); // nop
        binary
    }
}

// ============================================================================
// 8. CHUỖI ROP — `ROPCHAINBUILDER`
// ============================================================================

/// Bộ xây chuỗi ROP.
///
/// Mỗi phần tử của chuỗi là một cặp `(địa chỉ gadget, tham số tùy chọn)`. Khi lắp ráp ra byte,
/// mỗi gadget chiếm 8 byte địa chỉ (little-endian) và — nếu là gadget `pop` — thêm 8 byte
/// giá trị được nạp vào thanh ghi.
#[derive(Debug, Clone)]
pub struct RopChainBuilder {
    /// Các gadget của chuỗi: `(địa chỉ gadget, tham số)`.
    pub gadgets: Vec<(u64, Option<u64>)>,
    /// Địa chỉ nền nơi chuỗi được đặt trong bộ nhớ (dùng để tính con trỏ ngăn xếp).
    pub base_address: u64,
}

impl RopChainBuilder {
    /// Tạo chuỗi ROP rỗng với địa chỉ nền `base_address`.
    pub fn new(base_address: u64) -> Self {
        Self {
            gadgets: Vec::new(),
            base_address,
        }
    }

    /// Thêm một gadget vào cuối chuỗi.
    ///
    /// `argument` là giá trị sẽ được nạp vào thanh ghi (với gadget `pop rdi` chẳng hạn) hoặc
    /// `None` nếu gadget không nhận tham số (ví dụ `ret` hay một lệnh gọi hàm).
    pub fn add_gadget(&mut self, gadget_address: u64, argument: Option<u64>) -> &mut Self {
        self.gadgets.push((gadget_address, argument));
        self
    }

    /// Lắp ráp chuỗi thành byte.
    ///
    /// Bố cục: `địa chỉ_1 ‖ tham số_1 ‖ địa chỉ_2 ‖ tham số_2 ‖ …`, mỗi số là 8 byte LE.
    pub fn build(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.byte_len());
        for (address, argument) in &self.gadgets {
            out.extend_from_slice(&address.to_le_bytes());
            if let Some(value) = argument {
                out.extend_from_slice(&value.to_le_bytes());
            }
        }
        out
    }

    /// Alias của [`RopChainBuilder::build`].
    pub fn to_bytes(&self) -> Vec<u8> {
        self.build()
    }

    /// Số gadget trong chuỗi.
    pub fn chain_length(&self) -> usize {
        self.gadgets.len()
    }

    /// Tổng số byte chuỗi chiếm trên ngăn xếp.
    pub fn byte_len(&self) -> usize {
        self.gadgets
            .iter()
            .map(|(_, argument)| 8 + usize::from(argument.is_some()) * 8)
            .sum()
    }

    /// Mô phỏng việc thực thi chuỗi ROP.
    ///
    /// `registry` là bảng ánh xạ **tên → địa chỉ** của các gadget/hàm trong binary giả
    /// (ví dụ `{"pop_rdi": 0x401234, "/bin/sh": 0x402000, "system": 0x401500}`).
    ///
    /// Hàm trả về một bảng trạng thái gồm:
    ///
    /// * `rdi`, `rsi`, `rdx`, `rax` — giá trị cuối cùng của các thanh ghi;
    /// * `called` — **địa chỉ** của hàm được gọi (tên hàm tra lại được từ `registry`);
    /// * `ret_addr` — địa chỉ gadget `ret` kết thúc chuỗi;
    /// * `stack_pointer` — con trỏ ngăn xếp sau khi chuỗi chạy hết;
    /// * `chain_base` — địa chỉ nền của chuỗi.
    pub fn simulate_execution(&self, registry: &HashMap<String, u64>) -> HashMap<String, u64> {
        let mut state: HashMap<String, u64> = HashMap::new();
        let mut registers: HashMap<&str, u64> = HashMap::new();
        registers.insert("rdi", 0);
        registers.insert("rsi", 0);
        registers.insert("rdx", 0);
        registers.insert("rax", 0);

        let mut called: Option<(String, u64)> = None;
        let mut ret_addr: Option<u64> = None;
        let mut consumed = 0u64;

        for (address, argument) in &self.gadgets {
            consumed += 8;
            if argument.is_some() {
                consumed += 8;
            }

            let Some(name) = Self::lookup_name(registry, *address) else {
                break;
            };

            if let Some(register) = Self::register_of(&name) {
                if let Some(value) = argument {
                    registers.insert(register, *value);
                }
                continue;
            }

            if name == "ret" {
                ret_addr = Some(*address);
                break;
            }

            called = Some((name, *address));
        }

        for (register, value) in registers {
            state.insert(register.to_string(), value);
        }
        if let Some((name, address)) = called {
            state.insert("called".to_string(), address);
            state.insert("called_name".to_string(), Self::name_hash(&name));
        }
        if let Some(address) = ret_addr {
            state.insert("ret_addr".to_string(), address);
        }
        state.insert("chain_base".to_string(), self.base_address);
        state.insert("stack_pointer".to_string(), self.base_address + consumed);
        state.insert("consumed".to_string(), consumed);
        state
    }

    /// Mô phỏng thực thi và trả về bản ghi **dạng văn bản** (dễ đọc khi in minh hoạ).
    ///
    /// Trong bản ghi này, `called` mang **tên** hàm được gọi (ví dụ `"system"`).
    pub fn simulate_execution_named(&self, registry: &HashMap<String, u64>) -> Vec<String> {
        let mut trace = Vec::new();
        for (address, argument) in &self.gadgets {
            let Some(name) = Self::lookup_name(registry, *address) else {
                trace.push(format!(
                    "0x{address:016X}: gadget không xác định → dừng chuỗi"
                ));
                break;
            };
            match Self::register_of(&name) {
                Some(register) => {
                    let value = argument.unwrap_or(0);
                    trace.push(format!("{register} ← 0x{value:016X}"));
                }
                None if name == "ret" => {
                    trace.push("ret → kết thúc chuỗi".to_string());
                    break;
                }
                None => {
                    trace.push(format!("gọi hàm `{name}` (0x{address:016X})"));
                }
            }
        }
        trace
    }

    /// Tra cứu tên của một địa chỉ trong `registry`.
    fn lookup_name(registry: &HashMap<String, u64>, address: u64) -> Option<String> {
        registry
            .iter()
            .find(|(_, value)| **value == address)
            .map(|(name, _)| name.clone())
    }

    /// Xác định gadget `pop` nạp giá trị vào thanh ghi nào.
    fn register_of(name: &str) -> Option<&'static str> {
        match name {
            "pop_rdi" => Some("rdi"),
            "pop_rsi" => Some("rsi"),
            "pop_rdx" => Some("rdx"),
            "pop_rax" => Some("rax"),
            _ => None,
        }
    }

    /// Băm tên hàm (FNV-1a 64-bit) để đưa vào bảng trạng thái dạng số.
    fn name_hash(name: &str) -> u64 {
        let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
        for byte in name.as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
        }
        hash
    }
}

// ============================================================================
// 9. VỎ HỆ ĐIỀU HÀNH GIẢ — `MOCKSHELL`
// ============================================================================

/// Nội dung `/etc/shadow` giả do [`MockShell`] trả về.
pub const MOCK_SHADOW: &str = "root:$6$mock$Zy7Kq1xWvTn0pLm3sRd8Hc:19000:0:99999:7:::\n\
                              daemon:*:19000:0:99999:7:::\n\
                              mock-user:!:19000:0:99999:7:::";

/// Vỏ hệ điều hành **giả lập** — dùng để minh hoạ hậu quả của một chuỗi ROP thành công.
///
/// Không lệnh nào ở đây được gửi tới hệ điều hành thật: `execute` chỉ tra cứu một bảng điều
/// kiện cố định và ghi lại lịch sử.
#[derive(Debug, Clone, Default)]
pub struct MockShell {
    /// Lịch sử các lệnh đã thực thi: `(lệnh, kết quả)`.
    pub commands: Vec<(String, String)>,
}

impl MockShell {
    /// Tạo vỏ giả với lịch sử rỗng.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Thực thi một lệnh và trả về kết quả.
    ///
    /// Lệnh được hiểu: `whoami`, `id`, `cat /etc/shadow`, `uname -a`. Mọi lệnh khác trả về
    /// thông báo "command not found" giống `sh` của POSIX.
    pub fn execute(&mut self, command: &str) -> String {
        let trimmed = command.trim();
        let output = match trimmed {
            "whoami" => "root".to_string(),
            "id" => "uid=0(root) gid=0(root) groups=0(root)".to_string(),
            "cat /etc/shadow" => MOCK_SHADOW.to_string(),
            "uname -a" => "Linux mock 6.1.0 #1 SMP x86_64".to_string(),
            other => format!("sh: {other}: command not found"),
        };
        self.commands.push((command.to_string(), output.clone()));
        output
    }

    /// Lịch sử lệnh đã thực thi.
    pub fn history(&self) -> &[(String, String)] {
        &self.commands
    }

    /// Đã leo thang lên tài khoản `root` hay chưa?
    pub fn is_root(&self) -> bool {
        self.commands
            .iter()
            .any(|(_, output)| output.contains("root"))
    }

    /// Số lệnh đã thực thi.
    pub fn command_count(&self) -> usize {
        self.commands.len()
    }

    /// Kết quả của lệnh cuối cùng (nếu có).
    pub fn last_output(&self) -> Option<&str> {
        self.commands.last().map(|(_, output)| output.as_str())
    }
}

// ============================================================================
// 10. KỊCH BẢN TỔNG THỂ — `BINARYEXPLOITLAB`
// ============================================================================

/// Báo cáo tổng kết của một lần khai thác mô phỏng.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExploitRunReport {
    /// Canary có bị phát hiện hỏng hay không.
    pub canary_intact: bool,
    /// Địa chỉ trả về đã bị kiểm soát hay chưa.
    pub return_address_controlled: bool,
    /// Offset tìm được từ mẫu tuần hoàn (nếu xác định được).
    pub crash_offset: Option<usize>,
    /// Số byte tràn ra ngoài bộ đệm.
    pub overflow_bytes: usize,
    /// Tổng chiều dài payload cuối cùng.
    pub payload_len: usize,
    /// Các byte xấu đã phát hiện.
    pub bad_chars: Vec<u8>,
    /// Độ dài payload sau khi mã hoá.
    pub encoded_len: usize,
    /// Độ dài khoá mã hoá được chọn.
    pub key_len: usize,
    /// Số gadget ROP tìm được trong binary giả.
    pub gadget_count: usize,
    /// Số phần tử trong chuỗi ROP.
    pub chain_length: usize,
    /// Chuỗi ROP có gọi được `system` hay không.
    pub called_system: bool,
    /// Vỏ giả có báo leo thang `root` hay không.
    pub shell_is_root: bool,
}

/// Bộ điều phối toàn bộ kịch bản khai thác mô phỏng trong một lần chạy.
///
/// Đây là "trình diễn tổng thể" của chương: tràn bộ đệm → dò byte xấu → mã hoá shellcode →
/// lắp ráp payload → dựng chuỗi ROP → kiểm tra hậu quả trên vỏ giả.
#[derive(Debug)]
pub struct BinaryExploitLab {
    /// Con trỏ tới chương trình đích giả lập.
    pub target: VulnerableBuffer,
    /// Bộ sinh mẫu tuần hoàn.
    pub pattern: CyclicPatternGenerator,
    /// Bộ dò byte xấu.
    pub detector: BadCharDetector,
    /// Bộ mã hoá shellcode.
    pub encoder: ShellcodeEncoder,
    /// Bộ dựng NOP sled.
    pub sled_builder: NopSledBuilder,
    /// Bộ quét gadget.
    pub finder: RopGadgetFinder,
    /// Vỏ giả.
    pub shell: MockShell,
    /// Địa chỉ nền của binary giả.
    pub base_address: u64,
}

impl Default for BinaryExploitLab {
    /// Tạo phòng thí nghiệm với toàn bộ thành phần ở trạng thái mặc định.
    fn default() -> Self {
        Self::new()
    }
}

impl BinaryExploitLab {
    /// Tạo phòng thí nghiệm với binary giả chuẩn và các thành phần mặc định.
    pub fn new() -> Self {
        Self {
            target: VulnerableBuffer::new(),
            pattern: CyclicPatternGenerator::new(),
            detector: BadCharDetector::new(),
            encoder: ShellcodeEncoder::new(vec![0x41, 0x42, 0x43, 0x44]),
            sled_builder: NopSledBuilder::new(32),
            finder: RopGadgetFinder::new(RopGadgetFinder::demo_binary()),
            shell: MockShell::new(),
            base_address: 0x0040_0000,
        }
    }

    /// Chạy trọn kịch bản và trả về báo cáo.
    pub fn run(&mut self) -> ExploitRunReport {
        // (1) Dò byte xấu qua kênh truyền giả lập.
        let mut channel = MockChannel::new();
        let bad_chars = self.detector.probe_all_bytes(&mut channel);

        // (2) Sinh mẫu tuần hoàn đủ dài để ghi đè tới địa chỉ trả về.
        let pattern_len = FRAME_SIZE;
        let pattern = self.pattern.generate(pattern_len);
        self.target.write_input(&pattern);
        let observed = self.target.return_address_bytes();
        let crash_offset = self.pattern.find_offset(&observed);

        // (3) Mã hoá shellcode sao cho không chứa byte xấu.
        let shellcode = Self::demo_shellcode();
        let (encoded, key) = self.encoder.encode_best(&shellcode, &bad_chars);

        // (4) Lắp ráp payload hoàn chỉnh.
        let sled = self.sled_builder.build();
        let assembler = ExploitAssembler::new(BUFFER_SIZE, self.base_address + 0x1000)
            .with_nop_sled(sled.clone())
            .with_shellcode(encoded.clone());
        let payload = assembler.assemble();

        // (5) Dựng chuỗi ROP kiểu ret2libc trên binary giả.
        let pop_rdi = self
            .finder
            .address_of(self.base_address, GadgetType::PopRdi)
            .unwrap_or(self.base_address);
        let system_addr = self.base_address + 0x1500;
        let binsh_addr = self.base_address + 0x2000;
        let registry: HashMap<String, u64> = HashMap::from([
            ("pop_rdi".to_string(), pop_rdi),
            ("/bin/sh".to_string(), binsh_addr),
            ("system".to_string(), system_addr),
        ]);
        let mut chain = RopChainBuilder::new(self.base_address);
        chain.add_gadget(pop_rdi, Some(binsh_addr));
        chain.add_gadget(system_addr, None);
        let simulation = chain.simulate_execution(&registry);
        let called_system = simulation.get("called") == Some(&system_addr);

        // (6) Kiểm tra hậu quả trên vỏ giả.
        let whoami = self.shell.execute("whoami");
        let shell_is_root = whoami == "root" && self.shell.is_root();

        ExploitRunReport {
            canary_intact: self.target.is_canary_intact(),
            return_address_controlled: self.target.did_overwrite_return_address(),
            crash_offset,
            overflow_bytes: self.target.overflow_amount(pattern_len),
            payload_len: payload.len(),
            bad_chars,
            encoded_len: encoded.len(),
            key_len: key.len(),
            gadget_count: self.finder.gadget_count(),
            chain_length: chain.chain_length(),
            called_system,
            shell_is_root,
        }
    }

    /// Shellcode giả (mảng byte mô tả, **không bao giờ được thực thi**).
    ///
    /// Nội dung mang tính chất minh hoạ: `xor eax, eax; push rax; push "/bin/sh"`.
    pub fn demo_shellcode() -> Vec<u8> {
        let mut code = vec![0x31, 0xC0, 0x50];
        code.extend_from_slice(b"/bin/sh");
        code.push(0x00);
        code
    }
}

// ============================================================================
// 11. HÀM MAIN MINH HOẠ
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 98: PHÒNG THÍ NGHIỆM KHAI THÁC NHỊ PHÂN (BINARY EXPLOIT LAB) ===");
    println!(
        "--- Toàn bộ mô phỏng chạy trên dữ liệu tổng hợp tất định, không đụng hệ thống thật ---\n"
    );

    // (1) Khung ngăn xếp dễ bị khai thác.
    println!("[1] KHUNG NGĂN XẾP GIẢ LẬP");
    let mut target = VulnerableBuffer::new();
    println!(
        "   canary nằm tại offset {} — {}",
        target.canary_position(),
        target.describe()
    );
    let filler = b"A".repeat(BUFFER_SIZE);
    println!(
        "   ghi {} byte (vừa khít) → canary nguyên vẹn: {}",
        filler.len(),
        target.write_input(&filler)
    );
    let overflow = b"B".repeat(BUFFER_SIZE + 24);
    let fits = target.write_input(&overflow);
    println!(
        "   ghi {} byte (tràn {} byte) → canary nguyên vẹn: {} | đã kiểm soát ret: {}",
        overflow.len(),
        target.overflow_amount(overflow.len()),
        target.is_canary_intact(),
        target.did_overwrite_return_address()
    );
    println!(
        "   `ret` mô phỏng sẽ nhảy tới 0x{:016X} — payload đã tràn nên không còn vừa khít: {}",
        target.trigger(),
        !fits
    );

    // (2) Mẫu tuần hoàn & định vị offset.
    println!("\n[2] MẪU TUẦN HOÀN KIỂU DE BRUIJN");
    let pattern = CyclicPatternGenerator::new();
    println!(
        "   bảng chữ cái {} ký tự | chu kỳ {} byte | nhãn đầu = \"{}\"",
        pattern.alphabet.len(),
        pattern.cycle_len,
        pattern.chunk_label(0)
    );
    let probe = pattern.generate(FRAME_SIZE + 8);
    let observed = &probe[BUFFER_SIZE + 16..BUFFER_SIZE + 24];
    let offset = pattern.find_offset(observed);
    println!(
        "   8 byte ghi đè return_address = {:?} → offset xác định: {:?} (khớp mẫu: {})",
        observed,
        offset,
        offset
            .map(|value| pattern.verify_offset(value, observed))
            .unwrap_or(false)
    );
    println!(
        "   cần gửi tối thiểu {} byte để phủ nhãn tại offset 80",
        pattern.pattern_size_for_offset(BUFFER_SIZE + 16)
    );

    // (3) Dò byte xấu.
    println!("\n[3] DÒ BYTE XẤU");
    let mut detector = BadCharDetector::new();
    let mut channel = MockChannel::new();
    let bad = detector.probe_all_bytes(&mut channel);
    println!("   kênh truyền giả lập loại bỏ: {}", detector.summary());
    let dirty = b"shell\x00code\npayload".to_vec();
    println!(
        "   payload thô {} byte → sau khi lọc còn {} byte",
        dirty.len(),
        detector.filter_payload(&dirty).len()
    );

    // (4) Mã hoá shellcode.
    println!("\n[4] MÃ HOÁ SHELLCODE");
    let encoder = ShellcodeEncoder::new(vec![0x41, 0x42, 0x43, 0x44]);
    let shellcode = BinaryExploitLab::demo_shellcode();
    let encoded = encoder.xor_encode(&shellcode);
    let decoded = encoder.xor_decode(&encoded);
    println!(
        "   shellcode {} byte → mã hoá {} byte → giải mã {} byte | khớp gốc: {}",
        shellcode.len(),
        encoded.len(),
        decoded.len(),
        decoded == shellcode
    );
    let (best, best_key) = encoder.encode_best(&shellcode, &bad);
    println!(
        "   khoá tối ưu {} byte → payload sạch byte xấu: {} | vân tay 0x{:016X}",
        best_key.len(),
        !best.iter().any(|byte| bad.contains(byte)),
        encoder.key_fingerprint()
    );
    let stub = encoder.build_decoder_stub(encoded.len());
    println!(
        "   stub giải mã mô phỏng {} byte (magic 0x{:02X}) → giải mã mô phỏng khớp: {}",
        stub.len(),
        stub[0],
        ShellcodeEncoder::simulate_decode(&stub, &encoded, &encoder.key) == shellcode
    );

    // (5) NOP sled.
    println!("\n[5] ĐƯỜNG TRƯỢT NOP");
    let classic = NopSledBuilder::new(32).build();
    let multi = NopSledBuilder::new(32).multibyte(0xDEAD_BEEF).build();
    println!(
        "   sled 1-byte: {} | sled đa byte: {}",
        NopSledBuilder::new(32).describe(&classic),
        NopSledBuilder::new(32)
            .multibyte(0xDEAD_BEEF)
            .describe(&multi)
    );
    println!(
        "   sled đa byte mở đầu bằng: {:?}",
        &multi[..8.min(multi.len())]
    );

    // (6) Lắp ráp payload.
    println!("\n[6] LẮP RÁP PAYLOAD");
    let sled = NopSledBuilder::new(16).build();
    let assembler = ExploitAssembler::new(BUFFER_SIZE, 0x0040_5000)
        .with_nop_sled(sled)
        .with_shellcode(best.clone());
    println!("   {}", assembler.describe());
    println!(
        "   payload lắp ráp {} byte | offset ret = {} | offset shellcode = {}",
        assembler.assemble().len(),
        assembler.return_address_offset(),
        assembler.shellcode_offset()
    );

    // (7) Gadget ROP & chuỗi ret2libc.
    println!("\n[7] GADGET ROP & CHUỖI RET2LIBC");
    let finder = RopGadgetFinder::new(RopGadgetFinder::demo_binary());
    println!(
        "   binary giả {} byte → tìm được {} gadget",
        finder.binary.len(),
        finder.gadget_count()
    );
    for kind in [
        GadgetType::PopRdi,
        GadgetType::PopRsi,
        GadgetType::PopRdx,
        GadgetType::PopRax,
        GadgetType::Ret,
        GadgetType::LeaveRet,
        GadgetType::JmpRsp,
        GadgetType::Nop,
    ] {
        let found = finder.find_by_kind(kind);
        println!(
            "   {:<14} → {} bản, ví dụ tại offset {}",
            kind.name(),
            found.len(),
            found.first().map(|gadget| gadget.offset).unwrap_or(0)
        );
    }

    let base = 0x0040_0000u64;
    let pop_rdi = finder.address_of(base, GadgetType::PopRdi).unwrap_or(base);
    let registry: HashMap<String, u64> = HashMap::from([
        ("pop_rdi".to_string(), pop_rdi),
        ("/bin/sh".to_string(), base + 0x2000),
        ("system".to_string(), base + 0x1500),
    ]);
    let mut chain = RopChainBuilder::new(base);
    chain.add_gadget(pop_rdi, Some(base + 0x2000));
    chain.add_gadget(base + 0x1500, None);
    println!(
        "   chuỗi ROP {} gadget, {} byte:",
        chain.chain_length(),
        chain.build().len()
    );
    for line in chain.simulate_execution_named(&registry) {
        println!("     • {line}");
    }
    let simulation = chain.simulate_execution(&registry);
    println!(
        "   rdi = 0x{:016X} | hàm được gọi @ 0x{:016X} | SP cuối = 0x{:016X}",
        simulation.get("rdi").copied().unwrap_or(0),
        simulation.get("called").copied().unwrap_or(0),
        simulation.get("stack_pointer").copied().unwrap_or(0)
    );

    // (8) Hậu quả trên vỏ giả.
    println!("\n[8] HẬU QUẢ TRÊN VỎ GIẢ");
    let mut shell = MockShell::new();
    for command in ["whoami", "id", "uname -a", "cat /etc/shadow"] {
        let output = shell.execute(command);
        let first_line = output.lines().next().unwrap_or("");
        println!("   $ {command:<16} → {first_line}");
    }
    println!(
        "   leo thang root: {} | số lệnh đã chạy: {}",
        shell.is_root(),
        shell.command_count()
    );

    // (9) Báo cáo tổng thể.
    println!("\n[9] BÁO CÁO TỔNG THỂ");
    let mut lab = BinaryExploitLab::new();
    let report = lab.run();
    println!(
        "   tràn {} byte → canary nguyên vẹn: {}",
        report.overflow_bytes, report.canary_intact
    );
    println!(
        "   kiểm soát return_address: {} | offset xác định: {:?}",
        report.return_address_controlled, report.crash_offset
    );
    println!(
        "   payload {} byte (mã hoá {} byte, khoá {} byte) | gadget {} | chuỗi ROP {}",
        report.payload_len,
        report.encoded_len,
        report.key_len,
        report.gadget_count,
        report.chain_length
    );
    println!(
        "   gọi được `system`: {} | vỏ giả báo root: {}",
        report.called_system, report.shell_is_root
    );

    println!("\n=== HOÀN TẤT PHÒNG THÍ NGHIỆM KHAI THÁC NHỊ PHÂN CH98 ===");
}

// ============================================================================
// 12. BỘ KIỂM THỬ TẤT ĐỊNH (40 BÀI TEST CHỨNG MINH CÁC BẤT BIẾN)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------------
    // NHÓM 1: KHUNG NGĂN XẾP & CANARY (TEST 01 - 05)
    // ---------------------------------------------------------------------

    #[test]
    fn test_01_canary_detects_buffer_overflow() {
        let mut target = VulnerableBuffer::new();
        assert!(target.is_canary_intact(), "Canary phải nguyên vẹn lúc đầu");
        assert!(target.write_input(&[b'A'; BUFFER_SIZE]));
        assert!(
            target.is_canary_intact(),
            "Ghi đúng 64 byte không được phá canary"
        );
        assert!(!target.write_input(&[b'A'; BUFFER_SIZE + 1]));
        assert!(
            !target.is_canary_intact(),
            "Tràn 1 byte sang canary phải bị phát hiện"
        );
    }

    #[test]
    fn test_02_return_address_overwrite_is_observed_by_trigger() {
        let mut target = VulnerableBuffer::new();
        assert_eq!(target.trigger(), DEFAULT_RETURN_ADDRESS);
        assert!(!target.did_overwrite_return_address());

        let crafted: Vec<u8> = {
            let mut payload = vec![b'A'; BUFFER_SIZE + 16];
            payload.extend_from_slice(&0xDEAD_BEEF_CAFE_BABE_u64.to_le_bytes());
            payload
        };
        assert!(!target.write_input(&crafted));
        assert!(target.did_overwrite_return_address());
        assert_eq!(target.trigger(), 0xDEAD_BEEF_CAFE_BABE);
        assert!(!target.is_canary_intact());
    }

    #[test]
    fn test_03_canary_position_and_stack_frame_layout() {
        let target = VulnerableBuffer::new();
        assert_eq!(target.canary_position(), BUFFER_SIZE);
        let layout = VulnerableBuffer::stack_frame_layout();
        assert_eq!(layout.len(), 4);
        assert_eq!(layout[0], ("buffer", 0, 64));
        assert_eq!(layout[1], ("canary", 64, 8));
        assert_eq!(layout[2], ("saved_rbp", 72, 8));
        assert_eq!(layout[3], ("return_address", 80, 8));
        let total: usize = layout.iter().map(|(_, _, size)| size).sum();
        assert_eq!(total, FRAME_SIZE);
        assert_eq!(target.frame_bytes().len(), FRAME_SIZE);
    }

    #[test]
    fn test_04_exact_fit_payload_leaves_frame_untouched() {
        let mut target = VulnerableBuffer::new();
        let payload = vec![0x41u8; BUFFER_SIZE];
        assert!(target.write_input(&payload));
        assert!(target.is_canary_intact());
        assert!(!target.did_overwrite_return_address());
        assert_eq!(target.trigger(), DEFAULT_RETURN_ADDRESS);
        assert_eq!(target.buffer, [0x41u8; BUFFER_SIZE]);
        assert_eq!(target.saved_rbp, DEFAULT_SAVED_RBP);
    }

    #[test]
    fn test_05_overflow_amount_matches_frame_arithmetic() {
        let target = VulnerableBuffer::new();
        assert_eq!(target.overflow_amount(BUFFER_SIZE), 0);
        assert_eq!(target.overflow_amount(BUFFER_SIZE + 1), 1);
        assert_eq!(target.overflow_amount(FRAME_SIZE), 24);
        assert_eq!(target.overflow_amount(FRAME_SIZE + 100), 124);
    }

    // ---------------------------------------------------------------------
    // NHÓM 2: MẪU TUẦN HOÀN (TEST 06 - 11)
    // ---------------------------------------------------------------------

    #[test]
    fn test_06_cyclic_alphabet_and_cycle_length() {
        let generator = CyclicPatternGenerator::new();
        assert_eq!(generator.alphabet.len(), 62);
        assert_eq!(generator.cycle_len, 62 * 3);
        assert_eq!(generator.cycle_len, 186);
        for byte in b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789" {
            assert!(generator.is_alphabet_byte(*byte));
        }
        assert!(!generator.is_alphabet_byte(0x00));
        assert!(!generator.is_alphabet_byte(0x20));
    }

    #[test]
    fn test_07_chunk_labels_are_unique_and_printable() {
        let generator = CyclicPatternGenerator::new();
        assert_eq!(generator.chunk_label(0), "AaAa");
        let labels = generator.labels(500);
        let unique: std::collections::HashSet<&String> = labels.iter().collect();
        assert_eq!(unique.len(), labels.len(), "Nhãn phải là DUY NHẤT");
        for label in &labels {
            assert_eq!(label.chars().count(), 4);
            assert!(label.chars().all(|ch| ch.is_ascii_alphanumeric()));
        }

        // Mỗi nhãn phải xuất hiện ĐÚNG MỘT LẦN trong cửa sổ quét 2048 byte.
        let stream = generator.generate(MIN_SEARCH_WINDOW);
        for index in 0..200usize {
            let offset = index * CHUNK_LABEL_LEN;
            let label = generator.chunk_label(offset);
            let occurrences = stream
                .windows(CHUNK_LABEL_LEN)
                .filter(|window| *window == label.as_bytes())
                .count();
            assert_eq!(
                occurrences, 1,
                "Nhãn {label} tại offset {offset} xuất hiện {occurrences} lần"
            );
        }
    }

    #[test]
    fn test_08_cyclic_find_offset_recovers_exact_offset() {
        let generator = CyclicPatternGenerator::new();
        for offset in [0usize, 16, 64, 80, 120, 180] {
            let label = generator.chunk_label(offset);
            let bytes = label.as_bytes();
            assert_eq!(
                generator.find_offset(bytes),
                Some(offset),
                "Phải tìm lại đúng offset {offset}"
            );
        }

        // Quét rộng hơn: 100 nhãn đầu tiên phải được định vị tuyệt đối, không mơ hồ.
        for index in 0..100usize {
            let offset = index * CHUNK_LABEL_LEN;
            let label = generator.chunk_label(offset);
            assert_eq!(
                generator.find_offset(label.as_bytes()),
                Some(offset),
                "Nhãn {label} tại offset {offset} không được định vị lệch"
            );
        }
    }

    #[test]
    fn test_09_verify_offset_accepts_true_and_rejects_false() {
        let generator = CyclicPatternGenerator::new();
        let stream = generator.generate(200);
        assert!(generator.verify_offset(64, &stream[64..68]));
        assert!(!generator.verify_offset(64, &stream[65..69]));
        assert!(!generator.verify_offset(64, b"ZZZZ"));
        assert!(!generator.verify_offset(0, &[]));
    }

    #[test]
    fn test_10_pattern_size_for_offset_covers_the_chunk() {
        let generator = CyclicPatternGenerator::new();
        for offset in [0usize, 40, 80, 120] {
            let needed = generator.pattern_size_for_offset(offset);
            assert_eq!(needed, offset + 4);
            let stream = generator.generate(needed);
            assert!(stream.len() >= needed);
            assert_eq!(
                &stream[offset..offset + 4],
                generator.chunk_label(offset).as_bytes()
            );
        }
    }

    #[test]
    fn test_11_generated_pattern_uses_only_alphabet_bytes() {
        let generator = CyclicPatternGenerator::new();
        let stream = generator.generate(1000);
        assert_eq!(stream.len(), 1000);
        for &byte in &stream {
            assert!(
                generator.is_alphabet_byte(byte),
                "Byte lạ trong mẫu: 0x{byte:02X}"
            );
        }
        assert!(generator.is_chunk_aligned(64));
        assert!(!generator.is_chunk_aligned(65));
        assert_eq!(generator.chunk_index_of(80), 20);
    }

    // ---------------------------------------------------------------------
    // NHÓM 3: DÒ BYTE XẤU (TEST 12 - 15)
    // ---------------------------------------------------------------------

    #[test]
    fn test_12_bad_char_detector_finds_filtered_bytes() {
        let mut detector = BadCharDetector::new();
        let mut channel = MockChannel::new();
        let bad = detector.probe_all_bytes(&mut channel);
        assert_eq!(bad.len(), 2, "Kênh mặc định lọc 0x00 và 0x0A");
        assert!(bad.contains(&0x00));
        assert!(bad.contains(&0x0A));
        assert!(detector.is_bad(0x00));
        assert!(!detector.is_bad(b'A'));
        assert_eq!(detector.responses.len(), 256);
    }

    #[test]
    fn test_13_bad_char_detector_finds_replaced_bytes() {
        let mut detector = BadCharDetector::new();
        detector.record(b'A', vec![b'A']);
        detector.record(b'B', vec![b'X']);
        let bad = detector.detect();
        assert!(bad.contains(&b'B'));
        assert!(!bad.contains(&b'A'));
        assert_eq!(detector.bad_char_count(), 1);
    }

    #[test]
    fn test_14_detect_from_transformation_marks_xor_delta() {
        let mut detector = BadCharDetector::new();
        detector.detect_from_transformation(b'A', vec![b'A']);
        assert!(
            !detector.is_bad(b'A'),
            "Byte giữ nguyên không phải byte xấu"
        );
        detector.detect_from_transformation(b'A', vec![b'a']);
        assert!(detector.is_bad(b'A'), "Byte bị XOR phải bị đánh dấu");
        assert_eq!(detector.responses.len(), 2);
    }

    #[test]
    fn test_15_filter_payload_removes_exactly_bad_bytes() {
        let mut detector = BadCharDetector::new();
        detector.record(0x00, Vec::new());
        detector.record(0x0A, Vec::new());
        let payload = b"AA\x00BB\nCC".to_vec();
        let filtered = detector.filter_payload(&payload);
        assert_eq!(filtered, b"AABBCC".to_vec());
        assert_eq!(filtered.len(), 6);
    }

    // ---------------------------------------------------------------------
    // NHÓM 4: MÃ HOÁ SHELLCODE (TEST 16 - 22)
    // ---------------------------------------------------------------------

    #[test]
    fn test_16_xor_encode_decode_is_involutive() {
        let encoder = ShellcodeEncoder::new(vec![0x11, 0x22, 0x33]);
        let shellcode = vec![0x90u8, 0x31, 0xC0, 0x50, 0x68, 0x2F];
        let encoded = encoder.xor_encode(&shellcode);
        assert_eq!(encoded.len(), shellcode.len());
        assert_ne!(encoded, shellcode);
        assert_eq!(encoder.xor_decode(&encoded), shellcode);
    }

    #[test]
    fn test_17_xor_key_rotation_matches_formula() {
        let encoder = ShellcodeEncoder::new(vec![0xAA, 0xBB]);
        let encoded = encoder.xor_encode(&[0x00, 0x00, 0x00, 0x00, 0x00]);
        assert_eq!(encoded, vec![0xAA, 0xBB, 0xAA, 0xBB, 0xAA]);
    }

    #[test]
    fn test_18_empty_key_is_identity() {
        let encoder = ShellcodeEncoder::new(Vec::new());
        let shellcode = vec![1u8, 2, 3];
        assert_eq!(encoder.xor_encode(&shellcode), shellcode);
        assert_eq!(encoder.xor_decode(&shellcode), shellcode);
        assert_eq!(encoder.key_len(), 0);
    }

    #[test]
    fn test_19_single_byte_key_avoids_bad_chars() {
        let encoder = ShellcodeEncoder::new(vec![0x01]);
        let shellcode = vec![0x90u8, 0x31, 0xC0, 0x50];
        let bad = vec![0x00u8, 0x0A];
        let key = encoder
            .find_single_byte_key(&shellcode, &bad)
            .expect("phải tìm được khoá 1 byte");
        let encoded: Vec<u8> = shellcode.iter().map(|&byte| byte ^ key).collect();
        assert!(!encoded.iter().any(|byte| bad.contains(byte)));
        let decoder = ShellcodeEncoder::new(vec![key]);
        assert_eq!(decoder.xor_decode(&encoded), shellcode);
    }

    #[test]
    fn test_20_single_byte_key_returns_none_when_impossible() {
        let encoder = ShellcodeEncoder::new(vec![0x01]);
        let shellcode = vec![0x00u8, 0x01];
        let all_bytes: Vec<u8> = (0u16..=255).map(|value| value as u8).collect();
        assert_eq!(encoder.find_single_byte_key(&shellcode, &all_bytes), None);
    }

    #[test]
    fn test_21_encode_best_prefers_single_byte_key() {
        let encoder = ShellcodeEncoder::new(vec![0x77, 0x88, 0x99]);
        let shellcode = vec![0x90u8, 0x31, 0xC0, 0x50, 0x68];
        let bad = vec![0x00u8, 0x0A, 0x0D];
        let (encoded, key) = encoder.encode_best(&shellcode, &bad);
        assert_eq!(key.len(), 1);
        assert!(!encoded.iter().any(|byte| bad.contains(byte)));
        let decoder = ShellcodeEncoder::new(key);
        assert_eq!(decoder.xor_decode(&encoded), shellcode);
    }

    #[test]
    fn test_22_transform_roundtrip_for_all_variants() {
        let encoder = ShellcodeEncoder::new(vec![0x0F, 0xF0, 0x33]);
        let shellcode = vec![0x90u8, 0x31, 0xC0, 0x50];
        for transform in [
            BitwiseTransform::Identity,
            BitwiseTransform::Not,
            BitwiseTransform::Rol(3),
            BitwiseTransform::Ror(5),
        ] {
            let encoded = encoder.encode_with_transform(&shellcode, transform);
            let decoded = encoder.decode_with_transform(&encoded, transform);
            assert_eq!(decoded, shellcode, "Sai với {}", transform.name());
        }
    }

    // ---------------------------------------------------------------------
    // NHÓM 5: STUB GIẢI MÃ & NOP SLED (TEST 23 - 26)
    // ---------------------------------------------------------------------

    #[test]
    fn test_23_decoder_stub_simulate_decode_recovers_shellcode() {
        let key = vec![0x11u8, 0x22, 0x33, 0x44];
        let encoder = ShellcodeEncoder::new(key.clone());
        let shellcode = vec![0x90u8, 0x31, 0xC0, 0x50];
        let encoded = encoder.xor_encode(&shellcode);
        let stub = encoder.build_decoder_stub(encoded.len());
        assert_eq!(stub[0], STUB_MAGIC);
        assert_eq!(stub[0], 0xEB, "Marker stub phải là lệnh JMP short 0xEB");
        assert!(stub.len() > STUB_HEADER_LEN + key.len());
        assert_eq!(
            ShellcodeEncoder::simulate_decode(&stub, &encoded, &key),
            shellcode
        );
    }

    #[test]
    fn test_24_invalid_decoder_stub_is_rejected() {
        let key = vec![0x11u8, 0x22];
        let encoder = ShellcodeEncoder::new(key.clone());
        let encoded = encoder.xor_encode(&[1u8, 2, 3]);
        let mut stub = encoder.build_decoder_stub(encoded.len());
        stub[0] = 0x00; // phá magic
        assert!(ShellcodeEncoder::simulate_decode(&stub, &encoded, &key).is_empty());
        assert!(
            ShellcodeEncoder::simulate_decode(&[], &encoded, &key).is_empty(),
            "Stub rỗng phải bị từ chối"
        );
        let good = encoder.build_decoder_stub(encoded.len());
        assert!(
            ShellcodeEncoder::simulate_decode(&good, &encoded, &[0x99, 0x99]).is_empty(),
            "Khoá không khớp phải bị từ chối"
        );
    }

    #[test]
    fn test_25_classic_nop_sled_is_all_nops() {
        let builder = NopSledBuilder::new(48);
        let sled = builder.build();
        assert_eq!(sled.len(), 48);
        assert!(sled.iter().all(|&byte| byte == NOP_BYTE));
        assert_eq!(builder.coverage(&sled), 48);
        assert_eq!(builder.instruction_count(&sled), 48);
        assert!(builder.is_nop_at(&sled, 0));
        assert!(builder.is_nop_at(&sled, 47));
        assert!(!builder.is_nop_at(&sled, 48));
    }

    #[test]
    fn test_26_multibyte_nop_sled_contains_only_valid_nops() {
        let builder = NopSledBuilder::new(64).multibyte(0xDEAD_BEEF);
        let sled = builder.build();
        assert_eq!(sled.len(), 64);
        assert!(
            builder.is_valid_nop_sled(&sled),
            "Sled đa byte phải phân rã trọn vẹn thành lệnh NOP hợp lệ"
        );
        assert_eq!(
            builder.coverage(&sled),
            sled.len(),
            "Toàn bộ sled phải được phủ bởi lệnh NOP"
        );
        assert!(builder.instruction_count(&sled) < sled.len());
        assert!(builder.is_nop_at(&sled, 0));
        assert!(!builder.is_nop_at(&sled, sled.len()));
        // Sled đa byte phải chứa ít nhất một mẫu NOP dài hơn 1 byte.
        assert!(
            sled.windows(2).any(|window| window == MULTIBYTE_NOP_2)
                || sled
                    .windows(4)
                    .any(|window| *window == MULTIBYTE_NOP_7[3..]),
            "Sled phải chứa mẫu NOP đa byte"
        );
    }

    #[test]
    fn test_27_nop_sled_is_deterministic_for_a_given_seed() {
        let first = NopSledBuilder::new(80).multibyte(0x1234_5678).build();
        let second = NopSledBuilder::new(80).multibyte(0x1234_5678).build();
        assert_eq!(first, second, "Cùng hạt giống phải cho cùng sled");
        let other = NopSledBuilder::new(80).multibyte(0x8765_4321).build();
        assert_ne!(first, other, "Hạt giống khác phải cho sled khác");
    }

    // ---------------------------------------------------------------------
    // NHÓM 6: LẮP RÁP PAYLOAD (TEST 28 - 29)
    // ---------------------------------------------------------------------

    #[test]
    fn test_28_assembler_orders_segments_and_offsets() {
        let sled = vec![0x90u8; 8];
        let shellcode = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let assembler = ExploitAssembler::new(64, 0x0040_1234)
            .with_nop_sled(sled.clone())
            .with_shellcode(shellcode.clone());
        let payload = assembler.assemble();

        assert_eq!(payload.len(), 64 + 8 + 8 + 4);
        assert_eq!(assembler.total_len(), payload.len());
        assert_eq!(assembler.return_address_offset(), 64);
        assert_eq!(assembler.nop_sled_offset(), 72);
        assert_eq!(assembler.shellcode_offset(), 80);

        assert!(payload[..64].iter().all(|&byte| byte == b'A'));
        assert_eq!(&payload[64..72], &0x0040_1234u64.to_le_bytes());
        assert_eq!(&payload[72..80], &sled[..]);
        assert_eq!(&payload[80..84], &shellcode[..]);

        let segments = assembler.segment_offsets();
        assert_eq!(segments[0], ("padding", 0, 64));
        assert_eq!(segments[1], ("return_address", 64, 8));
        assert_eq!(segments[2], ("nop_sled", 72, 8));
        assert_eq!(segments[3], ("encoded_shellcode", 80, 4));
    }

    #[test]
    fn test_29_assembler_without_extra_segments_is_minimal() {
        let assembler = ExploitAssembler::new(16, 0x1000);
        let payload = assembler.assemble();
        assert_eq!(payload.len(), 24);
        assert_eq!(assembler.total_len(), 24);
        assert_eq!(assembler.return_address_offset(), 16);
        assert_eq!(assembler.shellcode_offset(), 24);
    }

    // ---------------------------------------------------------------------
    // NHÓM 7: GADGET & CHUỖI ROP (TEST 30 - 34)
    // ---------------------------------------------------------------------

    #[test]
    fn test_30_gadget_finder_locates_every_kind() {
        let finder = RopGadgetFinder::new(RopGadgetFinder::demo_binary());
        for kind in [
            GadgetType::PopRdi,
            GadgetType::PopRsi,
            GadgetType::PopRdx,
            GadgetType::PopRax,
            GadgetType::Ret,
            GadgetType::LeaveRet,
            GadgetType::JmpRsp,
            GadgetType::Nop,
        ] {
            let found = finder.find_by_kind(kind);
            assert!(!found.is_empty(), "Không tìm thấy gadget {}", kind.name());
            assert!(found.iter().all(|gadget| gadget.kind == kind));
        }
        assert_eq!(finder.gadget_count(), finder.find_all().len());
    }

    #[test]
    fn test_31_gadget_offsets_are_sorted_and_addresses_are_absolute() {
        let finder = RopGadgetFinder::new(RopGadgetFinder::demo_binary());
        let gadgets = finder.find_all();
        let offsets: Vec<usize> = gadgets.iter().map(|gadget| gadget.offset).collect();
        let mut sorted = offsets.clone();
        sorted.sort_unstable();
        assert_eq!(offsets, sorted, "Gadget phải được sắp theo offset tăng dần");
        let base = 0x0040_0000u64;
        let address = finder
            .address_of(base, GadgetType::PopRdi)
            .expect("phải có gadget pop rdi");
        assert_eq!(
            address,
            base + finder.find_by_kind(GadgetType::PopRdi)[0].offset as u64
        );
    }

    #[test]
    fn test_32_rop_chain_byte_layout_is_little_endian() {
        let mut chain = RopChainBuilder::new(0x0040_0000);
        chain.add_gadget(0x1111_2222_3333_4444, Some(0xAAAA_BBBB_CCCC_DDDD));
        chain.add_gadget(0x5555_6666_7777_8888, None);
        let bytes = chain.build();
        assert_eq!(chain.chain_length(), 2);
        assert_eq!(chain.byte_len(), 24);
        assert_eq!(bytes.len(), 24);
        assert_eq!(&bytes[0..8], &0x1111_2222_3333_4444u64.to_le_bytes());
        assert_eq!(&bytes[8..16], &0xAAAA_BBBB_CCCC_DDDDu64.to_le_bytes());
        assert_eq!(&bytes[16..24], &0x5555_6666_7777_8888u64.to_le_bytes());
        assert_eq!(chain.to_bytes(), bytes);
    }

    #[test]
    fn test_33_ret2libc_chain_simulation_sets_rdi_and_calls_system() {
        let base = 0x0040_0000u64;
        let finder = RopGadgetFinder::new(RopGadgetFinder::demo_binary());
        let pop_rdi = finder.address_of(base, GadgetType::PopRdi).unwrap();
        let binsh = base + 0x2000;
        let system = base + 0x1500;
        let registry: HashMap<String, u64> = HashMap::from([
            ("pop_rdi".to_string(), pop_rdi),
            ("/bin/sh".to_string(), binsh),
            ("system".to_string(), system),
        ]);

        let mut chain = RopChainBuilder::new(base);
        chain.add_gadget(pop_rdi, Some(binsh));
        chain.add_gadget(system, None);

        let state = chain.simulate_execution(&registry);
        assert_eq!(
            state.get("rdi"),
            Some(&binsh),
            "rdi phải trỏ tới chuỗi /bin/sh"
        );
        assert_eq!(
            state.get("called"),
            Some(&system),
            "Phải gọi được hàm system"
        );
        assert_eq!(state.get("chain_base"), Some(&base));
        assert_eq!(state.get("consumed"), Some(&24));

        let trace = chain.simulate_execution_named(&registry);
        assert_eq!(trace.len(), 2);
        assert!(
            trace[1].contains("system"),
            "Bản ghi phải nêu đích danh hàm system"
        );
    }

    #[test]
    fn test_34_chain_with_unknown_gadget_stops_safely() {
        let registry: HashMap<String, u64> = HashMap::from([("ret".to_string(), 0x1000)]);
        let mut chain = RopChainBuilder::new(0x4000);
        chain.add_gadget(0xDEAD_BEEF, Some(0x1234));
        chain.add_gadget(0x1000, None);
        let state = chain.simulate_execution(&registry);
        assert_eq!(state.get("rdi"), Some(&0));
        assert!(!state.contains_key("called"));
    }

    // ---------------------------------------------------------------------
    // NHÓM 8: VỎ GIẢ (TEST 35 - 37)
    // ---------------------------------------------------------------------

    #[test]
    fn test_35_mock_shell_whoami_reports_root() {
        let mut shell = MockShell::new();
        assert_eq!(shell.execute("whoami"), "root");
        assert!(shell.is_root());
        assert_eq!(shell.command_count(), 1);
        assert_eq!(shell.last_output(), Some("root"));
    }

    #[test]
    fn test_36_mock_shell_known_commands_and_history() {
        let mut shell = MockShell::new();
        assert_eq!(
            shell.execute("id"),
            "uid=0(root) gid=0(root) groups=0(root)"
        );
        assert_eq!(shell.execute("uname -a"), "Linux mock 6.1.0 #1 SMP x86_64");
        let shadow = shell.execute("cat /etc/shadow");
        assert!(shadow.contains("root:"));
        assert_eq!(shell.history().len(), 3);
        assert_eq!(shell.history()[0].0, "id");
        assert!(shell.history()[2].1.contains("mock-user"));
    }

    #[test]
    fn test_37_mock_shell_reports_unknown_command() {
        let mut shell = MockShell::new();
        let output = shell.execute("rm -rf /");
        assert_eq!(output, "sh: rm -rf /: command not found");
        assert!(!shell.is_root(), "Lệnh lỗi không được tạo ra đặc quyền");
    }

    // ---------------------------------------------------------------------
    // NHÓM 9: KỊCH BẢN TỔNG THỂ (TEST 38 - 40)
    // ---------------------------------------------------------------------

    #[test]
    fn test_38_end_to_end_overflow_reaches_return_address() {
        let mut target = VulnerableBuffer::new();
        let generator = CyclicPatternGenerator::new();
        let pattern = generator.generate(FRAME_SIZE);
        assert!(!target.write_input(&pattern));
        assert!(!target.is_canary_intact());
        assert!(target.did_overwrite_return_address());

        let observed = target.return_address_bytes();
        let offset = generator.find_offset(&observed);
        assert_eq!(
            offset,
            Some(BUFFER_SIZE + 16),
            "Offset phải trùng vị trí return_address"
        );
        assert!(generator.verify_offset(BUFFER_SIZE + 16, &observed));
        assert_eq!(target.overflow_amount(pattern.len()), 24);
    }

    #[test]
    fn test_39_end_to_end_payload_is_bad_char_free() {
        let mut detector = BadCharDetector::new();
        let mut channel = MockChannel::new();
        let bad = detector.probe_all_bytes(&mut channel);

        let encoder = ShellcodeEncoder::new(vec![0x41, 0x42, 0x43, 0x44]);
        let shellcode = BinaryExploitLab::demo_shellcode();
        let (encoded, key) = encoder.encode_best(&shellcode, &bad);
        assert!(!encoded.iter().any(|byte| bad.contains(byte)));

        let sled = NopSledBuilder::new(24).build();
        let assembler = ExploitAssembler::new(BUFFER_SIZE, 0x0040_5000)
            .with_nop_sled(sled)
            .with_shellcode(encoded.clone());
        let payload = assembler.assemble();
        assert_eq!(payload.len(), assembler.total_len());

        let decoder = ShellcodeEncoder::new(key);
        assert_eq!(decoder.xor_decode(&encoded), shellcode);
    }

    #[test]
    fn test_40_full_lab_report_satisfies_every_invariant() {
        let mut lab = BinaryExploitLab::new();
        let report = lab.run();
        assert!(!report.canary_intact, "Tràn phải phá canary");
        assert!(report.return_address_controlled);
        assert_eq!(report.crash_offset, Some(BUFFER_SIZE + 16));
        assert_eq!(report.overflow_bytes, 24);
        assert!(report.payload_len > 0);
        assert!(report.bad_chars.contains(&0x00));
        assert_eq!(report.encoded_len, BinaryExploitLab::demo_shellcode().len());
        assert!(report.key_len >= 1);
        assert!(report.gadget_count >= 8);
        assert_eq!(report.chain_length, 2);
        assert!(report.called_system, "Chuỗi ROP phải gọi được system");
        assert!(report.shell_is_root, "Vỏ giả phải báo leo thang root");
    }
}
