<script setup lang="ts">
import { ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";

const isSidebarCollapsed = ref(false);

function toggleSidebar() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value;
}

async function closeApp() {
  await getCurrentWindow().close();
}
</script>

<template>
  <!-- Боковое меню в стиле JIRA -->
  <div :class="['sidebar', { collapsed: isSidebarCollapsed }]">
    <button class="sidebar-toggle" @click="toggleSidebar">
      <svg viewBox="0 0 24 24" class="toggle-icon">
        <line x1="3" y1="12" x2="21" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <line x1="3" y1="6" x2="21" y2="6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <line x1="3" y1="18" x2="21" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
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
      <router-link to="/automatization" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <rect x="2" y="6" width="20" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M12 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <path d="M17 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <path d="M7 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span class="nav-text">Automatization</span>
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
    </nav>
  </div>

  <button class="close-button" @click="closeApp">
    <svg viewBox="0 0 24 24" class="close-icon">
      <line x1="18" y1="6" x2="6" y2="18" stroke="white" stroke-width="2" stroke-linecap="round"/>
      <line x1="6" y1="6" x2="18" y2="18" stroke="white" stroke-width="2" stroke-linecap="round"/>
    </svg>
  </button>

  <!-- Контент страниц -->
  <div class="main-content">
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

    .nav-icon
      margin-right 0

    .nav-text
      opacity 0
      width 0
      overflow hidden

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
  justify-content center
  color #ffffff
  transition background-color 0.2s ease
  border-bottom 1px solid rgba(255, 255, 255, 0.1)

  &:hover
    background-color rgba(255, 255, 255, 0.1)

.toggle-icon
  width 24px
  height 24px

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
</style>
<style lang="stylus">
*
  margin 0
  padding 0
  box-sizing border-box

:root
  font-family Inter, Avenir, Helvetica, Arial, sans-serif
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
