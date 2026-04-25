export interface DirectoryNode {
  path: string
  name: string
  is_dir: boolean
  is_hidden: boolean
  has_children: boolean
  children?: DirectoryNode[]
}

export interface ScanConfig {
  selected_paths: string[]
  selected_extensions: string[]
  enabled_sensitive_types: string[]
  ignore_dir_names: string[]
  max_file_size_mb: number
  max_pdf_size_mb: number
  scan_concurrency: number
}

export interface ScanResultItem {
  file_path: string
  file_size: number
  modified_time: string
  counts: Record<string, number>
  total: number
  unsupported_preview: boolean
}

export interface HighlightRange {
  start: number
  end: number
  type_id: string
  type_name: string
}

export interface PreviewResult {
  content: string
  highlights: HighlightRange[]
}

export interface AppConfig {
  selected_paths: string[]
  selected_extensions: string[]
  enabled_sensitive_types: string[]
  ignore_dir_names: string[]
  max_file_size_mb: number
  max_pdf_size_mb: number
  scan_concurrency: number
  theme: string
  language: string
  enable_experimental_parsers: boolean
  enable_office_parsers: boolean
  delete_to_trash: boolean
}

export interface SensitiveRule {
  id: string
  name: string
  enabled_by_default: boolean
}
