# DataGuard Scanner - 敏感数据扫描工具

<div align="center">

![Version](https://img.shields.io/badge/version-1.0.1-blue.svg)
![License](https://img.shields.io/badge/license-MIT-green.svg)
![Tauri](https://img.shields.io/badge/Tauri-2.x-FFC131.svg)
![Vue](https://img.shields.io/badge/Vue-3.x-4FC08D.svg)
![Rust](https://img.shields.io/badge/Rust-2024-orange.svg)

**一款强大的跨平台敏感数据检测工具，帮助您快速发现和定位文件中的隐私信息**

[功能特性](#功能特性) • [技术栈](#技术栈) • [安装指南](#安装指南) • [使用说明](#使用说明) • [开发指南](#开发指南)

</div>

---

## 📖 项目简介

DataGuard Scanner 是一款基于 Tauri 2.x 和 Vue 3 构建的跨平台桌面应用程序，专门用于扫描和检测文件系统中的敏感数据。它能够智能识别身份证号、手机号、邮箱、银行卡号、地址、IP 地址和密码等隐私信息，并提供可视化的高亮预览和报告导出功能。

### 核心优势

- 🔍 **智能检测**：采用正则表达式 + 校验算法（Luhn、身份证校验码）确保准确性
- ⚡ **高性能**：基于 Rust 后端，支持并发扫描，处理大文件高效稳定
- 🎯 **多格式支持**：支持文本文件、PDF、Excel、Word、PowerPoint 等多种格式
- 🌐 **跨平台**：完美支持 Windows、macOS 和 Linux
- 📊 **可视化报告**：支持 CSV、JSON、Excel 三种格式导出扫描结果
- 🔒 **安全可靠**：本地运行，数据不上传，保护隐私安全

---

## ✨ 功能特性

### 1. 敏感数据类型检测

| 类型 | 说明 | 默认启用 | 校验方式 |
|------|------|---------|---------|
| 🆔 身份证号 | 18位中国居民身份证 | ✅ | 校验码 + 日期验证 |
| 📱 手机号 | 中国大陆11位手机号 | ✅ | 号段验证 + 边界检查 |
| 📧 电子邮箱 | 标准邮箱格式 | ✅ | 正则匹配 |
| 💳 银行卡号 | 借记卡/信用卡 | ✅ | Luhn算法校验 |
| 🏠 地址 | 中国行政区划地址 | ✅ | 严格模式匹配 |
| 🌐 IPv4地址 | IP地址格式 | ✅ | 范围验证(0-255) |
| 🔑 密码 | password/pwd等关键词 | ✅ | 模式匹配 |
| 👤 中文姓名 | 2-4个连续汉字 | ❌ | 正则匹配（易误报） |

### 2. 文件格式支持

#### 文本文件
- 基础格式：`.txt`, `.log`, `.md`, `.ini`, `.conf`, `.cfg`, `.env`
- 代码文件：`.js`, `.ts`, `.py`, `.java`, `.c`, `.cpp`, `.go`, `.rs`, `.php`, `.rb`, `.swift`
- 配置文件：`.csv`, `.json`, `.xml`, `.yaml`, `.yml`, `.properties`, `.toml`

#### 文档文件
- PDF 文档（使用 `pdf-extract` 库解析）
- Excel 表格（使用 `calamine` 库解析）
- Word 文档（实验性支持）
- PowerPoint 演示文稿（实验性支持）

### 3. 核心功能

#### 🗂️ 目录树浏览
- 懒加载目录结构，性能优化
- 支持显示/隐藏隐藏文件
- 智能路径选择（父子节点自动处理）
- 全选/全不选快捷操作

#### 🔎 智能扫描
- 自定义扫描路径（支持多选）
- 文件类型筛选（按扩展名）
- 文件大小限制（普通文件 50MB，PDF 100MB）
- 并发扫描（默认8线程，可配置）
- 实时进度显示
- 支持取消扫描

#### 👁️ 文件预览
- 内容高亮显示敏感数据
- 不同颜色标识不同类型
- 字符边界安全截断（避免乱码）
- 支持取消长时间预览

#### 📈 结果管理
- 表格展示扫描结果
- 按文件路径、大小、修改时间排序
- 统计各类敏感数据数量
- 移除误报结果

#### 📤 报告导出
- **CSV 格式**：通用表格格式，可用 Excel 打开
- **JSON 格式**：结构化数据，便于程序处理
- **Excel 格式**：带样式和颜色高亮的专业报告

#### 🗑️ 文件删除
- 移入回收站（可恢复）
- 永久删除（不可恢复）
- 可配置默认行为

#### ⚙️ 配置管理
- 自动保存用户配置
- 主题设置（系统/浅色/深色）
- 语言设置（中文/英文）
- 忽略目录配置
- 系统目录排除
- 实验性功能开关

#### 🛡️ 环境检查
- 启动时自动检测系统环境
- Windows：WebView2、VC++ Redistributable
- macOS：版本兼容性检查
- Linux：依赖库检查（WebKit2GTK、GTK3、libsoup）
- 提供详细解决方案和下载链接

#### 📝 日志系统
- 实时记录扫描过程
- 错误和警告信息捕获
- 可查看历史日志

---

## 🛠️ 技术栈

### 前端技术
- **框架**：Vue 3 (Composition API)
- **状态管理**：Pinia
- **构建工具**：Vite 6.x
- **语言**：TypeScript 5.x
- **UI**：原生 CSS（无第三方 UI 库）

### 后端技术
- **框架**：Tauri 2.x
- **语言**：Rust 2024 Edition
- **异步运行时**：Tokio
- **序列化**：Serde + serde_json

### 核心依赖库

| 库名 | 版本 | 用途 |
|------|------|------|
| `tauri` | 2.x | 桌面应用框架 |
| `tokio` | 1.x | 异步运行时 |
| `regex` | 1.x | 正则表达式引擎 |
| `walkdir` | 2.x | 目录遍历 |
| `pdf-extract` | 0.10 | PDF 文本提取 |
| `calamine` | 0.34 | Excel/Office 解析 |
| `rust_xlsxwriter` | 0.94 | Excel 文件写入 |
| `trash` | 5.x | 文件回收站操作 |
| `open` | 5.x | 打开文件/URL |
| `encoding_rs` | 0.8 | 编码转换 |
| `chrono` | 0.4 | 时间处理 |
| `dirs` | 6.x | 系统目录获取 |

### 包管理器
- **前端**：pnpm（推荐）或 npm
- **后端**：Cargo

---

## 📦 安装指南

### 系统要求

#### Windows
- Windows 7 SP1 或更高版本（推荐 Windows 10/11）
- WebView2 Runtime（Windows 10/11 通常已预装）
- Visual C++ Redistributable 2015+

#### macOS
- macOS 10.15 (Catalina) 或更高版本
- 系统自带 WebKit

#### Linux
- WebKit2GTK 4.1+
- GTK 3+
- libsoup 3+

**Ubuntu/Debian 安装依赖：**
```bash
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libsoup-3.0-dev \
  libjavascriptcoregtk-4.1-dev libayatana-appindicator3-dev \
  librsvg2-dev
```

**Fedora 安装依赖：**
```bash
sudo dnf install webkit2gtk4.1-devel gtk3-devel libsoup3-devel \
  javascriptcoregtk4.1-devel libappindicator-gtk3-devel librsvg2-devel
```

### 从源码构建

#### 前置条件
1. 安装 [Rust](https://www.rust-lang.org/tools/install)（最新稳定版）
2. 安装 [Node.js](https://nodejs.org/)（22+ 推荐）
3. 安装 [pnpm](https://pnpm.io/installation)

```bash
# 安装 pnpm
npm install -g pnpm
```

#### 构建步骤

```bash
# 1. 克隆仓库
git clone https://github.com/your-org/DataGuardScanner.git
cd DataGuardScanner

# 2. 安装前端依赖
cd frontend
pnpm install

# 3. 返回项目根目录
cd ..

# 4. 开发模式运行
pnpm dev

# 5. 生产模式构建
pnpm build
```

#### 构建安装包

```bash
# 进入 Tauri 目录
cd src-tauri

# 构建安装包（根据系统生成对应格式）
cargo tauri build
```

生成的安装包位于 `src-tauri/target/release/bundle/`：
- **Windows**: `.msi` 或 `.exe`
- **macOS**: `.dmg` 或 `.app`
- **Linux**: `.deb` 或 `.AppImage`

### 直接下载安装

访问 [Releases 页面](https://github.com/your-org/DataGuardScanner/releases) 下载最新版本的安装包。

---

## 📖 使用说明

### 快速开始

1. **启动应用**
   ```bash
   pnpm dev
   ```

2. **选择扫描路径**
   - 在左侧目录树中勾选要扫描的文件夹
   - 支持多选和全选
   - 智能处理父子路径关系

3. **配置扫描选项**
   - 点击顶部菜单栏"设置"
   - 选择要检测的敏感数据类型
   - 配置文件类型过滤器
   - 调整文件大小限制和并发数

4. **开始扫描**
   - 点击"开始扫描"按钮
   - 实时查看扫描进度
   - 可随时点击"取消"停止扫描

5. **查看结果**
   - 右侧表格显示包含敏感数据的文件
   - 双击文件行预览内容
   - 敏感信息会以不同颜色高亮显示

6. **导出报告**
   - 点击"导出报告"按钮
   - 选择格式（CSV/JSON/Excel）
   - 选择保存路径

### 高级功能

#### 忽略目录配置

在设置中可以配置两类忽略规则：

1. **忽略目录名**（任意位置）
   - 例如：`node_modules`, `.git`, `.vscode`
   - 所有同名目录都会被跳过

2. **系统目录**（完整路径）
   - 例如：`C:\Windows`, `/Applications`
   - 只在特定位置忽略

#### 文件预览

- 点击文件行即可预览
- 敏感数据用不同颜色标识：
  - 🔴 红色：身份证号
  - 🟠 橙色：手机号
  - 🟡 黄色：邮箱
  - 🟢 绿色：银行卡号
  - 🔵 蓝色：地址
  - 🟣 紫色：IP地址
  - ⚫ 黑色：密码

#### 文件删除

- 右键点击文件行
- 选择"删除文件"
- 根据配置移入回收站或永久删除

#### 日志查看

- 点击菜单栏"查看日志"
- 查看扫描过程中的详细信息
- 帮助排查问题

---

## 🔧 开发指南

### 项目结构

```
DataGuardScanner/
├── frontend/                 # 前端 Vue 应用
│   ├── src/
│   │   ├── components/      # Vue 组件
│   │   │   ├── DirectoryTree.vue      # 目录树
│   │   │   ├── FileTypeFilter.vue     # 文件类型筛选
│   │   │   ├── ResultsTable.vue       # 结果表格
│   │   │   ├── PreviewModal.vue       # 预览弹窗
│   │   │   ├── SettingsModal.vue      # 设置弹窗
│   │   │   ├── ExportModal.vue        # 导出弹窗
│   │   │   ├── LogsModal.vue          # 日志弹窗
│   │   │   └── AboutModal.vue         # 关于弹窗
│   │   ├── stores/          # Pinia 状态管理
│   │   │   └── app.ts
│   │   ├── types/           # TypeScript 类型定义
│   │   │   └── index.ts
│   │   ├── utils/           # 工具函数
│   │   │   ├── format.ts    # 格式化函数
│   │   │   └── tauri-api.ts # Tauri API 封装
│   │   ├── App.vue          # 主应用组件
│   │   └── main.ts          # 入口文件
│   ├── package.json
│   └── vite.config.ts
│
├── src-tauri/               # Tauri Rust 后端
│   ├── src/
│   │   ├── main.rs          # 主入口
│   │   ├── commands.rs      # Tauri 命令
│   │   ├── models.rs        # 数据模型
│   │   ├── scanner.rs       # 扫描引擎
│   │   ├── file_parser.rs   # 文件解析器
│   │   ├── sensitive_detector.rs  # 敏感数据检测
│   │   └── environment.rs   # 环境检查
│   ├── Cargo.toml
│   └── tauri.conf.json      # Tauri 配置
│
├── scripts/                 # 辅助脚本
│   └── update-version.js    # 版本号更新脚本
│
├── package.json             # 根级别 npm 脚本
└── README.md
```

### 开发工作流

#### 开发模式

```bash
# 启动开发服务器（热重载）
pnpm dev
```

这会同时启动：
- 前端 Vite 开发服务器（http://localhost:1420）
- Tauri 应用窗口

#### 代码规范

**Rust 代码：**
```bash
# 格式化代码
cargo fmt

# 检查代码质量
cargo clippy

# 运行测试
cargo test
```

**TypeScript 代码：**
```bash
cd frontend

# 类型检查
pnpm exec vue-tsc --noEmit

# 格式化（如果配置了 Prettier）
pnpm exec prettier --write "src/**/*.{ts,vue}"
```

### 添加新的敏感数据类型

1. **在 `sensitive_detector.rs` 中添加规则：**

```rust
SensitiveRuleDef {
    id: "new_type",
    name: "新类型名称",
    pattern: r"你的正则表达式",
    enabled_by_default: true,
}
```

2. **在前端类型定义中添加：**

```typescript
// frontend/src/types/index.ts
enabled_sensitive_types: string[]  // 已支持，无需修改
```

3. **更新 UI 显示：**

在 `PreviewModal.vue` 中添加对应的颜色样式。

### 添加新的文件格式支持

1. **在 `file_parser.rs` 中实现解析函数：**

```rust
pub fn extract_text_from_new_format(path: &Path) -> Result<String, String> {
    // 实现解析逻辑
}
```

2. **在 `extract_text_from_file` 中添加分支：**

```rust
match extension {
    "newext" => extract_text_from_new_format(path),
    // ...
}
```

3. **更新配置默认值：**

在 `models.rs` 的 `AppConfig::default()` 中添加新扩展名。

### 版本号管理

```bash
# 更新版本号（自动同步 frontend/package.json 和 src-tauri/Cargo.toml）
npm run version:update 1.0.2
```

---

## 🧪 测试

### 运行后端测试

```bash
cd src-tauri
cargo test
```

关键测试用例包括：
- 身份证号校验（日期、校验码）
- 银行卡号 Luhn 算法
- 手机号边界检查
- IP 地址范围验证
- 地址严格匹配

### 前端测试

```bash
cd frontend
pnpm test  # 如果配置了测试框架
```

---

## 📊 性能优化

### 已实现的优化

1. **并发扫描**：使用 Tokio 异步运行时，默认 8 线程并发
2. **懒加载目录树**：只加载展开的目录节点
3. **文件大小限制**：跳过大文件，避免内存溢出
4. **预览取消机制**：支持取消长时间的预览任务
5. **智能路径去重**：避免重复扫描父子路径
6. **字符边界截断**：预览时避免破坏多字节字符

### 调优建议

- **并发数调整**：根据 CPU 核心数调整 `scan_concurrency`
- **文件大小限制**：根据实际需求调整 `max_file_size_mb`
- **PDF 单独限制**：PDF 解析较慢，单独设置 `max_pdf_size_mb`

---

## 🔐 安全说明

### 数据处理
- ✅ 所有扫描在本地完成，数据不会上传
- ✅ 配置文件存储在本地
- ✅ 不使用网络通信（除环境检查时的下载链接）

### 权限需求
- **文件系统读取**：扫描选定目录
- **文件系统写入**：保存配置和导出报告
- **删除文件**：用户主动触发的删除操作

### 注意事项
- ⚠️ 扫描系统目录可能导致性能下降
- ⚠️ 大文件扫描会占用较多内存
- ⚠️ 建议排除 `node_modules` 等大型目录

---

## 🤝 贡献指南

欢迎贡献代码、报告问题或提出建议！

### 提交 Issue
1. 搜索现有 Issue，避免重复
2. 提供详细的复现步骤
3. 附上系统环境和版本信息
4. 如有可能，提供截图或日志

### 提交 PR
1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交更改 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 开启 Pull Request

### 代码规范
- Rust 代码遵循 `rustfmt` 和 `clippy` 建议
- TypeScript 代码通过 `vue-tsc` 类型检查
- 提交前确保所有测试通过

---

## 📝 更新日志

### v1.0.1 (当前版本)
- ✅ 支持 PDF、Excel、Word、PowerPoint 文档解析
- ✅ 添加身份证号、银行卡号校验算法
- ✅ 优化地址匹配，减少误报
- ✅ 支持 Excel 格式报告导出（带样式）
- ✅ 添加系统环境检查功能
- ✅ 支持文件移入回收站
- ✅ 优化目录树懒加载性能
- ✅ 修复预览取消机制

### v1.0.0
- 🎉 初始版本发布
- 基础的敏感数据扫描功能
- 支持文本文件解析
- CSV/JSON 报告导出

---

## 📄 许可证

本项目采用 MIT 许可证 - 详见 [LICENSE](LICENSE) 文件

---

## 🙏 致谢

感谢以下开源项目的支持：

- [Tauri](https://tauri.app/) - 优秀的跨平台桌面应用框架
- [Vue.js](https://vuejs.org/) - 渐进式 JavaScript 框架
- [pdf-extract](https://crates.io/crates/pdf-extract) - PDF 文本提取库
- [calamine](https://crates.io/crates/calamine) - Excel/Office 解析库
- [regex](https://crates.io/crates/regex) - Rust 正则表达式引擎

---

## 📞 联系方式

- 📧 Email: yifengjob@qq.com
- 🐛 Issues: [GitHub Issues](https://github.com/your-org/DataGuardScanner/issues)

---

<div align="center">

**⭐ 如果这个项目对您有帮助，请给我一个 Star！**

Made with ❤️ by YiFeng

</div>
