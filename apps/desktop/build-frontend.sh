#!/usr/bin/env bash
# Tauri 在 build/dev 前调用此脚本构建/启动前端开发服务器。
# 用法：build-frontend.sh [build|dev]
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR/ui"
cmd="${1:-build}"
if [ "$cmd" = "dev" ]; then
  exec npm run dev
else
  exec npm run build
fi