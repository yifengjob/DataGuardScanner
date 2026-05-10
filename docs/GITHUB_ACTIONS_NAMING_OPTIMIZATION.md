# GitHub Actions 产物命名规范化 - 修改记录

## 📋 修改概述

将 GitHub Actions 构建产物的命名规则统一为：
- **Windows**: `产品-平台-版本-架构-Setup/Portable.ext`
- **Linux/macOS**: `产品-平台-版本-架构.ext`

---

## ✅ 完成的修改

### 1. 新增版本信息提取步骤（第 59-68 行）

```yaml
- name: Extract version and product info
  id: info
  shell: bash
  run: |
    VERSION=$(cat package.json | grep '"version"' | head -1 | sed 's/.*: "\(.*\)".*/\1/')
    PRODUCT_NAME=$(cat src-tauri/tauri.conf.json | grep '"productName"' | head -1 | sed 's/.*: "\(.*\)".*/\1/' | sed 's/ /-/g')
    echo "version=$VERSION" >> $GITHUB_OUTPUT
    echo "product_name=$PRODUCT_NAME" >> $GITHUB_OUTPUT
    echo "Product: $PRODUCT_NAME, Version: $VERSION"
```

**输出变量**：
- `${{ steps.info.outputs.version }}` → `1.0.5`
- `${{ steps.info.outputs.product_name }}` → `DataGuard-Scanner`

---

### 2. Matrix 配置调整（第 14-54 行）

#### 修改前：
```yaml
- os: windows-latest
  target: x86_64-pc-windows-msvc
  platform: windows-x64  # ← 合并了平台和架构
```

#### 修改后：
```yaml
- os: windows-latest
  target: x86_64-pc-windows-msvc
  platform: windows      # ← 只保留平台
  arch: x86_64           # ← 新增独立架构字段
```

**所有平台的架构命名**：
- Windows 64-bit: `arch: x86_64`
- Windows 32-bit: `arch: x86`
- Linux x86_64: `arch: x86_64`
- Linux ARM64: `arch: arm64`
- macOS Intel: `arch: x86_64`
- macOS ARM: `arch: arm64`

---

### 3. 条件判断更新

将所有 `startsWith(matrix.platform, 'windows')` 改为 `matrix.platform == 'windows'`

**影响的步骤**：
- Setup QEMU for ARM（第 98 行）
- Configure ARM64 cross-compilation（第 105 行）
- Install Tauri CLI（第 120 行）
- Install Linux dependencies（第 125 行）
- Build Windows Portable EXE（第 225 行）
- List build outputs（第 233 行）
- Upload Windows artifacts（第 243 行）
- Upload Linux artifacts（第 253 行）
- Upload macOS artifacts（第 264 行）
- Rename Windows Portable EXE（第 275 行）
- Upload Windows Portable EXE Artifact（第 290 行）

**ARM64 特殊判断**：
```yaml
# 修改前
if: matrix.platform == 'linux-arm64'

# 修改后
if: matrix.arch == 'arm64' && matrix.platform == 'linux'
```

---

### 4. Artifact 命名规范化

#### Windows 安装版（第 246 行）
```yaml
name: ${{ steps.info.outputs.product_name }}-${{ matrix.platform }}-${{ steps.info.outputs.version }}-${{ matrix.arch }}-Setup
# 示例：DataGuard-Scanner-windows-1.0.5-x86_64-Setup
```

#### Linux（第 256 行）
```yaml
name: ${{ steps.info.outputs.product_name }}-${{ matrix.platform }}-${{ steps.info.outputs.version }}-${{ matrix.arch }}
# 示例：DataGuard-Scanner-linux-1.0.5-x86_64
```

#### macOS（第 267 行）
```yaml
name: ${{ steps.info.outputs.product_name }}-${{ matrix.platform }}-${{ steps.info.outputs.version }}-${{ matrix.arch }}
# 示例：DataGuard-Scanner-macos-1.0.5-x86_64
```

#### Windows Portable EXE（第 293 行）
```yaml
name: ${{ steps.info.outputs.product_name }}-${{ matrix.platform }}-${{ steps.info.outputs.version }}-${{ matrix.arch }}-Portable
# 示例：DataGuard-Scanner-windows-1.0.5-x86_64-Portable
```

---

### 5. Windows Portable EXE 重命名逻辑（第 274-287 行）

#### 修改前：
```powershell
$arch = if ('${{ matrix.target }}' -match 'x86_64') { 'x64' } else { 'x86' }
$newName = "dataguard-scanner-${arch}-portable.exe"
```

#### 修改后：
```powershell
$productName = "${{ steps.info.outputs.product_name }}"
$version = "${{ steps.info.outputs.version }}"
$arch = "${{ matrix.arch }}"
$newName = "${productName}-${{ matrix.platform }}-${version}-${arch}-Portable.exe"
```

**示例输出**：
- `DataGuard-Scanner-windows-1.0.5-x86_64-Portable.exe`
- `DataGuard-Scanner-windows-1.0.5-x86-Portable.exe`

---

### 6. Portable EXE 上传路径优化（第 294 行）

#### 修改前：
```yaml
path: |
  target/${{ matrix.target }}/release/dataguard-scanner-x64-portable.exe
  target/${{ matrix.target }}/release/dataguard-scanner-x86-portable.exe
```

#### 修改后：
```yaml
path: target/${{ matrix.target }}/release/*-Portable.exe
```

**优势**：使用通配符自动匹配，无需硬编码文件名

---

## 📊 最终产物命名示例

### Windows 平台

| 架构 | 安装版 Artifact | 便携版 Artifact |
|------|----------------|----------------|
| x86_64 | `DataGuard-Scanner-windows-1.0.5-x86_64-Setup` | `DataGuard-Scanner-windows-1.0.5-x86_64-Portable` |
| x86 | `DataGuard-Scanner-windows-1.0.5-x86-Setup` | `DataGuard-Scanner-windows-1.0.5-x86-Portable` |

**包含文件**：
- Setup: `*.exe` (NSIS), `*.msi` (MSI)
- Portable: `*-Portable.exe`

---

### Linux 平台

| 架构 | Artifact 名称 |
|------|--------------|
| x86_64 | `DataGuard-Scanner-linux-1.0.5-x86_64` |
| arm64 | `DataGuard-Scanner-linux-1.0.5-arm64` |

**包含文件**：
- `*.deb` (Debian/Ubuntu)
- `*.rpm` (RedHat/CentOS)
- `*.AppImage` (仅 x86_64)

---

### macOS 平台

| 架构 | Artifact 名称 |
|------|--------------|
| x86_64 | `DataGuard-Scanner-macos-1.0.5-x86_64` |
| arm64 | `DataGuard-Scanner-macos-1.0.5-arm64` |

**包含文件**：
- `*.dmg` (磁盘映像)
- `*.app` (应用程序包)

---

## ⚠️ 注意事项

### 1. JSON 解析方式
- 使用 `grep + sed` 而非 `jq`
- **原因**：避免安装额外工具导致构建失败
- **风险**：JSON 格式大幅变化时可能解析失败（概率极低）

### 2. Tauri 生成的安装包文件名
- **保持不变**：由 Tauri 自动生成（如 `DataGuard.Scanner_1.0.5_x64_en-US.msi`）
- **Artifact 名称已规范化**：下载时显示规范名称
- **用户可自行重命名**：如有需要

### 3. Release 步骤
- **无需修改**：使用通配符 `artifacts/**/*.exe` 自动匹配
- **兼容性**：无论 Artifact 如何命名，都能正确上传到 GitHub Release

---

## 🔍 验证清单

- ✅ YAML 语法正确
- ✅ 所有条件判断已更新
- ✅ Matrix 配置包含 arch 字段
- ✅ Artifact 命名符合规范
- ✅ Windows Portable EXE 重命名逻辑正确
- ✅ 向后兼容（Release 步骤使用通配符）

---

## 📝 备份文件

原始文件已备份至：`.github/workflows/build.yml.backup`

如需回滚：
```bash
cp .github/workflows/build.yml.backup .github/workflows/build.yml
```

---

## 🚀 测试建议

1. **手动触发工作流**：在 GitHub Actions 页面点击 "Run workflow"
2. **检查 Artifact 名称**：确认命名符合预期
3. **下载测试**：验证文件可以正常下载和使用
4. **Release 测试**：打标签触发完整流程

---

**修改完成时间**: 2026-04-30  
**修改人**: AI Assistant  
**审核状态**: 待测试验证
