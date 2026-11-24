#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target/swift-package}"
export MACOSX_DEPLOYMENT_TARGET=14.0

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage"/{bindings,headers}

cargo build --locked --release --features uniffi --lib --target aarch64-apple-darwin
for target in aarch64-apple-ios aarch64-apple-ios-sim; do
    IPHONEOS_DEPLOYMENT_TARGET=17.0 cargo build --locked --release --features uniffi --lib --target "$target"
done

rust_archive() {
    printf '%s/%s/release/libcsustkit.a\n' "$CARGO_TARGET_DIR" "$1"
}

cargo run --locked --features uniffi-bindgen --bin csustkit-bindgen -- \
    generate --language swift --no-format --metadata-no-deps \
    --out-dir "$stage/bindings" "$(rust_archive aarch64-apple-darwin)"

mv "$stage/bindings/CSUSTKitRustFFI.h" "$stage/headers/"
mv "$stage/bindings/CSUSTKitRustFFI.modulemap" "$stage/headers/module.modulemap"
xcodebuild -create-xcframework \
    -library "$(rust_archive aarch64-apple-darwin)" -headers "$stage/headers" \
    -library "$(rust_archive aarch64-apple-ios)" -headers "$stage/headers" \
    -library "$(rust_archive aarch64-apple-ios-sim)" -headers "$stage/headers" \
    -output "$stage/CSUSTKitRustFFI.xcframework"

mkdir -p swift/Generated swift/Artifacts
rm -rf swift/Artifacts/CSUSTKitRustFFI.xcframework
mv "$stage/CSUSTKitRustFFI.xcframework" swift/Artifacts/
mv "$stage/bindings/CSUSTKitRust.swift" swift/Generated/
