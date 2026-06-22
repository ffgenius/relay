# Relay Roadmap

简写工具的核心价值在「人 ↔ 工具」的高频复用 —— 而不是「人 ↔ 项目」、不是命令市场、不是团队治理。
以下 roadmap 围绕这个判断展开，砍掉了边际价值低的方向。

---

## v0.1 MVP ✓

实现安全命令路由。

* init / add / remove / update / list / info / doctor
* prefix / exact command
* shim 生成
* npm 发布
* `~/.relay/bin` 自动加入 PATH（Win/Unix）

```bash
relay add v vite
v dev
v build
```

---

## v0.2 Discover ✓

按目标程序聚合查询已注册的命令简写。

```bash
relay discover           # 列出所有 program → aliases
relay discover vite      # 只列 vite 的别名
```

输出示例：

```text
vite (3 aliases):
  v   → vite             [Prefix]
  vd  → vite dev         [Exact]
  vb  → vite build       [Exact]
```

---

## v0.3 Sync

把 config 备份到云端，让用户换电脑/重装系统时配置不丢。
合并了原 roadmap 的 v0.4 (Import/Export) 和 v0.6 (Sync) —— 它们解决的是同一个问题：
**"我的简写跟着我走"**。

```bash
relay export                 # 导出当前配置到 stdout
relay export -o backup.yaml  # 导出到文件
relay import backup.yaml     # 从文件导入（合并/覆盖）

relay sync init              # 配置同步源（git 仓库或 GitHub Gist）
relay sync push              # 推送本地配置
relay sync pull              # 拉取远程配置
```

支持：

* 本地文件导入/导出
* Git Repository 同步
* GitHub Gist 同步

---

## v0.4 Completion

Shell 补全 —— alias 工具最有体验飞跃的功能。让 `g <Tab>` 不止补 alias 名，
能补到 `git` 子命令上。

```bash
relay completion bash      # 输出 bash 补全脚本
relay completion zsh
relay completion fish
relay completion powershell
```

效果：

```text
$ g <Tab>
add  branch  checkout  commit  diff  log  pull  push  status  ...
```

每种 shell 一个生成器，写完一次就稳定了。

---

## v0.5 Doctor 增强

让 `relay doctor` 从「能跑」升级到「能帮你排坑」：

* 检测 PATH 长度 → 提醒 Windows 截断风险（已部分实现）
* 检测多个 PATH 上的同名可执行文件冲突
* 检测 shim 文件被外部覆盖/损坏
* 检测注册的 program 是否还在 PATH 上
* `--fix` 自动修复能修的（已部分实现）

```bash
relay doctor --json        # 机器可读输出，给 CI 用
relay doctor --fix
```

---

## v1.0 Stable

目标：成为开发者**个人**统一命令入口。

能力：

* Secure Command Routing
* Discover / Sync / Completion / Doctor
* Stable config 格式（v1 → 永不破坏兼容）
* 完善文档 + 主流包管理器分发（npm、cargo、homebrew、scoop）

---

## 砍掉的方向

为对齐设计原则，明确从 roadmap 移除：

### ~~Workspace（项目级 relay.yaml）~~

项目启动命令是一次性的，不需要简写。`package.json scripts` / `Makefile` /
`justfile` 已经解决得很好，relay 重复造没意义。

### ~~Team Sharing（团队共享命令集）~~

简写是非常个人的偏好（每个人对 `gs` 是 `status` 还是 `stash` 都有不同直觉），
强行共享是反模式。

### ~~Registry（命令市场）~~

oh-my-zsh 的 plugin 生态已经覆盖这个场景。再造一个分发系统，运维成本远超价值。

### ~~IDE Integration~~

IDE 自己的 task runner / terminal alias 已经足够，relay 不需要在这里发力。
