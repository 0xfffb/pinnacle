# Pinnacle

基于 [Pingora](https://github.com/cloudflare/pingora) 的反爬 / 人机验证网关（Rust Cargo workspace）。

> **状态：开发预览（unstable）**  
> API、配置与行为随时可能破坏性变更。可用于本地试用与开发，**请勿用于生产环境**。

## 它做什么

在反向代理前面做人机验证：未通过时返回挑战页；验证通过后签发 pass cookie，后续请求校验通过再转发到 upstream。

```text
Client ──► Pingora Gateway ──► Turnstile (tower 中间件栈) ──► upstream
                 │                      │
                 │              Decision: None = 放行
                 │                       Some(Response) = 直接响应
                 └─ Session ⇄ http::Request / Response
```

## 架构

| Crate | 路径 | 作用 |
|-------|------|------|
| `pinnacle-core` | `crates/core` | `http` 类型、`Decision`、`Respond`、axum 风格 `from_fn` / `Stack` |
| `pinnacle-store` | `crates/store` | `MemoryStore`（ban / pass / challenge session） |
| `pinnacle-turnstile` | `crates/turnstile` | 中间件组装与 Cookie challenge |
| `pinnacle-gateway` | `crates/gateway` | Pingora `Session` ↔ `http::Request` / `Response` |
| `pinnacle-cli` | `crates/cli` | 二进制入口与 TOML 配置 |

**流程（中间件）** 与 **工具（state）** 分开：只有会短路或放行的逻辑进 `Stack`；规则引擎、检测器等能力应挂在 `TurnstileState` 上供层调用，而不是再叠一层「假中间件」。

当前栈（外 → 内）：

```text
[1] cookie    Cookie 挑战 / pass 校验
[2] captcha   占位（/__captcha）
[3] banned    封禁 IP 拦截
[+] default   None → 转发 upstream
```

## Cookie 挑战

路径：`/__pinnacle`（`GET` 脚本，`POST` 校验）。

| 条件 | 行为 |
|------|------|
| `GET /__pinnacle` | 返回指纹脚本；可附带 `__pinnacle_cid` |
| `POST /__pinnacle` | 校验指纹报告；成功则 `Set-Cookie: __pinnacle_pass` |
| 无有效 `__pinnacle_pass` | `503` 挑战页 + `__pinnacle_cid` |
| 持有有效 pass | 进入内层（最终可转发 upstream） |
| IP 已被 ban | `403` |

## 配置

默认读取 `pinnacle.toml`：

```toml
listen = "0.0.0.0:6188"
upstream = "127.0.0.1:8080"
```

可用 CLI 覆盖：

```bash
cargo run -p pinnacle-cli -- --config pinnacle.toml --listen 0.0.0.0:7000
```

## 运行

先启动 upstream（默认 `127.0.0.1:8080`），再启动网关：

```bash
# 示例 upstream
python3 -m http.server 8080

# 网关
cargo run -p pinnacle-cli -- --config pinnacle.toml
```

试用：

```bash
# 未验证 → 503 挑战页 + cid cookie
curl -v http://127.0.0.1:6188/

# 指纹脚本
curl -v http://127.0.0.1:6188/__pinnacle
```

浏览器访问任意路径完成挑战后，会带上 `__pinnacle_pass`，后续请求转发到 upstream。

## 开发

```bash
cargo build
cargo test
cargo run -p pinnacle-cli -- --help
```

## License

MIT
