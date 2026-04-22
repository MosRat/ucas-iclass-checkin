# UCAS iCLASS Check-in Workspace

这是一个按“自底向上”组织的 Rust workspace，用来实现 UCAS iCLASS 的登录、课程查询和打卡能力，并为后续桌面端和移动端 GUI 预留稳定核心层。

## Workspace 结构

- `crates/iclass-domain`
  业务领域模型，只保留真正会被上层使用的数据字段。
- `crates/iclass-api`
  HTTP 请求和接口 DTO 解析，负责把服务端返回转换成领域模型。
- `crates/iclass-session`
  会话持久化、本地凭证管理、自动重新登录和带重试的调用入口。
- `crates/iclass-core`
  登录、查询、选取最合适课程并发起打卡的核心流程。
- `crates/iclass-cli`
  命令行入口，适合本地调试和自动化运行。
- `crates/iclass-gui`
  给未来 Tauri v2 GUI 复用的桥接层，目前提供可序列化的 dashboard / check-in 视图模型。

## 设计要点

- 只提取实际使用到的接口字段，避免把整个服务端 payload 原样污染到上层。
- 使用 `reqwest + rustls`，避免系统 OpenSSL 依赖。
- CLI 使用 `mimalloc` 作为全局分配器。
- `core` 只依赖 `session/domain` 暴露能力，GUI 层后续可以直接调用。
- 支持把 session 和凭证持久化到本地 JSON 文件，便于 CLI 和 GUI 共享。
- 调用需要登录的接口时，会优先复用已有 session；如果识别到疑似 session 失效，会尝试重新登录并重试一次。

## 环境变量

建议在项目根目录放一个 `.env` 文件：

```env
UCAS_ICLASS_ACCOUNT=202528014629003
UCAS_ICLASS_PASSWORD=Ucas@2025
UCAS_ICLASS_BASE_URL=https://iclass.ucas.edu.cn:8181
```

可选：

```env
UCAS_ICLASS_SESSION_PATH=F:\\WorkSpace\\Rust\\ucas-iclass-chechin\\.session.json
RUST_LOG=info
```

仓库中应该提交 `.env.example`，但不要提交真实 `.env`、本地 session 文件、签名材料或任何个人凭证。

## 提交边界

推荐提交到 GitHub 的内容：

- Rust workspace 源码、测试、文档和 `Cargo.lock`
- `apps/iclass-gui/src` 与 `src-tauri/src` 下的手写源码
- Tauri capability/schema 文件
- `src-tauri/gen/android` 下稳定的 Android 工程骨架与手写原生代码
- Git hooks、CI/workflows、示例配置和图标资源

不应提交的内容：

- `target/`、`dist/`、`node_modules/`
- `apps/iclass-gui/src/**/*.js` 这类由 TypeScript 源码旁路生成的编译副产物
- `.env`、真实账号密码、session JSON、日志文件
- Android `keystore.properties`、`local.properties`、keystore 文件与构建输出
- 临时调试文件、模拟器截图、平台本地缓存

目前仓库已经按这个边界做了清理，公开仓库时可以直接沿用。

## GitHub Actions

- `.github/workflows/ci.yml`
  负责 PR / `main` 分支的构建校验，执行前端构建、`cargo test --workspace` 和 `clippy`。
- `.github/workflows/release.yml`
  负责手动触发或 `app-v*` tag 的桌面端与 Android 构建发布。

Android release 若需要签名，请在 GitHub Secrets 中提供：

- `ANDROID_KEYSTORE_BASE64`
- `ANDROID_KEY_ALIAS`
- `ANDROID_KEYSTORE_PASSWORD`

## CLI 用法

登录并保存 session：

```powershell
cargo run -p iclass-cli -- login
```

登录但不保存密码：

```powershell
cargo run -p iclass-cli -- login --remember-password false
```

查看保存的 session：

```powershell
cargo run -p iclass-cli -- session
```

查询指定日期课程：

```powershell
cargo run -p iclass-cli -- today --date 2026-04-23
```

查询学期和课程：

```powershell
cargo run -p iclass-cli -- semesters
cargo run -p iclass-cli -- courses
```

尝试打卡：

```powershell
cargo run -p iclass-cli -- checkin
cargo run -p iclass-cli -- checkin --date 2026-04-23
cargo run -p iclass-cli -- checkin --mode id
```

## 当前验证结果

已使用你提供的 mock 账号验证：

- `login` 成功
- `today --date 2026-04-23` 成功返回课程列表
- `checkin --date 2026-04-23` 返回 `ERRCODE=101` / `二维码已失效！`

这与“测试账户所有打卡都失败”的预期一致。

## Core 测试

默认只跑稳定的单元测试和 mock 集成测试：

```powershell
cargo test -p iclass-core
```

显式启用 live 接口测试：

```powershell
$env:ICLASS_RUN_LIVE_TESTS='1'
$env:UCAS_ICLASS_ACCOUNT='202528014629003'
$env:UCAS_ICLASS_PASSWORD='Ucas@2025'
cargo test -p iclass-core --test live_core -- --nocapture
```

如果还想验证真实打卡失败路径，再额外提供测试日期并启用 check-in live 测试：

```powershell
$env:ICLASS_RUN_LIVE_TESTS='1'
$env:ICLASS_RUN_LIVE_CHECKIN_TESTS='1'
$env:UCAS_ICLASS_ACCOUNT='202528014629003'
$env:UCAS_ICLASS_PASSWORD='Ucas@2025'
$env:UCAS_ICLASS_TEST_DATE='2026-04-23'
cargo test -p iclass-core --test live_core -- --nocapture
```

## 下一步建议

1. 在 `iclass-gui` 基础上接入 Tauri v2 command 层。
2. 为 `session` 增加更安全的凭证存储后端，例如系统钥匙串。
3. 增加定时打卡策略、前后台通知和多策略选课规则。
4. 增加 live integration tests，并通过环境变量控制是否执行。
