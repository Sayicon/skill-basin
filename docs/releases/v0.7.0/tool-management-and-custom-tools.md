# 工具管理与自定义工具目录

## 功能概述

新增管理中心下的「工具」页，用于管理哪些工具参与 Skills 同步。内置工具默认保持全部启用，用户可以按需关闭不再使用的工具；被关闭的工具不会继续出现在 Skill 卡片、添加 Skill 的同步目标和后续同步流程中。

同时支持自定义工具目录，覆盖 Issue [#24](https://github.com/qufei1993/skills-hub/issues/24) 中提到的内部工具目录场景，例如 `/Users/xxx/.claude_xxx/skills/`。

## 背景与目标

- 之前工具检测主要依赖默认检测目录是否存在。如果用户卸载工具后配置目录仍残留，Skills Hub 仍可能把该工具识别为可同步目标。
- 工具数量增加后，默认展示所有检测到的工具会让 Skill 卡片和新增弹窗显得拥挤。
- 一些内部工具或二次封装工具使用非标准 skills 目录，需要允许用户手动添加同步目标。

本次改动的目标是把「工具启用状态」和「自定义工具目录」统一到管理中心，默认不改变现有用户行为，同时允许用户主动收敛工具列表，避免设置页承载过多高频管理操作。

## 主要变更

### 工具管理入口

- 主导航提供「管理中心」入口，工具管理作为其中的「工具」页展示工具检测状态、全局 skills 目录和项目级目录。
- 页面顶部提供总计、已启用、已检测、自定义四个统计数字，帮助用户理解下方工具列表的整体规模。
- 已检测工具和自定义工具优先展示；未检测到的内置工具默认折叠在底部，避免把用户可能不需要的工具全部铺开。
- 设置页不再承载完整工具管理表单，继续聚焦语言、外观、存储、网络、应用版本更新和缓存等应用偏好。
- 内置工具默认全部启用；关闭后该工具不再作为可用同步目标。
- 工具开关配置即时保存，保存后刷新工具状态，并同步更新新增 Skill 弹窗中的默认目标选择。
- 保留检测状态展示，方便用户识别「目录仍在但工具已不再使用」的情况。

### 自定义工具目录

- 支持在管理中心的「工具」页添加自定义工具：
  - 工具名称
  - 全局 skills 目录，支持 `~` 展开
  - 可选项目级目录，例如 `.agents/skills`
  - 启用状态
- 保存启用的自定义工具时会自动创建全局 skills 目录；项目级目录是相对具体项目的路径，仍在同步到项目时创建。
- 自定义工具启用后会进入统一工具列表，参与添加 Skill、同步/取消同步和 Skill 卡片展示。
- 自定义工具使用自动生成的稳定 key，避免和内置工具 key 冲突。

### 后端配置与同步逻辑

- 新增工具配置模型，配置存储在现有 `settings` 表的 `tool_config_v1` key 中，不新增数据库 schema migration。
- 新增 Tauri commands：
  - `get_tool_config`
  - `set_tool_config`
- `get_tool_status` 改为返回完整工具状态，包括：
  - `enabled`
  - `is_custom`
  - `installed`
  - skills 目录信息
- `sync_skill_to_tool` 改为通过运行时工具列表解析目标工具，确保停用工具不会继续参与同步。
- 共享 skills 目录的工具仍会保持目标记录一致，自定义工具也复用同一套目录去重逻辑。

## 国际化

新增 `toolManagement` 文案键，包含：

- `toolManagement.title`
- `toolManagement.hint`
- `toolManagement.pageHint`
- `toolManagement.builtinSection`
- `toolManagement.builtinHint`
- `toolManagement.customSection`
- `toolManagement.customHint`
- `toolManagement.missingSection`
- `toolManagement.noDetectedTools`
- `toolManagement.totalCount`
- `toolManagement.enabledCount`
- `toolManagement.detectedCount`
- `toolManagement.customCount`
- `toolManagement.detected`
- `toolManagement.notDetected`
- `toolManagement.custom`
- `toolManagement.projectDir`
- `toolManagement.namePlaceholder`
- `toolManagement.skillsDirPlaceholder`
- `toolManagement.projectDirPlaceholder`
- `toolManagement.addCustom`
- `toolManagement.removeCustom`
- `toolManagement.saved`

英文和中文资源已同步更新。

## 涉及文件

- `src-tauri/src/core/tool_adapters/mod.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/lib.rs`
- `src/App.tsx`
- `src/App.css`
- `src/components/skills/SettingsPage.tsx`
- `src/components/skills/ToolsPage.tsx`
- `src/components/skills/types.ts`
- `src/i18n/resources.ts`

## 验证

实现完成后运行：

```bash
npm run check
```

覆盖 ESLint、前端测试、TypeScript/Vite 构建、Rust 格式、Clippy 和 Rust 测试。
