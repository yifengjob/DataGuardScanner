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
          <th 
            class="sortable path-col"
            :class="{ 'sorted-asc': sortField === 'file_path' && sortOrder === 'asc', 'sorted-desc': sortField === 'file_path' && sortOrder === 'desc' }"
            @click="sortBy('file_path')"
            title="点击排序"
          >
            文件名
            <span v-if="sortField === 'file_path'" class="sort-indicator">
              {{ sortOrder === 'asc' ? '↑' : '↓' }}
            </span>
          </th>
          <th 
            class="sortable" 
            :class="{ 'sorted-asc': sortField === 'file_size' && sortOrder === 'asc', 'sorted-desc': sortField === 'file_size' && sortOrder === 'desc' }"
            @click="sortBy('file_size')"
            title="点击排序"
          >
            文件大小
            <span v-if="sortField === 'file_size'" class="sort-indicator">
              {{ sortOrder === 'asc' ? '↑' : '↓' }}
            </span>
          </th>
          <th 
            class="sortable" 
            :class="{ 'sorted-asc': sortField === 'modified_time' && sortOrder === 'asc', 'sorted-desc': sortField === 'modified_time' && sortOrder === 'desc' }"
            @click="sortBy('modified_time')"
            title="点击排序"
          >
            修改时间
            <span v-if="sortField === 'modified_time'" class="sort-indicator">
              {{ sortOrder === 'asc' ? '↑' : '↓' }}
            </span>
          </th>
          <th 
            v-for="type in sensitiveTypes" 
            :key="type.id"
            class="sortable"
            :class="{ 'sorted-asc': sortField === `counts.${type.id}` && sortOrder === 'asc', 'sorted-desc': sortField === `counts.${type.id}` && sortOrder === 'desc' }"
            @click="sortBy(`counts.${type.id}`)"
            title="点击排序"
          >
            {{ type.name }}
            <span v-if="sortField === `counts.${type.id}`" class="sort-indicator">
              {{ sortOrder === 'asc' ? '↑' : '↓' }}
            </span>
          </th>
          <th 
            class="sortable"
            :class="{ 'sorted-asc': sortField === 'total' && sortOrder === 'asc', 'sorted-desc': sortField === 'total' && sortOrder === 'desc' }"
            @click="sortBy('total')"
            title="点击排序"
          >
            总计
            <span v-if="sortField === 'total'" class="sort-indicator">
              {{ sortOrder === 'asc' ? '↑' : '↓' }}
            </span>
          </th>
          <th class="actions-col">操作</th>
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
          <td class="actions-col">
            <div class="actions-cell">
            <button class="btn-action" @click="handlePreview(item)" title="预览">
              <svg class="action-icon">
                <use href="#icon-preview"></use>
              </svg>
            </button>
            <button class="btn-action" @click="handleOpen(item)" title="打开">
              <svg class="action-icon">
                <use href="#icon-openfile"></use>
              </svg>
            </button>
            <button class="btn-action" @click="handleOpenLocation(item)" title="所在目录">
              <svg class="action-icon">
                <use href="#icon-directory"></use>
              </svg>
            </button>
            <button class="btn-action btn-delete" @click="handleDelete(item)" title="删除">
              <svg class="action-icon delete-icon">
                <use href="#icon-delete"></use>
              </svg>
            </button>
            </div>
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
import {openFile, openFileLocation, deleteFile, getSensitiveRules} from '../utils/tauri-api'
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
      let aVal: any
      let bVal: any

      // 处理 counts.xxx 字段（敏感类型计数）
      if (sortField.value.startsWith('counts.')) {
        const typeId = sortField.value.replace('counts.', '')
        aVal = a.counts[typeId] || 0
        bVal = b.counts[typeId] || 0
      } else {
        // 普通字段
        aVal = a[sortField.value as keyof typeof a]
        bVal = b[sortField.value as keyof typeof b]
      }

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

const handleOpenLocation = async (item: any) => {
  try {
    await openFileLocation(item.file_path)
  } catch (error) {
    console.error('打开目录失败:', error)
    alert('打开目录失败')
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
  padding: 0.5em 1em;                /* 8px 16px - 表头内边距 */
  background-color: var(--toolbar-bg);
  border-bottom: var(--border-width) solid var(--border-color);
}

.table-header h3 {
  font-size: 0.95em;                 /* 接近基础字体 */
  font-weight: 600;
}

.table-actions {
  display: flex;
  gap: var(--spacing-sm);
  align-items: center;
}

.btn-batch-delete {
  padding: 0.25em 0.75em;            /* 4px 12px - 紧凑按钮 */
  background-color: var(--error-color);
  color: white;
  border: none;
  border-radius: var(--radius-sm);
  cursor: pointer;
  font-size: 0.9em;                  /* 略小但可读 */
  font-weight: 500;
  transition: all 0.2s;
}

.btn-batch-delete:hover {
  background-color: #cf1322;
  box-shadow: 0 1px 3px rgba(245, 34, 45, 0.2);
}

.search-input {
  padding: 0.25em 0.625em;           /* 4px 10px - 搜索框 */
  border: var(--border-width) solid var(--border-color);
  border-radius: var(--radius-sm);
  font-size: 0.9em;                  /* 略小但可读 */
  width: clamp(10rem, 15vw, 15rem);
  background-color: var(--input-bg);
  color: var(--text-color);
}

.table-content {
  flex: 1;
  overflow: auto;                    /* 允许水平滚动 */
}

table {
  width: max-content;                /* 表格宽度根据内容自适应 */
  min-width: 100%;                   /* 至少占满容器 */
  border-collapse: collapse;
  font-size: 0.95em;                 /* 表格字体略大于默认 */
}

thead {
  position: sticky;
  top: 0;
  background-color: var(--bg-hover);
  z-index: 20;                       /* 高于所有固定列 */
}

th {
  padding: 0.5em 0.75em;             /* 8px 12px - VS Code 风格 */
  text-align: left;
  font-weight: 600;
  border-bottom: var(--border-width-thick) solid var(--border-color);
  user-select: none;
  transition: background-color 0.15s ease;
  position: relative;
  font-size: 0.9em;                  /* VS Code 表头略小 */
  white-space: nowrap;               /* 防止表头换行 */
  overflow: hidden;
  text-overflow: ellipsis;
}

th.path-col {
  position: sticky;                  /* 固定列 */
  left: 3.5em;                       /* 在复选框列右侧 (56px) */
  z-index: 9;                        /* 略低于复选框列 */
  background-color: var(--bg-hover); /* 需要背景色 */
  box-shadow: 2px 0 4px rgba(0, 0, 0, 0.05);  /* 右侧阴影 */
}

th.sortable {
  cursor: pointer;
}

th.sortable:hover {
  background-color: var(--bg-selected);
}

th.checkbox-col {
  position: sticky;                  /* 固定列 */
  left: 0;                           /* 固定在左侧 */
  min-width: 3.5em;                  /* 56px - 最小宽度，避免被按钮遮挡 */
  max-width: 3.5em;                  /* 固定宽度 */
  width: 3.5em;
  text-align: center;
  cursor: default;
  z-index: 10;                       /* 高于普通列 */
  background-color: var(--bg-hover); /* 需要背景色，否则透明 */
  box-shadow: 2px 0 4px rgba(0, 0, 0, 0.05);  /* 右侧阴影 */
}

th.checkbox-col:hover {
  background-color: var(--bg-selected);
}

/* 操作列固定宽度 */
th.actions-col {
  position: sticky;                  /* 固定列 */
  right: 0;                          /* 固定在右侧 */
  width: 10.5em;                     /* 168px - 容纳 4 个 32px 按钮 + 间距 */
  text-align: center;
  cursor: default;
  z-index: 10;                       /* 高于普通列 */
  background-color: var(--bg-hover); /* 需要背景色 */
  box-shadow: -2px 0 4px rgba(0, 0, 0, 0.05); /* 左侧阴影 */
}

th.actions-col:hover {
  background-color: var(--bg-selected);
}

.sort-indicator {
  display: inline-block;
  margin-left: var(--spacing-xs);
  font-size: 0.9em;  /* 相对于表头字体 */
  opacity: 0.8;
}

td {
  padding: 0.4375em 0.75em;          /* 7px 12px - VS Code 风格 */
  border-bottom: var(--border-width) solid var(--border-color);
  color: var(--text-color);
  font-size: 0.9em;                  /* 与表头一致 */
  white-space: nowrap;               /* 所有内容不换行 */
  overflow: hidden;                  /* 超出隐藏 */
  text-overflow: ellipsis;           /* 显示省略号 */
}

td.checkbox-col {
  position: sticky;                  /* 固定列 */
  left: 0;                           /* 固定在左侧 */
  min-width: 3.5em;                  /* 56px */
  max-width: 3.5em;                  /* 固定宽度 */
  width: 3.5em;
  text-align: center;
  overflow: visible;                 /* 复选框完整显示 */
  text-overflow: clip;
  z-index: 10;                       /* 高于普通列 */
  background-color: var(--bg-color); /* 需要背景色 */
  box-shadow: 2px 0 4px rgba(0, 0, 0, 0.05);  /* 右侧阴影 */
}

/* 操作列固定宽度 */
td.actions-col {
  position: sticky;                  /* 固定列 */
  right: 0;                          /* 固定在右侧 */
  width: 10.5em;                     /* 168px */
  text-align: center;
  padding: 0.3125em 0.5em;          /* 5px 8px - 舒适的垂直间距 */
  overflow: visible;                 /* 按钮完整显示 */
  text-overflow: clip;               /* 不显示省略号 */
  z-index: 10;                       /* 高于普通列 */
  background-color: var(--bg-color); /* 需要背景色 */
  box-shadow: -2px 0 4px rgba(0, 0, 0, 0.05); /* 左侧阴影 */
}

td.checkbox-col input[type="checkbox"] {
  cursor: pointer;
  width: var(--btn-icon-size);
  height: var(--btn-icon-size);
}

tr {
  transition: background-color 0.15s ease;
}

tr:hover {
  background-color: var(--bg-hover);
}

.path-cell {
  position: sticky;                  /* 固定列 */
  left: 3.5em;                       /* 在复选框列右侧 (56px) */
  max-width: 25em;                   /* 400px - 限制最大宽度 */
  min-width: 10em;                   /* 160px - 最小宽度 */
  z-index: 9;                        /* 略低于复选框列 */
  background-color: var(--bg-color); /* 需要背景色 */
  box-shadow: 2px 0 4px rgba(0, 0, 0, 0.05);  /* 右侧阴影 */
}

.size-cell, .number-cell, .total-cell {
  text-align: right;
  overflow: visible;                 /* 数字列完整显示 */
  text-overflow: clip;               /* 不显示省略号 */
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
  display: flex;
  gap: 0.25em;                       /* 4px - 按钮间距 */
  justify-content: center;           /* 居中对齐 */
}

.btn-action {
  padding: 0.25em;                   /* 4px - 内边距 */
  border: none;
  background-color: transparent;
  color: var(--text-color);
  border-radius: var(--radius-sm);
  cursor: pointer;
  transition: all 0.15s ease;
  display: flex;
  align-items: center;
  justify-content: center;
  min-width: 2em;                    /* 32px - 舒适的点击区域 */
  min-height: 2em;
}

.btn-action:hover {
  background-color: var(--bg-hover);
}

.btn-action:active {
  transform: translateY(0);
}

.action-icon {
  width: 1.5em;                      /* 24px - 更大的图标 */
  height: 1.5em;
  fill: currentColor;
}

.delete-icon {
  color: var(--error-color);
}

.btn-delete {
  /* 不再需要边框颜色，由 .delete-icon 控制 */
}

.btn-delete:hover {
  background-color: rgba(255, 77, 79, 0.1);
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
