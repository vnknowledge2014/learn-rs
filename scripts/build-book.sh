#!/usr/bin/env bash
# Dựng sách mdBook từ một thư mục tạm chỉ chứa các tệp Markdown và book.toml.
#
# Lý do: book.toml đặt `src = "."` để các chương nằm ở gốc repo, nhưng mdBook chép mọi tệp
# không phải .md trong `src` vào kết quả — kể cả code/ (và code/target), .git, .claude...
# Dựng từ thư mục tạm giữ cho `book/` chỉ có sách.
#
#   scripts/build-book.sh     # xuất HTML tĩnh vào book/
#
# Cảnh báo của mdBook (ví dụ thẻ HTML trần trong văn xuôi bị trình duyệt ẩn mất) được coi là
# lỗi, trừ cảnh báo về cỡ chỉ mục tìm kiếm.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STAGE="$(mktemp -d)"
trap 'rm -rf "$STAGE"' EXIT

cp "$ROOT"/*.md "$ROOT/book.toml" "$STAGE"/
LOG="$STAGE/build.log"

mdbook build "$STAGE" -d "$ROOT/book" 2>&1 | tee "$LOG"

if grep -E '\bWARN\b' "$LOG" | grep -v -i 'search index is very large' | grep -q .; then
  echo "mdBook có cảnh báo (xem ở trên)." >&2
  exit 1
fi
