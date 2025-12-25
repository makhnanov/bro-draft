<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const templateId = ref('');
const templateName = ref('');
const isDragging = ref(false);
const position = ref({ x: 100, y: 100 });
const dragStart = ref({ x: 0, y: 0 });
const buttonElement = ref<HTMLElement | null>(null);
let rafId: number | null = null;
let cursorCheckInterval: number | null = null;

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

  // Запускаем отслеживание позиции курсора
  startCursorTracking();
});

onUnmounted(() => {
  window.removeEventListener('keydown', handleEscape);
  stopCursorTracking();
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

// Отслеживание позиции курсора для управления click-through
function startCursorTracking() {
  // Используем глобальный mousemove для отслеживания позиции курсора
  document.addEventListener('mousemove', handleGlobalMouseMove);

  // Также проверяем периодически на случай, если курсор находится над кнопкой, но не движется
  cursorCheckInterval = window.setInterval(checkCursorPosition, 100);
}

function stopCursorTracking() {
  document.removeEventListener('mousemove', handleGlobalMouseMove);
  if (cursorCheckInterval !== null) {
    clearInterval(cursorCheckInterval);
    cursorCheckInterval = null;
  }
}

async function handleGlobalMouseMove(event: MouseEvent) {
  await checkCursorOverButton(event.clientX, event.clientY);
}

async function checkCursorPosition() {
  // Получаем текущую позицию курсора через DOM API
  // Это сработает, только если курсор над окном
  const button = buttonElement.value;
  if (!button) return;

  // Проверяем через :hover pseudo-class
  const isHovered = button.matches(':hover');
  await setCursorEventsState(!isHovered);
}

async function checkCursorOverButton(mouseX: number, mouseY: number) {
  const button = buttonElement.value;
  if (!button) return;

  const rect = button.getBoundingClientRect();
  const isOver = (
    mouseX >= rect.left &&
    mouseX <= rect.right &&
    mouseY >= rect.top &&
    mouseY <= rect.bottom
  );

  await setCursorEventsState(!isOver);
}

let currentIgnoreState = false; // Начальное состояние - НЕ игнорируем курсор, чтобы можно было взаимодействовать с кнопкой

async function setCursorEventsState(ignore: boolean) {
  // Во время перетаскивания всегда включаем события
  if (isDragging.value && ignore) {
    ignore = false;
  }

  // Избегаем лишних вызовов если состояние не изменилось
  if (currentIgnoreState === ignore) return;

  currentIgnoreState = ignore;

  try {
    await invoke('set_window_ignore_cursor_events', { ignore });
  } catch (error) {
    console.error('Failed to set cursor events:', error);
  }
}

function startDrag(event: MouseEvent) {
  event.preventDefault();
  isDragging.value = true;
  dragStart.value = {
    x: event.clientX - position.value.x,
    y: event.clientY - position.value.y
  };

  // Добавляем глобальные слушатели
  document.addEventListener('mousemove', onDrag);
  document.addEventListener('mouseup', stopDrag);
}

function onDrag(event: MouseEvent) {
  if (!isDragging.value) return;

  // Отменяем предыдущий RAF если есть
  if (rafId !== null) {
    cancelAnimationFrame(rafId);
  }

  // Используем requestAnimationFrame для плавного обновления
  rafId = requestAnimationFrame(() => {
    position.value = {
      x: event.clientX - dragStart.value.x,
      y: event.clientY - dragStart.value.y
    };
    rafId = null;
  });
}

async function stopDrag() {
  if (!isDragging.value) return;

  isDragging.value = false;

  // Отменяем RAF если есть
  if (rafId !== null) {
    cancelAnimationFrame(rafId);
    rafId = null;
  }

  // Удаляем глобальные слушатели
  document.removeEventListener('mousemove', onDrag);
  document.removeEventListener('mouseup', stopDrag);

  // Сохраняем позицию
  localStorage.setItem(`overlay_button_position_${templateId.value}`, JSON.stringify(position.value));

  // Состояние курсора будет автоматически обновлено через cursor tracking
}

async function closeOverlay() {
  try {
    // Команда hide_overlay_button теперь сама отправит событие
    await invoke('hide_overlay_button');
  } catch (error) {
    console.error('Error hiding overlay:', error);
  }
}
</script>

<template>
  <div class="overlay">
    <div
      ref="buttonElement"
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
  pointer-events none
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
  pointer-events auto

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
