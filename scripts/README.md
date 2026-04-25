# 版本管理说明

## 📦 统一版本管理

本项目使用自动化脚本来统一管理版本号，避免手动修改多个文件。

### 需要更新的文件

版本号需要同步更新以下文件：
- `package.json` (根目录)
- `frontend/package.json`
- `src-tauri/Cargo.toml`
- `src-tauri/tauri.conf.json` (自动从 package.json 读取)

### 使用方法

#### 方式一：使用 npm 脚本（推荐）

```bash
pnpm version:update <新版本号>
```

例如：
```bash
pnpm version:update 0.2.0
```

#### 方式二：直接运行脚本

```bash
node scripts/update-version.js <新版本号>
```

例如：
```bash
node scripts/update-version.js 0.2.0
```

### 版本号格式

版本号必须符合语义化版本规范：`x.y.z`
- x: 主版本号（重大变更）
- y: 次版本号（新功能）
- z: 修订号（bug 修复）

示例：
- ✅ `0.1.1`
- ✅ `1.0.0`
- ✅ `2.3.14`
- ❌ `0.1`
- ❌ `v1.0.0`
- ❌ `1.0.0-beta`

### 更新流程

1. **更新版本号**
   ```bash
   pnpm version:update 0.2.0
   ```

2. **验证更改**
   ```bash
   git diff
   ```

3. **提交更改**
   ```bash
   git add .
   git commit -m "chore: bump version to 0.2.0"
   ```

4. **创建 Git 标签**
   ```bash
   git tag v0.2.0
   ```

5. **推送**
   ```bash
   git push && git push --tags
   ```

### 注意事项

- ⚠️ 运行脚本前请确保已提交所有未提交的更改
- ⚠️ 版本号一旦发布不应修改，应递增版本号
- ⚠️ Tauri 构建时会使用 `tauri.conf.json` 中的版本号，该配置已设置为从 `package.json` 自动读取

### 故障排除

如果脚本执行失败：
1. 检查 Node.js 是否已安装：`node --version`
2. 检查文件权限：`ls -la scripts/update-version.js`
3. 手动检查文件格式是否正确

### 技术实现

- `tauri.conf.json` 使用 `"version": "../package.json"` 从根 package.json 读取版本
- 脚本会自动同步更新所有相关的配置文件
- 支持 JSON 和 TOML 格式的配置文件
