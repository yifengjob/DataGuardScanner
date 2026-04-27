import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'
import './style.css'
import { applyTheme, loadTheme } from './utils/theme'
// 导入 SVG Sprite 虚拟模块（自动生成）
import 'virtual:svg-icons-register'

// 初始化主题
const initialTheme = loadTheme()
applyTheme(initialTheme)

const app = createApp(App)
const pinia = createPinia()

app.use(pinia)
app.mount('#app')
