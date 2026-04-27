// SVG 模块类型声明
declare module '*.svg' {
  const content: string
  export default content
}

// vite-plugin-svg-icons 类型声明
declare module 'virtual:svg-icons-register' {
  const content: any
  export default content
}
