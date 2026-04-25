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
      >
        {{ isScanning ? '扫描中...' : '开始扫描' }}
      </button>
      <button 
        class="btn btn-danger" 
        @click="handleCancelScan"
        :disabled="!isScanning"
      >
        取消
      </button>
      <button 
        class="btn" 
        @click="handleExportReport"
        :disabled="scanResults.length === 0"
        :title="scanResults.length === 0 ? '暂无扫描结果，无法导出' : '导出报告'"
      >
        导出报告
      </button>
      <button class="btn" @click="showSettings = true">设置</button>
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

const appStore = useAppStore()
const { isScanning, scannedCount, sensitiveFilesCount, errorCount, totalSensitiveItems, config, scanResults } = storeToRefs(appStore)

const showPreview = ref(false)
const previewFilePath = ref('')
const showSettings = ref(false)
const showLogs = ref(false)
const showAbout = ref(false)
const showExport = ref(false)
const isSidebarCollapsed = ref(false)

// 加载配置
onMounted(async () => {
  try {
    const loadedConfig = await loadConfig()
    Object.assign(config.value, loadedConfig)
  } catch (error) {
    console.error('加载配置失败:', error)
  }
  
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
  
  console.log('开始扫描，选中的路径:', Array.from(appStore.selectedPaths))
  console.log('配置的扩展名:', config.value.selected_extensions)
  console.log('启用的敏感类型:', config.value.enabled_sensitive_types)
  
  appStore.clearScanResults()
  appStore.logs = [] // 清空旧日志
  isScanning.value = true
  
  const scanConfig = {
    selected_paths: Array.from(appStore.selectedPaths),
    selected_extensions: config.value.selected_extensions,
    enabled_sensitive_types: config.value.enabled_sensitive_types,
    ignore_dir_names: config.value.ignore_dir_names,
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
  background-color: #f0f0f0;
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
  background-color: #fafafa;
  border-bottom: 1px solid var(--border-color);
}

.btn {
  padding: 6px 16px;
  border: 1px solid var(--border-color);
  background-color: white;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
  transition: all 0.2s ease;
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

.btn-danger:hover:not(:disabled) {
  background-color: #ff7875;
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(255, 77, 79, 0.3);
}

.btn-danger:active:not(:disabled) {
  transform: translateY(0);
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
  background-color: white;
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
  background-color: #f5f5f5;
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
  background-color: #f0f0f0;
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
