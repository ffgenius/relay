# Relay Roadmap

## v0.1 MVP

目标：

实现安全命令路由。

功能：

* init
* add
* remove
* update
* list
* info
* doctor
* prefix command
* exact command
* shim生成
* npm发布

示例：

```bash
relay add v vite

v dev
v build
```

---

## v0.2 Discover

自动发现命令。

```bash
relay discover vite
```

分析：

```bash
vite --help
```

生成：

```yaml
v:
  program: vite
```

支持自动补全。

---

## v0.3 Workspace

项目级配置：

```text
project/
└── relay.yaml
```

执行：

```bash
relay use
```

启用项目命令。

---

## v0.4 Import / Export

支持：

```bash
relay export yaml
relay export json

relay import
```

---

## v0.5 Team Sharing

支持：

```bash
relay share
relay install
```

团队共享命令集。

---

## v0.6 Sync

支持：

```bash
relay sync push
relay sync pull
```

同步到：

* Git Repository
* GitHub Gist

---

## v0.7 Registry

命令市场：

```bash
relay install frontend
relay install node
relay install docker
```

---

## v1.0 Stable

目标：

成为开发者统一命令入口。

能力：

* Secure Command Routing
* Workspace
* Team Sharing
* Sync
* Registry
* Shell Completion
* IDE Integration
