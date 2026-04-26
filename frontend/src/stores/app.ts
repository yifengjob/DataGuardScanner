import { defineStore } from 'pinia'
import { ref, computed } from 'vue'
import type {ScanResultItem, AppConfig, DirectoryNode} from '../types'

export const useAppStore = defineStore('app', () => {
  // 扫描结果
  const scanResults = ref<ScanResultItem[]>([])
  
  // 扫描状态
  const isScanning = ref(false)
  const scannedCount = ref(0)
  const currentFile = ref('')
  const logs = ref<string[]>([])
  
  // 配置
  const config = ref<AppConfig>({
    selected_paths: [],
    selected_extensions: [
      'txt', 'log', 'md', 'ini', 'conf', 'cfg', 'env',
      'js', 'ts', 'py', 'java', 'c', 'cpp', 'go', 'rs', 'php', 'rb', 'swift',
      'csv', 'json', 'xml', 'yaml', 'yml', 'properties', 'toml',
      'pdf',
    ],
    enabled_sensitive_types: [
      'person_id', 'phone', 'email', 'bank_card', 
      'address', 'ip_address', 'password'
    ],
    ignore_dir_names: ['node_modules', '.git', 'System Volume Information'],
    system_dirs: [], // 会在加载配置时从后端获取
    max_file_size_mb: 50,
    max_pdf_size_mb: 100,
    scan_concurrency: 8,
    theme: 'system',
    language: 'zh-CN',
    enable_experimental_parsers: false,
    enable_office_parsers: true,
    delete_to_trash: false, // 默认永久删除
  })
  
  // 目录树选中状态
  const selectedPaths = ref<Set<string>>(new Set())
  
  // 计算属性
  const sensitiveFilesCount = computed(() => scanResults.value.length)
  const errorCount = computed(() => logs.value.filter(l => l.includes('错误') || l.includes('失败')).length)
  const totalSensitiveItems = computed(() => 
    scanResults.value.reduce((sum, item) => sum + item.total, 0)
  )
  
  // 获取节点的选择状态：'checked' | 'unchecked' | 'indeterminate'
  function getNodeCheckState(nodePath: string, allNodes: Map<string, DirectoryNode>): 'checked' | 'unchecked' | 'indeterminate' {
    // 查找所有直接子节点（只找一层）
    const directChildren = Array.from(allNodes.values()).filter(n => {
      // 必须是子节点（路径以 nodePath/ 开头）
      if (!n.path.startsWith(nodePath + '/')) return false
      
      // 排除更深层的子孙节点（路径中只能有一个额外的 /）
      const relativePath = n.path.substring(nodePath.length + 1)
      return !relativePath.includes('/')
    })
    
    if (directChildren.length === 0) {
      // 叶子节点，直接返回自身状态
      return selectedPaths.value.has(nodePath) ? 'checked' : 'unchecked'
    }
    
    // 递归检查每个直接子节点的状态
    let checkedCount = 0
    let uncheckedCount = 0
    
    for (const child of directChildren) {
      const childState = getNodeCheckState(child.path, allNodes)
      if (childState === 'checked') {
        checkedCount++
      } else if (childState === 'unchecked') {
        uncheckedCount++
      } else {
        // 有子节点是半选，父节点也应该是半选
        return 'indeterminate'
      }
    }
    
    // 根据直接子节点的状态决定
    if (checkedCount === 0) {
      return 'unchecked'
    } else if (checkedCount === directChildren.length) {
      return 'checked'
    } else {
      return 'indeterminate'
    }
  }
  
  // 方法
  function addScanResult(item: ScanResultItem) {
    scanResults.value.push(item)
  }
  
  function clearScanResults() {
    scanResults.value = []
    scannedCount.value = 0
    logs.value = []
  }
  
  function removeResult(filePath: string) {
    const index = scanResults.value.findIndex(r => r.file_path === filePath)
    if (index !== -1) {
      scanResults.value.splice(index, 1)
    }
  }
  
  function togglePath(path: string) {
    if (selectedPaths.value.has(path)) {
      selectedPaths.value.delete(path)
    } else {
      selectedPaths.value.add(path)
    }
  }
  
  // 智能切换节点（考虑父子关系）
  function smartToggleNode(nodePath: string, allNodes: Map<string, DirectoryNode>) {
    const currentState = getNodeCheckState(nodePath, allNodes)
    
    // 查找所有子孙节点
    const descendants = Array.from(allNodes.values()).filter(n => 
      n.path.startsWith(nodePath + '/')
    )
    
    if (currentState === 'checked' || currentState === 'indeterminate') {
      // 当前是全选或半选，取消选中自己和所有子节点
      selectedPaths.value.delete(nodePath)
      descendants.forEach(d => selectedPaths.value.delete(d.path))
    } else {
      // 当前是未选中，选中自己和所有子节点
      selectedPaths.value.add(nodePath)
      descendants.forEach(d => selectedPaths.value.add(d.path))
    }
  }
  
  function selectAllPaths(paths: string[]) {
    paths.forEach(p => selectedPaths.value.add(p))
  }
  
  function deselectAllPaths() {
    selectedPaths.value.clear()
  }
  
  // 全选所有目录
  function selectAllDirectories(allNodes: DirectoryNode[]) {
    allNodes.filter(n => n.is_dir).forEach(n => {
      selectedPaths.value.add(n.path)
    })
  }
  
  // 全不选
  function deselectAllDirectories() {
    selectedPaths.value.clear()
  }
  
  // 获取有效的扫描路径（只保留叶子节点，避免重复扫描）
  // 例如：如果 C:\Users 和 C:\Users\John 都选中了，只返回 C:\Users\John
  function getEffectiveScanPaths(): string[] {
    const paths = Array.from(selectedPaths.value)
    
    // 按路径长度排序（短的在前）
    paths.sort((a, b) => a.length - b.length)
    
    const effectivePaths: string[] = []
    
    for (const path of paths) {
      // 检查这个路径是否是其他已选路径的祖先
      const hasDescendantSelected = paths.some(otherPath => 
        otherPath !== path && otherPath.startsWith(path + '\\')
      )
      
      // 如果没有子孙节点被选中，则这是一个有效的扫描路径
      if (!hasDescendantSelected) {
        effectivePaths.push(path)
      }
    }
    
    return effectivePaths
  }
  
  return {
    scanResults,
    isScanning,
    scannedCount,
    currentFile,
    logs,
    config,
    selectedPaths,
    sensitiveFilesCount,
    errorCount,
    totalSensitiveItems,
    addScanResult,
    clearScanResults,
    removeResult,
    togglePath,
    smartToggleNode,
    getNodeCheckState,
    selectAllPaths,
    deselectAllPaths,
    selectAllDirectories,
    deselectAllDirectories,
    getEffectiveScanPaths,
  }
})
