<template>
  <div class="app-container">
    <!-- 菜单栏 -->
    <div class="menu-bar">
      <div class="menu-item" @click="showSettings = true">设置</div>
      <div 
        class="menu-item" 
        :class="{ disabled: scanResults.length === 0 }"
        @click="handleExportReport"
      >
        导出报告
      </div>
      <div class="menu-item" @click="showLogs = true">查看日志</div>
      <div class="menu-item" @click="showAbout = true">关于</div>
    </div>

    <!-- 工具栏 -->
    <div class="toolbar">
      <button 
        class="btn btn-primary" 
        @click="handleStartScan"
        :disabled="isScanning"
        title="开始扫描选中的目录"
      >
        <svg class="btn-icon" viewBox="0 0 16 16" xmlns="http://www.w3.org/2000/svg">
          <path fill="currentColor" fill-rule="evenodd" d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1m3.901 7L6 4.066v7.868z" clip-rule="evenodd" />
        </svg>
        <span>{{ isScanning ? '扫描中...' : '开始扫描' }}</span>
      </button>
      <button 
        class="btn btn-danger" 
        @click="handleCancelScan"
        :disabled="!isScanning"
        title="取消当前扫描任务"
      >
        <svg class="btn-icon" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
          <path fill="currentColor" fill-rule="evenodd" d="M256 469.334c117.821 0 213.334-95.513 213.334-213.334c0-117.82-95.513-213.333-213.334-213.333C138.18 42.667 42.667 138.18 42.667 256c0 117.821 95.513 213.334 213.333 213.334m-64-298.667h42.667v170.667H192zm85.334 0H320v170.667h-42.666z" />
        </svg>
        <span>取消</span>
      </button>
      <button 
        class="btn btn-icon-only" 
        @click="handleExportReport"
        :disabled="scanResults.length === 0"
        :title="scanResults.length === 0 ? '暂无扫描结果，无法导出' : '导出报告'"
      >
        <svg class="btn-icon" viewBox="0 0 32 32" xmlns="http://www.w3.org/2000/svg">
          <path fill="currentColor" d="M24.086 20.904c-1.805 3.113-5.163 5.212-9.023 5.22A10.45 10.45 0 0 1 4.625 15.688A10.45 10.45 0 0 1 15.063 5.25c3.86.007 7.216 2.105 9.022 5.218l3.962 2.284l.143.082C26.88 6.784 21.504 2.25 15.063 2.248C7.64 2.25 1.625 8.265 1.623 15.688c.003 7.42 6.018 13.435 13.44 13.437c6.442-.002 11.82-4.538 13.127-10.59l-.14.082zm4.314-5.216l-7.15-4.13v2.298H10.275v3.66H21.25v2.298z" />
        </svg>
      </button>
      <button 
        class="btn btn-icon-only" 
        @click="showSettings = true"
        title="打开设置"
      >
        <svg class="btn-icon" viewBox="0 0 1024 1024" xmlns="http://www.w3.org/2000/svg">
          <path fill="currentColor" d="M512.5 390.6c-29.9 0-57.9 11.6-79.1 32.8c-21.1 21.2-32.8 49.2-32.8 79.1s11.7 57.9 32.8 79.1c21.2 21.1 49.2 32.8 79.1 32.8s57.9-11.7 79.1-32.8c21.1-21.2 32.8-49.2 32.8-79.1s-11.7-57.9-32.8-79.1a110.96 110.96 0 0 0-79.1-32.8m412.3 235.5l-65.4-55.9c3.1-19 4.7-38.4 4.7-57.7s-1.6-38.8-4.7-57.7l65.4-55.9a32.03 32.03 0 0 0 9.3-35.2l-.9-2.6a442.5 442.5 0 0 0-79.6-137.7l-1.8-2.1a32.12 32.12 0 0 0-35.1-9.5l-81.2 28.9c-30-24.6-63.4-44-99.6-57.5l-15.7-84.9a32.05 32.05 0 0 0-25.8-25.7l-2.7-.5c-52-9.4-106.8-9.4-158.8 0l-2.7.5a32.05 32.05 0 0 0-25.8 25.7l-15.8 85.3a353.4 353.4 0 0 0-98.9 57.3l-81.8-29.1a32 32 0 0 0-35.1 9.5l-1.8 2.1a445.9 445.9 0 0 0-79.6 137.7l-.9 2.6c-4.5 12.5-.8 26.5 9.3 35.2l66.2 56.5c-3.1 18.8-4.6 38-4.6 57c0 19.2 1.5 38.4 4.6 57l-66 56.5a32.03 32.03 0 0 0-9.3 35.2l.9 2.6c18.1 50.3 44.8 96.8 79.6 137.7l1.8 2.1a32.12 32.12 0 0 0 35.1 9.5l81.8-29.1c29.8 24.5 63 43.9 98.9 57.3l15.8 85.3a32.05 32.05 0 0 0 25.8 25.7l2.7.5a448.3 448.3 0 0 0 158.8 0l2.7-.5a32.05 32.05 0 0 0 25.8-25.7l15.7-84.9c36.2-13.6 69.6-32.9 99.6-57.5l81.2 28.9a32 32 0 0 0 35.1-9.5l1.8-2.1c34.8-41.1 61.5-87.4 79.6-137.7l.9-2.6c4.3-12.4.6-26.3-9.5-35m-412.3 52.2c-97.1 0-175.8-78.7-175.8-175.8s78.7-175.8 175.8-175.8s175.8 78.7 175.8 175.8s-78.7 175.8-175.8 175.8" />
        </svg>
      </button>
      <button 
        class="btn btn-icon-only theme-toggle" 
        @click="toggleTheme" 
        :title="getThemeTooltip()"
      >
        <!-- 跟随系统主题 -->
        <svg v-if="currentTheme === 'system'" class="btn-icon" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg">
          <path fill="currentColor" d="M7.5 2c-1.79 1.15-3 3.18-3 5.5s1.21 4.35 3.03 5.5C4.46 13 2 10.54 2 7.5A5.5 5.5 0 0 1 7.5 2m11.57 1.5l1.43 1.43L4.93 20.5L3.5 19.07zm-6.18 2.43L11.41 5L9.97 6l.42-1.7L9 3.24l1.75-.12l.58-1.65L12 3.1l1.73.03l-1.35 1.13zm-3.3 3.61l-1.16-.73l-1.12.78l.34-1.32l-1.09-.83l1.36-.09l.45-1.29l.51 1.27l1.36.03l-1.05.87zM19 13.5a5.5 5.5 0 0 1-5.5 5.5c-1.22 0-2.35-.4-3.26-1.07l7.69-7.69c.67.91 1.07 2.04 1.07 3.26m-4.4 6.58l2.77-1.15l-.24 3.35zm4.33-2.7l1.15-2.77l2.2 2.54zm1.15-4.96l-1.14-2.78l3.34.24zM9.63 18.93l2.77 1.15l-2.53 2.19z" />
        </svg>
        <!-- 浅色/深色主题 -->
        <svg v-else class="btn-icon" viewBox="0 0 512 512" xmlns="http://www.w3.org/2000/svg">
          <path fill="currentColor" fill-rule="evenodd" d="M277.333 405.333v85.333h-42.667v-85.333zm99.346-58.824l60.34 60.34l-30.17 30.17l-60.34-60.34zm-241.359 0l30.17 30.17l-60.34 60.34l-30.17-30.17zM256 139.353c64.422 0 116.647 52.224 116.647 116.647c0 64.422-52.225 116.647-116.647 116.647A116.427 116.427 0 0 1 139.352 256c0-64.423 52.225-116.647 116.648-116.647m0 42.666c-40.859 0-73.981 33.123-73.981 74.062a73.76 73.76 0 0 0 21.603 52.296c13.867 13.867 32.685 21.64 52.378 21.603zm234.666 52.647v42.667h-85.333v-42.667zm-384 0v42.667H21.333v-42.667zM105.15 74.98l60.34 60.34l-30.17 30.17l-60.34-60.34zm301.7 0l30.169 30.17l-60.34 60.34l-30.17-30.17zM277.332 21.333v85.333h-42.667V21.333z" />
        </svg>
      </button>
    </div>

    <!-- 主内容区 -->
    <div class="main-content">
      <!-- 左侧区域（侧边栏 + 按钮） -->
      <div class="sidebar-area" :class="{ collapsed: isSidebarCollapsed }">
        <!-- 侧边栏 -->
        <div class="sidebar">
          <!-- 目录树 -->
          <DirectoryTree />
          
          <!-- 文件类型筛选 -->
          <FileTypeFilter />
        </div>
        
        <!-- 折叠按钮（独立于侧边栏，始终可见） -->
        <div 
          class="sidebar-toggle" 
          @click="isSidebarCollapsed = !isSidebarCollapsed"
          :title="isSidebarCollapsed ? '展开侧边栏' : '收起侧边栏'"
        >
          {{ isSidebarCollapsed ? '▶' : '◀' }}
        </div>
      </div>

      <!-- 右侧结果表格 -->
      <div class="results-panel">
        <ResultsTable @preview="handlePreview" />
      </div>
    </div>

    <!-- 状态栏 -->
    <div class="status-bar">
      <span>{{ isScanning ? '扫描中...' : '就绪' }}</span>
      <span>已扫描 {{ scannedCount }} 个文件</span>
      <span>非文档类型文件 {{ errorCount }} 个</span>
      <span>敏感文件 {{ sensitiveFilesCount }} 个</span>
      <span>敏感信息 {{ totalSensitiveItems.toLocaleString() }} 条</span>
    </div>

    <!-- 预览弹窗 -->
    <PreviewModal :file-path="previewFilePath" :visible="showPreview" @close="showPreview = false" />
    
    <!-- 设置窗口 -->
    <Transition name="modal">
      <SettingsModal v-if="showSettings" @close="showSettings = false" />
    </Transition>
    
    <!-- 日志窗口 -->
    <Transition name="modal">
      <LogsModal v-if="showLogs" @close="showLogs = false" />
    </Transition>
    
    <!-- 关于窗口 -->
    <Transition name="modal">
      <AboutModal v-if="showAbout" @close="showAbout = false" />
    </Transition>
    
    <!-- 导出窗口 -->
    <Transition name="modal">
      <ExportModal v-if="showExport" :results="scanResults" @close="showExport = false" />
    </Transition>
    
    <!-- 环境检查窗口 -->
    <EnvironmentCheck />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAppStore } from './stores/app'
import { storeToRefs } from 'pinia'
import { message } from '@tauri-apps/plugin-dialog'
import DirectoryTree from './components/DirectoryTree.vue'
import FileTypeFilter from './components/FileTypeFilter.vue'
import ResultsTable from './components/ResultsTable.vue'
import PreviewModal from './components/PreviewModal.vue'
import SettingsModal from './components/SettingsModal.vue'
import LogsModal from './components/LogsModal.vue'
import AboutModal from './components/AboutModal.vue'
import ExportModal from './components/ExportModal.vue'
import EnvironmentCheck from './components/EnvironmentCheck.vue'
import { startScan, cancelScan, loadConfig, onScanProgress, onScanResult, onScanFinished, onScanError, onScanLog } from './utils/tauri-api'
import { applyTheme, loadTheme, watchSystemTheme } from './utils/theme'
import type { ThemeMode } from './utils/theme'

// 导入 SVG 图标（不再需要，已改为内联）
// import playIcon from '@/assets/play.svg'
// import pauseIcon from '@/assets/pause.svg'
// import exportIcon from '@/assets/export.svg'
// import settingIcon from '@/assets/setting.svg'
// import lightDarkIcon from '@/assets/light-dark.svg'

const appStore = useAppStore()
const { isScanning, scannedCount, sensitiveFilesCount, errorCount, totalSensitiveItems, config, scanResults } = storeToRefs(appStore)

const showPreview = ref(false)
const previewFilePath = ref('')
const showSettings = ref(false)
const showLogs = ref(false)
const showAbout = ref(false)
const showExport = ref(false)
const isSidebarCollapsed = ref(false)
const currentTheme = ref<ThemeMode>('system')

// 加载配置
onMounted(async () => {
  try {
    const loadedConfig = await loadConfig()
    Object.assign(config.value, loadedConfig)
  } catch (error) {
    console.error('加载配置失败:', error)
  }
  
  // 初始化主题
  currentTheme.value = loadTheme()
  applyTheme(currentTheme.value)
  
  // 监听系统主题变化（仅在 system 模式下）
  watchSystemTheme(() => {
    if (currentTheme.value === 'system') {
      applyTheme('system')
    }
  })
  
  // 监听扫描事件
  await onScanProgress((data) => {
    scannedCount.value = data.scanned_count
    appStore.currentFile = data.current_file
  })
  
  await onScanResult((item) => {
    appStore.addScanResult(item)
  })
  
  await onScanFinished(() => {
    console.log('扫描完成')
    isScanning.value = false
  })
  
  await onScanError(async (error) => {
    console.error('扫描错误:', error)
    isScanning.value = false
    await message(`扫描出错: ${error}`, {
      title: '错误',
      kind: 'error',
      buttons: {ok: '确定'},
    })
  })
  
  // 监听日志事件
  await onScanLog((log) => {
    appStore.logs.push(log)
  })
})

// 开始扫描
const handleStartScan = async () => {
  if (appStore.selectedPaths.size === 0) {
    await message('请至少选择一个扫描路径', {
      title: '提示',
      kind: 'warning',
      buttons: {ok: '确定'},
    })
    return
  }
  
  // 获取有效的扫描路径（只保留叶子节点）
  const effectivePaths = appStore.getEffectiveScanPaths()
  console.log('开始扫描，选中的路径:', Array.from(appStore.selectedPaths))
  console.log('有效的扫描路径:', effectivePaths)
  console.log('配置的扩展名:', config.value.selected_extensions)
  console.log('启用的敏感类型:', config.value.enabled_sensitive_types)
  
  appStore.clearScanResults()
  appStore.logs = [] // 清空旧日志
  isScanning.value = true
  
  const scanConfig = {
    selected_paths: effectivePaths,
    selected_extensions: config.value.selected_extensions,
    enabled_sensitive_types: config.value.enabled_sensitive_types,
    ignore_dir_names: config.value.ignore_dir_names,
    system_dirs: config.value.system_dirs || [],
    max_file_size_mb: config.value.max_file_size_mb,
    max_pdf_size_mb: config.value.max_pdf_size_mb,
    scan_concurrency: config.value.scan_concurrency,
  }
  
  try {
    await startScan(scanConfig)
  } catch (error) {
    console.error('启动扫描失败:', error)
    isScanning.value = false
  }
}

// 取消扫描
const handleCancelScan = async () => {
  try {
    await cancelScan()
    isScanning.value = false
  } catch (error) {
    console.error('取消扫描失败:', error)
  }
}

// 导出报告
const handleExportReport = async () => {
  if (scanResults.value.length === 0) {
    await message('暂无扫描结果，无法导出报告', {
      title: '提示',
      kind: 'warning',
      buttons: {ok: '确定'},
    })
    return
  }
  showExport.value = true
}

// 预览文件
const handlePreview = (filePath: string) => {
  console.log('handlePreview called:', filePath, 'timestamp:', Date.now())
  // 同时设置，让 watch 立即触发
  previewFilePath.value = filePath
  showPreview.value = true
  console.log('showPreview set to true')
}

// 主题切换
const toggleTheme = () => {
  const themes: ThemeMode[] = ['light', 'dark', 'system']
  const currentIndex = themes.indexOf(currentTheme.value)
  const nextIndex = (currentIndex + 1) % themes.length
  currentTheme.value = themes[nextIndex]
  applyTheme(currentTheme.value)
}

// 获取主题图标

// 获取主题提示文本
const getThemeTooltip = () => {
  switch (currentTheme.value) {
    case 'light':
      return '当前：浅色主题，点击切换到深色'
    case 'dark':
      return '当前：深色主题，点击切换到跟随系统'
    case 'system':
      return '当前：跟随系统，点击切换到浅色'
    default:
      return '切换主题'
  }
}
</script>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  height: 100vh;
  width: 100vw;
}

.menu-bar {
  display: flex;
  gap: 20px;
  padding: 8px 16px;
  background-color: var(--menu-bg);
  border-bottom: 1px solid var(--border-color);
}

.menu-item {
  cursor: pointer;
  padding: 4px 8px;
  border-radius: 4px;
  transition: all 0.15s ease;
}

.menu-item:hover {
  background-color: var(--bg-hover);
  transform: translateY(-1px);
}

.menu-item.disabled {
  opacity: 0.5;
  cursor: not-allowed;
  pointer-events: none;
}

.toolbar {
  display: flex;
  gap: 10px;
  padding: 10px 16px;
  background-color: var(--toolbar-bg);
  border-bottom: 1px solid var(--border-color);
}

.btn {
  padding: 6px 16px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-color);
  color: var(--text-color);
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s ease;
  display: flex;
  align-items: center;
  gap: 6px;
}

.btn-icon {
  width: 18px;
  height: 18px;
  flex-shrink: 0;
  color: currentColor; /* 确保继承父元素颜色 */
}

.btn-icon-only {
  padding: 6px 10px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.btn:hover:not(:disabled) {
  background-color: var(--bg-hover);
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.btn:active:not(:disabled) {
  transform: translateY(0);
}

.btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.btn-primary {
  background-color: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.btn-primary .btn-icon {
  filter: brightness(0) invert(1); /* 确保图标为白色 */
}

.btn-primary:hover:not(:disabled) {
  background-color: #40a9ff;
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(24, 144, 255, 0.3);
}

.btn-primary:active:not(:disabled) {
  transform: translateY(0);
}

.btn-danger {
  background-color: var(--error-color);
  color: white;
  border-color: var(--error-color);
}

.btn-danger .btn-icon {
  filter: brightness(0) invert(1); /* 确保图标为白色 */
}

.btn-danger:hover:not(:disabled) {
  background-color: #ff7875;
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(255, 77, 79, 0.3);
}

.btn-danger:active:not(:disabled) {
  transform: translateY(0);
}

.theme-toggle {
  transition: all 0.2s ease;
}

.theme-toggle .btn-icon {
  color: var(--text-color); /* 明确指定使用主题文本颜色 */
  transition: color 0.2s ease;
}

.main-content {
  display: flex;
  flex: 1;
  overflow: hidden;
}

/* 左侧区域容器 */
.sidebar-area {
  display: flex;
  flex-shrink: 0;
  position: relative; /* 为按钮提供定位上下文 */
  width: 300px; /* 固定宽度 */
  transition: width 0.3s cubic-bezier(0.4, 0, 0.2, 1);
}

.sidebar-area.collapsed {
  width: 0;
}

/* 侧边栏 - 使用 transform 平移，避免重排 */
.sidebar {
  width: 300px;
  height: 100%;
  border-right: 1px solid var(--border-color);
  overflow-y: auto;
  display: flex;
  flex-direction: column;
  background-color: var(--sidebar-bg);
  position: absolute; /* 绝对定位，脱离文档流 */
  left: 0;
  top: 0;
  transition: transform 0.3s cubic-bezier(0.4, 0, 0.2, 1);
  transform: translateX(0);
}

.sidebar-area.collapsed .sidebar {
  transform: translateX(-100%); /* 向左平移，完全隐藏 */
}

/* 折叠按钮 - 绝对定位，始终在容器右侧 */
.sidebar-toggle {
  position: absolute;
  right: -16px;
  top: 50%;
  transform: translateY(-50%);
  width: 16px;
  height: 60px;
  display: flex;
  align-items: center;
  justify-content: center;
  background-color: var(--bg-hover);
  border: 1px solid var(--border-color);
  border-left: none;
  border-radius: 0 4px 4px 0;
  cursor: pointer;
  user-select: none;
  font-size: 12px;
  color: var(--text-secondary);
  transition: all 0.2s ease;
  z-index: 100;
}

.sidebar-toggle:hover {
  background-color: var(--bg-hover);
  color: var(--primary-color);
  transform: translateY(-50%) scale(1.1);
}

.results-panel {
  flex: 1;
  overflow: hidden;
}

.status-bar {
  display: flex;
  gap: 30px;
  padding: 6px 16px;
  background-color: var(--menu-bg);
  border-top: 1px solid var(--border-color);
  font-size: 13px;
  color: var(--text-secondary);
}

/* 模态框过渡动画 */
.modal-enter-active,
.modal-leave-active {
  transition: opacity 0.25s ease;
}

.modal-enter-from,
.modal-leave-to {
  opacity: 0;
}

.modal-enter-active :deep(.modal-container),
.modal-leave-active :deep(.modal-container) {
  transition: transform 0.3s cubic-bezier(0.34, 1.56, 0.64, 1);
}

.modal-enter-from :deep(.modal-container),
.modal-leave-to :deep(.modal-container) {
  transform: scale(0.9) translateY(20px);
}
</style>
