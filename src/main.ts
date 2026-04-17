import { createApp } from 'vue'
import { createPinia } from 'pinia'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import 'virtual:uno.css'
import './styles/global.css'

const router = createRouter({
  history: createWebHistory(),
  routes: [
    {
      path: '/',
      name: 'main',
      component: () => import('./pages/main/index.vue'),
    },
    {
      path: '/preference',
      name: 'preference',
      component: () => import('./pages/preference/index.vue'),
    },
    {
      path: '/status',
      name: 'status',
      component: () => import('./pages/status/index.vue'),
    },
  ],
})

const pinia = createPinia()
const app = createApp(App)

app.use(pinia)
app.use(router)
app.mount('#app')
