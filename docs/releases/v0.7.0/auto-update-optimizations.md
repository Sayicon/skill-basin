# 自动更新体验优化

## 功能概述

v0.7.0 增加并优化 Skills 自动更新能力：支持通过用户级系统定时任务在应用关闭时更新 Skills，复用现有单个 Skill 更新逻辑；同时补充后台更新状态、进度详情、失败原因、网络代理配置和运行耗时信息，降低用户判断“是否真的在更新”“哪里失败了”“本次运行耗时多久”的成本。

## 背景与目标

- 之前 Skills 更新主要依赖用户在 UI 中手动点击单个 Skill 更新，应用关闭后不会继续执行。
- 国内网络环境下访问 GitHub 可能不稳定，需要可控的本地代理配置，但不能默认让所有用户都依赖代理。
- 后台任务执行时，用户需要能在重新打开应用后看到任务状态、更新进度、失败项和失败原因。
- 进度详情只展示总数、成功、失败和等待数量，不方便判断上次任务从何时开始、何时完成以及耗时多久。

## 主要变更

### 系统定时更新

- 新增用户级系统定时任务配置，默认周期为 24 小时，用户可在设置页开启/关闭并调整周期。
- macOS 使用 LaunchAgent，Windows 使用当前用户计划任务，Linux 使用 user systemd timer。
- 定时任务通过应用自身后台入口触发，复用现有 `update_managed_skill_from_source` 更新逻辑，不重新实现 Skill 更新流程。
- 手动“立即更新”通过系统调度触发后台任务，用于验证真实后台路径。

### 后台状态与进度详情

- 设置页展示后台更新状态、上次运行状态、检查数量、更新数量和失败数量。
- 新增“查看进度”弹窗，按“当前处理 / 失败 / 成功 / 等待中”分组展示。
- 失败项展示 Skill 名称和失败原因，避免只暴露内部 ID 或完整堆栈。
- 成功项默认仅展示汇总数量，避免大量成功列表占用空间。
- 开始新一轮更新时清理上一次的进度和失败结果，避免用户误认为旧结果属于本次运行。

### 开始时间、完成时间与耗时

- 后端新增独立持久化字段：
  - `skill_auto_update_last_started_at`
  - `skill_auto_update_last_finished_at`
- 进度详情弹窗展示开始时间、完成时间和耗时。
- 运行中时完成时间显示“运行中”，耗时按当前时间动态计算。
- 旧字段 `skill_auto_update_last_run_at` 继续保留，用于兼容现有“上次运行”和调度判断。

### 网络代理配置

- 新增“网络代理”设置，用于 GitHub API 和 Git 更新。
- 首次未配置时自动检测本地 7890 端口；检测到后自动开启代理，否则默认关闭。
- 用户可显式开启/关闭代理，并只需要配置端口；主机固定为 `127.0.0.1`，不在 UI 中暴露。
- GitHub API、精选 Skills、GitHub Contents API 下载以及系统 `git clone/fetch` 均使用同一代理配置。
- 已配置代理时，系统 `git` 失败不会回退到未显式代理的 libgit2，避免绕过代理直连 GitHub。

## 涉及文件

- `src-tauri/src/core/auto_update.rs`
- `src-tauri/src/core/system_scheduler.rs`
- `src-tauri/src/core/network_proxy.rs`
- `src-tauri/src/core/git_fetcher.rs`
- `src-tauri/src/core/github_search.rs`
- `src-tauri/src/core/github_download.rs`
- `src-tauri/src/core/featured_skills.rs`
- `src-tauri/src/core/installer.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- `src/components/skills/SettingsPage.tsx`
- `src/components/skills/autoUpdateSettings.ts`
- `src/components/skills/types.ts`
- `src/App.tsx`
- `src/App.css`
- `src/i18n/resources.ts`

## 验证

实现完成后运行：

```bash
npm run check
```

覆盖 ESLint、前端测试、TypeScript/Vite 构建、Rust 格式、Clippy 和 Rust 测试。

