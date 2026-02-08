<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const iconDataUri = ref('');
const command = ref('');
const buttonId = ref('');

let isDragging = false;
let hideTimer: number | null = null;

onMounted(async () => {
  const params = new URLSearchParams(window.location.hash.split('?')[1]);
  buttonId.value = params.get('id') || '';
  const iconPath = params.get('iconPath') || '';
  command.value = params.get('command') || '';

  // Set background for this popup window only
  document.documentElement.style.background = '#0052cc';
  document.body.style.background = '#0052cc';
  document.body.style.overflow = 'hidden';
  document.body.style.margin = '0';
  document.body.style.scrollbarWidth = 'none';
  const appEl = document.getElementById('app');
  if (appEl) {
    appEl.style.width = '100%';
    appEl.style.height = '100%';
    appEl.style.overflow = 'hidden';
    appEl.style.background = '#0052cc';
  }

  if (iconPath) {
    try {
      iconDataUri.value = await invoke<string>('read_icon_base64', { path: iconPath });
    } catch (e) {
      console.error('Failed to load icon:', e);
    }
  }

  window.addEventListener('keydown', handleEscape);

  // Auto-hide after delay
  hideTimer = window.setTimeout(() => {
    hideTimer = null;
    invoke('slide_side_button_hide', { id: buttonId.value }).catch(console.error);
  }, 2000);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleEscape);
  if (hideTimer !== null) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }
});

function handleEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    invoke('hide_side_button', { id: buttonId.value }).catch(console.error);
  }
}

function onMouseEnter() {
  // Cancel any pending hide
  if (hideTimer !== null) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }
  if (!isDragging) {
    invoke('slide_side_button_show', { id: buttonId.value }).catch(console.error);
  }
}

function onMouseLeave() {
  if (!isDragging) {
    // Debounce hide — wait 500ms before sliding away
    if (hideTimer !== null) {
      clearTimeout(hideTimer);
    }
    hideTimer = window.setTimeout(() => {
      hideTimer = null;
      invoke('slide_side_button_hide', { id: buttonId.value }).catch(console.error);
    }, 500);
  }
}

async function onContainerMouseDown(event: MouseEvent) {
  // Only drag on left mouse button, and not on the icon button itself
  if (event.button !== 0) return;
  if ((event.target as HTMLElement).closest('.side-button')) return;

  isDragging = true;

  // Cancel any pending hide during drag
  if (hideTimer !== null) {
    clearTimeout(hideTimer);
    hideTimer = null;
  }

  event.preventDefault();

  // Delegate drag to Rust/GDK — it polls global mouse position natively
  try {
    await invoke('start_side_button_drag', {
      id: buttonId.value,
      startRootX: Math.round(event.screenX),
      startRootY: Math.round(event.screenY),
    });
  } catch (e) {
    console.error('Native drag failed:', e);
  }

  isDragging = false;
}

async function launchApp() {
  try {
    await invoke('launch_side_button_app', { command: command.value });
  } catch (error) {
    console.error('Error launching app:', error);
  }
}
</script>

<template>
  <div
    class="side-button-container"
    @mouseenter="onMouseEnter"
    @mouseleave="onMouseLeave"
    @mousedown="onContainerMouseDown"
  >
    <button class="side-button" @click="launchApp" @mousedown.stop>
      <img
        v-if="iconDataUri"
        :src="iconDataUri"
        alt="icon"
        class="side-button-icon"
      />
      <span v-else class="side-button-fallback">&#9654;</span>
    </button>
  </div>
</template>

<style scoped lang="stylus">
.side-button-container
  width 100%
  height 100%
  display flex
  align-items center
  justify-content center
  background #0052cc
  border-radius 12px
  padding 4px
  cursor grab

  &:active
    cursor grabbing

.side-button
  width 40px
  height 40px
  background white
  border none
  border-radius 8px
  cursor pointer
  display flex
  align-items center
  justify-content center
  padding 4px
  transition background 0.15s ease

  &:hover
    background #f0f1f3

  &:active
    background #e0e1e3

.side-button-icon
  width 100%
  height 100%
  object-fit contain

.side-button-fallback
  font-size 18px
  color #0052cc
</style>
