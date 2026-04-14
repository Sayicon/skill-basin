# UX 优化记录

收录不需要单独文档的小型 UX 改进。

---

## 关闭按钮改为隐藏窗口（macOS）

**变更：** 点击红色 X 按钮不再退出应用，而是隐藏窗口。

**原因：** macOS 上许多主流应用（Slack、Discord 等）均采用此交互模式——应用在后台持续运行，下次打开时响应更快。需要真正退出时使用 `Cmd+Q` 或菜单栏退出。

**实现方式：**
- 拦截 `CloseRequested` 窗口事件，阻止默认关闭行为，改为隐藏窗口。
- 点击 Dock 图标时触发 `RunEvent::Reopen`，重新显示窗口并聚焦。

**涉及文件：** `src-tauri/src/lib.rs`

---

## Skill 描述与 Markdown 预览优化

**变更：** 修复 `SKILL.md` frontmatter 在列表和详情页中的展示问题。

**原因：** 部分 Skill 使用 YAML 折叠块语法（例如 `description: >-`）。旧解析逻辑没有识别 `>-`、`>+`、`|-`、`|+`，导致列表卡片错误显示 `>-`，详情页元信息也可能出现描述缺失或排版异常。

**实现方式：**
- 后端 `SKILL.md` 解析支持 YAML block scalar 的 chomping indicator：`>`、`>-`、`>+`、`|`、`|-`、`|+`。
- 启动时重新从 `SKILL.md` 比对并回填描述，纠正已入库的旧错误值。
- 详情页 frontmatter 改为响应式 key/value 元信息区，避免短字段被表格挤压成竖排。
- Markdown 预览内容区居中展示，并保留最大可读宽度。

**涉及文件：**
- `src-tauri/src/core/installer.rs`
- `src-tauri/src/core/skill_store.rs`
- `src-tauri/src/core/tests/installer.rs`
- `src-tauri/src/core/tests/skill_store.rs`
- `src/components/skills/SkillDetailView.tsx`
- `src/App.css`
