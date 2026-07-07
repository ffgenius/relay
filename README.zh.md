# Relay

> **安全的跨平台命令路由器。** 输入 `v dev`，执行 `vite dev`。不依赖 shell alias，不藏任何意外。

[![npm](https://img.shields.io/npm/v/@ffgenius/relay.svg)](https://www.npmjs.com/package/@ffgenius/relay)
[![homebrew](https://img.shields.io/badge/homebrew-ffgenius%2Ftap-orange.svg)](https://github.com/ffgenius/homebrew-tap)
[![license](https://img.shields.io/npm/l/@ffgenius/relay.svg)](./LICENSE)

[English](./README.md)

---

## 为什么需要 Relay？

Shell alias 能用，直到它不好用：

- bash / zsh / fish / PowerShell 的 alias 语法各不相同，跨 shell 共享配置很折腾。
- alias 在 **shell 内部** 展开 —— `alias g='git'` 本质是字符串宏，你打 `g` 不一定真的跑你以为的那个二进制。
- 它们藏在 dotfiles 里。换电脑就丢，换 shell 也丢。

**Relay 不走 shell 路由命令。** 每个简写对应一个小启动器（"shim"），它调用 relay 的 runner，runner 直接 spawn 目标程序 —— 不走 `sh -c`，不解析字符串，没有意外。

```bash
relay add v vite      # 注册
v dev                 # 执行 vite dev — 从不经过 shell
v build               # 执行 vite build
```

Linux、macOS、Windows 行为完全一致，配置文件可以直接跨平台同步。

---

## 安装

选择你喜欢的方式 —— Homebrew 和 shell 安装器 **不需要 Node.js**。

### Homebrew（macOS / Linux）

```bash
brew tap ffgenius/tap
brew install relay
relay init
```

[Homebrew](https://brew.sh) 会自动安装二进制文件，后续 `brew upgrade` 即可更新。

### Shell 安装器（curl / wget）

```bash
curl -fsSL https://raw.githubusercontent.com/ffgenius/relay/master/install.sh | sh
```

或用 `wget`：

```bash
wget -qO- https://raw.githubusercontent.com/ffgenius/relay/master/install.sh | sh
```

脚本会自动检测你的操作系统和 CPU 架构，从 GitHub Releases 下载对应二进制，
安装到 `~/.relay/bin/`，并执行 `relay init` 完成 shell 集成（bash / zsh / fish）。

**可选项：** `--version 0.1.0` 指定版本；`--no-init` 跳过 shell 配置写入。

### PowerShell（Windows）

```powershell
irm https://raw.githubusercontent.com/ffgenius/relay/master/install.ps1 | iex
```

行为同上：下载对应二进制，安装到 `~\.relay\bin\`，通过注册表将目录加入用户 `PATH`。

**可选项：** `-Version 0.1.0` 指定版本；`-NoInit` 跳过 PATH 设置。

### npm

```bash
npm install -g @ffgenius/relay
```

npm 会通过 optionalDependencies 自动下载当前平台的二进制（`linux-x64` /
`linux-arm64` / `darwin-x64` / `darwin-arm64` / `win32-x64` / `win32-arm64`），
其余平台的包不占空间。

### 安装后

以上所有方式都会自动执行 `relay init`（除非你显式跳过）。这会创建 `~/.relay/`
目录，并把 `~/.relay/bin` 加入到你的 `PATH`。

**请打开一个新的终端** 让 PATH 改动生效。

---

## 快速上手

```bash
# Prefix 简写 —— `v <任何参数>` 等价于 `vite <任何参数>`
relay add v vite

# Prefix 简写带默认参数 —— `gt <参数>` 等价于 `git clone <参数>`，运行时参数会追加
relay add gt git -p clone

# Exact 简写 —— `vd` 永远执行 `vite dev`，不接受额外参数
relay add vd vite dev

# 使用
v dev                 # → vite dev
v build               # → vite build
gt https://xxx/repo.git            # → git clone https://xxx/repo.git
vd                    # → vite dev

# 查询
relay list            # 列出所有简写（也可以 relay ls）
relay info v          # 查看单个简写的详情
relay discover vite   # 按目标程序聚合查看

# 存储和运行 Shell 片段（支持跨 Shell 自动翻译）
relay snippet add goback "cd ../"
relay snippet run goback --dry-run

# Snippet 支持 {{0}} {{1}} … 占位符，运行时传入参数自动替换
relay snippet add killport --shell powershell "Get-NetTCPConnection -LocalPort {{0}} ^| ForEach-Object { Stop-Process -Id `$_.OwningProcess -Force }"
relay snippet run killport --dry-run 4600
# → Get-NetTCPConnection -LocalPort 4600 | ...

# 诊断
relay doctor          # 检查 PATH / shim / 配置
relay doctor --fix    # 自动修复缺失的 shim 和 PATH
```

---

## 三种项目类型

### Prefix 简写

`relay add <name> <program>` —— 你在 `<name>` 之后输入的所有参数都透传给目标程序。

```bash
relay add v vite
v dev      # → vite dev
v build    # → vite build
v --help   # → vite --help
```

你也可以通过 `--prefix` / `-p` 注册**带默认参数**的 Prefix 简写。
默认参数始终包含在内，运行时输入的额外参数会追加到后面：

```bash
relay add gt git -p clone
gt https://example.com/repo.git          # → git clone https://example.com/repo.git
gt --depth 1 https://example.com/repo.git # → git clone --depth 1 https://example.com/repo.git
```

### Exact 简写

`relay add <name> <program> <args...>`（不加 `--prefix`）—— 参数被固化，返回值输入的任何额外参数都会被拒绝。

```bash
relay add vd vite dev
vd         # → vite dev（永远）
vd preview # → 报错：exact command does not accept extra arguments
```

**Prefix** 适合你经常用多个子命令的工具（`v`、`g`、`n`）；**Prefix 带参数** 适合命令本身包含固定子命令但你仍需要传递额外参数（如 `gt` 对应 `git clone`）；**Exact** 适合你天天敲的高频组合（`vd`、`gp`、`nci`）。

### Snippet（Shell 片段）

`relay snippet add <name> <content...>` —— 存储任意 Shell 代码片段。与普通简写（绕过 Shell 直接执行）不同，snippet 通过 Shell 解释器执行，并支持通过 [polysh](https://github.com/ffgenius/polysh) 进行**跨 Shell 自动翻译**。

```bash
# 创建 snippet —— relay 自动检测当前 Shell 环境
relay snippet add goback "cd ../"

# 运行 —— 如果当前 Shell 与创建时不同，relay 会自动翻译
# （Unix ↔ PowerShell ↔ CMD）
relay snippet run goback

# 预览翻译结果，不实际执行
relay snippet run goback --dry-run
```

**占位符：** Snippet 内容中可用 `{{0}}` `{{1}}` … 占位符，运行时传入的参数会依次替换：

```bash
relay snippet add greet --shell powershell "Write-Host Hello {{0}}"
relay snippet run greet --dry-run World        # → Write-Host Hello World
relay snippet add killport --shell powershell "Get-NetTCPConnection -LocalPort {{0}} ^| ForEach-Object { Stop-Process -Id `$_.OwningProcess -Force }"
killport 4600                                   # 通过 shim 传参同样有效
```

如果占位符索引超出传入参数数量，relay 会报错并提示缺少哪个参数。

**为什么需要 snippet？** `cd`、`export`、复杂管道、Shell 内置命令等无法通过 relay 的直接执行模型工作。Snippet 填补了这一空白，同时保持跨 Shell 的移植性。

---

## 命令参考

### 基础

| 命令 | 说明 |
|---|---|
| `relay init` | 创建 `~/.relay`，写入空 config，并把 `~/.relay/bin` 加入 PATH |
| `relay add <name> <program> [args...]` | 注册简写（无参数或带 `-p` 为 prefix，否则为 exact） |
| `relay add <name> <program> -p [args...]` | 注册带默认参数的 prefix 简写（运行时参数会追加） |
| `relay remove <name>`（别名 `rm`） | 删除简写 |
| `relay update <name> <program> [args...]` | 修改已有的简写（支持 `--prefix` / `-p`） |
| `relay list`（别名 `ls`） | 列出所有简写（按名字排序） |
| `relay info <name>` | 查看单个简写的详情 |
| `relay clear`（别名 `cls`） | 删除所有简写（会先确认） |
| `relay clear --yes` | 同上，但不确认 |

### 查询

| 命令 | 说明 |
|---|---|
| `relay discover` | 按目标程序聚合显示所有简写 |
| `relay discover <program>` | 只显示某个程序的所有简写 |

### 备份与同步

| 命令 | 说明 |
|---|---|
| `relay export` | 把当前配置以 YAML 形式打印到 stdout（默认包含 snippets） |
| `relay export -o <file>` | 导出到文件（如果省略后缀会自动补 `.yaml`） |
| `relay export --no-snippet` | 只导出 commands，排除 snippets |
| `relay import <file>` | 从文件合并配置。Snippets **默认跳过**（安全考虑） |
| `relay import <file> --overwrite` | 从文件合并配置（冲突时覆盖本地） |
| `relay import <file> --allow-snippet` | 同时导入文件中的 snippets |
| `relay sync init` | 创建私密 GitHub Gist 并把本机绑定上去 |
| `relay sync link <gist_id>` | 把本机绑定到已有的 Gist |
| `relay sync unlink` | 取消本机的 Gist 绑定（远端 Gist 不会被删） |
| `relay sync push` | 上传本地配置（commands + snippets）到 Gist |
| `relay sync push --no-snippet` | 只上传 commands，排除 snippets |
| `relay sync pull` | 从 Gist 下载配置。Snippets **默认跳过** |
| `relay sync pull --allow-snippet` | 下载并包含 snippets |
| `relay sync status` | 查看同步状态、commands 和 snippets 数量 |

### Snippets

| 命令 | 说明 |
|---|---|
| `relay snippet add <name> <content...>` | 创建 snippet（自动检测当前 Shell） |
| `relay snippet add <name> <content...> --shell <d>` | 指定 Shell 方言（`unix`、`powershell`、`cmd`） |
| `relay snippet add <name> <content...> --desc <d>` | 添加描述 |
| `relay snippet remove <name>`（别名 `rm`） | 删除 snippet |
| `relay snippet list`（别名 `ls`） | 列出所有 snippets |
| `relay snippet info <name>` | 查看单个 snippet 详情 |
| `relay snippet edit <name> --content <c>` | 修改内容 |
| `relay snippet edit <name> --desc <d>` | 修改描述（传 `""` 清除） |
| `relay snippet edit <name> --shell <d>` | 修改 Shell 方言 |
| `relay snippet run <name> [args...]` | 执行 snippet（自动翻译到当前 Shell）。`args` 按序替换 `{{0}}` `{{1}}` … 占位符 |
| `relay snippet run <name> --dry-run [args...]` | 只打印替换和翻译后的命令，不执行 |
| `relay snippet run <name> --no-translate [args...]` | 跳过翻译，原样执行（仍支持占位符替换） |
| `relay snippet clear` | 删除所有 snippets（会先确认） |
| `relay snippet clear --yes` | 同上，但不确认 |

### 系统

| 命令 | 说明 |
|---|---|
| `relay doctor` | 检查 PATH / shim / 配置完整性 |
| `relay doctor --fix` | 自动重建缺失的 shim 并加入 PATH |
| `relay rebuild` | 全量重置：根据当前配置重新生成所有 shim |

---

## 多机同步

Relay 通过你已登录的 `gh` CLI 把配置同步到 **私密的 GitHub Gist**。无需自己管理 token。

**机器 A（第一次）：**

```bash
gh auth login                 # 还没登过的话
relay add v vite              # 先注册几条
relay add g git
relay sync init               # → 创建 Gist，打印 ID
```

**机器 B（另一台）：**

```bash
gh auth login
relay sync link <gist_id>     # 上面 sync init 打印的 ID
relay sync pull               # 下载所有简写、重建 shim
```

**日常使用：**

```bash
relay add p pnpm              # 在机器 A 加新简写
relay sync push               # 推送
# ...稍后，在机器 B：
relay sync pull               # 拉取最新
```

`relay sync status` 会告诉你本地相对远端是 clean 还是 dirty；`pull` 在本地有未推送的修改时会先警告再确认。

---

## 安全模型

Relay 的卖点就是 **构造性安全** —— 跑 `v dev` 必须和直接跑 `vite dev` **等价到无聊**。下面 4 条原则是代码级强制：

> **原则 1 — Relay 不执行 shell（snippet 除外）。**
> 普通简写用 `std::process::Command` 直接 spawn 目标二进制 —— 不调 `sh -c`、不调 `cmd /c`、不调 `powershell -Command`。**Snippet 是有意为之的例外**：因为 snippet 本身就是 Shell 代码，它必须通过 Shell 解释器执行。正因如此，import/pull 默认跳过 snippets，需 `--allow-snippet` 显式授权。

> **原则 2 — Relay 不执行字符串。**
> 一条简写在内部是 `(program, args)` 元组。不存在 `exec: "vite dev && rm -rf /"` 这种字段。"字符串当命令"在数据结构里根本不存在。

> **原则 3 — Relay 只执行已注册的、确实存在的可执行文件。**
> `relay add` 会调 `which(<program>)` 校验存在；含路径分隔符（`/` 或 `\`）的程序名也会被拒绝 —— 只允许裸命令名。这意味着恶意 Gist 没法通过 `relay sync pull` 偷塞 `/tmp/evil-cargo` 到你的配置里。

> **原则 4 — Shell 在黑名单上。**
> `sh`、`bash`、`zsh`、`cmd`、`powershell`、`pwsh` 不能作为简写的目标程序。即使你 `relay add x sh`，relay 也会拒绝。

这些规则同时也保证 Relay 自身不会成为攻击面 —— shim 到二进制这条路径上不存在 shell escape。

---

## 配置文件

所有数据都在 `~/.relay/` 下：

```
~/.relay/
├── config.yaml          # 已注册的简写 + snippets
├── sync-state.yaml      # （可选）已绑定的 Gist ID + 同步哈希
└── bin/                 # 生成的 shim，这个目录被加入 PATH
    ├── v                # Windows 下是 v.cmd
    ├── vd
    └── ...
```

`config.yaml` 是可读可手改的（手改后记得跑 `relay rebuild` 重建 shim）：

```yaml
version: 1
commands:
  v:
    type: prefix
    program: vite
  vd:
    type: exact
    program: vite
    args:
      - dev
snippets:
  goback:
    type: snippet
    content: "cd ../"
    shell: unix
  serve:
    type: snippet
    content: "python3 -m http.server 8080"
    shell: unix
    description: "启动本地文件服务器"
```

---

## 故障排查

### 明明 `relay add n nvm` 成功了，跑 `n` 却报 "command not found"

你当前的 shell 还没拿到新 PATH。先跑：

```bash
relay doctor
```

如果显示 `shim dir is NOT on PATH`，跑 `relay doctor --fix`，**然后开一个新终端**。

Windows 上，注册表的 PATH 改动可能不会立刻传播到已开的 `cmd` 窗口 —— 必要时注销重新登录。开了新终端还不行的话，再跑一次 `relay doctor` 看具体原因。

### 第一次执行 `relay` 报 `EPERM`（Windows）

Windows Defender / SmartScreen 第一次见到刚装的 `relay.exe` 时会做实时扫描，扫描期间 spawn 会失败。等一两秒重跑同样的命令就好了，每次安装只会发生一次。

### PATH "太长"，新加的条目看不见

Windows 在创建进程时会把组合后的 PATH 截断到 2047 字符。`relay init` 把 shim 目录写到用户 PATH 的 **开头**（不是末尾），就是为了避开截断；不过如果你的 PATH 已经非常拥挤，一些条目仍然可能丢。`relay doctor` 在用户 PATH 超过 1900 字符时会发出提醒。

### Sync 报 `gh: not authenticated`

跑一次 `gh auth login` 即可。Relay 直接复用你的 GitHub CLI 登录态，不会自己管理 token。

---

## 参与

```bash
git clone https://github.com/ffgenius/relay
cd relay
cargo build
cargo test
```

欢迎 issue 和 PR。

## 协议

[MIT](./LICENSE) © Bin
