# v0.6.2 小需求与体验优化记录

这个文件用于记录 v0.6.2 周期内较小的需求、体验优化和界面修正。后续同类变更继续追加到这里，避免为每个小项单独创建发布记录文件。

## 2026-06-19

### 新增 WorkBuddy 支持

- 新增 WorkBuddy 工具适配器，支持将全局 Skill 同步到 `~/.workbuddy/skills/`（PR [#73](https://github.com/qufei1993/skills-hub/pull/73)）。
- 当用户目录下存在 `~/.workbuddy/` 时，Skills Hub 会将 WorkBuddy 识别为已安装工具。
- WorkBuddy 当前仅支持全局 Skill 同步；由于项目级 Skill 目录尚未确认，暂不提供项目级同步。
- 英文和中文工具支持列表已同步补充 WorkBuddy。
- 增加 Rust 测试，覆盖工具标识查询和项目级同步能力判断。
- 修复验证：`npm run check`。
