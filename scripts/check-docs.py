#!/usr/bin/env python3
"""Kiểm tra cấu trúc tài liệu của giáo trình (không cần biên dịch Rust).

    python3 scripts/check-docs.py

Kiểm:
  1. Các chương 1..N liên tục; mỗi chương có crate `chNN` trong workspace và ngược lại.
  2. Mọi khối code fence (``` / ~~~) đều đóng.
  3. Mọi liên kết tương đối trỏ tới tệp tồn tại.
  4. SUMMARY.md liên kết đủ mọi chương và tiêu đề H1 của chương khớp tiêu đề trong SUMMARY.
  5. Mọi tham chiếu "Chương N" trỏ tới một chương hiện có.
  6. Dòng số liệu của README ("N chương · M chủ đề · K crate") khớp số chương và số crate thực.

Thoát mã 1 nếu có lỗi.
"""
import glob
import os
import re
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
errors = []


def err(message):
    errors.append(message)


def read(path):
    with open(os.path.join(ROOT, path), encoding="utf-8") as f:
        return f.read()


def chapter_number(path):
    return int(re.search(r"(\d+)", os.path.basename(path)).group(1))


def prose_and_fences(name, text):
    """Tách phần văn xuôi (không có code); báo lỗi nếu fence không đóng."""
    prose, state = [], None
    for i, line in enumerate(text.split("\n"), 1):
        if state is None:
            m = re.match(r"^\s*(`{3,}|~{3,})(.*)$", line)
            if m:
                state = (m.group(1), i)
            else:
                prose.append(line)
        else:
            fence = state[0]
            if re.match(r"^\s*" + re.escape(fence[0]) + "{" + str(len(fence)) + r",}\s*$", line):
                state = None
    if state:
        err(f"{name}: khối code mở ở dòng {state[1]} không được đóng")
    return "\n".join(prose)


chapter_files = sorted(glob.glob(os.path.join(ROOT, "chuong_*.md")), key=chapter_number)
numbers = [chapter_number(p) for p in chapter_files]
max_chapter = max(numbers, default=0)
expected_numbers = list(range(1, max_chapter + 1))
if not numbers:
    err("không tìm thấy tệp chuong_*.md")
elif numbers != expected_numbers:
    err(f"danh sách chương không liên tục 1..{max_chapter}: "
        f"thiếu {sorted(set(expected_numbers) - set(numbers))}")

# 1. chương <-> crate
members = re.findall(r'"([^"]+)"', read("code/Cargo.toml").split("members")[1].split("]")[0])
for n in numbers:
    if f"ch{n:02d}" not in members and f"ch{n}" not in members:
        err(f"chuong_{n:02d}.md không có crate tương ứng trong code/Cargo.toml")
for m in members:
    mm = re.fullmatch(r"ch(\d+)", m)
    if mm and int(mm.group(1)) not in numbers:
        err(f"crate {m} không có chương tương ứng")
    if not os.path.isdir(os.path.join(ROOT, "code", m)):
        err(f"thành viên workspace {m} không có thư mục")

docs = [os.path.basename(p) for p in chapter_files] + [
    "README.md", "ROADMAP.md", "SUMMARY.md", "GLOSSARY.md", "PHU_LUC_A_FANTASY_LAND.md"]
link = re.compile(r"\[[^\]]*\]\(([^)\s]+)\)")
xref = re.compile(r"\b[Cc]hương\s+(\d{1,3})\b")
for name in docs:
    prose = prose_and_fences(name, read(name))
    # 3. liên kết tương đối
    for target in link.findall(prose):
        if re.match(r"^(https?:|mailto:|#)", target):
            continue
        path = target.partition("#")[0]
        if path and not os.path.exists(os.path.normpath(os.path.join(ROOT, os.path.dirname(name), path))):
            err(f"{name}: liên kết hỏng -> {target}")
    # 5. tham chiếu chéo
    for ref in xref.findall(prose):
        if not 1 <= int(ref) <= max_chapter:
            err(f"{name}: tham chiếu 'Chương {ref}' nằm ngoài 1..{max_chapter}")

# 4. SUMMARY
summary = read("SUMMARY.md")
entries = {int(m.group(2)): m.group(1).strip()
           for m in re.finditer(r"\[([^\]]+)\]\(\.?/?chuong_(\d+)\.md\)", summary)}
for n in numbers:
    if n not in entries:
        err(f"SUMMARY.md không liên kết chuong_{n:02d}.md")
        continue
    h1 = next((l[2:].strip() for l in read(f"chuong_{n:02d}.md" if n < 100 else f"chuong_{n}.md").split("\n")
               if l.startswith("# ")), "")
    if h1 != entries[n]:
        err(f"chương {n}: tiêu đề H1 khác SUMMARY.md")

# 6. số liệu README
m = re.search(r"\*\*(\d+) chương · (\d+) chủ đề · (\d+) crate", read("README.md"))
if m:
    if int(m.group(1)) != len(numbers):
        err(f"README ghi {m.group(1)} chương, thực tế {len(numbers)}")
    if int(m.group(3)) != len(members):
        err(f"README ghi {m.group(3)} crate, thực tế {len(members)}")
else:
    err("README.md không có dòng số liệu 'N chương · M chủ đề · K crate'")

for e in errors:
    print("LỖI:", e)
print(f"Đã kiểm tra {len(chapter_files)} chương, {len(members)} crate: {len(errors)} lỗi.")
sys.exit(1 if errors else 0)
