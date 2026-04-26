<template>
  <div class="tree-node">
    <div 
      class="node-content" 
      :style="{ paddingLeft: (level * 16 + 8) + 'px' }"
      :class="{ selected: checkState === 'checked', hidden: node.is_hidden }"
    >
      <span 
        v-if="node.is_dir && node.has_children" 
        class="expand-icon"
        @click="handleExpand"
      >
        {{ isExpanded ? '▼' : '▶' }}
      </span>
      <span v-else class="expand-icon-placeholder"></span>
      
      <input 
        ref="checkboxRef"
        type="checkbox" 
        :checked="checkState === 'checked'"
        @change="handleCheck"
        class="node-checkbox"
      />
      
      <span class="node-name">{{ node.name }}</span>
    </div>
    
    <div v-if="isExpanded && children.length > 0" class="node-children">
      <TreeNode
        v-for="child in children"
        :key="child.path"
        :node="child"
        :level="level + 1"
        :all-nodes-map="props.allNodesMap"
        @toggle="(path: string) => $emit('toggle', path)"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useAppStore } from '../stores/app'
import type { DirectoryNode } from '../types'
import { getDirectoryTree } from '../utils/tauri-api'

const props = defineProps<{
  node: DirectoryNode
  level: number
  allNodesMap: Map<string, DirectoryNode>
}>()

const emit = defineEmits<{
  toggle: [path: string]
}>()

const appStore = useAppStore()
const isExpanded = ref(false)
const children = ref<DirectoryNode[]>([])
const checkboxRef = ref<HTMLInputElement | null>(null)

// 计算节点的选中状态
const checkState = computed(() => {
  return appStore.getNodeCheckState(props.node.path, props.allNodesMap)
})

// 监听 checkState 变化，更新 indeterminate 属性
watch(checkState, (newState) => {
  if (checkboxRef.value) {
    checkboxRef.value.indeterminate = newState === 'indeterminate'
  }
}, { immediate: true })

// 加载子节点时构建映射表
const loadChildren = async () => {
  if (!props.node.is_dir || !props.node.has_children) return
  
  if (children.value.length === 0) {
    try {
      children.value = await getDirectoryTree(props.node.path)
      // 将子节点添加到父组件的映射表
      children.value.forEach(child => {
        props.allNodesMap.set(child.path, child)
      })
    } catch (error) {
      console.error('加载子目录失败:', error)
    }
  }
}

const handleExpand = async () => {
  if (!props.node.is_dir || !props.node.has_children) return
  
  isExpanded.value = !isExpanded.value
  
  if (isExpanded.value) {
    await loadChildren()
  }
}

const handleCheck = () => {
  emit('toggle', props.node.path)
}
</script>

<style scoped>
.tree-node {
  user-select: none;
}

.node-content {
  display: flex;
  align-items: center;
  padding: 4px 8px;
  cursor: pointer;
  border-radius: 3px;
  min-width: max-content;  /* 确保内容不被压缩 */
}

.node-content:hover {
  background-color: var(--bg-hover);
}

.node-content.selected {
  background-color: var(--bg-selected);
}

.node-content.hidden {
  color: var(--text-secondary);
  font-style: italic;
}

.expand-icon {
  width: 16px;
  text-align: center;
  font-size: 10px;
  cursor: pointer;
  margin-right: 4px;
}

.expand-icon:hover {
  color: var(--primary-color);
}

.expand-icon-placeholder {
  width: 16px;
  margin-right: 4px;
}

.node-checkbox {
  margin-right: 6px;
  cursor: pointer;
}

.node-checkbox:indeterminate {
  accent-color: var(--primary-color);
}

.node-name {
  flex: 1;
  font-size: 13px;
  /* 移除文本截断，允许水平滚动 */
}

.node-children {
  margin-left: 0;
}
</style>
