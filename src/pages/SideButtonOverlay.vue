<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, PhysicalPosition } from '@tauri-apps/api/window';

const iconDataUri = ref('');
const command = ref('');
const buttonId = ref('');

let isDragging = false;
let dragStartX = 0;
let dragStartY = 0;
let windowStartX = 0;
let windowStartY = 0;

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
  window.addEventListener('mousemove', onGlobalMouseMove);
  window.addEventListener('mouseup', onGlobalMouseUp);

  // Auto-hide after delay — Rust handles the window movement
  setTimeout(() => {
    invoke('slide_side_button_hide', { id: buttonId.value }).catch(console.error);
  }, 2000);
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleEscape);
  window.removeEventListener('mousemove', onGlobalMouseMove);
  window.removeEventListener('mouseup', onGlobalMouseUp);
});

function handleEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    invoke('hide_side_button', { id: buttonId.value }).catch(console.error);
  }
}

function onMouseEnter() {
  if (!isDragging) {
    invoke('slide_side_button_show', { id: buttonId.value }).catch(console.error);
  }
}

function onMouseLeave() {
  if (!isDragging) {
    invoke('slide_side_button_hide', { id: buttonId.value }).catch(console.error);
  }
}

async function onContainerMouseDown(event: MouseEvent) {
  // Only drag on left mouse button, and not on the icon button itself
  if (event.button !== 0) return;
  if ((event.target as HTMLElement).closest('.side-button')) return;

  isDragging = true;
  dragStartX = event.screenX;
  dragStartY = event.screenY;

  try {
    const win = getCurrentWindow();
    const pos = await win.outerPosition();
    windowStartX = pos.x;
    windowStartY = pos.y;
  } catch (e) {
    console.error('Failed to get window position for drag:', e);
    isDragging = false;
  }

  event.preventDefault();
}

function onGlobalMouseMove(event: MouseEvent) {
  if (!isDragging) return;

  const newX = windowStartX + (event.screenX - dragStartX);
  const newY = windowStartY + (event.screenY - dragStartY);

  const win = getCurrentWindow();
  win.setPosition(new PhysicalPosition(newX, newY)).catch(console.error);
}

function onGlobalMouseUp() {
  if (!isDragging) return;
  isDragging = false;

  // Tell Rust to update the base position after drag
  invoke('update_side_button_base', { id: buttonId.value }).catch(console.error);
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
