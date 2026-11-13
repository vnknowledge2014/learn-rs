#!/usr/bin/env python3
"""Kiểm tra mã Rust nhúng trong các chương Markdown.

Chạy từ gốc repo:

    python3 scripts/check-book-code.py                  # toàn bộ 105 chương
    python3 scripts/check-book-code.py --chapters 86-91 # một dải chương
    python3 scripts/check-book-code.py --chapters 96,97 --verbose

Ba nhóm kiểm tra (đều biên dịch bằng `rustc --edition 2024 --emit=metadata`):

  1. Khối mã dạng mục (fn/struct/impl/...) không có `fn main`, biên dịch như thư viện.
  2. Chương trình hoàn chỉnh (có `fn main`), biên dịch như tệp thực thi.
  3. Mã trong thẻ `<details>` của phần Lời giải, ghép vào mã nguồn crate tương ứng
     (đặt trong module con `sol` với `use super::*`) để kiểm tra khớp API.

Mức độ:
  LỖI     (thoát mã 1): khối bị cắt giữa chừng (ngoặc không đóng/không mở), hàm có
          `self` đứng ngoài `impl`, hoặc lời giải không biên dịch cùng crate.
  XEM LẠI (không đổi mã thoát): lỗi kiểu/mượn/cú pháp khác ở khối không được đánh dấu là
          minh hoạ lỗi; cần người đọc phán đoán.

Một khối cố ý không biên dịch (minh hoạ lỗi, mã giả) nên khai báo ```rust,ignore.
Khối chỉ thiếu tên được định nghĩa ở khối khác (E0425, E0433...) được coi là đoạn trích
hợp lệ và không bị báo.
"""
import argparse
import collections
import concurrent.futures
import os
import re
import subprocess
import sys
import tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

EXT = re.compile(
    r"\b(?:use|extern crate)\s+(tokio|axum|serde|serde_json|syn|quote|proc_macro2|proc_macro|"
    r"tower|hyper|http_body_util|leptos|gpui|bevy|wgpu|tauri|rand|regex|ch\d+\w*)\b"
    r"|#\[(?:tokio::|derive\((?:Serialize|Deserialize))|#\[proc_macro"
)
EXPECT = re.compile(
    r"LỖI|lỗi biên dịch|không (?:thể )?biên dịch|không compile|E0\d{4}|❌|error\[|"
    r"sẽ (?:bị )?(?:báo|từ chối)|không hợp lệ|bỏ chú thích|ĐOẠN MÃ GÂY|"
    r"trình biên dịch (?:chặn|từ chối|báo)|borrow checker|Borrow Checker|buggy|BUG",
    re.I,
)
ITEM = re.compile(
    r"^\s*(?:pub(?:\([a-z]+\))?\s+)?(?:fn|struct|enum|trait|impl|mod|const|static|type|use|"
    r"unsafe\s+(?:fn|impl)|macro_rules!|#\[)",
    re.M,
)
SOLUTION = re.compile(r"Lời giải|lời giải|Đáp án")
NAME_ERRORS = {"E0425", "E0412", "E0433", "E0432", "E0405", "E0422", "E0423", "E0531", "E0574", "E0407", "E0576"}
DUP_ERRORS = {"E0592", "E0428", "E0119", "E0201", "E0124"}
TRUNCATED = re.compile(
    r"unclosed delimiter|unexpected closing delimiter|expected item after doc comment|"
    r"unterminated double quote"
)
METHOD_OUTSIDE_IMPL = re.compile(r"`self` parameter is only allowed|cannot find type `Self` in this scope")
STMT_SNIPPET = re.compile(r"expected item, found keyword `(?:let|if|for|while|match|loop|return)`")
EXTERNAL_CRATE_CHAPTERS = {23, 24, 61}


def chapter_path(n):
    return os.path.join(ROOT, f"chuong_{n:02d}.md" if n < 100 else f"chuong_{n}.md")


def crate_source(n):
    d = os.path.join(ROOT, "code", f"ch{n:02d}" if n < 100 else f"ch{n}", "src")
    for name in ("main.rs", "lib.rs"):
        p = os.path.join(d, name)
        if os.path.exists(p):
            return open(p, encoding="utf-8").read()
    return None


def parse_selection(spec):
    if not spec:
        return list(range(1, 106))
    out = []
    for part in spec.split(","):
        if "-" in part:
            a, b = part.split("-")
            out.extend(range(int(a), int(b) + 1))
        else:
            out.append(int(part))
    return sorted(set(out))


def solution_ranges(lines):
    ranges, start = [], None
    for i, line in enumerate(lines, 1):
        if start is None and "<details" in line:
            start = i
        if start is not None and "</details>" in line:
            if SOLUTION.search("\n".join(lines[start - 1:i])):
                ranges.append((start, i))
            start = None
    return ranges


def rust_blocks(path):
    """Trả về danh sách (dòng mở, cờ, thân, ngữ cảnh 3 dòng trước, id nhóm lời giải hoặc None)."""
    lines = open(path, encoding="utf-8").read().split("\n")
    sol = solution_ranges(lines)
    blocks, state = [], None
    for i, line in enumerate(lines, 1):
        if state is None:
            m = re.match(r"^\s*(`{3,}|~{3,})\s*([^\s`]*)(.*)$", line)
            if m:
                state = (m.group(1), m.group(2).lower(), (m.group(3) or "").lower(), i, [])
        else:
            fence = state[0]
            if re.match(r"^\s*" + re.escape(fence[0]) + "{" + str(len(fence)) + r",}\s*$", line):
                if state[1].startswith("rust"):
                    start = state[3]
                    group = next((k for k, (a, b) in enumerate(sol) if a <= start <= b), None)
                    ctx = "\n".join(lines[max(0, start - 4):start - 1])
                    blocks.append((start, state[1] + state[2], "\n".join(state[4]), ctx, group))
                state = None
            else:
                state[4].append(line)
    return blocks


def rustc(source, crate_type, workdir, tag):
    path = os.path.join(workdir, f"{tag}.rs")
    with open(path, "w", encoding="utf-8") as f:
        f.write(source)
    try:
        r = subprocess.run(
            ["rustc", "--edition", "2024", "--crate-type", crate_type, "--emit=metadata",
             "--cap-lints", "allow", "--out-dir", workdir, path],
            capture_output=True, text=True, timeout=180,
        )
    except subprocess.TimeoutExpired:
        return 1, [], ["error: hết thời gian biên dịch"]
    codes = re.findall(r"^error\[(E\d{4})\]", r.stderr, re.M)
    msgs = [l for l in r.stderr.split("\n") if l.startswith("error")]
    return r.returncode, codes, msgs


def classify(rc, codes, msgs):
    if rc == 0:
        return "ok"
    text = " ".join(msgs)
    if TRUNCATED.search(text):
        return "truncated"
    if METHOD_OUTSIDE_IMPL.search(text):
        return "method"
    if codes and all(c in NAME_ERRORS for c in codes):
        return "name"
    if not codes and STMT_SNIPPET.search(text):
        return "stmt"
    if not codes:
        return "syntax"
    return "other"


def check_block(job):
    chapter, start, flags, body, ctx, workdir = job
    tag = f"c{chapter}_l{start}"
    if re.search(r"\bfn main\s*\(", body):
        rc, codes, msgs = rustc(body, "bin", workdir, tag)
        kind = "program"
    else:
        rc, codes, msgs = rustc("#![allow(unused, dead_code)]\n" + body, "lib", workdir, tag)
        kind = "fragment"
    cls = classify(rc, codes, msgs)
    expected = bool(EXPECT.search(body + "\n" + ctx)) or "compile_fail" in flags
    return dict(chapter=chapter, line=start, kind=kind, cls=cls, expected=expected,
                msg=(msgs[0] if msgs else ""), codes=sorted(set(codes)))


def check_solution_group(job):
    chapter, blocks, workdir = job  # blocks: [(line, body)]
    src = crate_source(chapter)
    if src is None:
        return []
    prefix = "#![allow(unused, dead_code, unused_imports)]\n" + src + "\nmod sol {\nuse super::*;\n"
    tag = f"s{chapter}_l{blocks[0][0]}"
    rc, _, _ = rustc(prefix + "\n".join(b for _, b in blocks) + "\n}\n", "lib", workdir, tag)
    if rc == 0:
        return []
    results = []
    for line, body in blocks:
        rc, codes, msgs = rustc(prefix + body + "\n}\n", "lib", workdir, f"{tag}_{line}")
        if rc == 0:
            continue
        cls = classify(rc, codes, msgs)
        if codes and all(c in DUP_ERRORS for c in codes):
            cls = "dup"
        results.append(dict(chapter=chapter, line=line, kind="solution", cls=cls, expected=False,
                            msg=(msgs[0] if msgs else ""), codes=sorted(set(codes))))
    return results


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--chapters", help="ví dụ 86-91 hoặc 96,97 (mặc định: tất cả)")
    ap.add_argument("--jobs", type=int, default=4, help="số rustc chạy song song (mặc định 4)")
    ap.add_argument("--no-solutions", action="store_true", help="bỏ qua kiểm tra lời giải (nhanh hơn)")
    ap.add_argument("--verbose", action="store_true", help="in cả các khối XEM LẠI loại nhẹ")
    args = ap.parse_args()

    chapters = parse_selection(args.chapters)
    results, block_jobs, sol_jobs = [], [], []
    with tempfile.TemporaryDirectory() as workdir:
        for n in chapters:
            path = chapter_path(n)
            if not os.path.exists(path):
                continue
            groups = collections.defaultdict(list)
            for start, flags, body, ctx, group in rust_blocks(path):
                if "ignore" in flags or EXT.search(body):
                    continue
                if group is not None:
                    if not args.no_solutions and n not in EXTERNAL_CRATE_CHAPTERS:
                        groups[group].append((start, body))
                    continue
                if not re.search(r"\bfn main\s*\(", body) and not ITEM.search(body):
                    continue  # đoạn câu lệnh rời, không phải mục cấp module
                block_jobs.append((n, start, flags, body, ctx, workdir))
            for blocks in groups.values():
                sol_jobs.append((n, blocks, workdir))

        with concurrent.futures.ThreadPoolExecutor(args.jobs) as ex:
            results.extend(ex.map(check_block, block_jobs))
            for rs in ex.map(check_solution_group, sol_jobs):
                results.extend(rs)

    errors, review = [], []
    for r in results:
        if r["cls"] in ("ok", "name", "dup"):
            continue
        if r["cls"] in ("truncated", "method") or r["kind"] == "solution":
            errors.append(r)
        elif r["cls"] == "stmt":
            if args.verbose:
                review.append(r)
        elif not r["expected"]:
            review.append(r)

    labels = {"truncated": "bị cắt giữa chừng", "method": "hàm self ngoài impl", "syntax": "lỗi cú pháp",
              "other": "lỗi biên dịch", "stmt": "câu lệnh rời", "name": "thiếu tên"}

    def show(r):
        name = f"chuong_{r['chapter']:02d}.md" if r["chapter"] < 100 else f"chuong_{r['chapter']}.md"
        print(f"  {name + ':' + str(r['line']):<22} [{r['kind']}] {labels.get(r['cls'], r['cls'])} "
              f"{','.join(r['codes'])} {r['msg'][:96]}")

    print(f"Đã kiểm tra {len(chapters)} chương: {len(block_jobs)} khối mã, {len(sol_jobs)} nhóm lời giải.")
    if errors:
        print(f"\nLỖI ({len(errors)}):")
        for r in sorted(errors, key=lambda r: (r["chapter"], r["line"])):
            show(r)
    if review:
        print(f"\nXEM LẠI ({len(review)}):")
        for r in sorted(review, key=lambda r: (r["chapter"], r["line"])):
            show(r)
    if not errors:
        print("\nKhông có LỖI.")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
