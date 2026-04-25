// 格式化文件大小
export function formatFileSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  
  const k = 1024
  const sizes = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(k))
  
  return Math.round((bytes / Math.pow(k, i)) * 100) / 100 + ' ' + sizes[i]
}

// 格式化时间
export function formatTime(isoString: string): string {
  if (!isoString || isoString === '未知') return '未知'
  
  try {
    const date = new Date(isoString)
    const year = date.getFullYear()
    const month = String(date.getMonth() + 1).padStart(2, '0')
    const day = String(date.getDate()).padStart(2, '0')
    const hours = String(date.getHours()).padStart(2, '0')
    const minutes = String(date.getMinutes()).padStart(2, '0')
    const seconds = String(date.getSeconds()).padStart(2, '0')
    
    return `${year}-${month}-${day} ${hours}:${minutes}:${seconds}`
  } catch {
    return isoString
  }
}

// 获取文件扩展名
export function getFileExtension(filePath: string): string {
  const parts = filePath.split('.')
  return parts.length > 1 ? parts.pop()?.toLowerCase() || '' : ''
}

// 高亮文本中的敏感信息
export function highlightText(
  content: string, 
  highlights: Array<{start: number, end: number, type_id: string, type_name: string}>
): string {
  if (!highlights || highlights.length === 0) {
    return escapeHtml(content)
  }
  
  // 按起始位置排序
  const sorted = [...highlights].sort((a, b) => a.start - b.start)
  
  let result = ''
  let lastIndex = 0
  
  for (const highlight of sorted) {
    // 添加普通文本
    if (highlight.start > lastIndex) {
      result += escapeHtml(content.substring(lastIndex, highlight.start))
    }
    
    // 添加高亮文本
    const highlightedText = escapeHtml(content.substring(highlight.start, highlight.end))
    const colorClass = getColorClass(highlight.type_id)
    result += `<mark class="${colorClass}" title="${highlight.type_name}">${highlightedText}</mark>`
    
    lastIndex = highlight.end
  }
  
  // 添加剩余文本
  if (lastIndex < content.length) {
    result += escapeHtml(content.substring(lastIndex))
  }
  
  return result
}

// 转义 HTML
function escapeHtml(text: string): string {
  const div = document.createElement('div')
  div.textContent = text
  return div.innerHTML
}

// 根据类型获取颜色类
function getColorClass(typeId: string): string {
  const colorMap: Record<string, string> = {
    person_id: 'highlight-id',
    phone: 'highlight-phone',
    email: 'highlight-email',
    bank_card: 'highlight-bank',
    name: 'highlight-name',
    address: 'highlight-address',
    ip_address: 'highlight-ip',
    password: 'highlight-password',
  }
  
  return colorMap[typeId] || 'highlight-default'
}
