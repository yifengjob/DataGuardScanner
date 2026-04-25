<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal-container">
      <div class="modal-header">
        <h3>导出报告</h3>
        <button class="close-btn" @click="$emit('close')">×</button>
      </div>

      <div class="modal-body">
        <!-- 无数据提示 -->
        <div v-if="results.length === 0" class="no-data-hint">
          <p>⚠️ 暂无扫描结果，无法导出报告</p>
          <p class="hint-text">请先执行扫描，待发现敏感文件后再导出</p>
        </div>

        <div v-else class="export-options">
          <div class="option-group">
            <label>选择格式：</label>
            <div class="format-options">
              <label class="format-option">
                <input
                    type="radio"
                    value="xlsx"
                    v-model="selectedFormat"
                />
                <span class="format-info">
                  <strong>Excel (.xlsx)</strong>
                  <span class="format-desc">推荐，支持样式和格式化</span>
                </span>
              </label>

              <label class="format-option">
                <input
                    type="radio"
                    value="csv"
                    v-model="selectedFormat"
                />
                <span class="format-info">
                  <strong>CSV (.csv)</strong>
                  <span class="format-desc">通用格式，可用 Excel 打开</span>
                </span>
              </label>

              <label class="format-option">
                <input
                    type="radio"
                    value="json"
                    v-model="selectedFormat"
                />
                <span class="format-info">
                  <strong>JSON (.json)</strong>
                  <span class="format-desc">结构化数据，适合程序处理</span>
                </span>
              </label>
            </div>
          </div>

          <div class="option-group">
            <label>保存路径：</label>
            <div class="path-input">
              <input
                  type="text"
                  v-model="savePath"
                  placeholder="选择保存位置..."
                  readonly
              />
              <button class="btn-browse" @click="handleBrowse">浏览...</button>
            </div>
          </div>

          <div class="summary">
            <p><strong>扫描结果统计：</strong></p>
            <ul>
              <li>敏感文件数：{{ results.length }} 个</li>
              <li>总敏感项数：{{ totalSensitiveItems }} 个</li>
            </ul>
          </div>
        </div>
      </div>

      <div class="modal-footer">
        <button class="btn" @click="$emit('close')">关闭</button>
        <button
            v-if="results.length > 0"
            class="btn btn-primary"
            @click="handleExport"
            :disabled="!savePath || exporting"
        >
          {{ exporting ? '导出中...' : '开始导出' }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {ref, computed} from 'vue'
import {save, message} from '@tauri-apps/plugin-dialog'
import {exportReport} from '../utils/tauri-api'
import type {ScanResultItem} from '../types'

const props = defineProps<{
  results: ScanResultItem[]
}>()

const emit = defineEmits<{
  close: []
}>()

const selectedFormat = ref('xlsx')
const savePath = ref('')
const exporting = ref(false)

const totalSensitiveItems = computed(() => {
  return props.results.reduce((sum, item) => sum + item.total, 0)
})

const handleBrowse = async () => {
  try {
    const extensions = selectedFormat.value === 'xlsx' ? ['xlsx'] :
        selectedFormat.value === 'csv' ? ['csv'] : ['json']

    console.log('打开保存对话框，格式:', selectedFormat.value, '扩展名:', extensions)

    const path = await save({
      filters: [{
        name: selectedFormat.value.toUpperCase(),
        extensions
      }]
    })

    console.log('用户选择的路径:', path)

    if (path) {
      savePath.value = path
    } else {
      console.log('用户取消了选择')
    }
  } catch (error) {
    console.error('打开保存对话框失败:', error)
    await message(`无法打开保存对话框: ${error}`, {
      title: '错误',
      kind: 'error',
      buttons: {ok: '确定'},
    })
  }
}

const handleExport = async () => {
  if (!savePath.value) {
    await message('请选择保存路径', {
      title: '提示',
      kind: 'warning',
      buttons: {ok: '确定'},
    })
    return
  }

  exporting.value = true

  try {
    const format = selectedFormat.value as 'csv' | 'json' | 'xlsx'
    await exportReport(props.results, format, savePath.value)

    // 导出成功，显示成功信息后关闭
    await message(`导出成功！\n\n文件已保存到：\n${savePath.value}`, {
      title: '成功',
      kind: 'info',
      buttons: {ok: '确定'},
    })
    emit('close')
  } catch (error) {
    console.error('导出失败:', error)
    await message(`导出失败: ${error}`, {
      title: '错误',
      kind: 'error',
      buttons: {ok: '确定'},
    })
  } finally {
    exporting.value = false
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
}

.modal-container {
  background-color: white;
  border-radius: 8px;
  width: 600px;
  max-height: 80vh;
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
  color: #333;
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 24px;
}

.no-data-hint {
  text-align: center;
  padding: 40px 20px;
  color: #999;
}

.no-data-hint p {
  margin: 8px 0;
  font-size: 14px;
}

.no-data-hint .hint-text {
  font-size: 13px;
  color: #bbb;
}

.export-options {
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.option-group {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.option-group > label {
  font-size: 14px;
  font-weight: 600;
  color: #333;
}

.format-options {
  display: flex;
  flex-direction: column;
  gap: 12px;
}

.format-option {
  display: flex;
  align-items: flex-start;
  gap: 10px;
  padding: 12px;
  border: 2px solid var(--border-color);
  border-radius: 6px;
  cursor: pointer;
  transition: all 0.2s;
}

.format-option:hover {
  border-color: var(--primary-color);
  background-color: #f0f9ff;
}

.format-option input[type="radio"] {
  margin-top: 2px;
  cursor: pointer;
}

.format-info {
  display: flex;
  flex-direction: column;
  gap: 4px;
}

.format-info strong {
  font-size: 14px;
  color: #333;
}

.format-desc {
  font-size: 12px;
  color: #999;
}

.path-input {
  display: flex;
  gap: 8px;
}

.path-input input {
  flex: 1;
  padding: 8px 12px;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-size: 13px;
}

.btn-browse {
  padding: 8px 16px;
  border: 1px solid var(--border-color);
  background-color: white;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.btn-browse:hover {
  background-color: var(--bg-hover);
}

.summary {
  padding: 12px;
  background-color: #f9f9f9;
  border-radius: 6px;
}

.summary p {
  margin: 0 0 8px 0;
  font-size: 14px;
}

.summary ul {
  margin: 0;
  padding-left: 20px;
  font-size: 13px;
  color: #666;
}

.summary li {
  margin: 4px 0;
}

.modal-footer {
  display: flex;
  gap: 10px;
  justify-content: flex-end;
  padding: 12px 20px;
  border-top: 1px solid var(--border-color);
}

.btn {
  padding: 8px 20px;
  border: 1px solid var(--border-color);
  background-color: white;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}

.btn:hover:not(:disabled) {
  background-color: var(--bg-hover);
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
}
</style>
