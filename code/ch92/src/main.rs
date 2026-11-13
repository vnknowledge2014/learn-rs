//! # Chương 92: Bộ Tách Từ Byte-Level BPE Tăng Tốc Heap, Tensor Bước Nhảy & Động Cơ Vi Phân Ngược Autograd
//!
//! Crate này cài đặt hoàn chỉnh từ đầu (100% thuần `std`, không dùng thư viện ngoài)
//! ba trụ cột toán học và cấu trúc dữ liệu cốt lõi của một hệ thống LLM:
//! 1. `BpeTokenizer`: Bộ tách từ mức byte UTF-8 với hàng đợi ưu tiên `BinaryHeap` phá hòa tất định.
//! 2. `Tensor`: Tensor vùng nhớ phẳng với `shape` và `strides`, hỗ trợ `transpose` chỉ hoán đổi `shape`/`strides` (logic O(1)).
//! 3. `AutogradTape`: Động cơ tính vi phân ngược tự động (Reverse-Mode Autograd) duyệt thứ tự tô-pô,
//!    kiểm chứng độ chính xác giải tích so với sai phân hữu hạn trung tâm đạt sai số < 1e-8.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};

// ============================================================================
// 1. BỘ SINH SỐ GIẢ NGẪU NHIÊN SPLITMIX64 & BIẾN ĐỔI BOX-MULLER
// ============================================================================

/// Bộ sinh số giả ngẫu nhiên 64-bit tất định SplitMix64.
///
/// Lý do thiết kế: Chu kỳ $2^{64}$, chi phí tính toán cực thấp (chỉ dùng phép toán bit
/// và nhân tràn số nguyên), độc lập hoàn toàn với hệ điều hành, đảm bảo tái lập 100%
/// kết quả khởi tạo trọng số mạng nơ-ron trên mọi nền tảng phần cứng.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SplitMix64 {
    /// Trạng thái nội bộ 64-bit
    pub state: u64,
}

impl SplitMix64 {
    /// Khởi tạo bộ sinh số với hạt giống tùy chọn
    pub const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Sinh số nguyên không dấu 64-bit tiếp theo
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = (self.state ^ (self.state >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Sinh số thực trong khoảng $[0.0, 1.0)$
    pub fn next_f64(&mut self) -> f64 {
        ((self.next_u64() >> 11) as f64) * (1.0 / ((1u64 << 53) as f64))
    }

    /// Sinh số thực đối xứng trong khoảng $[-scale, scale)$
    pub fn next_signed(&mut self, scale: f64) -> f64 {
        (self.next_f64() * 2.0 - 1.0) * scale
    }

    /// Sinh số ngẫu nhiên theo phân phối chuẩn tắc Gaussian $\mathcal{N}(\mu, \sigma^2)$
    /// thông qua biến đổi Box–Muller cổ điển.
    pub fn next_gaussian(&mut self, mean: f64, std_dev: f64) -> f64 {
        let mut u1 = self.next_f64();
        // Tránh ln(0.0) gây giá trị âm vô cực NaN
        if u1 <= 1e-15 {
            u1 = 1e-15;
        }
        let u2 = self.next_f64();
        let radius = (-2.0 * u1.ln()).sqrt();
        let theta = 2.0 * std::f64::consts::PI * u2;
        mean + std_dev * radius * theta.cos()
    }
}

// ============================================================================
// 2. TIỀN TÁCH TỪ UTF-8 BẢO TOÀN DỮ LIỆU (PRE-TOKENIZATION)
// ============================================================================

/// Tiền tách từ chuỗi ký tự UTF-8 thành các mảnh ngữ nghĩa độc lập.
///
/// Bất biến bảo tồn: `chunks.concat() == text` (không mất bất kỳ byte nào, kể cả khoảng trắng).
/// Việc nhóm này ngăn thuật toán BPE gộp các byte xuyên qua ranh giới từ hoặc dấu câu,
/// tương tự như regex pre-tokenizer của GPT-2/GPT-4 nhưng thuần Rust 100% không phụ thuộc regex.
pub fn pre_tokenize_utf8(text: &str) -> Vec<String> {
    if text.is_empty() {
        return Vec::new();
    }

    #[derive(Copy, Clone, PartialEq, Eq)]
    enum CharCategory {
        Whitespace,
        Alphanumeric,
        Punctuation,
    }

    fn categorize(c: char) -> CharCategory {
        if c.is_whitespace() {
            CharCategory::Whitespace
        } else if c.is_alphanumeric() {
            CharCategory::Alphanumeric
        } else {
            CharCategory::Punctuation
        }
    }

    let mut chunks = Vec::new();
    let mut current_chunk = String::new();
    let mut current_cat: Option<CharCategory> = None;

    for c in text.chars() {
        let cat = categorize(c);
        match current_cat {
            Some(prev_cat) if prev_cat == cat => {
                current_chunk.push(c);
            }
            Some(_) => {
                chunks.push(current_chunk);
                current_chunk = String::new();
                current_chunk.push(c);
                current_cat = Some(cat);
            }
            None => {
                current_chunk.push(c);
                current_cat = Some(cat);
            }
        }
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk);
    }

    chunks
}

// ============================================================================
// 3. BỘ TÁCH TỪ BYTE-LEVEL BPE TĂNG TỐC BẰNG HEAP
// ============================================================================

/// Cặp token kế tiếp nhau làm ứng viên gộp BPE
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TokenPair {
    /// Token thứ nhất
    pub first: u16,
    /// Token thứ hai
    pub second: u16,
}

/// Thông tin thứ hạng gộp và định danh mới
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankInfo {
    /// Thứ tự xuất hiện trong danh sách quy tắc gộp
    pub rank: usize,
    /// Token ID mới được sinh ra từ việc gộp
    pub new_id: u16,
}

/// Quy tắc gộp BPE lưu trong bộ tách từ
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeRule {
    /// Cặp token được gộp
    pub pair: TokenPair,
    /// Token ID mới
    pub new_id: u16,
}

/// Bộ tách từ mức byte BPE (Byte-Level Byte Pair Encoding).
///
/// Bất biến kiến trúc:
/// - 256 byte cơ sở UTF-8 (0..=255) luôn chiếm 256 token đầu tiên, đảm bảo mọi chuỗi byte
///   đều mã hóa và giải mã được mà không bao giờ phát sinh lỗi `<unk>`.
/// - Hàng đợi ưu tiên `BinaryHeap` phá hòa tất định theo cặp `TokenPair` nhỏ nhất khi hai cặp
///   có tần số xuất hiện bằng nhau.
/// - Các token đặc biệt (`<|bos|>`, `<|eos|>`, `<think>`, `</think>`, `<answer>`, `</answer>`)
///   được bảo vệ nguyên khối, không bao giờ bị tách vụn thành các byte UTF-8 rời rạc.
#[derive(Debug, Clone)]
pub struct BpeTokenizer {
    /// Từ điển ánh xạ từ `u16` sang chuỗi byte
    pub vocab: Vec<Vec<u8>>,
    /// Danh sách quy tắc gộp theo thứ tự huấn luyện
    pub merges: Vec<MergeRule>,
    /// Bảng băm tra cứu thứ hạng và ID gộp nhanh $O(1)$
    pub ranks: HashMap<TokenPair, RankInfo>,
    /// Bảng ánh xạ token đặc biệt từ chuỗi sang token ID
    pub special_tokens: HashMap<String, u16>,
    /// Bảng ánh xạ ngược từ token ID sang chuỗi token đặc biệt
    pub special_token_ids: HashMap<u16, String>,
}

fn merge_sequence(tokens: &[u16], pair: TokenPair, new_id: u16) -> Vec<u16> {
    let mut out = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        if i + 1 < tokens.len() && tokens[i] == pair.first && tokens[i + 1] == pair.second {
            out.push(new_id);
            i += 2;
        } else {
            out.push(tokens[i]);
            i += 1;
        }
    }
    out
}

impl BpeTokenizer {
    /// Huấn luyện bảng gộp BPE trên kho ngữ liệu.
    ///
    /// Cơ chế phá hòa tất định:
    /// Nếu hai cặp có cùng tần số đếm `count`, hàng đợi ưu tiên `BinaryHeap` kết hợp với
    /// `Reverse(TokenPair)` sẽ chọn cặp có giá trị số học nhỏ nhất. Điều này bảo đảm tính
    /// lặp lại 100% độc lập với thứ tự duyệt bảng băm `HashMap`.
    pub fn train(corpus: &str, max_merges: usize) -> Self {
        // 256 byte cơ sở
        let mut vocab: Vec<Vec<u8>> = (0..=255u8).map(|b| vec![b]).collect();

        // Tiền tách từ bảo toàn byte
        let pieces = pre_tokenize_utf8(corpus);
        let mut words: Vec<Vec<u16>> = pieces
            .into_iter()
            .map(|s| s.bytes().map(u16::from).collect())
            .collect();

        let mut merges = Vec::new();
        let mut ranks = HashMap::new();

        for _ in 0..max_merges {
            let mut counts: HashMap<TokenPair, usize> = HashMap::new();
            for word in &words {
                for window in word.windows(2) {
                    let pair = TokenPair {
                        first: window[0],
                        second: window[1],
                    };
                    *counts.entry(pair).or_insert(0) += 1;
                }
            }

            // BinaryHeap lưu (count, Reverse(pair)) để cặp nhỏ nhất được ưu tiên khi hòa điểm
            let mut heap = BinaryHeap::new();
            for (pair, count) in counts {
                if count >= 2 {
                    heap.push((count, Reverse(pair)));
                }
            }

            let Some((_, Reverse(best_pair))) = heap.pop() else {
                // Không còn cặp nào xuất hiện >= 2 lần
                break;
            };

            let new_id = u16::try_from(vocab.len()).expect("Vượt giới hạn u16 cho từ vựng BPE");
            let mut new_bytes = vocab[usize::from(best_pair.first)].clone();
            new_bytes.extend_from_slice(&vocab[usize::from(best_pair.second)]);
            vocab.push(new_bytes);

            let rule = MergeRule {
                pair: best_pair,
                new_id,
            };
            ranks.insert(
                best_pair,
                RankInfo {
                    rank: merges.len(),
                    new_id,
                },
            );
            merges.push(rule);

            // Cập nhật các từ bị ảnh hưởng
            for word in &mut words {
                if word.len() >= 2 {
                    *word = merge_sequence(word, best_pair, new_id);
                }
            }
        }

        let mut tokenizer = Self {
            vocab,
            merges,
            ranks,
            special_tokens: HashMap::new(),
            special_token_ids: HashMap::new(),
        };

        // Đăng ký mặc định 6 token đặc biệt phục vụ suy luận có cấu trúc
        tokenizer.register_special_token("<|bos|>");
        tokenizer.register_special_token("<|eos|>");
        tokenizer.register_special_token("<think>");
        tokenizer.register_special_token("</think>");
        tokenizer.register_special_token("<answer>");
        tokenizer.register_special_token("</answer>");

        tokenizer
    }

    /// Đăng ký thêm token đặc biệt với token ID tăng tiến
    pub fn register_special_token(&mut self, token: &str) -> u16 {
        if let Some(&id) = self.special_tokens.get(token) {
            return id;
        }
        let id = u16::try_from(self.vocab.len()).expect("Vượt giới hạn u16 cho từ vựng BPE");
        self.vocab.push(token.as_bytes().to_vec());
        self.special_tokens.insert(token.to_string(), id);
        self.special_token_ids.insert(id, token.to_string());
        id
    }

    /// Mã hóa chuỗi văn bản thành danh sách token ID, bảo vệ nguyên khối các token đặc biệt
    pub fn encode(&self, text: &str) -> Vec<u16> {
        let mut tokens = Vec::new();
        let mut remainder = text;

        while !remainder.is_empty() {
            // Tìm token đặc biệt xuất hiện sớm nhất trong phần văn bản còn lại
            let mut earliest_special: Option<(usize, usize, u16)> = None;
            for (special, &id) in &self.special_tokens {
                if let Some(pos) = remainder.find(special.as_str()) {
                    let len = special.len();
                    match earliest_special {
                        None => earliest_special = Some((pos, len, id)),
                        Some((min_pos, _, _)) if pos < min_pos => {
                            earliest_special = Some((pos, len, id));
                        }
                        _ => {}
                    }
                }
            }

            if let Some((pos, len, special_id)) = earliest_special {
                // Mã hóa phần văn bản thường đứng trước token đặc biệt
                if pos > 0 {
                    let normal_slice = &remainder[..pos];
                    self.encode_normal_text(normal_slice, &mut tokens);
                }
                // Thêm trực tiếp token đặc biệt
                tokens.push(special_id);
                remainder = &remainder[pos + len..];
            } else {
                // Không còn token đặc biệt nào
                self.encode_normal_text(remainder, &mut tokens);
                break;
            }
        }

        tokens
    }

    fn encode_normal_text(&self, text: &str, out: &mut Vec<u16>) {
        for piece in pre_tokenize_utf8(text) {
            let mut ids: Vec<u16> = piece.bytes().map(u16::from).collect();
            while ids.len() >= 2 {
                let mut best: Option<(usize, TokenPair, u16)> = None;
                for window in ids.windows(2) {
                    let pair = TokenPair {
                        first: window[0],
                        second: window[1],
                    };
                    if let Some(&rank_info) = self.ranks.get(&pair) {
                        let should_replace = match best {
                            None => true,
                            Some((best_rank, _, _)) => rank_info.rank < best_rank,
                        };
                        if should_replace {
                            best = Some((rank_info.rank, pair, rank_info.new_id));
                        }
                    }
                }

                let Some((_, pair, new_id)) = best else {
                    break;
                };
                ids = merge_sequence(&ids, pair, new_id);
            }
            out.extend(ids);
        }
    }

    /// Giải mã danh sách token ID trở lại chuỗi ký tự UTF-8 ban đầu
    pub fn decode(&self, ids: &[u16]) -> Result<String, String> {
        let mut raw_bytes = Vec::new();
        for &id in ids {
            if let Some(special) = self.special_token_ids.get(&id) {
                raw_bytes.extend_from_slice(special.as_bytes());
            } else {
                let bytes = self
                    .vocab
                    .get(usize::from(id))
                    .ok_or_else(|| format!("Token ID {id} không tồn tại trong từ điển"))?;
                raw_bytes.extend_from_slice(bytes);
            }
        }
        String::from_utf8(raw_bytes).map_err(|e| format!("Lỗi chuyển đổi UTF-8: {e}"))
    }

    /// Mã hóa BPE có xác suất ngẫu nhiên bỏ qua quy tắc gộp (BPE-Dropout)
    pub fn encode_with_dropout(
        &self,
        text: &str,
        dropout_prob: f64,
        rng: &mut SplitMix64,
    ) -> Vec<u16> {
        let mut tokens = Vec::new();
        for piece in pre_tokenize_utf8(text) {
            let mut ids: Vec<u16> = piece.bytes().map(u16::from).collect();
            while ids.len() >= 2 {
                let mut best: Option<(usize, TokenPair, u16)> = None;
                for window in ids.windows(2) {
                    let pair = TokenPair {
                        first: window[0],
                        second: window[1],
                    };
                    if let Some(&rank_info) = self.ranks.get(&pair) {
                        if dropout_prob > 0.0 && rng.next_f64() < dropout_prob {
                            continue;
                        }
                        let should_replace = match best {
                            None => true,
                            Some((best_rank, _, _)) => rank_info.rank < best_rank,
                        };
                        if should_replace {
                            best = Some((rank_info.rank, pair, rank_info.new_id));
                        }
                    }
                }

                let Some((_, pair, new_id)) = best else {
                    break;
                };
                ids = merge_sequence(&ids, pair, new_id);
            }
            tokens.extend(ids);
        }
        tokens
    }
}

// ============================================================================
// 4. MẶT NẠ GIẢI MÃ CÓ RÀNG BUỘC (CONSTRAINED DECODER MASK)
// ============================================================================

/// Trạng thái máy hữu hạn điều khiển cấu trúc sinh văn bản
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecoderState {
    /// Bắt đầu, chờ `<|bos|>` hoặc chuyển thẳng vào suy nghĩ
    Start,
    /// Đang sinh nội dung suy ngẫm trong thẻ `<think> ... </think>`
    InThinking,
    /// Đã đóng thẻ suy nghĩ, bắt buộc chuyển sang mở thẻ `<answer>`
    AfterThinking,
    /// Đang sinh câu trả lời trong thẻ `<answer> ... </answer>`
    InAnswer,
    /// Đã hoàn tất, chỉ cho phép sinh `<|eos|>`
    Finished,
}

/// Mặt nạ giải mã ép tuân thủ luật ngữ pháp reasoning model
#[derive(Debug, Clone)]
pub struct ConstrainedDecoderMask {
    /// Tập các token ID được phép phát sinh ở bước hiện tại
    pub allowed_token_ids: HashSet<u16>,
    /// Trạng thái ngữ pháp hiện tại
    pub current_state: DecoderState,
}

impl ConstrainedDecoderMask {
    /// Khởi tạo mặt nạ ràng buộc mới
    pub fn new(tokenizer: &BpeTokenizer) -> Self {
        let mut mask = Self {
            allowed_token_ids: HashSet::new(),
            current_state: DecoderState::Start,
        };
        mask.recompute_allowed(tokenizer);
        mask
    }

    /// Tính toán lại tập hợp các token hợp lệ dựa trên trạng thái máy hữu hạn
    pub fn recompute_allowed(&mut self, tokenizer: &BpeTokenizer) {
        self.allowed_token_ids.clear();
        let bos_id = tokenizer.special_tokens.get("<|bos|>").copied();
        let eos_id = tokenizer.special_tokens.get("<|eos|>").copied();
        let think_open = tokenizer.special_tokens.get("<think>").copied();
        let think_close = tokenizer.special_tokens.get("</think>").copied();
        let ans_open = tokenizer.special_tokens.get("<answer>").copied();
        let ans_close = tokenizer.special_tokens.get("</answer>").copied();

        match self.current_state {
            DecoderState::Start => {
                if let Some(id) = bos_id {
                    self.allowed_token_ids.insert(id);
                }
                if let Some(id) = think_open {
                    self.allowed_token_ids.insert(id);
                }
            }
            DecoderState::InThinking => {
                // Cho phép toàn bộ từ điển thường (trừ các token đặc biệt sai vị trí)
                let total_vocab = tokenizer.vocab.len();
                for id in 0..total_vocab {
                    let u16_id = id as u16;
                    if !tokenizer.special_token_ids.contains_key(&u16_id) {
                        self.allowed_token_ids.insert(u16_id);
                    }
                }
                if let Some(id) = think_close {
                    self.allowed_token_ids.insert(id);
                }
            }
            DecoderState::AfterThinking => {
                // Chỉ cho phép mở thẻ answer
                if let Some(id) = ans_open {
                    self.allowed_token_ids.insert(id);
                }
            }
            DecoderState::InAnswer => {
                let total_vocab = tokenizer.vocab.len();
                for id in 0..total_vocab {
                    let u16_id = id as u16;
                    if !tokenizer.special_token_ids.contains_key(&u16_id) {
                        self.allowed_token_ids.insert(u16_id);
                    }
                }
                if let Some(id) = ans_close {
                    self.allowed_token_ids.insert(id);
                }
                if let Some(id) = eos_id {
                    self.allowed_token_ids.insert(id);
                }
            }
            DecoderState::Finished => {
                if let Some(id) = eos_id {
                    self.allowed_token_ids.insert(id);
                }
            }
        }
    }

    /// Áp dụng mặt nạ vào vector logit: gán giá trị âm vô cực cho token vi phạm
    pub fn apply_mask(&self, logits: &mut [f64]) {
        for (id, logit) in logits.iter_mut().enumerate() {
            if !self.allowed_token_ids.contains(&(id as u16)) {
                *logit = f64::NEG_INFINITY;
            }
        }
    }

    /// Bước chuyển trạng thái khi một token đã được mô hình chọn sinh ra
    pub fn step(&mut self, emitted_token: u16, tokenizer: &BpeTokenizer) {
        let bos_id = tokenizer.special_tokens.get("<|bos|>").copied();
        let eos_id = tokenizer.special_tokens.get("<|eos|>").copied();
        let think_open = tokenizer.special_tokens.get("<think>").copied();
        let think_close = tokenizer.special_tokens.get("</think>").copied();
        let ans_open = tokenizer.special_tokens.get("<answer>").copied();
        let ans_close = tokenizer.special_tokens.get("</answer>").copied();

        match self.current_state {
            DecoderState::Start => {
                if Some(emitted_token) == think_open {
                    self.current_state = DecoderState::InThinking;
                } else if Some(emitted_token) == bos_id {
                    // Giữ nguyên Start hoặc chuyển nếu cần
                }
            }
            DecoderState::InThinking => {
                if Some(emitted_token) == think_close {
                    self.current_state = DecoderState::AfterThinking;
                }
            }
            DecoderState::AfterThinking => {
                if Some(emitted_token) == ans_open {
                    self.current_state = DecoderState::InAnswer;
                }
            }
            DecoderState::InAnswer => {
                if Some(emitted_token) == ans_close || Some(emitted_token) == eos_id {
                    self.current_state = DecoderState::Finished;
                }
            }
            DecoderState::Finished => {}
        }
        self.recompute_allowed(tokenizer);
    }
}

// ============================================================================
// 5. TENSOR VÙNG NHỚ PHẲNG CÓ BƯỚC NHẢY (STRIDED TENSOR)
// ============================================================================

/// Tensor n chiều lưu trữ trên vùng nhớ liên tục `Vec<f64>`.
///
/// Bất biến kiến trúc:
/// Phép chuyển vị `transpose` chỉ hoán đổi `shape` và `strides` (thao tác logic tốn $O(1)$),
/// không hề xáo trộn thứ tự phần tử trong `data`. Lưu ý: để giữ kiểu sở hữu `Vec<f64>` đơn giản,
/// bản minh họa này vẫn `clone()` bộ đệm khi dựng tensor mới; thư viện thực tế dùng
/// `Rc<[f64]>`/`Arc<[f64]>` hoặc view mượn để chia sẻ bộ đệm mà không sao chép.
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    /// Mảng dữ liệu số thực phẳng
    pub data: Vec<f64>,
    /// Kích thước các chiều
    pub shape: Vec<usize>,
    /// Bước nhảy bộ nhớ tương ứng cho từng chiều
    pub strides: Vec<usize>,
    /// Vị trí dịch bắt đầu trong mảng dữ liệu
    pub offset: usize,
}

impl Tensor {
    /// Tạo tensor mới có bố cục bộ nhớ liên tục (Row-Major / C-contiguous)
    pub fn new(data: Vec<f64>, shape: Vec<usize>) -> Self {
        let expected_len: usize = shape.iter().product();
        assert_eq!(
            data.len(),
            expected_len,
            "Kích thước dữ liệu ({}) không khớp với shape {:?}",
            data.len(),
            shape
        );
        let strides = Self::compute_contiguous_strides(&shape);
        Self {
            data,
            shape,
            strides,
            offset: 0,
        }
    }

    /// Tạo tensor chứa toàn số 0 với kích thước chỉ định
    pub fn zeros(shape: Vec<usize>) -> Self {
        let len: usize = shape.iter().product();
        let strides = Self::compute_contiguous_strides(&shape);
        Self {
            data: vec![0.0; len],
            shape,
            strides,
            offset: 0,
        }
    }

    fn compute_contiguous_strides(shape: &[usize]) -> Vec<usize> {
        let mut strides = vec![0; shape.len()];
        let mut current_stride = 1;
        for i in (0..shape.len()).rev() {
            strides[i] = current_stride;
            current_stride *= shape[i];
        }
        strides
    }

    /// Kiểm tra xem tensor có đang ở trạng thái bộ nhớ liên tục hay không
    pub fn is_contiguous(&self) -> bool {
        let expected = Self::compute_contiguous_strides(&self.shape);
        self.strides == expected && self.offset == 0
    }

    /// Lấy giá trị phần tử tại tọa độ đa chiều
    pub fn get(&self, indices: &[usize]) -> f64 {
        assert_eq!(indices.len(), self.shape.len());
        let mut idx = self.offset;
        for (i, &coord) in indices.iter().enumerate() {
            assert!(
                coord < self.shape[i],
                "Chỉ số {} vượt quá kích thước chiều {}",
                coord,
                self.shape[i]
            );
            idx += coord * self.strides[i];
        }
        self.data[idx]
    }

    /// Gán giá trị phần tử tại tọa độ đa chiều
    pub fn set(&mut self, indices: &[usize], val: f64) {
        assert_eq!(indices.len(), self.shape.len());
        let mut idx = self.offset;
        for (i, &coord) in indices.iter().enumerate() {
            assert!(
                coord < self.shape[i],
                "Chỉ số {} vượt quá kích thước chiều {}",
                coord,
                self.shape[i]
            );
            idx += coord * self.strides[i];
        }
        self.data[idx] = val;
    }

    /// Hoán đổi hai chiều: chỉ đổi chỗ `shape`/`strides` (logic $O(1)$, không xáo trộn dữ liệu)
    pub fn transpose(&self, dim0: usize, dim1: usize) -> Self {
        assert!(dim0 < self.shape.len() && dim1 < self.shape.len());
        let mut new_shape = self.shape.clone();
        let mut new_strides = self.strides.clone();
        new_shape.swap(dim0, dim1);
        new_strides.swap(dim0, dim1);
        Self {
            data: self.data.clone(),
            shape: new_shape,
            strides: new_strides,
            offset: self.offset,
        }
    }

    /// Cắt lát ma trận 2D mà không sao chép lại vùng dữ liệu
    pub fn slice_2d(&self, row_range: (usize, usize), col_range: (usize, usize)) -> Self {
        assert_eq!(self.shape.len(), 2, "Chỉ hỗ trợ ma trận 2D");
        let (r_start, r_end) = row_range;
        let (c_start, c_end) = col_range;

        assert!(r_start <= r_end && r_end <= self.shape[0]);
        assert!(c_start <= c_end && c_end <= self.shape[1]);

        let new_rows = r_end - r_start;
        let new_cols = c_end - c_start;

        let new_offset = self.offset + r_start * self.strides[0] + c_start * self.strides[1];

        Self {
            data: self.data.clone(),
            shape: vec![new_rows, new_cols],
            strides: self.strides.clone(),
            offset: new_offset,
        }
    }

    /// Chuyển đổi tensor về bố cục liên tục trong bộ nhớ
    pub fn to_contiguous(&self) -> Self {
        if self.is_contiguous() {
            return self.clone();
        }
        let total_len: usize = self.shape.iter().product();
        let mut contiguous_data = Vec::with_capacity(total_len);
        let mut coords = vec![0; self.shape.len()];

        for _ in 0..total_len {
            contiguous_data.push(self.get(&coords));
            // Tăng tọa độ theo thứ tự hàng (Row-Major)
            for d in (0..self.shape.len()).rev() {
                coords[d] += 1;
                if coords[d] < self.shape[d] {
                    break;
                }
                coords[d] = 0;
            }
        }

        Self::new(contiguous_data, self.shape.clone())
    }

    /// Nhân hai ma trận 2D: $C = A \times B$
    ///
    /// Hỗ trợ cả tensor có bước nhảy tùy ý (ví dụ $B$ là kết quả của `transpose`).
    pub fn matmul(&self, other: &Self) -> Self {
        assert_eq!(self.shape.len(), 2, "A phải là ma trận 2D");
        assert_eq!(other.shape.len(), 2, "B phải là ma trận 2D");
        let (m, k1) = (self.shape[0], self.shape[1]);
        let (k2, n) = (other.shape[0], other.shape[1]);
        assert_eq!(
            k1, k2,
            "Chiều trong của hai ma trận không khớp: {} != {}",
            k1, k2
        );

        let mut out = vec![0.0; m * n];
        for i in 0..m {
            let row_offset_a = self.offset + i * self.strides[0];
            for j in 0..n {
                let col_offset_b = other.offset + j * other.strides[1];
                let mut sum = 0.0;
                for k in 0..k1 {
                    let a_val = self.data[row_offset_a + k * self.strides[1]];
                    let b_val = other.data[col_offset_b + k * other.strides[0]];
                    sum += a_val * b_val;
                }
                out[i * n + j] = sum;
            }
        }

        Self::new(out, vec![m, n])
    }

    /// Tính hàm `log_softmax` theo chiều cuối cùng với kỹ thuật trừ giá trị cực đại
    /// để chống tràn số học (Max-Subtraction Numerical Stability).
    ///
    /// Công thức: $\log \text{softmax}(z_i) = (z_i - \max_k z_k) - \ln\left(\sum_j \exp(z_j - \max_k z_k)\right)$.
    pub fn log_softmax(&self) -> Self {
        assert_eq!(self.shape.len(), 2, "Chỉ hỗ trợ tensor 2D [batch, vocab]");
        let (rows, cols) = (self.shape[0], self.shape[1]);
        let mut out_data = vec![0.0; rows * cols];

        for i in 0..rows {
            // 1. Tìm giá trị lớn nhất trong hàng
            let mut max_val = f64::NEG_INFINITY;
            for j in 0..cols {
                let v = self.get(&[i, j]);
                if v > max_val {
                    max_val = v;
                }
            }

            // 2. Tính tổng exp sau khi đã trừ max
            let mut sum_exp = 0.0;
            for j in 0..cols {
                let v = self.get(&[i, j]);
                sum_exp += (v - max_val).exp();
            }
            let log_sum_exp = sum_exp.ln();

            // 3. Ghi kết quả log_softmax
            for j in 0..cols {
                let v = self.get(&[i, j]);
                out_data[i * cols + j] = (v - max_val) - log_sum_exp;
            }
        }

        Self::new(out_data, vec![rows, cols])
    }

    /// Tính hàm mất mát Cross Entropy có mặt nạ bỏ qua nhãn `ignore_index = -100`.
    ///
    /// Trả về cặp `(loss, valid_count)`.
    pub fn cross_entropy_masked(&self, targets: &[i64], ignore_index: i64) -> (f64, usize) {
        assert_eq!(self.shape.len(), 2);
        let (rows, cols) = (self.shape[0], self.shape[1]);
        assert_eq!(rows, targets.len());

        let mut total_loss = 0.0;
        let mut valid_count = 0;

        for (i, &target) in targets.iter().enumerate() {
            if target == ignore_index {
                continue;
            }
            assert!(
                target >= 0 && (target as usize) < cols,
                "Mục tiêu {} nằm ngoài phạm vi số cột {}",
                target,
                cols
            );
            let log_p = self.get(&[i, target as usize]);
            total_loss -= log_p;
            valid_count += 1;
        }

        let mean_loss = if valid_count > 0 {
            total_loss / (valid_count as f64)
        } else {
            0.0
        };

        (mean_loss, valid_count)
    }
}

// ============================================================================
// 6. ĐỘNG CƠ VI PHÂN NGƯỢC AUTOGRAD TAPE
// ============================================================================

/// Định danh nút trong băng ghi vi phân tự động
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ValueId {
    /// Chỉ số trong danh sách nút của Tape
    pub id: usize,
}

/// Các phép toán được ghi lại trên băng vi phân
#[derive(Debug, Clone)]
pub enum TapeOp {
    /// Nút lá (biến đầu vào hoặc trọng số tham số)
    Leaf,
    /// Phép cộng hai tensor cùng kích thước
    Add,
    /// Phép trừ hai tensor cùng kích thước
    Sub,
    /// Phép nhân từng phần tử (Hadamard product)
    Mul,
    /// Phép nhân ma trận 2D
    MatMul,
    /// Phép nhân với một số vô hướng
    Scale {
        /// Hệ số tỉ lệ
        factor: f64,
    },
    /// Hàm kích hoạt ReLU phi tuyến
    Relu,
    /// Hàm chuẩn hóa log_softmax theo chiều cuối
    LogSoftmax,
    /// Hàm mất mát CrossEntropy với nhãn đích
    CrossEntropy {
        /// Danh sách nhãn đích
        targets: Vec<i64>,
        /// Nhãn bị bỏ qua
        ignore_index: i64,
    },
    /// Chuẩn hóa RMSNorm kèm tham số trọng số tỉ lệ $w$
    RmsNorm {
        /// Hằng số bảo vệ chống chia cho 0
        eps: f64,
    },
    /// Phép tính tổng toàn bộ các phần tử thành một số vô hướng
    Sum,
}

/// Bản ghi của một nút tính toán trong đồ thị vô hướng có hướng (DAG)
#[derive(Debug, Clone)]
pub struct TapeNode {
    /// Định danh nút
    pub id: ValueId,
    /// Giá trị kết quả tính toán xuôi (Forward pass)
    pub value: Tensor,
    /// Gradient tích lũy trong lan truyền ngược (Backward pass)
    pub grad: Option<Tensor>,
    /// Phép toán đã sinh ra nút này
    pub op: TapeOp,
    /// Các nút cha trực tiếp làm đầu vào
    pub parents: Vec<ValueId>,
}

/// Băng ghi vi phân ngược tự động (Reverse-Mode Autograd Tape).
///
/// Cơ chế vận hành:
/// 1. Giai đoạn xuôi: Ghi nhận từng phép toán và lưu trữ giá trị tạm thời vào danh sách nút `nodes`.
/// 2. Giai đoạn ngược `backward()`: Duyệt ngược danh sách từ nút mất mát (theo thứ tự tô-pô),
///    tính toán đạo hàm giải tích và tích lũy vào các nút cha thông qua quy tắc dây chuyền (Chain Rule).
#[derive(Debug, Clone, Default)]
pub struct AutogradTape {
    /// Danh sách các nút theo thứ tự thực thi
    pub nodes: Vec<TapeNode>,
}

impl AutogradTape {
    /// Khởi tạo băng vi phân mới
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    /// Xóa toàn bộ gradient đã tích lũy về `None`
    pub fn zero_grad(&mut self) {
        for node in &mut self.nodes {
            node.grad = None;
        }
    }

    /// Đưa một biến lá (trọng số hoặc đầu vào) vào băng vi phân
    pub fn leaf(&mut self, value: Tensor) -> ValueId {
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value,
            grad: None,
            op: TapeOp::Leaf,
            parents: Vec::new(),
        });
        id
    }

    /// Lấy tham chiếu đến giá trị của nút
    pub fn value(&self, id: ValueId) -> &Tensor {
        &self.nodes[id.id].value
    }

    /// Lấy tham chiếu đến gradient của nút
    pub fn grad(&self, id: ValueId) -> Option<&Tensor> {
        self.nodes[id.id].grad.as_ref()
    }

    /// Phép cộng hai tensor
    pub fn add(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let va = self.value(a);
        let vb = self.value(b);
        assert_eq!(va.shape, vb.shape);
        let mut out = Vec::with_capacity(va.data.len());
        for i in 0..va.data.len() {
            out.push(va.data[i] + vb.data[i]);
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Add,
            parents: vec![a, b],
        });
        id
    }

    /// Phép trừ hai tensor: $A - B$
    pub fn sub(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let va = self.value(a);
        let vb = self.value(b);
        assert_eq!(va.shape, vb.shape);
        let mut out = Vec::with_capacity(va.data.len());
        for i in 0..va.data.len() {
            out.push(va.data[i] - vb.data[i]);
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Sub,
            parents: vec![a, b],
        });
        id
    }

    /// Phép nhân từng phần tử (Hadamard product): $A \odot B$
    pub fn mul(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let va = self.value(a);
        let vb = self.value(b);
        assert_eq!(va.shape, vb.shape);
        let mut out = Vec::with_capacity(va.data.len());
        for i in 0..va.data.len() {
            out.push(va.data[i] * vb.data[i]);
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Mul,
            parents: vec![a, b],
        });
        id
    }

    /// Phép nhân với số vô hướng
    pub fn scale(&mut self, a: ValueId, factor: f64) -> ValueId {
        let va = self.value(a);
        let mut out = Vec::with_capacity(va.data.len());
        for v in &va.data {
            out.push(v * factor);
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Scale { factor },
            parents: vec![a],
        });
        id
    }

    /// Nhân ma trận 2D: $A \times B$
    pub fn matmul(&mut self, a: ValueId, b: ValueId) -> ValueId {
        let va = self.value(a);
        let vb = self.value(b);
        let res = va.matmul(vb);
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: res,
            grad: None,
            op: TapeOp::MatMul,
            parents: vec![a, b],
        });
        id
    }

    /// Hàm kích hoạt ReLU: $\max(0, x)$
    pub fn relu(&mut self, a: ValueId) -> ValueId {
        let va = self.value(a);
        let mut out = Vec::with_capacity(va.data.len());
        for &v in &va.data {
            out.push(if v > 0.0 { v } else { 0.0 });
        }
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, va.shape.clone()),
            grad: None,
            op: TapeOp::Relu,
            parents: vec![a],
        });
        id
    }

    /// Chuẩn hóa `log_softmax`
    pub fn log_softmax(&mut self, a: ValueId) -> ValueId {
        let va = self.value(a);
        let res = va.log_softmax();
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: res,
            grad: None,
            op: TapeOp::LogSoftmax,
            parents: vec![a],
        });
        id
    }

    /// Hàm mất mát Cross Entropy
    pub fn cross_entropy(&mut self, a: ValueId, targets: Vec<i64>, ignore_index: i64) -> ValueId {
        let va = self.value(a);
        let (loss, _) = va.cross_entropy_masked(&targets, ignore_index);
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(vec![loss], vec![1]),
            grad: None,
            op: TapeOp::CrossEntropy {
                targets,
                ignore_index,
            },
            parents: vec![a],
        });
        id
    }

    /// Chuẩn hóa RMSNorm có trọng số tỉ lệ $w$
    pub fn rmsnorm(&mut self, x: ValueId, w: ValueId, eps: f64) -> ValueId {
        let vx = self.value(x);
        let vw = self.value(w);
        assert_eq!(vx.shape.len(), 2);
        assert_eq!(vw.shape.len(), 1);
        let (rows, cols) = (vx.shape[0], vx.shape[1]);
        assert_eq!(cols, vw.shape[0]);

        let mut out = vec![0.0; rows * cols];
        for i in 0..rows {
            let mut sum_sq = 0.0;
            for j in 0..cols {
                let v = vx.get(&[i, j]);
                sum_sq += v * v;
            }
            let rms = (sum_sq / (cols as f64) + eps).sqrt();
            let inv_rms = 1.0 / rms;
            for j in 0..cols {
                let v = vx.get(&[i, j]);
                let weight = vw.get(&[j]);
                out[i * cols + j] = v * inv_rms * weight;
            }
        }

        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(out, vec![rows, cols]),
            grad: None,
            op: TapeOp::RmsNorm { eps },
            parents: vec![x, w],
        });
        id
    }

    /// Tính tổng toàn bộ phần tử của tensor
    pub fn sum(&mut self, a: ValueId) -> ValueId {
        let va = self.value(a);
        let s: f64 = va.data.iter().sum();
        let id = ValueId {
            id: self.nodes.len(),
        };
        self.nodes.push(TapeNode {
            id,
            value: Tensor::new(vec![s], vec![1]),
            grad: None,
            op: TapeOp::Sum,
            parents: vec![a],
        });
        id
    }

    fn accumulate_grad(&mut self, id: ValueId, grad: Tensor) {
        let node = &mut self.nodes[id.id];
        match &mut node.grad {
            Some(existing) => {
                assert_eq!(existing.shape, grad.shape);
                for i in 0..existing.data.len() {
                    existing.data[i] += grad.data[i];
                }
            }
            None => {
                node.grad = Some(grad);
            }
        }
    }

    /// Lan truyền ngược vi phân tự động (Reverse-Mode Backward Pass)
    pub fn backward(&mut self, loss_id: ValueId) {
        // Nút tổn thất bắt đầu với gradient = 1.0
        let loss_val = &self.nodes[loss_id.id].value;
        let seed_grad = Tensor::new(vec![1.0; loss_val.data.len()], loss_val.shape.clone());
        self.nodes[loss_id.id].grad = Some(seed_grad);

        // Duyệt ngược danh sách các nút (đã ở thứ tự tô-pô vì sinh tuần tự)
        for i in (0..=loss_id.id).rev() {
            let node_grad = match self.nodes[i].grad.clone() {
                Some(g) => g,
                None => continue,
            };

            let op = self.nodes[i].op.clone();
            let parents = self.nodes[i].parents.clone();

            match op {
                TapeOp::Leaf => {
                    // Nút lá: gradient dừng lại ở đây để bộ tối ưu sử dụng
                }
                TapeOp::Add => {
                    self.accumulate_grad(parents[0], node_grad.clone());
                    self.accumulate_grad(parents[1], node_grad);
                }
                TapeOp::Sub => {
                    self.accumulate_grad(parents[0], node_grad.clone());
                    let mut neg_grad = node_grad;
                    for v in &mut neg_grad.data {
                        *v = -*v;
                    }
                    self.accumulate_grad(parents[1], neg_grad);
                }
                TapeOp::Mul => {
                    let va = self.nodes[parents[0].id].value.clone();
                    let vb = self.nodes[parents[1].id].value.clone();

                    let mut ga = Vec::with_capacity(va.data.len());
                    let mut gb = Vec::with_capacity(vb.data.len());
                    for idx in 0..node_grad.data.len() {
                        ga.push(node_grad.data[idx] * vb.data[idx]);
                        gb.push(node_grad.data[idx] * va.data[idx]);
                    }
                    self.accumulate_grad(parents[0], Tensor::new(ga, va.shape));
                    self.accumulate_grad(parents[1], Tensor::new(gb, vb.shape));
                }
                TapeOp::Scale { factor } => {
                    let mut scaled = node_grad;
                    for v in &mut scaled.data {
                        *v *= factor;
                    }
                    self.accumulate_grad(parents[0], scaled);
                }
                TapeOp::Relu => {
                    let vin = &self.nodes[parents[0].id].value;
                    let mut gout = Vec::with_capacity(vin.data.len());
                    for idx in 0..vin.data.len() {
                        let g = if vin.data[idx] > 0.0 {
                            node_grad.data[idx]
                        } else {
                            0.0
                        };
                        gout.push(g);
                    }
                    self.accumulate_grad(parents[0], Tensor::new(gout, vin.shape.clone()));
                }
                TapeOp::MatMul => {
                    // C = A x B => dL/dA = dL/dC x B^T ; dL/dB = A^T x dL/dC
                    let va = self.nodes[parents[0].id].value.clone();
                    let vb = self.nodes[parents[1].id].value.clone();

                    let bt = vb.transpose(0, 1);
                    let grad_a = node_grad.matmul(&bt);

                    let at = va.transpose(0, 1);
                    let grad_b = at.matmul(&node_grad);

                    self.accumulate_grad(parents[0], grad_a);
                    self.accumulate_grad(parents[1], grad_b);
                }
                TapeOp::LogSoftmax => {
                    // Đạo hàm LogSoftmax: dL/dz_i = g_i - exp(log_p_i) * sum_j(g_j)
                    let log_p = &self.nodes[i].value;
                    let (rows, cols) = (log_p.shape[0], log_p.shape[1]);
                    let mut grad_in = vec![0.0; rows * cols];

                    for r in 0..rows {
                        let mut sum_g = 0.0;
                        for c in 0..cols {
                            sum_g += node_grad.get(&[r, c]);
                        }
                        for c in 0..cols {
                            let lp = log_p.get(&[r, c]);
                            let p = lp.exp();
                            let g = node_grad.get(&[r, c]);
                            grad_in[r * cols + c] = g - p * sum_g;
                        }
                    }
                    self.accumulate_grad(parents[0], Tensor::new(grad_in, vec![rows, cols]));
                }
                TapeOp::CrossEntropy {
                    targets,
                    ignore_index,
                } => {
                    // Đầu vào của nút này là LOG-XÁC SUẤT (đầu ra của log_softmax), nên
                    // dL/dlogP[r, c] = -1_{c == target} / valid_count * upstream_grad.
                    // Khi nối tiếp với nút LogSoftmax ở dưới, gradient hợp thành theo logit
                    // chính là (softmax - one_hot) / valid_count.
                    let vin = &self.nodes[parents[0].id].value;
                    let (rows, cols) = (vin.shape[0], vin.shape[1]);
                    let upstream = node_grad.data[0];

                    let mut valid_count = 0;
                    for &t in &targets {
                        if t != ignore_index {
                            valid_count += 1;
                        }
                    }

                    let scale = if valid_count > 0 {
                        upstream / (valid_count as f64)
                    } else {
                        0.0
                    };

                    let mut grad_in = vec![0.0; rows * cols];
                    for r in 0..rows {
                        let t = targets[r];
                        if t == ignore_index {
                            continue;
                        }
                        grad_in[r * cols + (t as usize)] = -scale;
                    }
                    self.accumulate_grad(parents[0], Tensor::new(grad_in, vec![rows, cols]));
                }
                TapeOp::RmsNorm { eps } => {
                    // Đạo hàm giải tích RMSNorm
                    let vx = self.nodes[parents[0].id].value.clone();
                    let vw = self.nodes[parents[1].id].value.clone();
                    let (rows, cols) = (vx.shape[0], vx.shape[1]);
                    let d = cols as f64;

                    let mut grad_x = vec![0.0; rows * cols];
                    let mut grad_w = vec![0.0; cols];

                    for r in 0..rows {
                        let mut sum_sq = 0.0;
                        for c in 0..cols {
                            let x_val = vx.get(&[r, c]);
                            sum_sq += x_val * x_val;
                        }
                        let inv_rms = 1.0 / (sum_sq / d + eps).sqrt();

                        let mut dot = 0.0;
                        for (c, gw) in grad_w.iter_mut().enumerate() {
                            let x_val = vx.get(&[r, c]);
                            let w_val = vw.get(&[c]);
                            let g_val = node_grad.get(&[r, c]);
                            let x_hat = x_val * inv_rms;

                            *gw += g_val * x_hat;
                            dot += g_val * w_val * x_hat;
                        }
                        dot /= d;

                        for c in 0..cols {
                            let x_val = vx.get(&[r, c]);
                            let w_val = vw.get(&[c]);
                            let g_val = node_grad.get(&[r, c]);
                            grad_x[r * cols + c] =
                                inv_rms * (g_val * w_val - (x_val * inv_rms) * dot);
                        }
                    }

                    self.accumulate_grad(parents[0], Tensor::new(grad_x, vec![rows, cols]));
                    self.accumulate_grad(parents[1], Tensor::new(grad_w, vec![cols]));
                }
                TapeOp::Sum => {
                    let vin = &self.nodes[parents[0].id].value;
                    let upstream = node_grad.data[0];
                    let grad_in = vec![upstream; vin.data.len()];
                    self.accumulate_grad(parents[0], Tensor::new(grad_in, vin.shape.clone()));
                }
            }
        }
    }
}

// ============================================================================
// 7. HÀM KIỂM CHỨNG SAI PHÂN HỮU HẠN TRUNG TÂM (CENTRAL FINITE DIFFERENCE)
// ============================================================================

/// Kiểm chứng độ chính xác của đạo hàm giải tích so với sai phân hữu hạn trung tâm.
///
/// Bất biến toán học:
/// Sai số giữa đạo hàm giải tích $g_{\text{analytic}}$ và xấp xỉ sai phân trung tâm
/// $g_{\text{approx}} = \frac{f(x + \epsilon) - f(x - \epsilon)}{2\epsilon}$
/// phải nhỏ hơn `tolerance` (thông thường $< 10^{-8}$) cho mọi phần tử trong tensor.
pub fn check_grad_central_diff<F>(
    f: F,
    x0: &Tensor,
    grad_analytical: &Tensor,
    eps: f64,
    tolerance: f64,
) -> Result<(), (usize, f64, f64, f64)>
where
    F: Fn(&Tensor) -> f64,
{
    assert_eq!(x0.data.len(), grad_analytical.data.len());
    let mut x_mut = x0.clone();

    for i in 0..x0.data.len() {
        let orig = x_mut.data[i];

        x_mut.data[i] = orig + eps;
        let f_plus = f(&x_mut);

        x_mut.data[i] = orig - eps;
        let f_minus = f(&x_mut);

        x_mut.data[i] = orig;

        let num_grad = (f_plus - f_minus) / (2.0 * eps);
        let ana_grad = grad_analytical.data[i];
        let diff = (num_grad - ana_grad).abs();

        if diff > tolerance {
            return Err((i, ana_grad, num_grad, diff));
        }
    }

    Ok(())
}

/// Hàm tính toán xuôi và đạo hàm ngược giải tích của LayerNorm
pub fn layernorm_forward_backward(
    x: &[f64],
    gamma: &[f64],
    beta: &[f64],
    eps: f64,
    upstream_grad: &[f64],
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let d = x.len() as f64;
    let mean = x.iter().sum::<f64>() / d;
    let var = x.iter().map(|&v| (v - mean) * (v - mean)).sum::<f64>() / d;
    let inv_std = 1.0 / (var + eps).sqrt();

    let mut y = vec![0.0; x.len()];
    let mut x_hat = vec![0.0; x.len()];
    for i in 0..x.len() {
        x_hat[i] = (x[i] - mean) * inv_std;
        y[i] = x_hat[i] * gamma[i] + beta[i];
    }

    let mut dgamma = vec![0.0; x.len()];
    let mut dbeta = vec![0.0; x.len()];
    for i in 0..x.len() {
        dgamma[i] = upstream_grad[i] * x_hat[i];
        dbeta[i] = upstream_grad[i];
    }

    let mut dx = vec![0.0; x.len()];
    let mut sum_g = 0.0;
    let mut sum_g_xhat = 0.0;
    for i in 0..x.len() {
        let g_gamma = upstream_grad[i] * gamma[i];
        sum_g += g_gamma;
        sum_g_xhat += g_gamma * x_hat[i];
    }

    for i in 0..x.len() {
        let g_gamma = upstream_grad[i] * gamma[i];
        dx[i] = inv_std * (g_gamma - (sum_g + x_hat[i] * sum_g_xhat) / d);
    }

    (y, dx, dgamma, dbeta)
}

// ============================================================================
// 8. CHƯƠNG TRÌNH CHÍNH MINH HỌA THỰC CHIẾN (MAIN PIPELINE)
// ============================================================================

fn main() {
    println!("=== CHƯƠNG 92: BPE TOKENIZER, TENSOR BƯỚC NHẢY & AUTOGRAD TAPE ===");

    // 1. Kiểm chứng BPE Tokenizer và token đặc biệt
    let corpus =
        "học lập trình Rust từ số không. học lập trình hệ thống cùng Rust. mô hình ngôn ngữ lớn.";
    let tokenizer = BpeTokenizer::train(corpus, 30);
    println!(
        "Đã huấn luyện BPE Tokenizer: từ vựng có {} token",
        tokenizer.vocab.len()
    );

    let test_prompt = "<|bos|><think>mình cần giải bài toán này</think><answer>học Rust rất thú vị!</answer><|eos|>";
    let encoded = tokenizer.encode(test_prompt);
    let decoded = tokenizer.decode(&encoded).expect("Lỗi giải mã UTF-8");
    println!(
        "Encode {} tokens -> Decode khớp nguyên bản: {}",
        encoded.len(),
        decoded == test_prompt
    );
    assert_eq!(decoded, test_prompt);

    // 2. Kiểm chứng Zero-Copy Transpose và MatMul
    let a = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
    let b = Tensor::new(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], vec![2, 3]);
    let b_t = b.transpose(0, 1); // shape [3, 2], bước nhảy [1, 3] không copy
    let c = a.matmul(&b_t);
    println!("Tensor A [2, 3] x B^T [3, 2] = C {:?}", c.shape);
    assert_eq!(c.shape, vec![2, 2]);

    // 3. Xây dựng Micro Network 2 tầng với AutogradTape và kiểm chứng sai phân
    let mut tape = AutogradTape::new();
    let mut rng = SplitMix64::new(42);

    // Đầu vào dương để ReLU có cả đơn vị bật lẫn tắt, tránh kiểm chứng "0 so với 0" vô nghĩa
    let x_val = Tensor::new(vec![0.5, 0.2, 0.8, 0.5], vec![2, 2]);
    let w1_data: Vec<f64> = (0..4).map(|_| rng.next_gaussian(0.0, 0.1)).collect();
    let w1_val = Tensor::new(w1_data, vec![2, 2]);

    let x_id = tape.leaf(x_val.clone());
    let w1_id = tape.leaf(w1_val.clone());

    let h1_id = tape.matmul(x_id, w1_id);
    let a1_id = tape.relu(h1_id);
    let log_probs_id = tape.log_softmax(a1_id);

    let targets = vec![1i64, 0i64];
    let loss_id = tape.cross_entropy(log_probs_id, targets.clone(), -100);

    let loss_val = tape.value(loss_id).data[0];
    println!("Giá trị mất mát Cross Entropy khởi đầu: {:.6}", loss_val);

    tape.backward(loss_id);
    let grad_w1 = tape.grad(w1_id).expect("Phải có gradient cho w1").clone();
    let grad_l1: f64 = grad_w1.data.iter().map(|g| g.abs()).sum();
    println!("Chuẩn L1 của gradient w1: {:.6}", grad_l1);
    assert!(
        grad_l1 > 1e-3,
        "Gradient bằng 0 làm kiểm chứng sai phân trở nên vô nghĩa"
    );

    // 4. Kiểm chứng sai phân hữu hạn trung tâm < 1e-8
    let f_loss = |w_test: &Tensor| -> f64 {
        let mut t = AutogradTape::new();
        let xi = t.leaf(x_val.clone());
        let wi = t.leaf(w_test.clone());
        let hi = t.matmul(xi, wi);
        let ai = t.relu(hi);
        let lpi = t.log_softmax(ai);
        let li = t.cross_entropy(lpi, targets.clone(), -100);
        t.value(li).data[0]
    };

    let check_res = check_grad_central_diff(f_loss, &w1_val, &grad_w1, 1e-6, 1e-8);
    match check_res {
        Ok(()) => println!("KIỂM CHỨNG SAI PHÂN HỮU HẠN THÀNH CÔNG: Sai số < 1e-8 tuyệt đối!"),
        Err((idx, ana, num, diff)) => {
            panic!(
                "Sai phân hữu hạn lệch tại chỉ số {}: giải tích={}, số học={}, sai số={}",
                idx, ana, num, diff
            );
        }
    }

    println!("Tất cả các thành phần toán học và cấu trúc dữ liệu của Chương 92 đều hợp lệ 100%!");
}

// ============================================================================
// 9. BỘ KIỂM THỬ ĐƠN VỊ TOÀN DIỆN (>= 30 TESTS TẤT ĐỊNH)
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splitmix64_deterministic_reproducibility() {
        let mut rng1 = SplitMix64::new(12345);
        let mut rng2 = SplitMix64::new(12345);
        for _ in 0..100 {
            assert_eq!(rng1.next_u64(), rng2.next_u64());
        }
    }

    #[test]
    fn test_splitmix64_signed_range() {
        let mut rng = SplitMix64::new(999);
        for _ in 0..500 {
            let val = rng.next_signed(2.5);
            assert!((-2.5..=2.5).contains(&val));
        }
    }

    #[test]
    fn test_splitmix64_gaussian_mean_variance() {
        let mut rng = SplitMix64::new(2026);
        let n = 10_000;
        let mut sum = 0.0;
        let mut sum_sq = 0.0;
        for _ in 0..n {
            let v = rng.next_gaussian(0.0, 1.0);
            sum += v;
            sum_sq += v * v;
        }
        let mean = sum / (n as f64);
        let variance = (sum_sq / (n as f64)) - (mean * mean);
        assert!((mean - 0.0).abs() < 0.05, "Mean: {}", mean);
        assert!((variance - 1.0).abs() < 0.05, "Variance: {}", variance);
    }

    #[test]
    fn test_pre_tokenize_utf8_lossless_roundtrip() {
        let samples = [
            "Hello, world! 123",
            "Học lập trình Rust cùng AI, không thư viện ngoài.",
            "fn main() { println!(\"ch92: 100% std\"); }\n",
            "   \t\n  khoảng trắng xen kẽ...!!!",
        ];
        for sample in samples {
            let chunks = pre_tokenize_utf8(sample);
            let reconstructed = chunks.concat();
            assert_eq!(reconstructed, sample);
        }
    }

    #[test]
    fn test_pre_tokenize_utf8_empty_string() {
        assert!(pre_tokenize_utf8("").is_empty());
    }

    #[test]
    fn test_bpe_base_256_bytes_exist() {
        let tokenizer = BpeTokenizer::train("ab", 0);
        assert!(tokenizer.vocab.len() >= 256);
        for i in 0..=255u8 {
            assert_eq!(tokenizer.vocab[i as usize], vec![i]);
        }
    }

    #[test]
    fn test_bpe_training_and_merge_order() {
        // "aaabdaaabac" -> "aa" xuất hiện nhiều nhất
        let corpus = "aaabdaaabac aaabdaaabac";
        let tokenizer = BpeTokenizer::train(corpus, 5);
        assert!(!tokenizer.merges.is_empty());
        let encoded = tokenizer.encode("aaabdaaabac");
        let decoded = tokenizer.decode(&encoded).unwrap();
        assert_eq!(decoded, "aaabdaaabac");
    }

    #[test]
    fn test_bpe_tie_breaking_deterministic() {
        // Hai cặp (a, b) và (c, d) có cùng tần suất xuất hiện
        let corpus = "ab cd ab cd";
        let tok1 = BpeTokenizer::train(corpus, 1);
        let tok2 = BpeTokenizer::train(corpus, 1);
        assert_eq!(tok1.merges, tok2.merges);
    }

    #[test]
    fn test_bpe_encode_decode_vietnamese_accents() {
        let corpus =
            "tiếng Việt có dấu: sắc, huyền, hỏi, ngã, nặng. mô hình suy luận trí tuệ nhân tạo.";
        let tokenizer = BpeTokenizer::train(corpus, 20);
        let test_phrase = "tiếng Việt có dấu: sắc, huyền";
        let ids = tokenizer.encode(test_phrase);
        let decoded = tokenizer.decode(&ids).unwrap();
        assert_eq!(decoded, test_phrase);
    }

    #[test]
    fn test_bpe_special_tokens_preservation() {
        let corpus = "chào thế giới";
        let tokenizer = BpeTokenizer::train(corpus, 10);
        let input = "<|bos|><think>suy nghĩ 123</think><answer>kết quả</answer><|eos|>";
        let ids = tokenizer.encode(input);
        let decoded = tokenizer.decode(&ids).unwrap();
        assert_eq!(decoded, input);

        // Kiểm tra chắc chắn các ID đặc biệt nằm trong chuỗi token
        let bos_id = tokenizer.special_tokens["<|bos|>"];
        let think_id = tokenizer.special_tokens["<think>"];
        assert_eq!(ids[0], bos_id);
        assert_eq!(ids[1], think_id);
    }

    #[test]
    fn test_bpe_never_splits_special_token() {
        let tokenizer = BpeTokenizer::train("think answer", 5);
        let input = "<think>";
        let ids = tokenizer.encode(input);
        assert_eq!(ids.len(), 1);
        assert_eq!(ids[0], tokenizer.special_tokens["<think>"]);
    }

    #[test]
    fn test_bpe_unknown_bytes_lossless() {
        // Chuỗi ký tự Unicode chưa từng xuất hiện trong tập huấn luyện
        let tokenizer = BpeTokenizer::train("abc def", 5);
        let unseen = "🦀 ốc sên 🚀";
        let ids = tokenizer.encode(unseen);
        let decoded = tokenizer.decode(&ids).unwrap();
        assert_eq!(decoded, unseen);
    }

    #[test]
    fn test_constrained_decoder_mask_initial_state() {
        let tokenizer = BpeTokenizer::train("test", 0);
        let mask = ConstrainedDecoderMask::new(&tokenizer);
        assert_eq!(mask.current_state, DecoderState::Start);
        let bos = tokenizer.special_tokens["<|bos|>"];
        let think = tokenizer.special_tokens["<think>"];
        assert!(mask.allowed_token_ids.contains(&bos));
        assert!(mask.allowed_token_ids.contains(&think));
    }

    #[test]
    fn test_constrained_decoder_mask_state_transitions() {
        let tokenizer = BpeTokenizer::train("hello world", 5);
        let mut mask = ConstrainedDecoderMask::new(&tokenizer);

        let think = tokenizer.special_tokens["<think>"];
        let think_close = tokenizer.special_tokens["</think>"];
        let ans = tokenizer.special_tokens["<answer>"];
        let ans_close = tokenizer.special_tokens["</answer>"];

        mask.step(think, &tokenizer);
        assert_eq!(mask.current_state, DecoderState::InThinking);

        mask.step(think_close, &tokenizer);
        assert_eq!(mask.current_state, DecoderState::AfterThinking);
        assert!(mask.allowed_token_ids.contains(&ans));

        mask.step(ans, &tokenizer);
        assert_eq!(mask.current_state, DecoderState::InAnswer);

        mask.step(ans_close, &tokenizer);
        assert_eq!(mask.current_state, DecoderState::Finished);
    }

    #[test]
    fn test_constrained_decoder_mask_apply_mask() {
        let tokenizer = BpeTokenizer::train("abc", 0);
        let mask = ConstrainedDecoderMask::new(&tokenizer);
        let mut logits = vec![1.0; tokenizer.vocab.len()];
        mask.apply_mask(&mut logits);

        let bos = tokenizer.special_tokens["<|bos|>"] as usize;
        assert_eq!(logits[bos], 1.0);

        let regular_byte = b'a' as usize;
        assert_eq!(logits[regular_byte], f64::NEG_INFINITY);
    }

    #[test]
    fn test_tensor_contiguous_strides() {
        let t = Tensor::new(vec![1.0; 24], vec![2, 3, 4]);
        assert_eq!(t.strides, vec![12, 4, 1]);
        assert!(t.is_contiguous());
    }

    #[test]
    fn test_tensor_get_and_set() {
        let mut t = Tensor::zeros(vec![2, 3]);
        t.set(&[1, 2], 42.0);
        assert_eq!(t.get(&[1, 2]), 42.0);
        assert_eq!(t.get(&[0, 0]), 0.0);
    }

    #[test]
    fn test_tensor_transpose_zero_copy() {
        let t = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
        let t_tr = t.transpose(0, 1);
        assert_eq!(t_tr.shape, vec![3, 2]);
        assert_eq!(t_tr.strides, vec![1, 3]);
        assert!(!t_tr.is_contiguous());

        // Kiểm tra đối xứng tọa độ
        for i in 0..2 {
            for j in 0..3 {
                assert_eq!(t.get(&[i, j]), t_tr.get(&[j, i]));
            }
        }
    }

    #[test]
    fn test_tensor_to_contiguous() {
        let t = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
        let t_tr = t.transpose(0, 1);
        let t_contig = t_tr.to_contiguous();
        assert!(t_contig.is_contiguous());
        assert_eq!(t_contig.data, vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0]);
    }

    #[test]
    fn test_tensor_matmul_standard() {
        // [2, 3] x [3, 2] = [2, 2]
        let a = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
        let b = Tensor::new(vec![7.0, 8.0, 9.0, 10.0, 11.0, 12.0], vec![3, 2]);
        let c = a.matmul(&b);
        assert_eq!(c.shape, vec![2, 2]);
        assert_eq!(c.data, vec![58.0, 64.0, 139.0, 154.0]);
    }

    #[test]
    fn test_tensor_matmul_transposed_operand() {
        let a = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3]);
        let b_orig = Tensor::new(vec![7.0, 9.0, 11.0, 8.0, 10.0, 12.0], vec![2, 3]);
        let b = b_orig.transpose(0, 1); // shape [3, 2]
        let c = a.matmul(&b);
        assert_eq!(c.shape, vec![2, 2]);
        assert_eq!(c.data, vec![58.0, 64.0, 139.0, 154.0]);
    }

    #[test]
    fn test_tensor_log_softmax_numerical_stability() {
        // Logit cực lớn không được gây ra tràn số NaN hay Inf
        let logits = Tensor::new(vec![1000.0, 1001.0, 1002.0], vec![1, 3]);
        let ls = logits.log_softmax();
        for &val in &ls.data {
            assert!(val.is_finite());
            assert!(val <= 0.0);
        }

        // Tổng xác suất exp(log_p) phải xấp xỉ bằng 1.0
        let sum_p: f64 = ls.data.iter().map(|v| v.exp()).sum();
        assert!((sum_p - 1.0).abs() < 1e-12);
    }

    #[test]
    fn test_tensor_cross_entropy_masked() {
        let logits = Tensor::new(vec![2.0, 1.0, 0.1, 0.5, 3.0, 0.2], vec![2, 3]);
        let log_p = logits.log_softmax();
        let targets = vec![0i64, -100i64];
        let (loss, valid_count) = log_p.cross_entropy_masked(&targets, -100);
        assert_eq!(valid_count, 1);
        let expected_loss = -log_p.get(&[0, 0]);
        assert!((loss - expected_loss).abs() < 1e-12);
    }

    #[test]
    fn test_tensor_cross_entropy_all_masked() {
        let log_p = Tensor::new(vec![0.0, 0.0], vec![1, 2]);
        let targets = vec![-100i64];
        let (loss, valid_count) = log_p.cross_entropy_masked(&targets, -100);
        assert_eq!(valid_count, 0);
        assert_eq!(loss, 0.0);
    }

    #[test]
    fn test_autograd_add_backward() {
        let mut tape = AutogradTape::new();
        let a = tape.leaf(Tensor::new(vec![2.0, 3.0], vec![2]));
        let b = tape.leaf(Tensor::new(vec![4.0, 5.0], vec![2]));
        let c = tape.add(a, b);
        let s = tape.sum(c);
        tape.backward(s);

        assert_eq!(tape.grad(a).unwrap().data, vec![1.0, 1.0]);
        assert_eq!(tape.grad(b).unwrap().data, vec![1.0, 1.0]);
    }

    #[test]
    fn test_autograd_sub_backward() {
        let mut tape = AutogradTape::new();
        let a = tape.leaf(Tensor::new(vec![10.0, 20.0], vec![2]));
        let b = tape.leaf(Tensor::new(vec![3.0, 7.0], vec![2]));
        let c = tape.sub(a, b);
        let s = tape.sum(c);
        tape.backward(s);

        assert_eq!(tape.grad(a).unwrap().data, vec![1.0, 1.0]);
        assert_eq!(tape.grad(b).unwrap().data, vec![-1.0, -1.0]);
    }

    #[test]
    fn test_autograd_mul_elementwise_backward() {
        let mut tape = AutogradTape::new();
        let a = tape.leaf(Tensor::new(vec![2.0, 4.0], vec![2]));
        let b = tape.leaf(Tensor::new(vec![3.0, 5.0], vec![2]));
        let c = tape.mul(a, b);
        let s = tape.sum(c);
        tape.backward(s);

        assert_eq!(tape.grad(a).unwrap().data, vec![3.0, 5.0]);
        assert_eq!(tape.grad(b).unwrap().data, vec![2.0, 4.0]);
    }

    #[test]
    fn test_autograd_relu_backward() {
        let mut tape = AutogradTape::new();
        let x = tape.leaf(Tensor::new(vec![-2.0, 0.0, 3.0], vec![3]));
        let y = tape.relu(x);
        let s = tape.sum(y);
        tape.backward(s);

        assert_eq!(tape.grad(x).unwrap().data, vec![0.0, 0.0, 1.0]);
    }

    #[test]
    fn test_autograd_scale_backward() {
        let mut tape = AutogradTape::new();
        let x = tape.leaf(Tensor::new(vec![1.5, 2.5], vec![2]));
        let y = tape.scale(x, 4.0);
        let s = tape.sum(y);
        tape.backward(s);

        assert_eq!(tape.grad(x).unwrap().data, vec![4.0, 4.0]);
    }

    #[test]
    fn test_autograd_matmul_backward() {
        let a_val = Tensor::new(vec![1.0, 2.0, 3.0, 4.0], vec![2, 2]);
        let b_val = Tensor::new(vec![5.0, 6.0, 7.0, 8.0], vec![2, 2]);

        let mut tape = AutogradTape::new();
        let a = tape.leaf(a_val.clone());
        let b = tape.leaf(b_val.clone());
        let c = tape.matmul(a, b);
        let s = tape.sum(c);
        tape.backward(s);

        let grad_a = tape.grad(a).unwrap().clone();
        let f_a = |t: &Tensor| -> f64 {
            let mut tp = AutogradTape::new();
            let ai = tp.leaf(t.clone());
            let bi = tp.leaf(b_val.clone());
            let ci = tp.matmul(ai, bi);
            let si = tp.sum(ci);
            tp.value(si).data[0]
        };
        if let Err((idx, ana, num, diff)) =
            check_grad_central_diff(f_a, &a_val, &grad_a, 1e-5, 1e-8)
        {
            panic!(
                "Lỗi sai phân tại {}: giải tích={}, số học={}, sai số={}",
                idx, ana, num, diff
            );
        }
    }

    #[test]
    fn test_autograd_rmsnorm_backward_central_diff() {
        let x_val = Tensor::new(vec![1.2, -0.7, 0.5, 2.1], vec![1, 4]);
        let w_val = Tensor::new(vec![0.8, 1.1, 0.9, 1.0], vec![4]);

        let mut tape = AutogradTape::new();
        let x = tape.leaf(x_val.clone());
        let w = tape.leaf(w_val.clone());
        let y = tape.rmsnorm(x, w, 1e-5);
        let s = tape.sum(y);
        tape.backward(s);

        let grad_x = tape.grad(x).unwrap().clone();
        let f_x = |xt: &Tensor| -> f64 {
            let mut tp = AutogradTape::new();
            let xi = tp.leaf(xt.clone());
            let wi = tp.leaf(w_val.clone());
            let yi = tp.rmsnorm(xi, wi, 1e-5);
            let si = tp.sum(yi);
            tp.value(si).data[0]
        };
        assert!(check_grad_central_diff(f_x, &x_val, &grad_x, 1e-6, 1e-8).is_ok());
    }

    #[test]
    fn test_autograd_cross_entropy_masked_gradient_zero_at_ignore() {
        let logits = Tensor::new(vec![2.0, 1.0, 0.5, 3.0], vec![2, 2]);
        let targets = vec![0i64, -100i64];

        let mut tape = AutogradTape::new();
        let l_id = tape.leaf(logits);
        let lp_id = tape.log_softmax(l_id);
        let loss_id = tape.cross_entropy(lp_id, targets, -100);
        tape.backward(loss_id);

        let grad = tape.grad(l_id).unwrap();
        // Hàng 1 (bị masked với -100) phải có gradient bằng đúng 0.0
        assert_eq!(grad.get(&[1, 0]), 0.0);
        assert_eq!(grad.get(&[1, 1]), 0.0);
        // Hàng 0 không masked phải có gradient khác 0.0
        assert!(grad.get(&[0, 0]) != 0.0);
    }

    #[test]
    fn test_autograd_cross_entropy_grad_wrt_log_probs_central_diff() {
        // Nút CrossEntropy nhận LOG-XÁC SUẤT: dL/dlogP[r, c] = -1_{c == target} / valid_count.
        let log_p_val = Tensor::new(vec![-0.4, -1.1, -2.3, -0.9, -0.6, -1.7], vec![2, 3]);
        let targets = vec![2i64, -100i64];

        let mut tape = AutogradTape::new();
        let lp = tape.leaf(log_p_val.clone());
        let loss = tape.cross_entropy(lp, targets.clone(), -100);
        tape.backward(loss);
        let grad = tape.grad(lp).unwrap().clone();
        assert_eq!(grad.data, vec![0.0, 0.0, -1.0, 0.0, 0.0, 0.0]);

        let f = |t: &Tensor| -> f64 { t.cross_entropy_masked(&targets, -100).0 };
        assert!(check_grad_central_diff(f, &log_p_val, &grad, 1e-6, 1e-8).is_ok());
    }

    #[test]
    fn test_autograd_diamond_dag_accumulation() {
        // Đồ thị kim cương: s = (a * 2) + (a * 3) = a * 5 => ds/da = 5.0
        let a_val = Tensor::new(vec![3.0], vec![1]);
        let mut tape = AutogradTape::new();
        let a = tape.leaf(a_val);
        let left = tape.scale(a, 2.0);
        let right = tape.scale(a, 3.0);
        let sum_node = tape.add(left, right);
        tape.backward(sum_node);

        let grad_a = tape.grad(a).unwrap().data[0];
        assert!((grad_a - 5.0).abs() < 1e-12);
    }

    #[test]
    fn test_bpe_dropout_reconstruction() {
        let corpus = "học lập trình Rust cùng các chuyên gia hàng đầu.";
        let tokenizer = BpeTokenizer::train(corpus, 20);
        let mut rng = SplitMix64::new(101);

        let text = "học lập trình Rust";
        for p in [0.0, 0.2, 0.5, 0.8] {
            let encoded = tokenizer.encode_with_dropout(text, p, &mut rng);
            let decoded = tokenizer.decode(&encoded).expect("Phải giải mã thành công");
            assert_eq!(
                decoded, text,
                "Giải mã phải khớp văn bản gốc ở dropout p={}",
                p
            );
        }
    }

    #[test]
    fn test_tensor_slice_2d_zero_copy() {
        let mut data = Vec::new();
        for i in 0..12 {
            data.push(i as f64);
        }
        let t = Tensor::new(data, vec![3, 4]);

        let sub = t.slice_2d((1, 3), (1, 3));
        assert_eq!(sub.shape, vec![2, 2]);
        assert_eq!(sub.get(&[0, 0]), 5.0);
        assert_eq!(sub.get(&[0, 1]), 6.0);
        assert_eq!(sub.get(&[1, 0]), 9.0);
        assert_eq!(sub.get(&[1, 1]), 10.0);
    }

    #[test]
    fn test_layernorm_analytic_grad_matches_finite_diff() {
        let x = vec![0.8, -1.2, 2.5, 0.1];
        let gamma = vec![1.2, 0.9, 1.1, 0.8];
        let beta = vec![0.1, -0.2, 0.05, 0.3];
        let upstream = vec![1.0, 1.0, 1.0, 1.0];
        let eps = 1e-5;

        let (_, dx, dgamma, _) = layernorm_forward_backward(&x, &gamma, &beta, eps, &upstream);

        let h = 1e-6;
        let mut x_plus = x.clone();
        x_plus[0] += h;
        let (y_plus, _, _, _) = layernorm_forward_backward(&x_plus, &gamma, &beta, eps, &upstream);
        let sum_plus: f64 = y_plus.iter().sum();

        let mut x_minus = x.clone();
        x_minus[0] -= h;
        let (y_minus, _, _, _) =
            layernorm_forward_backward(&x_minus, &gamma, &beta, eps, &upstream);
        let sum_minus: f64 = y_minus.iter().sum();

        let num_dx0 = (sum_plus - sum_minus) / (2.0 * h);
        let diff = (dx[0] - num_dx0).abs();
        assert!(
            diff < 1e-8,
            "Sai số giải tích LayerNorm cho x[0] ({}) phải < 1e-8",
            diff
        );

        let mut gamma_plus = gamma.clone();
        gamma_plus[0] += h;
        let (y_gplus, _, _, _) = layernorm_forward_backward(&x, &gamma_plus, &beta, eps, &upstream);
        let sum_gplus: f64 = y_gplus.iter().sum();

        let mut gamma_minus = gamma.clone();
        gamma_minus[0] -= h;
        let (y_gminus, _, _, _) =
            layernorm_forward_backward(&x, &gamma_minus, &beta, eps, &upstream);
        let sum_gminus: f64 = y_gminus.iter().sum();

        let num_dgamma0 = (sum_gplus - sum_gminus) / (2.0 * h);
        let diff_gamma = (dgamma[0] - num_dgamma0).abs();
        assert!(
            diff_gamma < 1e-8,
            "Sai số giải tích LayerNorm cho gamma[0] ({}) phải < 1e-8",
            diff_gamma
        );

        let (_, _, _, dbeta) = layernorm_forward_backward(&x, &gamma, &beta, eps, &upstream);
        let mut beta_plus = beta.clone();
        beta_plus[0] += h;
        let (y_bplus, _, _, _) = layernorm_forward_backward(&x, &gamma, &beta_plus, eps, &upstream);
        let mut beta_minus = beta.clone();
        beta_minus[0] -= h;
        let (y_bminus, _, _, _) =
            layernorm_forward_backward(&x, &gamma, &beta_minus, eps, &upstream);
        let num_dbeta0 = (y_bplus.iter().sum::<f64>() - y_bminus.iter().sum::<f64>()) / (2.0 * h);
        assert!((dbeta[0] - num_dbeta0).abs() < 1e-8);
    }
}
