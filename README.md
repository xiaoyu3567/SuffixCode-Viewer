# SuffixCode Viewer

一个用于查看和比较后缀代码文件的 GUI 应用程序。

## Windows 兼容性改进

### 解决的问题
1. **Windows 7 32位兼容性**：现在提供 32位版本，支持 Windows 7 32位系统
2. **Windows 10 闪退问题**：添加了异常处理和错误日志记录
3. **运行时依赖**：使用静态链接，减少对 Visual C++ 运行时的依赖

### 构建版本
- **64位版本**：适用于 Windows 10/11 64位系统
- **32位版本**：适用于 Windows 7/8/10/11 32位系统

### 改进特性
1. **静态链接**：所有依赖都静态链接到可执行文件中
2. **异常处理**：添加了 Windows 异常处理器
3. **错误日志**：程序崩溃时会生成日志文件
4. **优化编译**：使用 LTO 和代码优化

## 使用方法

### 下载
从 GitHub Actions 下载最新版本：
1. 进入 Actions 页面
2. 选择最新的构建
3. 下载对应的可执行文件：
   - `SuffixCode Viewer V0.1 (64-bit).exe` - 64位系统
   - `SuffixCode Viewer V0.1 (32-bit).exe` - 32位系统

### 本地构建
```bash
# 在 macOS/Linux 上构建 Windows 版本
chmod +x build_win.sh
./build_win.sh
```

### 故障排除
如果程序仍然出现问题：

1. **检查日志文件**：
   - `crash_log.txt` - 崩溃日志
   - `panic_log.txt` - panic 日志
   - `startup_error.txt` - 启动错误日志

2. **系统要求**：
   - Windows 7 SP1 或更高版本
   - 至少 4GB RAM（推荐 8GB）
   - 支持 OpenGL 2.1 的显卡

3. **常见问题**：
   - 如果程序闪退，检查是否有防病毒软件阻止
   - 确保有足够的磁盘空间用于日志文件
   - 尝试以管理员身份运行

## 技术细节

### 构建配置
- 使用 MSVC 工具链（更好的 Windows 兼容性）
- 静态链接所有依赖
- 启用 LTO 优化
- 添加异常处理

### 依赖库
- eframe/egui - GUI 框架
- rfd - 文件对话框
- image - 图像处理
- winapi - Windows API 访问

## 许可证
MIT License
