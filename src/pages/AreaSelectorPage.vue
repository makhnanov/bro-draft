<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';

console.log('AreaSelectorPage script loaded');

const isSelecting = ref(false);
const startX = ref(0);
const startY = ref(0);
const currentX = ref(0);
const currentY = ref(0);
const screenshot = ref<string>('');
const isLoading = ref(true);
const wrapperRef = ref<HTMLElement | null>(null);

onMounted(async () => {
  console.log('=== AreaSelectorPage onMounted START ===');
  console.log('Current URL:', window.location.href);
  console.log('Current path:', window.location.hash);

  // Получаем индекс монитора из URL
  const urlParams = new URLSearchParams(window.location.hash.split('?')[1]);
  const monitorIndex = parseInt(urlParams.get('monitor') || '0', 10);
  console.log('Monitor index:', monitorIndex);

  const currentWin = getCurrentWindow();
  console.log('Got current window');

  // Делаем окно полноэкранным
  try {
    await currentWin.setFullscreen(true);
    console.log('Set fullscreen');
    await currentWin.setAlwaysOnTop(true);
    console.log('Set always on top');
  } catch (error) {
    console.error('Error setting window properties:', error);
  }

  // Слушаем escape для закрытия
  document.addEventListener('keydown', handleEscape);
  console.log('Added escape listener');

  // Устанавливаем фокус на окно и wrapper
  setTimeout(async () => {
    try {
      await currentWin.setFocus();
      console.log('Set window focus');

      // Устанавливаем фокус на wrapper элемент
      if (wrapperRef.value) {
        wrapperRef.value.focus();
        console.log('Wrapper focused');
      }
    } catch (error) {
      console.error('Error setting focus:', error);
    }
  }, 200);

  // Получаем скриншот из state
  try {
    console.log('About to request screenshot from state for monitor', monitorIndex);
    const { invoke } = await import('@tauri-apps/api/core');
    console.log('invoke imported, calling get_stored_screenshot...');

    const screenshotData = await invoke<string>('get_stored_screenshot', { monitorIndex });
    console.log('Screenshot loaded from state, length:', screenshotData ? screenshotData.length : 'null');

    if (screenshotData && screenshotData.length > 0) {
      screenshot.value = screenshotData;
      console.log('Screenshot set successfully');
    } else {
      console.error('Screenshot is empty or null');
    }

    isLoading.value = false;
    console.log('Loading set to false');
  } catch (error) {
    console.error('Failed to get screenshot:', error);
    alert('Ошибка загрузки скриншота: ' + error);
    isLoading.value = false;
  }

  console.log('=== AreaSelectorPage onMounted END ===');
});

function handleMouseDown(event: MouseEvent) {
  isSelecting.value = true;
  startX.value = event.clientX;
  startY.value = event.clientY;
  currentX.value = event.clientX;
  currentY.value = event.clientY;
}

function handleMouseMove(event: MouseEvent) {
  if (!isSelecting.value) return;
  currentX.value = event.clientX;
  currentY.value = event.clientY;
}

async function handleMouseUp() {
  if (!isSelecting.value) return;

  const x = Math.min(startX.value, currentX.value);
  const y = Math.min(startY.value, currentY.value);
  const width = Math.abs(currentX.value - startX.value);
  const height = Math.abs(currentY.value - startY.value);

  // Минимальный размер области
  if (width < 10 || height < 10) {
    isSelecting.value = false;
    return;
  }

  // Отправляем событие с координатами
  try {
    const { emit } = await import('@tauri-apps/api/event');
    await emit('area-selected', { x, y, width, height });

    // Закрываем окно выбора
    const currentWin = getCurrentWindow();
    await currentWin.close();
  } catch (error) {
    console.error('Failed to send selection:', error);
  }
}

function handleEscape(event: KeyboardEvent) {
  if (event.key === 'Escape') {
    // Отменяем выбор и закрываем окно
    getCurrentWindow().close();
  }
}

function handleImageError(event: Event) {
  console.error('Failed to load screenshot image:', event);
  alert('Ошибка загрузки изображения скриншота');
}

// Вычисляемые стили для прямоугольника выделения
const selectionStyle = computed(() => {
  const x = Math.min(startX.value, currentX.value);
  const y = Math.min(startY.value, currentY.value);
  const width = Math.abs(currentX.value - startX.value);
  const height = Math.abs(currentY.value - startY.value);

  return {
    left: `${x}px`,
    top: `${y}px`,
    width: `${width}px`,
    height: `${height}px`,
    display: isSelecting.value ? 'block' : 'none'
  };
});
</script>

<template>
  <div
    class="area-selector-wrapper"
    tabindex="0"
    @keydown="handleEscape"
    ref="wrapperRef"
  >
    <!-- Индикатор загрузки -->
    <div v-if="isLoading || !screenshot" class="loading-overlay">
      <div class="spinner"></div>
      <p v-if="isLoading">Загрузка скриншота...</p>
      <p v-else>Скриншот не загружен</p>
    </div>

    <!-- Окно выбора области -->
    <div
      v-else-if="screenshot && screenshot.length > 0"
      class="area-selector"
      @mousedown="handleMouseDown"
      @mousemove="handleMouseMove"
      @mouseup="handleMouseUp"
    >
      <!-- Фоновый скриншот -->
      <img
        :src="`data:image/png;base64,${screenshot}`"
        class="screenshot-background"
        alt="Screenshot"
        @error="handleImageError"
      />

      <!-- Затемнение -->
      <div class="overlay"></div>

      <div class="instructions">
        <p>Выделите область для скриншота</p>
        <p class="hint">ESC - отмена</p>
      </div>

      <div
        class="selection-box"
        :style="selectionStyle"
      >
        <div class="selection-info">
          {{ Math.abs(currentX - startX) }} x {{ Math.abs(currentY - startY) }}
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="stylus">
.area-selector-wrapper
  position fixed
  top 0
  left 0
  width 100vw
  height 100vh
  background black
  z-index 999999
  margin 0
  padding 0
  outline none

  &:focus
    outline none

.loading-overlay
  position fixed
  top 0
  left 0
  width 100vw
  height 100vh
  background rgba(0, 0, 0, 0.9)
  display flex
  flex-direction column
  align-items center
  justify-content center
  z-index 100000

  p
    margin-top 20px
    font-size 16px
    color white

.spinner
  width 50px
  height 50px
  border 4px solid rgba(255, 255, 255, 0.3)
  border-top 4px solid white
  border-radius 50%
  animation spin 1s linear infinite

@keyframes spin
  0%
    transform rotate(0deg)
  100%
    transform rotate(360deg)

.area-selector
  position fixed
  top 0
  left 0
  width 100vw
  height 100vh
  cursor crosshair
  z-index 99999
  overflow hidden

.screenshot-background
  position absolute
  top 0
  left 0
  width 100%
  height 100%
  object-fit contain
  user-select none
  pointer-events none
  z-index 1

.overlay
  position absolute
  top 0
  left 0
  width 100%
  height 100%
  background rgba(0, 0, 0, 0.4)
  pointer-events none
  z-index 2

.instructions
  position absolute
  top 20px
  left 50%
  transform translateX(-50%)
  background rgba(0, 0, 0, 0.8)
  color white
  padding 15px 30px
  border-radius 8px
  text-align center
  pointer-events none
  z-index 100000

  p
    margin 5px 0
    font-size 16px

  .hint
    font-size 13px
    opacity 0.8

.selection-box
  position absolute
  border 3px solid #0052cc
  background rgba(255, 255, 255, 0.1)
  box-shadow 0 0 0 9999px rgba(0, 0, 0, 0.5)
  pointer-events none
  z-index 99999

.selection-info
  position absolute
  bottom -35px
  right 0
  background rgba(0, 0, 0, 0.9)
  color white
  padding 8px 12px
  border-radius 4px
  font-size 13px
  font-weight 600
  white-space nowrap
  font-family monospace
</style>
