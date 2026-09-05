#!/usr/bin/env bash
# اسکریپت‌های توسعه‌ی زاگرس
set -euo pipefail
cd "$(dirname "$0")/.."
source "$HOME/.cargo/env" 2>/dev/null || true

case "${1:-run}" in
  run)     cargo run -p web ;;
  build)   cargo build ;;
  release) cargo build --release ;;
  check)   cargo check ;;
  test)    cargo test ;;
  fmt)     cargo fmt ;;
  clippy)  cargo clippy ;;
  # پاک‌سازی فضای دیسک (artifactهای اضافی، نگه‌داشتن کش رجیستری)
  clean-disk)
    rm -rf target/debug/incremental target/debug/examples 2>/dev/null || true
    apt-get clean 2>/dev/null || true
    echo "دیسک آزاد:"; df -h / | tail -1
    ;;
  # نمایش وضعیت فضا
  disk)
    df -h / | tail -1
    echo "target: $(du -sh target 2>/dev/null | cut -f1) | registry: $(du -sh "$HOME/.cargo/registry" 2>/dev/null | cut -f1)"
    ;;
  *) echo "استفاده: dev.sh {run|build|release|check|test|fmt|clippy|clean-disk|disk}" ;;
esac
