#!/usr/bin/env python3
"""Dò các dấu `$` làm hỏng render KaTeX trong chương Markdown.

    python3 scripts/check-katex.py                  # toàn bộ các chương hiện có
    python3 scripts/check-katex.py --chapters 98-105

KaTeX (qua mdbook-katex) coi `$...$` là công thức nội dòng và `$$...$$` là công thức khối.
Một dấu `$` nghĩa đen (shell `$()`, `$IFS`; PHP `$_GET`, `$s`; tiền tệ `$3`; regex `$`)
trong VĂN XUÔI sẽ bị hiểu nhầm là mở công thức, ghép với một `$` ở xa và "nuốt" cả tiêu đề,
bảng, đoạn văn — làm vỡ trang. Mã trong khối ``` ``` ``` được bỏ qua (an toàn).

Script mô phỏng cách ghép cặp của KaTeX (bỏ fenced code, GIỮ inline code vì mdbook-katex
đôi khi vẫn tóm `$` trong `` `...` `` ) và báo mọi span `$...$` "nuốt" qua dòng trống hoặc
tiêu đề/`---`/`|` — tức dấu hiệu một `$` nghĩa đen chưa được escape.

Cách sửa: escape dấu `$` nghĩa đen thành `\$` trong văn xuôi; với `$` trong inline code bị
tóm, viết lại cụm đó (mô tả bằng chữ) hoặc tách khỏi code. Giữ `$` công thức thật nguyên vẹn.

Thoát mã 1 nếu còn span nghi vấn.
"""
import argparse
import bisect
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def chapter_numbers():
    numbers = []
    for path in glob.glob(os.path.join(ROOT, "chuong_*.md")):
        match = re.fullmatch(r"chuong_(\d+)\.md", os.path.basename(path))
        if match:
            numbers.append(int(match.group(1)))
    return sorted(numbers)


def parse_selection(spec):
    if not spec:
        return chapter_numbers()
    out = []
    for part in spec.split(","):
        if "-" in part:
            a, b = part.split("-")
            out.extend(range(int(a), int(b) + 1))
        else:
            out.append(int(part))
    return sorted(set(out))


def mask_fenced(text):
    """Thay nội dung khối ``` ``` ``` bằng dòng rỗng (giữ số dòng)."""
    out, fence = [], None
    for line in text.split("\n"):
        m = re.match(r"^(\s*)(`{3,}|~{3,})", line)
        if fence is not None:
            out.append("")
            if re.match(r"^\s*" + re.escape(fence[0]) + "{" + str(len(fence)) + r",}\s*$", line):
                fence = None
            continue
        if m:
            fence = m.group(2)
            out.append("")
            continue
        out.append(line)
    return "\n".join(out)


def suspect_spans(text):
    text = text.replace(r"\$", "")  # bỏ $ đã escape — đã an toàn
    nl = [0] + [i + 1 for i, c in enumerate(text) if c == "\n"]

    def lineof(i):
        return bisect.bisect_right(nl, i)

    spans, i, n = [], 0, len(text)
    while i < n:
        if text[i] == "$":
            if i + 1 < n and text[i + 1] == "$":
                j = text.find("$$", i + 2)
                if j < 0:
                    spans.append((lineof(i), lineof(i), "(`$$` không đóng)"))
                    break
                i = j + 2
                continue
            j = text.find("$", i + 1)
            if j < 0:
                spans.append((lineof(i), lineof(i), "(`$` không đóng)"))
                break
            seg = text[i + 1:j]
            if "\n\n" in seg or re.search(r"\n\s*(#|---|\|)", seg):
                spans.append((lineof(i), lineof(j), seg.strip()[:50].replace("\n", "\\n")))
            i = j + 1
            continue
        i += 1
    return spans


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--chapters", help="ví dụ 98-105 hoặc 100,104 (mặc định: tất cả)")
    args = ap.parse_args()

    total = 0
    for nmbr in parse_selection(args.chapters):
        p = os.path.join(ROOT, f"chuong_{nmbr:02d}.md" if nmbr < 100 else f"chuong_{nmbr}.md")
        if not os.path.exists(p):
            continue
        spans = suspect_spans(mask_fenced(open(p, encoding="utf-8").read()))
        if spans:
            name = os.path.basename(p)
            for a, b, s in spans:
                print(f"  {name}:{a}  span `$`->dòng {b}: ${s}...")
            total += len(spans)
    print(f"\nTổng span `$` nghi làm vỡ KaTeX: {total}")
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())
