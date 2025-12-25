import { createRouter, createWebHashHistory } from 'vue-router'
import { invoke } from '@tauri-apps/api/core'
import HomePage from '../pages/HomePage.vue'
import AliasesPage from '../pages/AliasesPage.vue'
import RecognitionPage from '../pages/RecognitionPage.vue'
import WatcherPage from '../pages/WatcherPage.vue'
import ConvertersPage from '../pages/ConvertersPage.vue'
import ShellPage from '../pages/ShellPage.vue'
import AutomatizationPage from '../pages/AutomatizationPage.vue'
import CirclePage from '../pages/CirclePage.vue'
import TestPage from '../pages/TestPage.vue'
import YouTubePage from '../pages/YouTubePage.vue'
import EditorPage from '../pages/EditorPage.vue'
import ScreenshotsPage from '../pages/ScreenshotsPage.vue'
import ScreenshotPopupPage from '../pages/ScreenshotPopupPage.vue'
import AreaSelectorPage from '../pages/AreaSelectorPage.vue'
import SettingsPage from '../pages/SettingsPage.vue'
import ProjectsPage from '../pages/ProjectsPage.vue'
import NotificationsPage from '../pages/NotificationsPage.vue'
import KeyboardPage from '../pages/KeyboardPage.vue'
import StreamingPage from '../pages/StreamingPage.vue'
import TerminalPage from '../pages/TerminalPage.vue'
import ButtonsPage from '../pages/ButtonsPage.vue'
import OverlayButtonPage from '../pages/OverlayButtonPage.vue'

const routes = [
  {
    path: '/',
    name: 'Home',
    component: HomePage
  },
  {
    path: '/aliases',
    name: 'Aliases',
    component: AliasesPage
  },
  {
    path: '/recognition',
    name: 'Recognition',
    component: RecognitionPage
  },
  {
    path: '/watcher',
    name: 'Watcher',
    component: WatcherPage
  },
  {
    path: '/converters',
    name: 'Converters',
    component: ConvertersPage
  },
  {
    path: '/shell',
    name: 'Shell',
    component: ShellPage
  },
  {
    path: '/automatization',
    name: 'Automatization',
    component: AutomatizationPage
  },
  {
    path: '/circle',
    name: 'Circle',
    component: CirclePage
  },
  {
    path: '/test',
    name: 'Test',
    component: TestPage
  },
  {
    path: '/youtube',
    name: 'YouTube',
    component: YouTubePage
  },
  {
    path: '/editor',
    name: 'Editor',
    component: EditorPage
  },
  {
    path: '/screenshots',
    name: 'Screenshots',
    component: ScreenshotsPage
  },
  {
    path: '/screenshot-popup',
    name: 'ScreenshotPopup',
    component: ScreenshotPopupPage
  },
  {
    path: '/area-selector',
    name: 'AreaSelector',
    component: AreaSelectorPage
  },
  {
    path: '/settings',
    name: 'Settings',
    component: SettingsPage
  },
  {
    path: '/projects',
    name: 'Projects',
    component: ProjectsPage
  },
  {
    path: '/notifications',
    name: 'Notifications',
    component: NotificationsPage
  },
  {
    path: '/keyboard',
    name: 'Keyboard',
    component: KeyboardPage
  },
  {
    path: '/streaming',
    name: 'Streaming',
    component: StreamingPage
  },
  {
    path: '/terminal',
    name: 'Terminal',
    component: TerminalPage
  },
  {
    path: '/buttons',
    name: 'Buttons',
    component: ButtonsPage
  },
  {
    path: '/overlay-button',
    name: 'OverlayButton',
    component: OverlayButtonPage
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

// Флаг для предотвращения сохранения при первой загрузке
let isInitialLoad = true

// Восстанавливаем последний маршрут при загрузке
router.isReady().then(async () => {
  try {
    const lastRoute = await invoke<string | null>('get_last_route')
    if (lastRoute && lastRoute !== '/' && router.currentRoute.value.path === '/') {
      console.log('Restoring last route:', lastRoute)
      await router.push(lastRoute)
    }
  } catch (err) {
    console.error('Failed to get last route:', err)
  } finally {
    // Снимаем флаг после восстановления маршрута
    setTimeout(() => {
      isInitialLoad = false
    }, 100)
  }
})

// Сохраняем маршрут при каждом переходе (кроме начальной загрузки)
router.afterEach((to) => {
  // Не сохраняем маршрут при начальной загрузке, area-selector и screenshot-popup
  if (!isInitialLoad && to.path !== '/area-selector' && to.path !== '/screenshot-popup') {
    invoke('save_last_route', { route: to.path }).catch(err => {
      console.error('Failed to save route:', err)
    })
  }
})

export default router
