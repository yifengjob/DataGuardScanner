<template>
  <div class="modal-overlay" @click.self="$emit('close')">
    <div class="modal-container">
      <div class="modal-header">
        <h3>设置</h3>
        <button class="close-btn" @click="$emit('close')">×</button>
      </div>
      
      <div class="modal-body">
        <div class="settings-section">
          <h4>扫描配置</h4>
          
          <div class="setting-item">
            <label>最大文件大小 (MB)</label>
            <input 
              type="number" 
              v-model.number="config.max_file_size_mb"
              min="1"
              max="500"
            />
          </div>
          
          <div class="setting-item">
            <label>PDF 最大大小 (MB)</label>
            <input 
              type="number" 
              v-model.number="config.max_pdf_size_mb"
              min="1"
              max="1000"
            />
          </div>
          
          <div class="setting-item">
            <label>扫描并发数</label>
            <input 
              type="number" 
              v-model.number="config.scan_concurrency"
              min="1"
              max="16"
            />
          </div>
        </div>
        
        <div class="settings-section">
          <h4>文件操作</h4>
          
          <div class="setting-item">
            <label>删除文件时移入回收站</label>
            <input 
              type="checkbox" 
              v-model="config.delete_to_trash"
            />
            <span class="hint">（取消勾选则永久删除）</span>
          </div>
        </div>
        
        <div class="settings-section">
          <h4>敏感类型管理</h4>
          <div class="sensitive-types">
            <label v-for="type in sensitiveTypes" :key="type.id" class="type-item">
              <input 
                type="checkbox"
                :checked="config.enabled_sensitive_types.includes(type.id)"
                @change="toggleSensitiveType(type.id, $event)"
              />
              {{ type.name }}
            </label>
          </div>
        </div>
        
        <div class="settings-section">
          <h4>忽略目录</h4>
          <div class="ignore-dirs">
            <div v-for="(dir, index) in config.ignore_dir_names" :key="index" class="dir-item">
              <span>{{ dir }}</span>
              <button class="btn-remove" @click="removeIgnoreDir(index)">×</button>
            </div>
            <div class="add-dir">
              <input 
                type="text" 
                v-model="newIgnoreDir"
                placeholder="输入目录名"
                @keyup.enter="addIgnoreDir"
              />
              <button class="btn-small" @click="addIgnoreDir">添加</button>
            </div>
          </div>
        </div>
      </div>
      
      <div class="modal-footer">
        <button class="btn" @click="$emit('close')">取消</button>
        <button class="btn btn-primary" @click="handleSave">保存</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useAppStore } from '../stores/app'
import { storeToRefs } from 'pinia'
import { saveConfig, getSensitiveRules } from '../utils/tauri-api'

const emit = defineEmits<{
  close: []
}>()

const appStore = useAppStore()
const { config } = storeToRefs(appStore)

const newIgnoreDir = ref('')
const sensitiveTypes = ref<Array<{id: string, name: string}>>([])

onMounted(async () => {
  try {
    const rules = await getSensitiveRules()
    sensitiveTypes.value = rules.map(([id, name]) => ({ id, name }))
  } catch (error) {
    console.error('获取敏感规则失败:', error)
  }
})

const toggleSensitiveType = (typeId: string, event: Event) => {
  const checked = (event.target as HTMLInputElement).checked
  
  if (checked) {
    if (!config.value.enabled_sensitive_types.includes(typeId)) {
      config.value.enabled_sensitive_types.push(typeId)
    }
  } else {
    config.value.enabled_sensitive_types = config.value.enabled_sensitive_types.filter(
      id => id !== typeId
    )
  }
}

const addIgnoreDir = () => {
  if (newIgnoreDir.value.trim()) {
    config.value.ignore_dir_names.push(newIgnoreDir.value.trim())
    newIgnoreDir.value = ''
  }
}

const removeIgnoreDir = (index: number) => {
  config.value.ignore_dir_names.splice(index, 1)
}

const handleSave = async () => {
  try {
    await saveConfig(config.value)
    alert('配置已保存')
    emit('close')
  } catch (error) {
    console.error('保存配置失败:', error)
    alert('保存配置失败')
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
  width: min(600px, 90vw);
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
  color: #333;
}

.modal-body {
  flex: 1;
  overflow-y: auto;
  padding: 20px;
}

.settings-section {
  margin-bottom: 24px;
}

.settings-section h4 {
  font-size: 14px;
  font-weight: 600;
  margin-bottom: 12px;
  padding-bottom: 8px;
  border-bottom: 1px solid var(--border-color);
}

.setting-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 12px;
}

.setting-item label {
  font-size: 13px;
}

.setting-item input[type="number"] {
  width: 100px;
  padding: 5px 8px;
  border: 1px solid var(--border-color);
  border-radius: 4px;
}

.setting-item input[type="checkbox"] {
  cursor: pointer;
}

.hint {
  font-size: 12px;
  color: #999;
  margin-left: 8px;
}

.sensitive-types {
  display: grid;
  grid-template-columns: repeat(2, 1fr);
  gap: 10px;
}

.type-item {
  font-size: 13px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 6px;
}

.ignore-dirs {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.dir-item {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 6px 10px;
  background-color: #f5f5f5;
  border-radius: 4px;
  font-size: 13px;
}

.btn-remove {
  background: none;
  border: none;
  font-size: 18px;
  cursor: pointer;
  color: #999;
}

.btn-remove:hover {
  color: var(--error-color);
}

.add-dir {
  display: flex;
  gap: 8px;
  margin-top: 8px;
}

.add-dir input {
  flex: 1;
  padding: 5px 10px;
  border: 1px solid var(--border-color);
  border-radius: 4px;
  font-size: 13px;
}

.btn-small {
  padding: 5px 12px;
  border: 1px solid var(--border-color);
  background-color: white;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
}

.btn-small:hover {
  background-color: var(--bg-hover);
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
