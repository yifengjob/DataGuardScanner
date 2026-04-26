<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal-container">
      <div class="modal-header">
        <h3>扫描日志</h3>
        <button class="close-btn" @click="$emit('close')">×</button>
      </div>
      
      <div class="modal-body" ref="logsContainer">
        <div v-if="logs.length === 0" class="empty-logs">
          <p>暂无日志信息</p>
        </div>
        <div v-else class="logs-content">
          <div 
            v-for="(log, index) in logs" 
            :key="index" 
            class="log-item"
            :class="{ error: log.includes('错误') || log.includes('失败') }"
          >
            {{ log }}
          </div>
        </div>
      </div>
      
      <div class="modal-footer">
        <button class="btn" @click="handleClearLogs">清空日志</button>
        <button class="btn btn-primary" @click="$emit('close')">关闭</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted, ref, watch, nextTick } from 'vue'
import { useAppStore } from '../stores/app'
import { storeToRefs } from 'pinia'
import { getLogs } from '../utils/tauri-api'

const appStore = useAppStore()
const { logs } = storeToRefs(appStore)

const logsContainer = ref<HTMLDivElement | null>(null)

defineEmits<{
  close: []
}>()

// 滚动到底部
const scrollToBottom = async () => {
  await nextTick()
  if (logsContainer.value) {
    logsContainer.value.scrollTop = logsContainer.value.scrollHeight
  }
}

// 监听日志变化，自动滚动到底部
watch(
  () => logs.value.length,
  () => {
    scrollToBottom()
  }
)

// 组件挂载时从后端获取日志
onMounted(async () => {
  try {
    const backendLogs = await getLogs()
    if (backendLogs.length > 0) {
      logs.value = backendLogs
      // 初始加载后滚动到底部
      await scrollToBottom()
    }
  } catch (error) {
    console.error('获取日志失败:', error)
  }
})

const handleClearLogs = () => {
  logs.value = []
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
}

.modal-container {
  background-color: var(--modal-bg);
  color: var(--text-color);
  border-radius: 8px;
  width: min(700px, 90vw);
  height: min(60vh, 500px);
  max-height: 85vh;
  display: flex;
  flex-direction: column;
  box-shadow: 0 4px 20px rgba(0, 0, 0, 0.3);
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
}

.close-btn:hover {
  color: var(--text-color);
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.empty-logs {
  text-align: center;
  padding: 40px;
  color: var(--text-secondary);
}

.logs-content {
  font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
  font-size: 12px;
  line-height: 1.8;
}

.log-item {
  padding: 4px 0;
  border-bottom: 1px solid var(--border-color);
  word-break: break-all;
  color: var(--text-color);
}

.log-item.error {
  color: var(--error-color);
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
  background-color: var(--bg-color);
  color: var(--text-color);
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.btn:hover {
  background-color: var(--bg-hover);
}

.btn-primary {
  background-color: var(--primary-color);
  color: white;
  border-color: var(--primary-color);
}

.btn-primary:hover {
  background-color: #40a9ff;
}
</style>
