# Windsurf Manager

基于 Tauri + Vue 3 的跨平台桌面应用，用于本地管理 Windsurf 账号：批量导入、额度查询、使用分析。

## 下载

发布版本在 [Releases](https://github.com/liubao-cn/avw/releases) 页面下载：

- **macOS Apple Silicon**（M1/M2/M3/M4）：`*_aarch64.dmg`
- **macOS Intel**：`*_x64.dmg`
- **Windows x64**：`*_x64.msi` 或 `*_x64-setup.exe`
- **Windows ARM64**：`*_arm64.msi` 或 `*_arm64-setup.exe`
- **Linux**：`*.deb`、`*.rpm` 或 `*.AppImage`

## 功能

- Auth1 Token 批量导入
- Devin Session Token 批量导入
- 本地账号缓存与额度刷新
- 切换账号前自动重置 Windsurf 机器码
- 启动时检查 GitHub Releases 新版本并提示更新
- 过期时间剩余天数颜色标记
- 使用分析（按模型、按语言、按日图表）
- Esc 栈式关闭弹窗

## 从源码构建

### 依赖

- [Rust](https://rustup.rs/) (stable)
- [Node.js](https://nodejs.org/) (≥ 20)
- 平台相关依赖见 [Tauri 文档](https://tauri.app/start/prerequisites/)

### 开发模式

```bash
npm install
npm run tauri dev
```

### 发布构建

```bash
npm install
npm run tauri build
```

产物在 `src-tauri/target/release/bundle/` 下。

## 技术栈

- **前端**：Vue 3 + Vite + vue-i18n + echarts
- **后端**：Rust + Tauri 2

## CI/CD

Push 一个形如 `v1.2.3` 的 tag 会自动触发跨平台构建，产物挂到 Draft Release：

- macOS Apple Silicon：`aarch64-apple-darwin`
- macOS Intel：`x86_64-apple-darwin`
- Windows x64：`x86_64-pc-windows-msvc`
- Windows ARM64：`aarch64-pc-windows-msvc`
- Linux x64：`deb`、`rpm`、`AppImage`

```bash
git tag v1.3.0
git push origin v1.3.0
```

## License

MIT
