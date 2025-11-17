<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, computed } from 'vue';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { invoke } from '@tauri-apps/api/core';

const isSelecting = ref(false);
const startX = ref(0);
const startY = ref(0);
const currentX = ref(0);
const currentY = ref(0);
const screenshot = ref<string>('');
const isLoading = ref(true);
const wrapperRef = ref<HTMLElement | null>(null);
const monitorIndex = ref(0);

// Функция для закрытия всех окон area-selector
async function handleEscapeKey() {
  console.log('ESC pressed, closing all area-selector windows');
  try {
    await invoke('close_all_area_selectors');
  } catch (error) {
    console.error('Failed to close area-selector windows:', error);
  }
}

// Обработчик клавиш
let keydownHandler: ((event: KeyboardEvent) => void) | null = null;

onMounted(async () => {
  // Получаем индекс монитора из URL
  const urlParams = new URLSearchParams(window.location.hash.split('?')[1]);
  monitorIndex.value = parseInt(urlParams.get('monitor') || '0', 10);

  const currentWin = getCurrentWindow();

  // Добавляем обработчик ESC клавиши
  keydownHandler = (event: KeyboardEvent) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      handleEscapeKey();
    }
  };
  window.addEventListener('keydown', keydownHandler);

  // Окно уже создано в полноэкранном режиме в Rust, только устанавливаем фокус
  // Устанавливаем фокус на окно и wrapper
  setTimeout(async () => {
    try {
      await currentWin.setFocus();

      // Устанавливаем фокус на wrapper элемент
      if (wrapperRef.value) {
        wrapperRef.value.focus();
      }
    } catch (error) {
      console.error('Error setting focus:', error);
    }
  }, 100);

  // Получаем скриншот из state
  try {
    const screenshotData = await invoke<string>('get_stored_screenshot', { monitorIndex: monitorIndex.value });

    if (screenshotData && screenshotData.length > 0) {
      screenshot.value = screenshotData;
    } else {
      console.error('Screenshot is empty or null');
    }

    isLoading.value = false;
  } catch (error) {
    console.error('Failed to get screenshot:', error);
    alert('Ошибка загрузки скриншота: ' + error);
    isLoading.value = false;
  }
});

onBeforeUnmount(() => {
  // Удаляем обработчик клавиш
  if (keydownHandler) {
    window.removeEventListener('keydown', keydownHandler);
  }
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

  console.log('Mouse up - selection:', { x, y, width, height });

  // Минимальный размер области
  if (width < 10 || height < 10) {
    console.log('Selection too small, ignoring');
    isSelecting.value = false;
    return;
  }

  // Вызываем команду Rust для обработки выбора области
  // Она закроет все окна и отправит событие в главное окно
  try {
    console.log('Calling handle_area_selection with monitor:', monitorIndex.value);
    await invoke('handle_area_selection', {
      x,
      y,
      width,
      height,
      monitorIndex: monitorIndex.value
    });
    console.log('handle_area_selection completed');
  } catch (error) {
    console.error('Failed to handle selection:', error);
  }
}


function handleImageError() {
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
