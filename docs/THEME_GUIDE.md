# 明暗色主题功能

## 功能概述

DataGuard Scanner v1.0.2 新增了完整的明暗色主题系统，提供以下特性：

- 🌓 **三种主题模式**：浅色、深色、跟随系统
- ⚡ **快速切换**：工具栏一键切换，即时生效
- 💾 **持久化保存**：自动保存偏好，重启后恢复
- 🎨 **全局适配**：所有组件完美支持主题切换
- 🔄 **自动响应**：跟随系统模式实时响应系统主题变化

## 实现细节

### 1. 主题管理工具 (`src/utils/theme.ts`)

创建了专门的主题管理模块，提供以下功能：

- `ThemeMode` 类型定义：`'light'` | `'dark'` | `'system'`
- `getSystemTheme()`：获取当前系统主题偏好
- `applyTheme(mode)`：应用主题到 DOM
- `loadTheme()`：从 localStorage 加载保存的主题
- `watchSystemTheme(callback)`：监听系统主题变化

### 2. CSS 变量系统 (`src/style.css`)

定义了完整的 CSS 变量体系：

**浅色主题（默认）：**
```css
--bg-color: #fff
--text-color: #333
--menu-bg: #f0f0f0
--toolbar-bg: #fafafa
--sidebar-bg: white
--modal-bg: white
--input-bg: white
--border-color: #d9d9d9
--bg-hover: #f5f5f5
--bg-selected: #e6f7ff
```

**深色主题：**
```css
--bg-color: #141414
--text-color: #e8e8e8
--menu-bg: #1f1f1f
--toolbar-bg: #1f1f1f
--sidebar-bg: #1f1f1f
--modal-bg: #1f1f1f
--input-bg: #141414
--border-color: #434343
--bg-hover: #1f1f1f
--bg-selected: #111d2c
```

### 3. 主题切换 UI

在工具栏添加了主题切换按钮：
- ☀️ 浅色主题
- 🌙 深色主题
- 💻 跟随系统

点击按钮会在三种模式间循环切换，并显示相应的提示文本。

### 4. 设置页面集成

在设置模态框中添加了"外观设置"部分，用户可以在这里选择主题模式。

### 5. 组件适配

已更新以下所有组件以支持主题切换：

- ✅ App.vue（主应用容器）
- ✅ SettingsModal.vue（设置窗口）
- ✅ PreviewModal.vue（预览窗口）
- ✅ LogsModal.vue（日志窗口）
- ✅ AboutModal.vue（关于窗口）
- ✅ ExportModal.vue（导出窗口）
- ✅ EnvironmentCheck.vue（环境检查）
- ✅ DirectoryTree.vue（目录树）
- ✅ ResultsTable.vue（结果表格）
- ✅ FileTypeFilter.vue（文件类型筛选）
- ✅ TreeNode.vue（树节点）

### 6. 初始化流程

在 `main.ts` 中，应用启动时会：
1. 从 localStorage 加载保存的主题设置
2. 立即应用主题到 DOM
3. 确保页面渲染前就设置了正确的主题

在 `App.vue` 中：
1. 再次确认主题设置
2. 注册系统主题变化监听器
3. 当选择"跟随系统"时，自动响应系统主题变化

## 使用方法

### 方法一：工具栏快速切换

点击工具栏最右侧的主题按钮（☀️/🌙/💻），在三种模式间循环切换。

### 方法二：设置页面

1. 点击菜单栏的"设置"或工具栏的"设置"按钮
2. 在"外观设置"部分选择主题模式
3. 点击"保存"应用设置

## 技术特点

1. **无闪烁切换**：在应用初始化时就应用主题，避免页面闪烁
2. **平滑过渡**：使用 CSS transition 实现平滑的颜色过渡
3. **系统响应**：选择"跟随系统"时，实时响应操作系统主题变化
4. **持久化**：主题选择保存在 localStorage，重启后自动恢复
5. **可扩展**：基于 CSS 变量的设计，易于添加新的主题或调整颜色

## 注意事项

1. 所有硬编码的颜色值都已替换为 CSS 变量
2. 高亮样式（mark 标签）保持固定颜色，以确保可读性
3. 某些渐变色（如环境检查头部）保持原样，在两种主题下都表现良好
4. 主题切换是即时的，无需刷新页面

## 未来优化建议

1. 可以添加更多自定义主题选项（如蓝色主题、绿色主题等）
2. 可以为特定组件添加更精细的主题控制
3. 可以添加主题切换动画效果
4. 可以记录用户的主题使用偏好并提供智能推荐
