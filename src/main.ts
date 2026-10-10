import { createApp } from 'vue'
import { createPinia } from 'pinia'
import App from './App.vue'

import './styles/tokens.css'
import './styles/global.css'

const app = createApp(App)

app.use(createPinia())

// 全局错误兜底
app.config.errorHandler = (err, _instance, info) => {
  console.error('[Vue Error]', err, info)
}

app.mount('#app')
