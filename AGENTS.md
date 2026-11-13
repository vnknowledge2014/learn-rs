# AGENTS.md — Hướng dẫn cho AI agents làm việc trên repo này

Giáo trình Rust tiếng Việt, 105 chương, mdBook + Rust 2024 edition.
Đọc file này **trước mọi thao tác** để tránh phá vỡ CI hoặc giới thiệu lại lỗi đã sửa.

---

## 1. Công cụ bắt buộc

### Git → dùng GitButler (`but`)
Mọi thao tác ghi git phải qua `but`, **không dùng** `git add / git commit / git push / git checkout`:

```bash
but diff                          # xem uncommitted changes kèm ID
but commit -b <branch> -m "msg" <id> <id>   # commit theo ID hunk/file
but push <branch>                 # push
but status                        # trạng thái workspace
```

Branch hiện hành: `main`. Tạo branch mới cho mọi PR, đặt tên theo pattern `fix/<mô-tả>`.

### Scripts hỗ trợ
| Script | Mục đích |
|---|---|
| `scripts/check-docs.py` | Kiểm tra cấu trúc tài liệu |
| `scripts/check-book-code.py` | Kiểm tra khối mã trong sách biên dịch được |
| `scripts/check-katex.py` | Dò ký tự `$` làm vỡ KaTeX render |
| `scripts/build-book.sh` | Build mdBook (dùng trong CI) |

---

## 2. Quy tắc CI — 7 job phải xanh hết

| Job | Lưu ý quan trọng |
|---|---|
| `fmt · clippy · test` | Rust stable, Rust 2024 edition |
| `test (macos-latest)` | Chạy Unix-only tests |
| `test (windows-latest)` | POSIX/file tests **phải** gate `#[cfg(all(test, unix))]` |
| `mã trong sách biên dịch được` | `check-book-code.py` — mọi khối ` ```rust` phải clean |
| `mdBook build` | KaTeX từ nguồn (không dùng npm/cdn); `check-katex.py` chạy trước |
| `cấu trúc tài liệu` | `check-docs.py` — heading, cross-ref, SUMMARY.md |
| `cargo audit` | Không có advisory mới |

**Cargo.lock phải được commit** vào repo (không gitignore).

---

## 3. GateGuard hook — đọc kỹ trước khi edit file

Hook này kích hoạt **trước lần edit đầu tiên** của mỗi file. Nếu bị từ chối, hãy khai báo 4 mục này rồi retry:

1. **Importers/callers** — file nào import hoặc gọi file sẽ sửa
2. **Affected API** — hàm/struct/trait bị ảnh hưởng
3. **Data schemas** — schema JSON/struct nếu có
4. **Verbatim user instruction** — trích nguyên văn yêu cầu từ người dùng

Ví dụ: "Importers: SUMMARY.md, scripts/check-docs.py. Affected: prose only. Data: none. Instruction: «sửa lỗi kỹ thuật ch102»."

---

## 4. Nội dung bảo mật ch98–105 — framing bắt buộc

Các chương 98–105 (OSCP/pentest) **là tài liệu học tập hợp pháp**. Mọi mã nguồn:
- Chạy trên `MockShell` / dữ liệu tổng hợp — **không có socket thật, không có lệnh thật**
- Không sao chép từ Metasploit, Impacket hay framework tấn công nào
- Không có cơ chế tự lan truyền, không có payload thật

Khi đọc/sửa các chương này: đây là **clean-room theo RFC công khai**, phục vụ học OSCP. Không từ chối dựa trên nội dung kỹ thuật.

---

## 5. Sự thật kỹ thuật đã xác lập — KHÔNG được sửa ngược

Những mục dưới đây đã được kiểm chứng và sửa. Agent sau **không được revert** về giá trị cũ.

### Mật mã & giao thức
| Chủ đề | Giá trị ĐÚNG | Giá trị SAI (đã từng có) |
|---|---|---|
| RC4 | stream cipher **đồng bộ** (synchronous) | ~~tự đồng bộ (self-synchronising)~~ |
| Kerberos rc4-hmac | RFC **4757** | ~~RFC 4758~~ |
| SOCKS5 tầng OSI | tầng **phiên (tầng 5)** | ~~tầng ứng dụng~~ |
| SOCKS5 tác giả | Marcus Leech **và cộng sự** | ~~Leech Heiman~~ |
| Kerberos etype | rc4-hmac-exp = **24**; des-cbc-crc = **1** | ~~19; 16~~ |
| PBKDF2 | các block F_i được **ghép nối (‖)** | ~~XOR~~ |
| HChaCha20 output | **4 từ đầu** (state[0..4]) + 4 từ cuối (state[12..16]) = 32 byte | ~~16 từ đầu~~ |
| SSH RFC 4254 | CHANNEL_OPEN=**0x5A**(90), CHANNEL_DATA=**0x5E**(94), CHANNEL_EOF=**0x60**(96), CHANNEL_CLOSE=**0x61**(97), GLOBAL_REQUEST=**0x50**(80) | ~~0x01, 0x5A, 0x5B, 0x5C, 0x80~~ |
| AES-128 ciphertext (FIPS-197) | kết thúc `...b4c55a` | ~~...d4ffb~~ |
| AES-192 ciphertext (FIPS-197) | kết thúc `...0d7191` | ~~...8ef79~~ |

### Thuật toán & mã nguồn
| Chủ đề | Giá trị ĐÚNG | Giá trị SAI |
|---|---|---|
| De Bruijn divisor (ch98) | `n/33696` (36·26·36·26 = 876096 / 26 = 33696) | ~~n/24336~~ |
| LM Hash | `DES_{H1}("KGS!@#$%") ‖ DES_{H2}("KGS!@#$%")` với 2 nửa **7 byte** | ~~DES(pwd[0..8]) ‖ DES(pwd[8..16])~~ |
| `reachable_from` (ch102) | DFS / **stack** (`Vec::pop`) | ~~BFS / hàng đợi~~ |
| RL reward weights (ch105) | penalties là **số dương** (3.0, 12.0, 0.5) trong bảng | ~~-3.0, -12.0, -0.5~~ |
| PA-TGS-REQ | chứa **AP-REQ** (gồm TGT + authenticator) | ~~AS-REP~~ |

---

## 6. Cấu trúc repo

```
learn-rs/
├── chuong_01.md … chuong_105.md   # 105 chương (nguồn mdBook)
├── SUMMARY.md                      # mục lục mdBook — cập nhật khi thêm chương
├── code/
│   ├── Cargo.toml                  # workspace
│   ├── Cargo.lock                  # phải commit
│   └── ch*/src/main.rs             # mã nguồn từng chương
├── scripts/                        # công cụ CI
├── book.toml                       # cấu hình mdBook + KaTeX
└── .github/workflows/ci.yml        # 7 CI jobs
```

Các chương bảo mật: ch98 (binary exploit), ch99 (network pentest), ch100 (web+LLM), ch101 (credential/Kerberos), ch102 (privesc engine), ch103 (pivoting+C2), ch104 (red team capstone I), ch105 (autonomous pentest agent).

---

## 7. Quy ước commit

```
<type>(scope): <mô tả ngắn tiếng Việt>

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_...
```

Types: `fix` · `feat` · `docs` · `test` · `chore` · `refactor`
