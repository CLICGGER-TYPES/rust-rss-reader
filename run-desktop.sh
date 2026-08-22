#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

# 1) 构建前端
echo "==> 构建前端 (apps/desktop/ui)"
npm --prefix apps/desktop/ui run build

# 2) 启动桌面端
echo "==> 启动桌面端"
exec cargo run -p rss-desktop