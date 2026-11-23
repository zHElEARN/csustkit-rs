#!/bin/bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$repo_root"

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$repo_root/target/swift-package}"
# macOS 27 dyld can reject proc-macro dylibs stripped by rustc.
export CARGO_PROFILE_RELEASE_BUILD_OVERRIDE_STRIP=none
export MACOSX_DEPLOYMENT_TARGET=14.0
unset IPHONEOS_DEPLOYMENT_TARGET

stage="$(mktemp -d)"
trap 'rm -rf "$stage"' EXIT
mkdir -p "$stage"/{bindings,headers,macos,simulator}

for target in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --locked --release --features uniffi --lib --target "$target"
done
for target in aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios; do
    IPHONEOS_DEPLOYMENT_TARGET=17.0 cargo build --locked --release --features uniffi --lib --target "$target"
done

rust_archive() {
    printf '%s/%s/release/libcsustkit.a\n' "$CARGO_TARGET_DIR" "$1"
}

xcrun lipo -create "$(rust_archive aarch64-apple-darwin)" "$(rust_archive x86_64-apple-darwin)" \
    -output "$stage/macos/libcsustkit.a"
xcrun lipo -create "$(rust_archive aarch64-apple-ios-sim)" "$(rust_archive x86_64-apple-ios)" \
    -output "$stage/simulator/libcsustkit.a"

cargo run --locked --features uniffi-bindgen --bin csustkit-bindgen-swift -- \
    "$(rust_archive aarch64-apple-darwin)" "$stage/bindings" \
    --swift-sources --headers --modulemap --metadata-no-deps \
    --module-name CSUSTKitRustFFI --modulemap-filename module.modulemap

mv "$stage/bindings/CSUSTKitRustFFI.h" "$stage/bindings/module.modulemap" "$stage/headers/"
xcodebuild -create-xcframework \
    -library "$stage/macos/libcsustkit.a" -headers "$stage/headers" \
    -library "$(rust_archive aarch64-apple-ios)" -headers "$stage/headers" \
    -library "$stage/simulator/libcsustkit.a" -headers "$stage/headers" \
    -output "$stage/CSUSTKitRustFFI.xcframework"

mkdir -p swift/Generated swift/Artifacts
rm -rf swift/Artifacts/CSUSTKitRustFFI.xcframework
mv "$stage/CSUSTKitRustFFI.xcframework" swift/Artifacts/
mv "$stage/bindings/CSUSTKitRust.swift" swift/Generated/
