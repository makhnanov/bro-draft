<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRoute } from "vue-router";
import { invoke } from '@tauri-apps/api/core';

const route = useRoute();
const isSidebarCollapsed = ref(false);
const toggleText = ref('BroLauncher');

// Проверяем, является ли текущая страница area-selector
const isAreaSelector = computed(() => route.path === '/area-selector');

function toggleSidebar() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value;
  // Меняем текст только при разворачивании меню (когда текст становится видимым)
  if (!isSidebarCollapsed.value) {
    toggleText.value = toggleText.value === 'BroLauncher' ? 'Multitool' : 'BroLauncher';
  }
}

async function closeApp() {
  await getCurrentWindow().close();
}

// Локальная обработка горячих клавиш (работает только когда окно в фокусе)
function handleKeydown(event: KeyboardEvent) {
  // F12 - переключение DevTools
  if (event.key === 'F12') {
    event.preventDefault();
    invoke('toggle_devtools').catch(err => console.error('Failed to toggle devtools:', err));
  }

  // F5 - перезагрузка страницы
  if (event.key === 'F5') {
    event.preventDefault();
    window.location.reload();
  }

  // F11 - полноэкранный режим
  if (event.key === 'F11') {
    event.preventDefault();
    getCurrentWindow().isFullscreen().then(isFullscreen => {
      getCurrentWindow().setFullscreen(!isFullscreen);
    }).catch(err => console.error('Failed to toggle fullscreen:', err));
  }
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown);
});
</script>

<template>
  <!-- Боковое меню в стиле JIRA (скрываем для area-selector) -->
  <div v-if="!isAreaSelector" :class="['sidebar', { collapsed: isSidebarCollapsed }]">
    <button class="sidebar-toggle" @click="toggleSidebar">
      <div class="toggle-content">
        <svg viewBox="0 0 24 24" class="toggle-icon">
          <line x1="3" y1="12" x2="21" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <line x1="3" y1="6" x2="21" y2="6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <line x1="3" y1="18" x2="21" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span class="toggle-text">{{ toggleText }}</span>
      </div>
    </button>

    <nav class="sidebar-nav">
      <router-link to="/" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M3 9l9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <polyline points="9 22 9 12 15 12 15 22" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Home</span>
      </router-link>
      <router-link to="/aliases" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <circle cx="12" cy="7" r="4" fill="none" stroke="currentColor" stroke-width="2"/>
        </svg>
        <span class="nav-text">Aliases</span>
      </router-link>
      <router-link to="/recognition" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="2"/>
        </svg>
        <span class="nav-text">Recognition</span>
      </router-link>
      <router-link to="/watcher" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/>
          <polyline points="12 6 12 12 16 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Watcher</span>
      </router-link>
      <router-link to="/converters" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <polyline points="16 3 21 3 21 8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <polyline points="8 21 3 21 3 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <line x1="21" y1="3" x2="14" y2="10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <line x1="3" y1="21" x2="10" y2="14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span class="nav-text">Converters</span>
      </router-link>
      <router-link to="/shell" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <rect x="2" y="3" width="20" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
          <line x1="8" y1="21" x2="16" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <line x1="12" y1="17" x2="12" y2="21" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <line x1="7" y1="7" x2="7" y2="13" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <line x1="11" y1="7" x2="11" y2="13" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <line x1="15" y1="7" x2="15" y2="13" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span class="nav-text">Shell</span>
      </router-link>
      <router-link to="/automatization" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <rect x="2" y="6" width="20" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M12 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <path d="M17 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <path d="M7 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span class="nav-text">Automatization</span>
      </router-link>
      <router-link to="/circle" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" stroke-width="2"/>
        </svg>
        <span class="nav-text">Circle</span>
      </router-link>
      <router-link to="/test" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M9 11l3 3L22 4" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M21 12v7a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Test</span>
      </router-link>
      <router-link to="/youtube" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814zM9.545 15.568V8.432L15.818 12l-6.273 3.568z" fill="currentColor"/>
        </svg>
        <span class="nav-text">YouTube</span>
      </router-link>
      <router-link to="/editor" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M3 17.25V21h3.75L17.81 9.94l-3.75-3.75L3 17.25zM20.71 7.04c.39-.39.39-1.02 0-1.41l-2.34-2.34a.9959.9959 0 0 0-1.41 0l-1.83 1.83 3.75 3.75 1.83-1.83z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Editor</span>
      </router-link>
      <router-link to="/translations" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M12.87 15.07l-2.54-2.51.03-.03c1.74-1.94 2.98-4.17 3.71-6.53H17V4h-7V2H8v2H1v1.99h11.17C11.5 7.92 10.44 9.75 9 11.35 8.07 10.32 7.3 9.19 6.69 8h-2c.73 1.63 1.73 3.17 2.98 4.56l-5.09 5.02L4 19l5-5 3.11 3.11.76-2.04zM18.5 10h-2L12 22h2l1.12-3h4.75L21 22h2l-4.5-12zm-2.62 7l1.62-4.33L19.12 17h-3.24z" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Translations</span>
      </router-link>
      <router-link to="/projects" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M10 4H4c-1.1 0-1.99.9-1.99 2L2 18c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2h-8l-2-2z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Projects</span>
      </router-link>
      <router-link to="/notifications" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M13.73 21a2 2 0 0 1-3.46 0" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Notifications</span>
      </router-link>
      <router-link to="/settings" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M12 15.5A3.5 3.5 0 0 1 8.5 12 3.5 3.5 0 0 1 12 8.5a3.5 3.5 0 0 1 3.5 3.5 3.5 3.5 0 0 1-3.5 3.5m7.43-2.53c.04-.32.07-.64.07-.97 0-.33-.03-.66-.07-1l2.11-1.63c.19-.15.24-.42.12-.64l-2-3.46c-.12-.22-.39-.31-.61-.22l-2.49 1c-.52-.39-1.06-.73-1.69-.98l-.37-2.65A.506.506 0 0 0 14 2h-4c-.25 0-.46.18-.5.42l-.37 2.65c-.63.25-1.17.59-1.69.98l-2.49-1c-.22-.09-.49 0-.61.22l-2 3.46c-.13.22-.07.49.12.64L4.57 11c-.04.34-.07.67-.07 1 0 .33.03.65.07.97l-2.11 1.66c-.19.15-.25.42-.12.64l2 3.46c.12.22.39.3.61.22l2.49-1.01c.52.4 1.06.74 1.69.99l.37 2.65c.04.24.25.42.5.42h4c.25 0 .46-.18.5-.42l.37-2.65c.63-.26 1.17-.59 1.69-.99l2.49 1.01c.22.08.49 0 .61-.22l2-3.46c.12-.22.07-.49-.12-.64l-2.11-1.66z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Settings</span>
      </router-link>
    </nav>
  </div>

  <button v-if="!isAreaSelector" class="close-button" @click="closeApp">
    <svg viewBox="0 0 24 24" class="close-icon">
      <line x1="18" y1="6" x2="6" y2="18" stroke="white" stroke-width="2" stroke-linecap="round"/>
      <line x1="6" y1="6" x2="18" y2="18" stroke="white" stroke-width="2" stroke-linecap="round"/>
    </svg>
  </button>

  <!-- Контент страниц -->
  <div :class="['main-content', { 'fullscreen': isAreaSelector }]">
    <router-view />
  </div>
</template>

<style scoped lang="stylus">
// Боковое меню в стиле JIRA
.sidebar
    position fixed
    left 0
    top 0
    bottom 0
    width 240px
    background linear-gradient(180deg, #0747a6 0%, #0052cc 100%)
    box-shadow 2px 0 8px rgba(0, 0, 0, 0.15)
    transition width 0.3s ease
    z-index 999
    display flex
    flex-direction column

    &.collapsed
        width 64px

        .sidebar-toggle
            justify-content center

        .nav-icon
            margin-right 0

        .nav-text
            opacity 0
            width 0
            overflow hidden

        .toggle-text
            opacity 0
            width 0
            margin-left 0

        ~ .main-content
            margin-left 64px

.sidebar-toggle
    width 100%
    height 64px
    background transparent
    border none
    cursor pointer
    display flex
    align-items center
    justify-content flex-start
    padding 0 20px
    color #ffffff
    transition background-color 0.2s ease
    border-bottom 1px solid rgba(255, 255, 255, 0.1)

    &:hover
        background-color rgba(255, 255, 255, 0.1)

.toggle-content
    display flex
    align-items center
    justify-content flex-start
    width 100%

.toggle-icon
    width 24px
    height 24px
    min-width 24px
    flex-shrink 0

.toggle-text
    margin-left 12px
    font-size 18px
    font-weight 600
    white-space nowrap
    overflow hidden
    opacity 1
    transition opacity 0.3s ease, width 0.3s ease, margin-left 0.3s ease

.sidebar-nav
    flex 1
    padding 16px 0
    overflow-y auto

.nav-item
    display flex
    align-items center
    padding 12px 20px
    color rgba(255, 255, 255, 0.8)
    text-decoration none
    font-size 14px
    font-weight 500
    transition all 0.2s ease
    position relative
    white-space nowrap

    &:hover
        background-color rgba(255, 255, 255, 0.1)
        color #ffffff

    &.router-link-active
        background-color rgba(255, 255, 255, 0.15)
        color #ffffff

        &::before
            content ''
            position absolute
            left 0
            top 0
            bottom 0
            width 3px
            background-color #ffffff

.nav-icon
    width 20px
    height 20px
    min-width 20px
    margin-right 12px

.nav-text
    opacity 1
    transition opacity 0.2s ease

.close-button
    position fixed
    top 16px
    right 16px
    width 40px
    height 40px
    background-color #dc3545
    border none
    border-radius 8px 8px 8px 0px
    cursor pointer
    display flex
    align-items center
    justify-content center
    padding 0
    transition all 0.2s ease
    box-shadow 0 2px 8px rgba(220, 53, 69, 0.3)
    z-index 1000

    &:hover
        background-color #c82333
        box-shadow 0 4px 12px rgba(220, 53, 69, 0.5)
        transform scale(1.05)

    &:active
        transform scale(0.95)

.close-icon
    width 20px
    height 20px

.main-content
    margin-left 240px
    // padding 120px 20px 20px 20px
    transition margin-left 0.3s ease
    min-height 100vh

    &.fullscreen
        margin-left 0
        position fixed
        top 0
        left 0
        width 100vw
        height 100vh
        z-index 99999
</style>
<style lang="stylus">
*
    margin 0
    padding 0
    box-sizing border-box

:root
    font-family -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif
    font-size 16px
    color #0f0f0f
    background-color #f6f6f6
    font-synthesis none
    text-rendering optimizeLegibility
    -webkit-font-smoothing antialiased
    -moz-osx-font-smoothing grayscale
    -webkit-text-size-adjust 100%

html, body
    width 100%
    height 100%
    overflow hidden

@media (prefers-color-scheme: dark)
    :root
        color #f6f6f6
        background-color #2f2f2f
</style>
