# v0.6.3 小需求与体验优化记录

这个文件用于记录 v0.6.3 周期内较小的需求、体验优化和修复。后续同类变更继续追加到这里，避免为每个小项单独创建发布记录文件。

## 2026-06-27

### 修复 Antigravity 2.0 全局 Skill 同步路径

- 修复将 Skill 同步到 Antigravity 时使用旧目录的问题（Issue [#79](https://github.com/qufei1993/skills-hub/issues/79)，PR [#80](https://github.com/qufei1993/skills-hub/pull/80)）。
- 全局 Skill 同步目录从 `~/.gemini/antigravity/skills/` 更新为 `~/.gemini/config/skills/`，匹配 Antigravity 2.0 当前文档。
- Antigravity 安装检测目录同步调整为 `~/.gemini/config/`。
- 项目级 Skill 同步目录保持为 `<project>/.agents/skills/`。
- 英文和中文工具支持列表已同步更新 Antigravity 路径。
- 增加 Rust 回归测试，覆盖 Antigravity 全局 Skill 目录、检测目录和项目级目录映射。
- 修复验证：`npm run check`。
