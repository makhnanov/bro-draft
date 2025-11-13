import { createRouter, createWebHashHistory } from 'vue-router'
import HomePage from '../pages/HomePage.vue'
import AliasesPage from '../pages/AliasesPage.vue'
import RecognitionPage from '../pages/RecognitionPage.vue'
import WatcherPage from '../pages/WatcherPage.vue'
import ConvertersPage from '../pages/ConvertersPage.vue'
import AutomatizationPage from '../pages/AutomatizationPage.vue'
import TestPage from '../pages/TestPage.vue'
import YouTubePage from '../pages/YouTubePage.vue'
import TranslationsPage from '../pages/TranslationsPage.vue'
import AreaSelectorPage from '../pages/AreaSelectorPage.vue'

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
    path: '/automatization',
    name: 'Automatization',
    component: AutomatizationPage
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
    path: '/translations',
    name: 'Translations',
    component: TranslationsPage
  },
  {
    path: '/area-selector',
    name: 'AreaSelector',
    component: AreaSelectorPage
  }
]

const router = createRouter({
  history: createWebHashHistory(),
  routes
})

export default router
