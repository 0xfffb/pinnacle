# Pinnacle

基于 Pingora 的反爬 / 人机验证网关（Cargo workspace）。

## 中间件栈

请求由外到内经过：

```
[1] challenge
[2] ban
[3] count
[4] policy
[5] detector
[6] forward
```

未持有有效 pass cookie 时，最外层 `challenge` 下发 JS 指纹页；验证通过后签发 `__pinnacle_pass`，后续每次请求都会校验该 token。

## Crates

| Crate | 路径 | 作用 |
|-------|------|------|
| `pinnacle-core` | `crates/core` | `Request` / `Context` / `Action`，Tower `Service` / `Layer` |
| `pinnacle-store` | `crates/store` | `MemoryStore`（ban / pass / challenge session） |
| `pinnacle-turnstile` | `crates/turnstile` | 中间件链与 Cookie challenge |
| `pinnacle-gateway` | `crates/gateway` | Pingora HTTP ↔ Turnstile |
| `pinnacle-cli` | `crates/cli` | 二进制入口与 TOML 配置 |

## Challenge

由 `CookieChallengerService` 处理：

| 条件 | 行为 |
|------|------|
| `GET /__pinnacle/fp.js` | 返回指纹脚本；有进行中的 challenge 时附带 `Set-Cookie: __pinnacle_cid` |
| `POST` + `__pinnacle_cid` | 校验指纹；成功则 `Set-Cookie: __pinnacle_pass=<token>` |
| 无有效 `__pinnacle_pass` | `Set-Cookie: __pinnacle_cid` + 503 挑战页 |
| 任意业务请求 | 校验 `__pinnacle_pass` 与服务端按 IP 保存的 token |
| 内层返回 `Challenge` | 再次下发挑战页 |

## 配置

默认读取 `pinnacle.toml`：

```toml
listen = "0.0.0.0:6188"
upstream = "127.0.0.1:8080"
rules = []
```

`rules` 示例：

```toml
[[rules]]
type = "ip_block"
ip = "1.2.3.4"
reason = "manual"

[[rules]]
type = "path_prefix"
prefix = "/admin"
action = "block"
```

支持的 `type`：`ip_allow`、`ip_block`、`path_prefix`、`path_rate`、`ua_allow_contains`。

## 运行

先启动 upstream（默认 `127.0.0.1:8080`），再启动网关：

```bash
cargo run -p pinnacle-cli -- --config pinnacle.toml
cargo run -p pinnacle-cli -- --config pinnacle.toml --listen 0.0.0.0:7000
```

```bash
# 未验证 → 503 挑战页
curl -v http://127.0.0.1:6188/

# 指纹脚本
curl -v http://127.0.0.1:6188/__pinnacle/fp.js
```

浏览器访问任意路径完成挑战后，会带上 `__pinnacle_pass`，后续请求转发到 upstream。

## 开发

```bash
cargo build
cargo test
cargo run -p pinnacle-cli -- --help
```
