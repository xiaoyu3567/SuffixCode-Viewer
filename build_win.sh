#!/bin/bash

# Windows 构建脚本 - 支持 32位和 64位
set -e

echo "开始构建 Windows 版本..."

# 检查 Rust 是否安装
if ! command -v cargo &> /dev/null; then
    echo "错误: 未找到 cargo，请先安装 Rust"
    exit 1
fi

# 添加目标平台
echo "添加目标平台..."
rustup target add x86_64-pc-windows-msvc
rustup target add i686-pc-windows-msvc

# 构建 64位版本
echo "构建 64位版本..."
cd egui_txt_viewer
RUSTFLAGS="-Ctarget-feature=+crt-static" cargo build --release --target x86_64-pc-windows-msvc

# 构建 32位版本
echo "构建 32位版本..."
RUSTFLAGS="-Ctarget-feature=+crt-static" cargo build --release --target i686-pc-windows-msvc

# 重命名文件
echo "重命名文件..."
cd target/x86_64-pc-windows-msvc/release/
if [ -f "egui_txt_viewer.exe" ]; then
    mv egui_txt_viewer.exe "SuffixCode Viewer V0.1 (64-bit).exe"
fi

cd ../../i686-pc-windows-msvc/release/
if [ -f "egui_txt_viewer.exe" ]; then
    mv egui_txt_viewer.exe "SuffixCode Viewer V0.1 (32-bit).exe"
fi

echo "构建完成！"
echo "64位版本: egui_txt_viewer/target/x86_64-pc-windows-msvc/release/SuffixCode Viewer V0.1 (64-bit).exe"
echo "32位版本: egui_txt_viewer/target/i686-pc-windows-msvc/release/SuffixCode Viewer V0.1 (32-bit).exe"

