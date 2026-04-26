<template>
  <div class="results-table">
    <div class="table-header">
      <h3>扫描结果</h3>
      <div class="table-actions">
        <button 
          v-if="selectedFiles.size > 0" 
          class="btn-batch-delete"
          @click="handleBatchDelete"
        >
          一键删除 ({{ selectedFiles.size }})
        </button>
        <input
            type="text"
            v-model="searchKeyword"
            placeholder="搜索文件路径..."
            class="search-input"
        />
      </div>
    </div>

    <div class="table-content">
      <table v-if="filteredResults.length > 0">
        <thead>
        <tr>
          <th class="checkbox-col">
            <input 
              type="checkbox" 
              ref="selectAllCheckbox"
              :checked="isAllSelected"
              @change="toggleSelectAll"
              title="全选/取消全选"
            />
          </th>
          <th @click="sortBy('file_path')">文件名</th>
          <th @click="sortBy('file_size')">文件大小</th>
          <th @click="sortBy('modified_time')">修改时间</th>
          <th v-for="type in sensitiveTypes" :key="type.id">{{ type.name }}</th>
          <th>总计</th>
          <th>操作</th>
        </tr>
        </thead>
        <tbody>
        <tr v-for="item in filteredResults" :key="item.file_path">
          <td class="checkbox-col">
            <input 
              type="checkbox" 
              :checked="selectedFiles.has(item.file_path)"
              @change="toggleSelectFile(item.file_path)"
            />
          </td>
          <td class="path-cell" :title="item.file_path">{{ getFileName(item.file_path) }}</td>
          <td class="size-cell">{{ formatFileSize(item.file_size) }}</td>
          <td>{{ formatTime(item.modified_time) }}</td>
          <td v-for="type in sensitiveTypes" :key="type.id" class="number-cell"
              :class="{ 'highlight-count': (item.counts[type.id] || 0) > 0 }">
            {{ (item.counts[type.id] || 0) > 0 ? Number(item.counts[type.id]).toLocaleString() : '-' }}
          </td>
          <td class="total-cell">{{ item.total }}</td>
          <td class="actions-cell">
            <button class="btn-action" @click="handlePreview(item)">预览</button>
            <button class="btn-action" @click="handleOpen(item)">打开</button>
            <button class="btn-action btn-delete" @click="handleDelete(item)">删除</button>
          </td>
        </tr>
        </tbody>
      </table>

      <div v-else class="empty-state">
        <p>{{ appStore.isScanning ? '扫描中...' : '暂无扫描结果' }}</p>
        <p v-if="!appStore.isScanning" class="hint">点击"开始扫描"按钮开始扫描</p>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import {ref, computed, onMounted, watch} from 'vue'
import {useAppStore} from '../stores/app'
import {storeToRefs} from 'pinia'
import {formatFileSize, formatTime} from '../utils/format'
import {openFile, deleteFile, getSensitiveRules} from '../utils/tauri-api'
import {ask} from '@tauri-apps/plugin-dialog'

const appStore = useAppStore()
const {scanResults, config} = storeToRefs(appStore)

const emit = defineEmits<{
  preview: [filePath: string]
}>()

const searchKeyword = ref('')
const sortField = ref<string>('')
const sortOrder = ref<'asc' | 'desc'>('asc')
const allSensitiveTypes = ref<Array<{ id: string; name: string }>>([])
const selectedFiles = ref<Set<string>>(new Set())
const selectAllCheckbox = ref<HTMLInputElement | null>(null)

// 加载敏感类型定义
onMounted(async () => {
  try {
    const rules = await getSensitiveRules()
    allSensitiveTypes.value = rules.map(([id, name]) => ({id, name}))
  } catch (error) {
    console.error('加载敏感类型失败:', error)
  }
})

// 只显示启用且存在于规则中的敏感类型
const sensitiveTypes = computed(() => {
  return allSensitiveTypes.value.filter(type =>
      config.value.enabled_sensitive_types.includes(type.id)
  )
})

const filteredResults = computed(() => {
  let results = scanResults.value

  // 搜索过滤
  if (searchKeyword.value) {
    const keyword = searchKeyword.value.toLowerCase().trim()
    if (keyword) {
      results = results.filter(item => {
        const path = item.file_path.toLowerCase()
        // 同时支持正斜杠和反斜杠的匹配
        const normalizedPath = path.replace(/\\/g, '/')
        const normalizedKeyword = keyword.replace(/\\/g, '/')
        return path.includes(keyword) || normalizedPath.includes(normalizedKeyword)
      })
    }
  }

  // 排序
  if (sortField.value) {
    results = [...results].sort((a, b) => {
      let aVal: any = a[sortField.value as keyof typeof a]
      let bVal: any = b[sortField.value as keyof typeof b]

      if (typeof aVal === 'string') {
        aVal = aVal.toLowerCase()
        bVal = bVal.toLowerCase()
      }

      if (aVal < bVal) return sortOrder.value === 'asc' ? -1 : 1
      if (aVal > bVal) return sortOrder.value === 'asc' ? 1 : -1
      return 0
    })
  }

  return results
})

const sortBy = (field: string) => {
  if (sortField.value === field) {
    sortOrder.value = sortOrder.value === 'asc' ? 'desc' : 'asc'
  } else {
    sortField.value = field
    sortOrder.value = 'asc'
  }
}

// 从完整路径中提取文件名
const getFileName = (filePath: string) => {
  // 处理 Windows 和 Unix 路径
  const separators = filePath.includes('\\') ? '\\' : '/'
  const parts = filePath.split(separators)
  return parts[parts.length - 1] || filePath
}

const handlePreview = (item: any) => {
  emit('preview', item.file_path)
}

const handleOpen = async (item: any) => {
  try {
    await openFile(item.file_path)
  } catch (error) {
    console.error('打开文件失败:', error)
    alert('打开文件失败')
  }
}

const handleDelete = async (item: any) => {
  const deleteMode = config.value.delete_to_trash ? '移入回收站' : '永久删除'
  const confirmed = await ask(`确定要${deleteMode}此文件吗？\n${item.file_path}`, {
    title: '确认删除',
    kind: 'warning',
    okLabel: '删除',
    cancelLabel: '取消'
  })
  
  if (!confirmed) {
    return
  }

  try {
    await deleteFile(item.file_path)
    appStore.removeResult(item.file_path)
  } catch (error) {
    console.error('删除文件失败:', error)
    alert('删除文件失败')
  }
}

// 计算是否全选
const isAllSelected = computed(() => {
  return filteredResults.value.length > 0 && 
         filteredResults.value.every(item => selectedFiles.value.has(item.file_path))
})

// 计算是否半选
const isIndeterminate = computed(() => {
  const selectedCount = filteredResults.value.filter(item => 
    selectedFiles.value.has(item.file_path)
  ).length
  return selectedCount > 0 && selectedCount < filteredResults.value.length
})

// 监听 indeterminate 状态变化
watch(isIndeterminate, (newValue) => {
  if (selectAllCheckbox.value) {
    selectAllCheckbox.value.indeterminate = newValue
  }
}, { immediate: true })

// 切换单个文件选择
const toggleSelectFile = (filePath: string) => {
  if (selectedFiles.value.has(filePath)) {
    selectedFiles.value.delete(filePath)
  } else {
    selectedFiles.value.add(filePath)
  }
}

// 切换全选
const toggleSelectAll = () => {
  if (isAllSelected.value) {
    // 取消全选
    filteredResults.value.forEach(item => {
      selectedFiles.value.delete(item.file_path)
    })
  } else {
    // 全选
    filteredResults.value.forEach(item => {
      selectedFiles.value.add(item.file_path)
    })
  }
}

// 批量删除
const handleBatchDelete = async () => {
  if (selectedFiles.value.size === 0) {
    return
  }
  
  const count = selectedFiles.value.size
  const deleteMode = config.value.delete_to_trash ? '移入回收站' : '永久删除'
  const warningText = config.value.delete_to_trash 
    ? `确定要${deleteMode}选中的 ${count} 个文件吗？`
    : `确定要${deleteMode}选中的 ${count} 个文件吗？\n\n此操作不可恢复！`
  
  const confirmed = await ask(warningText, {
    title: '确认批量删除',
    kind: 'warning',
    okLabel: '删除',
    cancelLabel: '取消'
  })
  
  if (!confirmed) {
    return
  }
  
  const filesToDelete = Array.from(selectedFiles.value)
  let successCount = 0
  let failCount = 0
  
  for (const filePath of filesToDelete) {
    try {
      await deleteFile(filePath)
      appStore.removeResult(filePath)
      successCount++
    } catch (error) {
      console.error(`删除文件失败: ${filePath}`, error)
      failCount++
    }
  }
  
  // 清空选中状态
  selectedFiles.value.clear()
  
  // 显示结果
  if (failCount > 0) {
    alert(`删除完成\n成功: ${successCount} 个\n失败: ${failCount} 个`)
  }
}
</script>

<style scoped>
.results-table {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.table-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  background-color: var(--toolbar-bg);
  border-bottom: 1px solid var(--border-color);
}

.table-header h3 {
  font-size: 14px;
  font-weight: 600;
}

.table-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.btn-batch-delete {
  padding: 5px 12px;
  background-color: var(--error-color);
  color: white;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  font-weight: 500;
  transition: all 0.2s;
}

.btn-batch-delete:hover {
  background-color: #cf1322;
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(245, 34, 45, 0.3);
}

.search-input {
  padding: 5px 10px;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-size: 13px;
  width: 200px;
  background-color: var(--input-bg);
  color: var(--text-color);
}

.table-content {
  flex: 1;
  overflow: auto;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 13px;
}

thead {
  position: sticky;
  top: 0;
  background-color: var(--bg-hover);
  z-index: 1;
}

th {
  padding: 10px 8px;
  text-align: left;
  font-weight: 600;
  border-bottom: 2px solid var(--border-color);
  cursor: pointer;
  user-select: none;
  transition: background-color 0.15s ease;
}

th.checkbox-col {
  width: 56px;
  text-align: center;
  cursor: default;
}

th.checkbox-col:hover {
  background-color: transparent;
}

th:hover {
  background-color: var(--bg-selected);
}

td {
  padding: 8px;
  border-bottom: 1px solid var(--border-color);
  color: var(--text-color);
}

td.checkbox-col {
  width: 56px;
  text-align: center;
}

td.checkbox-col input[type="checkbox"] {
  cursor: pointer;
  width: 14px;
  height: 14px;
}

tr {
  transition: background-color 0.15s ease;
}

tr:hover {
  background-color: var(--bg-hover);
}

.path-cell {
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.size-cell, .number-cell, .total-cell {
  text-align: right;
}

.total-cell {
  font-weight: 600;
  color: var(--primary-color);
}

.highlight-count {
  color: #ff4d4f;
  font-weight: 600;
}

.actions-cell {
  white-space: nowrap;
}

.btn-action {
  padding: 3px 10px;
  margin-right: 5px;
  border: 1px solid var(--border-color);
  background-color: var(--bg-color);
  color: var(--text-color);
  border-radius: 3px;
  cursor: pointer;
  font-size: 12px;
  transition: all 0.2s ease;
}

.btn-action:hover {
  background-color: var(--bg-hover);
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.btn-action:active {
  transform: translateY(0);
}

.btn-delete {
  color: var(--error-color);
  border-color: var(--error-color);
}

.btn-delete:hover {
  background-color: var(--bg-hover);
  transform: translateY(-1px);
  box-shadow: 0 2px 4px rgba(255, 77, 79, 0.2);
}

.btn-delete:active {
  transform: translateY(0);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  height: 100%;
  color: var(--text-secondary);
}

.empty-state p {
  margin: 8px 0;
}

.hint {
  font-size: 13px;
  color: #999;
}
</style>
