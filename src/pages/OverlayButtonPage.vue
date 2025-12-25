<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const templateId = ref('');
const templateName = ref('');
const isDragging = ref(false);
const position = ref({ x: 100, y: 100 });
const dragStart = ref({ x: 0, y: 0 });

onMounted(async () => {
  // Получаем параметры из URL
  const params = new URLSearchParams(window.location.hash.split('?')[1]);
  templateId.value = params.get('templateId') || '';
  templateName.value = params.get('templateName') || 'Button';

  // Загружаем сохраненную позицию
  const savedPos = localStorage.getItem(`overlay_button_position_${templateId.value}`);
  if (savedPos) {
    position.value = JSON.parse(savedPos);
  }

  // Обработчик для закрытия по Escape
  window.addEventListener('keydown', handleEscape);
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

function startDrag(event: MouseEvent) {
  isDragging.value = true;
  dragStart.value = {
    x: event.clientX - position.value.x,
    y: event.clientY - position.value.y
  };
}

function onDrag(event: MouseEvent) {
  if (isDragging.value) {
    position.value = {
      x: event.clientX - dragStart.value.x,
      y: event.clientY - dragStart.value.y
    };
  }
}

function stopDrag() {
  if (isDragging.value) {
    isDragging.value = false;
    // Сохраняем позицию
    localStorage.setItem(`overlay_button_position_${templateId.value}`, JSON.stringify(position.value));
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
  <div class="overlay" @mousemove="onDrag" @mouseup="stopDrag">
    <div
      class="overlay-button"
      :style="{
        left: position.x + 'px',
        top: position.y + 'px'
      }"
    >
      <div class="drag-handle" @mousedown="startDrag">
        ⋮⋮
      </div>
      <button class="action-button" @click="executeActions">
        {{ templateName }}
      </button>
      <button class="close-button" @click="closeOverlay">
        ✕
      </button>
    </div>
  </div>
</template>

<style scoped lang="stylus">
.overlay
  position fixed
  top 0
  left 0
  width 100vw
  height 100vh
  background transparent
  z-index 999999
  pointer-events auto
  user-select none

.overlay-button
  position absolute
  display flex
  align-items center
  gap 4px
  background rgba(0, 82, 204, 0.95)
  border-radius 12px
  padding 8px
  box-shadow 0 4px 20px rgba(0, 0, 0, 0.3)
  cursor move
  transition box-shadow 0.2s ease

  &:hover
    box-shadow 0 6px 30px rgba(0, 0, 0, 0.4)

.drag-handle
  color white
  font-size 16px
  padding 4px 8px
  cursor grab
  user-select none

  &:active
    cursor grabbing

.action-button
  padding 12px 24px
  background white
  color #0052cc
  border none
  border-radius 8px
  font-size 16px
  font-weight 600
  cursor pointer
  transition all 0.2s ease
  white-space nowrap

  &:hover
    background #f4f5f7
    transform scale(1.05)

  &:active
    transform scale(0.95)

.close-button
  width 32px
  height 32px
  background rgba(255, 255, 255, 0.2)
  color white
  border none
  border-radius 6px
  font-size 18px
  cursor pointer
  display flex
  align-items center
  justify-content center
  transition all 0.2s ease

  &:hover
    background rgba(255, 255, 255, 0.3)

  &:active
    background rgba(255, 255, 255, 0.4)
</style>
