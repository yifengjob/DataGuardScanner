<template>
  <div class="modal-overlay" @click.self="$emit('close')" :class="{ visible: visible }">
    <div class="modal-container">
      <div class="modal-header">
        <h3>文件预览</h3>
        <button class="close-btn" @click="$emit('close')">×</button>
      </div>
      
      <div class="modal-body">
        <div v-if="loading" class="loading-container">
          <div class="loading-spinner"></div>
          <div class="loading-text">加载中...</div>
          <div class="loading-hint">正在读取文件内容，请稍候</div>
        </div>
        <div v-else-if="error" class="error">
          <div class="error-icon">⚠️</div>
          <div class="error-text">{{ error }}</div>
        </div>
        <div v-else class="preview-content">
          <pre v-html="highlightedContent"></pre>
        </div>
      </div>
      
      <div class="modal-footer">
        <button class="btn" @click="handleOpenFile">打开文件</button>
        <button class="btn" @click="handleCopyContent">复制内容</button>
        <button class="btn btn-primary" @click="$emit('close')">关闭</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { previewFile, openFile } from '../utils/tauri-api'
import { highlightText } from '../utils/format'

const props = defineProps<{
  filePath: string
  visible: boolean
}>()

defineEmits<{
  close: []
}>()

const loading = ref(false)
const error = ref('')
const content = ref('')
const highlights = ref<Array<{start: number, end: number, type_id: string, type_name: string}>>([])

const highlightedContent = computed(() => {
  if (!content.value) return ''
  return highlightText(content.value, highlights.value)
})

// 监听 visible 和 filePath 的组合变化
watch([() => props.visible, () => props.filePath], ([isVisible, newPath]) => {
  console.log('PreviewModal watch triggered:', { isVisible, newPath: newPath?.substring(0, 50), timestamp: Date.now() })
  
  if (isVisible && newPath) {
    // 窗口打开且有文件路径时，立即加载
    console.log('Start loading file, setting loading=true')
    
    // 使用 requestAnimationFrame 确保在下一帧才设置 loading，让窗口先显示
    requestAnimationFrame(() => {
      loading.value = true
      error.value = ''
      content.value = ''
      highlights.value = []
      console.log('Loading state set, calling loadFile')
      loadFile(newPath)
    })
  } else if (!isVisible) {
    // 窗口关闭时，清空状态
    console.log('Modal closed, clearing state')
    loading.value = false
    error.value = ''
    content.value = ''
    highlights.value = []
  }
}, { immediate: true })

async function loadFile(filePath: string) {
  try {
    const result = await previewFile(filePath)
    content.value = result.content
    highlights.value = result.highlights
  } catch (err) {
    error.value = `预览失败: ${err}`
  } finally {
    loading.value = false
  }
}

const handleOpenFile = async () => {
  if (props.filePath) {
    await openFile(props.filePath)
  }
}

const handleCopyContent = async () => {
  try {
    await navigator.clipboard.writeText(content.value)
    alert('已复制到剪贴板')
  } catch (err) {
    alert('复制失败')
  }
}
</script>

<style scoped>
.modal-overlay {
  position: fixed;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background-color: rgba(0, 0, 0, 0.5);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 1000;
  /* 默认隐藏 */
  opacity: 0;
  pointer-events: none;
  transition: opacity 0.2s ease-out;
}

.modal-overlay.visible {
  opacity: 1;
  pointer-events: auto;
}

.modal-container {
  background-color: white;
  border-radius: 8px;
  width: min(80%, 900px);
  height: min(80%, 700px);
  min-width: 600px;
  min-height: 400px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
  /* 动画效果 */
  transform: scale(0.95) translateY(10px);
  opacity: 0;
  transition: transform 0.25s cubic-bezier(0.34, 1.56, 0.64, 1), 
              opacity 0.2s ease-out;
}

.modal-overlay.visible .modal-container {
  transform: scale(1) translateY(0);
  opacity: 1;
}

.modal-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 16px 20px;
  border-bottom: 1px solid var(--border-color);
}

.modal-header h3 {
  font-size: 16px;
  font-weight: 600;
}

.close-btn {
  background: none;
  border: none;
  font-size: 28px;
  cursor: pointer;
  color: #999;
  line-height: 1;
  transition: all 0.2s ease;
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
}

.close-btn:hover {
  color: #333;
  background-color: rgba(0, 0, 0, 0.05);
  transform: rotate(90deg);
}

.modal-body {
  flex: 1;
  overflow: auto;
  padding: 20px;
}

.error {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
}

.loading-container {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 16px;
}

.loading-spinner {
  width: 40px;
  height: 40px;
  border: 4px solid #f3f3f3;
  border-top: 4px solid var(--primary-color);
  border-radius: 50%;
  animation: spin 0.8s linear infinite;
}

@keyframes spin {
  0% { transform: rotate(0deg); }
  100% { transform: rotate(360deg); }
}

.loading-text {
  font-size: 16px;
  color: #333;
  font-weight: 500;
}

.loading-hint {
  font-size: 13px;
  color: var(--text-secondary);
}

.error {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  gap: 12px;
  color: var(--error-color);
}

.error-icon {
  font-size: 48px;
}

.error-text {
  font-size: 14px;
  text-align: center;
  max-width: 80%;
}

.preview-content pre {
  margin: 0;
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
}

.modal-footer {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: 12px 20px;
  border-top: 1px solid var(--border-color);
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

.btn:hover {
  background-color: var(--bg-hover);
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.btn:active {
  transform: translateY(0);
}

.btn-primary {
  background-color: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
  transition: all 0.2s ease;
}

.btn-primary:hover {
  background-color: #40a9ff;
  transform: translateY(-1px);
  box-shadow: 0 2px 8px rgba(24, 144, 255, 0.3);
}

.btn-primary:active {
  transform: translateY(0);
}
</style>
