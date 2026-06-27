# 网络代理配置

## 功能概述

v0.7.0 新增应用内“网络代理”配置，用于 GitHub API 和 Git 更新。该功能解决国内网络环境下 GitHub 访问不稳定的问题，同时避免让不需要代理的用户默认依赖本地代理。

## 背景与目标

- GitHub 搜索、精选 Skills、GitHub Contents API 下载和 Git clone/fetch 都依赖稳定的 GitHub 访问。
- 之前应用主要依赖进程启动环境或系统 Git 配置，Dock/Finder 启动时不一定继承 shell 中的代理环境变量，行为不确定。
- 直接默认强制 `127.0.0.1:7890` 会让没有代理软件的用户失败。
- 需要一个显式、可理解、可关闭的应用内代理配置。

## 主要变更

### 设置页配置

- 新增“网络代理”开关。
- 只暴露代理端口，主机固定为 `127.0.0.1`，降低普通用户理解成本。
- 首次未配置时自动检测本地 7890 端口：
  - 检测到可连接：自动开启代理，端口为 7890。
  - 检测不到：默认关闭代理。
- 用户手动开启、关闭或修改端口后，以用户配置为准，不再自动覆盖。

### 后端统一代理入口

- 新增 `network_proxy` 核心模块，统一读取和保存代理配置。
- HTTP 请求通过统一的 `github_http_client` 创建，显式配置代理。
- 支持 HTTP 代理，也通过 `reqwest` socks feature 支持 `socks5` 代理能力。

### Git 更新强制走配置代理

- 系统 `git` 命令会显式注入：
  - `http.proxy`
  - `https.proxy`
  - `http_proxy`
  - `https_proxy`
  - `all_proxy`
- 配置了代理时，如果系统 `git` 执行失败，不回退到未显式代理的 libgit2，避免绕过代理直连 GitHub。

## 影响范围

代理配置会影响以下 GitHub 相关访问：

- GitHub 搜索
- 精选 Skills 拉取
- GitHub Contents API 目录下载
- Git Skill 安装、更新和缓存刷新
- 自动更新中的 Git Skill 更新

## 涉及文件

- `src-tauri/src/core/network_proxy.rs`
- `src-tauri/src/core/git_fetcher.rs`
- `src-tauri/src/core/github_search.rs`
- `src-tauri/src/core/github_download.rs`
- `src-tauri/src/core/featured_skills.rs`
- `src-tauri/src/core/installer.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- `src/components/skills/SettingsPage.tsx`
- `src/components/skills/types.ts`
- `src/App.tsx`
- `src/App.css`
- `src/i18n/resources.ts`
- `src-tauri/Cargo.toml`

## 验证

实现完成后运行：

```bash
npm run check
```

重点覆盖：

- 未检测到 7890 时默认关闭代理。
- 用户可显式开启/关闭代理。
- 端口配置会生成 `http://127.0.0.1:<port>`。
- Git 命令会注入代理参数和代理环境变量。

