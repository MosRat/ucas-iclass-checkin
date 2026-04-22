# UCAS iCLASS Check-in

一个用于访问 UCAS iCLASS 的 Rust workspace，提供命令行与桌面/移动端图形界面所需的基础能力。

项目当前聚焦于以下内容：

- 登录与会话复用
- 课程与课表查询
- 课程签到入口
- 本地会话与应用设置持久化
- 基于 Tauri v2 的跨平台 GUI 构建基础

## 功能边界

本项目只实现与 iCLASS 交互所需的最小能力，不追求还原服务端全部字段，也不存放与功能无关的用户数据。

项目默认按“本地优先”方式工作：

- 凭证与会话仅保存在当前设备
- 不内置远程转发、云同步或第三方账号系统
- 不收集、不上传个人课表、签到记录或账号信息

## 仓库结构

- `crates/`
  Rust 核心库与 CLI。
- `apps/iclass-gui/`
  Tauri v2 GUI 工程，前端为 Vue 3 + Tailwind CSS。
- `docs/`
  接口整理文档与本地 API 资料。

## 快速开始

### 1. 准备环境

- Rust stable
- Node.js 22+
- pnpm
- Windows / macOS / Linux 其一

如需构建 Android，还需要 Android SDK、NDK、Java 17。

### 2. 配置环境变量

复制 `.env.example` 并填写你自己的账号信息：

```powershell
Copy-Item .env.example .env
```

建议只在本地保存 `.env`，不要提交真实凭证、session 文件、日志或签名材料。

### 3. 运行 CLI

```powershell
cargo run -p iclass-cli -- login
cargo run -p iclass-cli -- courses
cargo run -p iclass-cli -- checkin
```

### 4. 运行 GUI

```powershell
cd apps/iclass-gui
pnpm install
pnpm tauri dev
```

## 构建

### Rust 校验

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

### GUI 构建

```powershell
cd apps/iclass-gui
pnpm build
pnpm tauri build
```

## 隐私与风险提示

使用这类工具前，请先确认你了解并愿意承担以下风险：

- 账号凭证一旦泄露，可能影响你的个人账户安全。
- 第三方客户端可能因接口变化、策略变化或网络异常而失效。
- 自动化签到能力应仅在你明确知情并自行负责的前提下使用。
- 请勿在公共仓库、截图、日志或 issue 中暴露学号、密码、session、二维码数据或签名文件。

如果你计划公开部署或分享构建产物，建议：

- 使用独立测试账号进行验证
- 将签名材料与敏感配置保存在本地或 CI Secret 中
- 在发布前再次检查 `.env`、session 文件、日志与构建缓存是否被忽略

## 发布与 CI

仓库包含 GitHub Actions 工作流，用于：

- 常规构建校验
- 桌面端发布构建
- Android 发布构建

Android 签名请通过 GitHub Secrets 提供，不要将 keystore 或真实配置提交到仓库。

## 说明

本项目仅用于学习、研究与个人设备上的本地使用场景。请遵守你所在组织的相关规定，并自行判断是否适合在真实环境中使用。

许可证：MIT
