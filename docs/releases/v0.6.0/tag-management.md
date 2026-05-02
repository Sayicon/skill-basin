# Skill 标签管理功能设计与实现计划

## 背景

随着用户安装的 Skill 数量增加，当前列表主要依赖名称、描述和同步状态浏览。用户很难按技术方向或使用场景快速定位 Skill，例如：

- 前端 / React / UI
- Rust / Tauri
- 文档 / 图表 / 自动化
- 测试 / 代码审查

v0.6.0 聚焦解决这个问题：为 Skill 增加**自定义标签**能力，用于整理和筛选 Skill。

关联需求：

- GitHub Issue #15：添加 tag 功能。

---

## 本期目标

v0.6.0 只实现标签功能。

标签用于：

- 给 Skill 添加一个或多个自定义标签。
- 在 My Skills 页面按标签筛选 Skill。
- 找出还没有设置标签的 Skill。
- 通过独立的 Tags 页面查看和编辑标签。

标签不用于：

- 控制 Skill 是否同步。
- 控制 Skill 同步到哪些工具。
- 作为一套可切换的工作配置。

一句话定义：

```text
Tag 用于找 Skill，不用于改变 Skill 的生效范围。
```

---

## 产品规则

### 1. Skill 可以没有标签

没有标签是合法状态。

系统提供虚拟筛选项：

```text
Untagged
```

含义：

```text
显示没有任何标签的 Skill。
```

`Untagged` 不是用户创建的真实标签，不能重命名、删除，也不写入标签表。

### 2. Skill 可以有多个标签

一个 Skill 可以关联多个标签：

```text
frontend-design: Frontend, Docs, UI
```

同一个 Skill 下不能重复添加同一个标签。

### 3. 标签筛选支持多选

My Skills 页面提供 `Tags` 下拉筛选器：

```text
[All ▾] [Most recent ⇅] [Tags ▾] [Search skills...] [Refresh]
```

下拉内容：

```text
Tags                              Match any

[Search tags...]

[ ] Untagged                  5
[✓] Frontend                  8
[✓] React                     3
[ ] Rust                      2
[ ] Docs                      6

Clear all
```

筛选逻辑使用 `OR`：

```text
选择 Frontend + Docs
```

显示包含 `Frontend` 或 `Docs` 的 Skill。

### 4. 筛选即时生效

标签筛选只影响列表展示，不修改数据，因此不需要 `Apply`。

交互规则：

- 勾选标签后，Skill 列表立即更新。
- 取消勾选后，Skill 列表立即更新。
- 下拉保持打开，方便连续选择。
- 点击外部关闭下拉。
- `Clear all` 立即清空标签筛选。

### 5. 标签页面是独立入口

标签不放在 Settings 中，避免入口过深。

入口：

- 顶部主导航的 `Tags` / `标签`
- My Skills 页面标签筛选区的辅助入口（保留一个快速跳转点）

页面层级：

```text
Tags
```

页面内容：

```text
Tags

标签用于筛选和整理 skills，不会改变同步结果。

5 skills have no tags                         [Review]

[Search tags...]                              [+ New Tag]

Tag name        Skills      Last used       Actions
Frontend        8           2d ago          View  Rename  Delete
Docs            6           5d ago          View  Rename  Delete
Rust            2           9d ago          View  Rename  Delete
```

`Review` 点击后回到 My Skills，并应用 `Untagged` 筛选。

`View` 点击后回到 My Skills，并应用对应标签筛选。

---

## UI 行为

### Skill 卡片

Skill 卡片展示最多 2-3 个标签，避免挤压工具同步状态。

有标签：

```text
frontend-design    #Frontend #Docs
```

无标签：

```text
legacy-shell-helper
```

没有标签时不展示空文案，只保留标签编辑入口。

### 标签编辑

每个 Skill 需要有入口编辑标签。

当前实现使用 Skill 卡片右侧的标签图标按钮作为快捷入口，点击后打开标签编辑。

编辑内容：

```text
Edit Tags: frontend-design

[✓] Frontend
[✓] Docs
[ ] Rust
[ ] Testing

[Done]
```

### 新建标签

新建标签入口在 `Tags` 页面。

规则：

- 标签名不能为空。
- 标签名去除首尾空格。
- 标签名大小写不敏感去重。
- 新建后出现在标签管理表和筛选下拉中。

### 重命名标签

重命名标签会更新所有关联 Skill 的标签展示。

规则：

- 不能重命名为已存在标签。
- 重命名后保留原有关联关系。
- 如果当前正在按旧标签筛选，筛选条件同步切换为新标签名。

### 删除标签

删除标签只删除标签和关联关系，不删除 Skill。

删除前需要确认：

```text
Delete "Docs" from 6 skills?
This only removes the tag, not the skills.
```

删除后：

- 标签从所有 Skill 上移除。
- 如果某些 Skill 因此没有任何标签，它们会进入 `Untagged`。

---

## 数据模型

新增两张表：

```sql
CREATE TABLE skill_tags (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE skill_tag_links (
  skill_id INTEGER NOT NULL,
  tag_id INTEGER NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (skill_id, tag_id),
  FOREIGN KEY (skill_id) REFERENCES skills(id) ON DELETE CASCADE,
  FOREIGN KEY (tag_id) REFERENCES skill_tags(id) ON DELETE CASCADE
);
```

约束：

- `skill_tags.name` 唯一。
- `skill_tag_links` 使用 `(skill_id, tag_id)` 主键，防止同一 Skill 重复关联同一标签。
- `Untagged` 不入库，由 `NOT EXISTS skill_tag_links` 动态计算。

迁移：

- 升级数据库 schema 版本。
- 老用户升级后，既有 Skill 默认没有标签。
- 不自动生成标签，避免误分类。

---

## 后端能力

建议新增 core 方法：

```rust
create_tag(name)
rename_tag(tag_id, name)
delete_tag(tag_id)
list_tags_with_counts()
set_skill_tags(skill_id, tag_ids)
get_skill_tags(skill_id)
list_untagged_skill_ids()
```

建议新增 Tauri commands：

```text
get_tags
create_tag
rename_tag
delete_tag
set_skill_tags
get_skill_tags
```

`get_managed_skills` 返回的 Skill DTO 增加：

```ts
tags: TagDto[]
```

`TagDto`：

```ts
type TagDto = {
  id: number;
  name: string;
};
```

标签筛选第一版可在前端完成，不需要新增服务端筛选参数。

---

## 前端改动

主要涉及：

- `src/App.tsx`
- `src/components/skills/types.ts`
- `src/components/skills/FilterBar.tsx`
- `src/components/skills/SkillCard.tsx`
- 新增 Tags 页面组件
- 新增标签编辑弹窗或详情页 Tags 区块
- `src/i18n/resources.ts`
- `src/App.css`

### 状态

新增状态：

```ts
tags
selectedTagIds
tagSearch
tagManagerSearch
tagEditorSkill
```

### 筛选

前端筛选逻辑：

```text
selectedTags 为空 -> 显示全部
selectedTags 包含普通标签 -> 命中任意一个标签
selectedTags 包含 Untagged -> skill.tags.length === 0
```

### i18n

所有用户可见文案需要提供英文和中文：

- Tags
- Untagged
- New Tag
- Rename
- Delete
- Review
- Clear all
- Match any

---

## 测试重点

### 后端

- 新建标签成功。
- 重复标签名被拒绝。
- 重命名标签保留关联关系。
- 删除标签后关联关系清理。
- 同一 Skill 不能重复关联同一标签。
- 删除 Skill 后标签关联被清理。
- `Untagged` 统计正确。

### 前端

- 多选标签即时筛选。
- `Untagged` 筛选正确。
- `Clear all` 清空筛选。
- 顶部 `Tags` 进入标签页面。
- Tags 页面 `Review` 能筛选无标签 Skill。
- Tags 页面 `View` 能筛选指定标签。
- 无标签 Skill 不显示空文案。
- 编辑标签后卡片和筛选结果刷新。
- 在添加 Skill 弹窗中可直接选择标签，创建后自动绑定。

---

## 发布范围

v0.6.0 包含：

- 自定义标签数据模型。
- 标签 CRUD。
- Skill 标签关联编辑。
- My Skills 标签多选筛选。
- `Untagged` 虚拟筛选项。
- 独立 Tags 页面。
- 添加 Skill 时选择标签。
- 中英文文案。
- 后端和前端测试。

v0.6.0 不包含：

- 标签自动推荐。
- 批量给多个 Skill 添加标签。
