import { invoke } from '@tauri-apps/api/core'
import { listen, UnlistenFn } from '@tauri-apps/api/event'
import type { 
  DirectoryNode, 
  ScanConfig, 
  ScanResultItem, 
  PreviewResult,
  AppConfig 
} from '../types'

// 获取目录树
export async function getDirectoryTree(path: string, showHidden = true): Promise<DirectoryNode[]> {
  return await invoke('get_directory_tree', { path, showHidden })
}

// 开始扫描
export async function startScan(config: ScanConfig): Promise<void> {
  return await invoke('scan_start', { config })
}

// 取消扫描
export async function cancelScan(): Promise<boolean> {
  return await invoke('scan_cancel')
}

// 预览文件
export async function previewFile(path: string, maxBytes?: number): Promise<PreviewResult> {
  return await invoke('preview_file', { path, maxBytes })
}

// 打开文件
export async function openFile(path: string): Promise<void> {
  return await invoke('open_file', { path })
}

// 删除文件
export async function deleteFile(path: string): Promise<void> {
  return await invoke('delete_file', { path })
}

// 导出报告
export async function exportReport(
  results: ScanResultItem[],
  format: 'csv' | 'json' | 'xlsx',
  savePath: string
): Promise<string> {
  return await invoke('export_report', { results, format, savePath })
}

// 获取日志
export async function getLogs(): Promise<string[]> {
  return await invoke('get_logs')
}

// 获取敏感规则
export async function getSensitiveRules(): Promise<Array<[string, string, boolean]>> {
  return await invoke('get_sensitive_rules')
}

// 保存配置
export async function saveConfig(config: AppConfig): Promise<void> {
  return await invoke('save_config', { config })
}

// 加载配置
export async function loadConfig(): Promise<AppConfig> {
  return await invoke('load_config')
}

// 监听扫描进度事件
export async function onScanProgress(callback: (data: any) => void): Promise<UnlistenFn> {
  return await listen('scan-progress', (event) => {
    callback(event.payload)
  })
}

// 监听扫描结果事件
export async function onScanResult(callback: (data: ScanResultItem) => void): Promise<UnlistenFn> {
  return await listen('scan-result', (event) => {
    callback(event.payload as ScanResultItem)
  })
}

// 监听扫描完成事件
export async function onScanFinished(callback: () => void): Promise<UnlistenFn> {
  return await listen('scan-finished', () => {
    callback()
  })
}

// 监听扫描错误事件
export async function onScanError(callback: (error: string) => void): Promise<UnlistenFn> {
  return await listen('scan-error', (event) => {
    callback(event.payload as string)
  })
}

// 监听扫描日志事件
export async function onScanLog(callback: (log: string) => void): Promise<UnlistenFn> {
  return await listen('scan-log', (event) => {
    callback(event.payload as string)
  })
}
