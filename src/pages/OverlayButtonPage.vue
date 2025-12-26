<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow, LogicalSize } from '@tauri-apps/api/window';

const templateId = ref('');
const templateName = ref('');

onMounted(async () => {
  console.log('[OverlayButton] Component mounted');

  // Получаем параметры из URL
  const params = new URLSearchParams(window.location.hash.split('?')[1]);
  templateId.value = params.get('templateId') || '';
  templateName.value = params.get('templateName') || 'Button';

  console.log('[OverlayButton] Template:', templateName.value);

  // Обработчик для закрытия по Escape
  window.addEventListener('keydown', handleEscape);

  // Ждем рендеринга DOM
  await nextTick();
  await new Promise(resolve => setTimeout(resolve, 100));

  try {
    // Измеряем размер контента
    const buttonElement = document.querySelector('.overlay-button') as HTMLElement;
    if (!buttonElement) {
      console.error('[OverlayButton] Button element not found!');
      return;
    }

    const rect = buttonElement.getBoundingClientRect();
    console.log('[OverlayButton] Content size:', rect.width, 'x', rect.height);

    // Добавляем небольшой запас для padding
    const width = Math.ceil(rect.width) + 2;
    const height = Math.ceil(rect.height) + 2;

    console.log('[OverlayButton] Setting window size to:', width, 'x', height);

    // Изменяем размер окна под контент
    const window = getCurrentWindow();
    await window.setSize(new LogicalSize(width, height));

    // Отключаем resizable после подстройки
    await window.setResizable(false);

    console.log('[OverlayButton] Window resized successfully');
  } catch (error) {
    console.error('[OverlayButton] Error resizing window:', error);
  }
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleEscape);
});

function handleEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    closeOverlay();
  }
}

async function executeActions() {
  try {
    await invoke('execute_button_actions', { templateId: templateId.value });
    console.log('Actions executed successfully');
  } catch (error) {
    console.error('Error executing actions:', error);
    alert('Ошибка при выполнении действий: ' + error);
  }
}

async function closeOverlay() {
  try {
    await invoke('hide_overlay_button');
  } catch (error) {
    console.error('Error hiding overlay:', error);
  }
}
</script>

<template>
  <div class="overlay-button" data-tauri-drag-region>
    <div class="drag-handle" data-tauri-drag-region>
      ⋮⋮
    </div>
    <div class="button-container" @mousedown.stop>
      <button class="action-button" @click="executeActions" @mousedown.stop>
        {{ templateName }}
      </button>
      <button class="close-button" @click="closeOverlay" @mousedown.stop>
        ✕
      </button>
    </div>
  </div>
</template>

<style lang="stylus">
*
  margin 0
  padding 0
  box-sizing border-box

html, body
  margin 0
  padding 0
  overflow hidden

#app
  display inline-block
</style>

<style scoped lang="stylus">
.overlay-button
  display inline-flex
  align-items center
  gap 2px
  background rgba(0, 82, 204, 0.95)
  border-radius 6px
  padding 3px 4px
  box-shadow 0 2px 10px rgba(0, 0, 0, 0.3)

.drag-handle
  color white
  font-size 12px
  padding 2px 4px
  cursor grab
  user-select none
  line-height 1
  flex-shrink 0

  &:active
    cursor grabbing

.button-container
  display flex
  align-items center
  gap 2px
  flex-grow 1
  pointer-events auto

.action-button
  padding 4px 10px
  background white
  color #0052cc
  border none
  border-radius 4px
  font-size 12px
  font-weight 600
  cursor pointer
  white-space nowrap
  flex-grow 1
  pointer-events auto

  &:hover
    background #f4f5f7

  &:active
    background #e8e9eb

.close-button
  width 20px
  height 20px
  background rgba(255, 255, 255, 0.2)
  color white
  border none
  border-radius 3px
  font-size 12px
  cursor pointer
  display flex
  align-items center
  justify-content center
  flex-shrink 0
  pointer-events auto

  &:hover
    background rgba(255, 255, 255, 0.3)

  &:active
    background rgba(255, 255, 255, 0.4)
</style>
