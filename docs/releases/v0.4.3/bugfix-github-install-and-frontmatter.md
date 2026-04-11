# Bugfix：优化 GitHub Skill 安装速度并修复多行 Frontmatter 渲染

## 问题 1：从 GitHub 仓库安装 Skill 很慢并最终超时

### 问题描述

从 GitHub 仓库安装某些 Skill 时，安装弹窗会长时间停留在加载状态，最后超时失败。典型表现：

- 弹窗持续显示「正在安装技能...」
- 日志提示正在执行文件/网络操作
- GitHub 网络较慢或仓库文件较多时更容易复现

### 根因分析

对于形如 `https://github.com/owner/repo/tree/branch/path` 的 GitHub 子目录 URL，原逻辑会优先使用 GitHub Contents API 递归下载目录。

该方式的问题是：

- 需要按目录递归请求 GitHub API
- 文件下载是串行 HTTP 请求
- 文件较多时请求数量快速增加
- API 下载失败后还会 fallback 到 `git clone`，导致慢路径被走两遍

因此在网络不稳定或仓库较大时，用户会看到长时间转圈后超时。

### 修复方案

新增 `clone_or_pull_sparse`，对 GitHub 子目录安装优先使用系统 `git` 执行浅克隆 + 稀疏检出：

```bash
git clone --depth 1 --filter=blob:none --sparse --no-tags ...
git sparse-checkout set --no-cone <subpath>
```

这样只检出目标 Skill 子目录，避免下载整个仓库或逐文件调用 GitHub API。

修复后流程：

- 子目录 GitHub URL：优先走 sparse checkout
- sparse checkout 失败：再 fallback 到 GitHub Contents API
- 更新已安装 Skill：如果记录了 `source_subpath`，同样优先走 sparse checkout
- 缓存 key 加入 `subpath`，避免不同子目录复用同一个稀疏工作区造成冲突

### 影响范围

该优化主要改善精确到子目录的安装链接，例如：

```text
https://github.com/anthropics/skills/tree/main/skills/frontend-design
```

如果用户输入的是仓库根 URL，应用仍需要先扫描仓库中的 Skill 候选项，该场景仍可能触发完整浅克隆。

## 问题 2：搜索/手动添加无法处理非标准 Skill 容器目录

### 问题描述

从探索页在线搜索安装 `technical-writer` 时，搜索结果来自 `skills.sh`：

```json
{
  "name": "technical-writer",
  "skillId": "technical-writer",
  "source": "shubhamsaboo/awesome-llm-apps"
}
```

前端只能拿到仓库根地址：

```text
https://github.com/Shubhamsaboo/awesome-llm-apps
```

实际 Skill 位于：

```text
awesome_agent_skills/technical-writer/SKILL.md
```

原扫描逻辑只识别固定目录（如 `skills/*`、`.claude/skills/*`）和根目录直接子目录，无法发现 `awesome_agent_skills/*` 这种容器目录下的 Skill，导致搜索安装失败或回退到错误提示。

手动添加也存在类似问题：如果输入容器目录链接，例如：

```text
https://github.com/Shubhamsaboo/awesome-llm-apps/blob/main/awesome_agent_skills
```

原逻辑会尝试把整个容器目录当成 Skill 安装，导致 Hub 中出现大量子目录而不是一个有效 Skill。

### 根因分析

问题分为两层：

- 搜索结果只提供仓库根 `source`，不包含实际 Skill 文件夹路径
- 后端发现逻辑依赖目录白名单，无法覆盖 `awesome_agent_skills`、`agent-skills`、`custom-agent-skills` 等变体
- 手动 `tree/blob` 子路径绕过候选发现，直接调用安装命令，因此容器目录不会进入 picker
- 安装入口缺少最终校验，子路径存在但不是有效 Skill 目录时也可能被复制进 Hub

### 修复方案

将 Git Skill 发现改为分层模型，避免继续堆具体目录名白名单：

1. 固定目录快速扫描：
   - `skills/*`
   - `skills/.curated/*`
   - `skills/.experimental/*`
   - `skills/.system/*`
   - `.claude/skills/*`
2. 根目录直接 Skill：
   - `repo/my-skill/SKILL.md`
3. 根目录 Skill 容器：
   - 只扫描根目录下名称包含 `skill` 的容器目录的一层子目录
   - 示例：`repo/awesome_agent_skills/technical-writer/SKILL.md`
   - 示例：`repo/custom-agent-skills/python-expert/SKILL.md`

该方案不是全仓库递归扫描，只多扫一层 `*skill*` 容器目录，性能可控。

同时调整手动添加流程：

- `tree/blob` 路径先调用 `list_git_skills_cmd` 做候选发现
- 路径本身是 Skill：返回 1 个候选并自动安装
- 路径是 Skill 容器：列出容器下一层候选，单个自动安装，多个弹 picker
- 完全找不到 Skill：才提示未找到可导入 Skill

并在安装入口增加最终校验：

- 复制源必须是有效 Skill 目录
- 有 `SKILL.md`，或是允许的 `.claude/skills/*` 目录
- 容器目录本身没有 `SKILL.md` 时，拒绝安装并提示粘贴具体 Skill 文件夹链接

另外保留对 `blob/.../SKILL.md` 的规范化：

```text
.../blob/main/awesome_agent_skills/technical-writer/SKILL.md
```

会转换为：

```text
awesome_agent_skills/technical-writer
```

## 问题 3：`description: |` 只渲染出一个 `|`

### 问题描述

部分 `SKILL.md` 使用 YAML block scalar 写多行描述：

```yaml
---
name: technical-writer
description: |
  Creates clear documentation, API references, guides, and
  technical content for developers and users.
author: awesome-llm-apps
---
```

详情页 Frontmatter 表格中只显示 `|`，后面的描述内容没有显示。

### 根因分析

前端详情页的 `parseFrontmatter` 和后端 `parse_skill_md` 都只支持简单的 `key: value` 单行解析。

当遇到 `description: |` 时：

- `description` 被解析成字面量 `|`
- 后续缩进的多行文本没有关联到 `description`
- 后端存入数据库的描述也可能变成 `|`

### 修复方案

前端和后端同时支持 YAML block scalar：

- `description: |`：保留换行，适合多行描述
- `description: >`：折叠为单段文本，适合普通段落

同时调整 Markdown 表格样式：

- `td` 使用 `white-space: pre-wrap` 保留多行文本
- 单元格顶部对齐
- 长文本允许换行，避免撑破布局

## 验证

已完成以下验证：

- `npm run build` 通过
- `cargo test -q` 通过，`79 passed`
- `cargo fmt --all -- --check` 通过
- 新增 Rust 测试覆盖 `description: |` 解析
- 新增 Rust 测试覆盖 `blob/.../SKILL.md` 路径规范化
- 新增 Rust 测试覆盖 `*skill*` 容器目录发现、非 skill 容器跳过、候选去重
- 新增 Rust 测试覆盖容器目录拒绝安装、容器下具体子 Skill 可安装
- 使用真实 GitHub 仓库验证 sparse checkout 可在约 2 秒内检出目标子目录

## 修改文件

- `src-tauri/src/core/git_fetcher.rs`：新增 `clone_or_pull_sparse`
- `src-tauri/src/core/installer.rs`：GitHub 子目录安装和更新优先使用 sparse checkout；补充 block scalar 解析；新增分层 Skill 发现和安装前校验
- `src-tauri/src/core/github_download.rs`：避免将根目录 `.` 走 GitHub Contents API 子目录下载路径
- `src-tauri/src/core/tests/git_fetcher.rs`：新增 sparse checkout 测试
- `src-tauri/src/core/tests/installer.rs`：新增 `description: |`、路径规范化、分层发现、容器目录校验相关测试
- `src/App.tsx`：手动 `tree/blob` 路径改为先发现候选，再自动安装或弹出选择器
- `src/components/skills/SkillDetailView.tsx`：前端 Frontmatter 解析支持 `|` 和 `>`
- `src/App.css`：修复 Frontmatter 表格中多行描述的展示样式
