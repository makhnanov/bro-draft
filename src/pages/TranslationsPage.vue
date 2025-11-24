<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const apiKey = ref('');
const anthropicApiKey = ref('');
const currentHotkey = ref<string | null>(null);
const isListeningForHotkey = ref(false);
const isProcessing = ref(false);

// Автосохранение API ключа при изменении
async function handleApiKeyChange() {
  try {
    await invoke('save_openai_api_key', { apiKey: apiKey.value });
    console.log('OpenAI API key saved');
  } catch (error) {
    console.error('Failed to save OpenAI API key:', error);
  }
}

// Автосохранение Anthropic API ключа при изменении
async function handleAnthropicApiKeyChange() {
  try {
    await invoke('save_anthropic_api_key', { apiKey: anthropicApiKey.value });
    console.log('Anthropic API key saved');
  } catch (error) {
    console.error('Failed to save Anthropic API key:', error);
  }
}

// Загружаем сохраненную горячую клавишу и API ключ при монтировании
onMounted(async () => {
  try {
    const savedHotkey = await invoke<string | null>('get_translation_hotkey');
    if (savedHotkey) {
      currentHotkey.value = savedHotkey;
    }

    // Загружаем сохраненные API ключи
    const savedApiKey = await invoke<string | null>('get_openai_api_key');
    if (savedApiKey) {
      apiKey.value = savedApiKey;
    }

    const savedAnthropicKey = await invoke<string | null>('get_anthropic_api_key');
    if (savedAnthropicKey) {
      anthropicApiKey.value = savedAnthropicKey;
    }

    // Слушаем событие от App.vue для начала захвата скриншота
    window.addEventListener('start-screenshot-capture', () => {
      console.log('Start screenshot capture event received!');
      captureScreenshot();
    });

    // Проверяем флаг автозапуска скриншота (установлен при переходе через Ctrl+PrintScreen)
    if ((window as any).__pendingScreenshotCapture) {
      (window as any).__pendingScreenshotCapture = false;
      console.log('Auto-starting screenshot capture from pending flag');
      captureScreenshot();
    }
  } catch (error) {
    console.error('Failed to load settings:', error);
  }
});

// Функция для начала прослушивания горячей клавиши
function startListeningForHotkey() {
  isListeningForHotkey.value = true;
  document.addEventListener('keydown', handleHotkeyCapture);
}

// Обработка нажатия клавиш для записи горячей клавиши
function handleHotkeyCapture(event: KeyboardEvent) {
  event.preventDefault();

  const modifiers = [];
  if (event.ctrlKey) modifiers.push('CommandOrControl');
  if (event.shiftKey) modifiers.push('Shift');
  if (event.altKey) modifiers.push('Alt');
  if (event.metaKey) modifiers.push('Super'); // Meta/Windows/Command key

  let key = event.key;

  // Игнорируем сами модификаторы
  if (['CONTROL', 'SHIFT', 'ALT', 'META', 'SUPER'].includes(key.toUpperCase())) {
    return;
  }

  // Обрабатываем специальные клавиши
  const specialKeys: Record<string, string> = {
    'PrintScreen': 'PrintScreen',
    'Print': 'PrintScreen',
    ' ': 'Space',
    'ArrowUp': 'Up',
    'ArrowDown': 'Down',
    'ArrowLeft': 'Left',
    'ArrowRight': 'Right',
    'Enter': 'Return',
    'Escape': 'Escape',
    'Tab': 'Tab',
    'Backspace': 'Backspace',
    'Delete': 'Delete',
    'Home': 'Home',
    'End': 'End',
    'PageUp': 'PageUp',
    'PageDown': 'PageDown',
    'Insert': 'Insert',
  };

  // Проверяем специальные клавиши
  if (specialKeys[key]) {
    key = specialKeys[key];
  } else {
    // Для обычных клавиш используем верхний регистр
    key = key.toUpperCase();
  }

  // Проверяем, что есть хотя бы один модификатор или это специальная клавиша
  if (modifiers.length === 0 && !specialKeys[event.key]) {
    alert('Пожалуйста, используйте комбинацию с модификатором (Ctrl, Shift, Alt, Super)');
    return;
  }

  const hotkey = modifiers.length > 0 ? `${modifiers.join('+')}+${key}` : key;

  document.removeEventListener('keydown', handleHotkeyCapture);
  isListeningForHotkey.value = false;

  saveHotkey(hotkey);
}

// Сохранение горячей клавиши
async function saveHotkey(hotkey: string) {
  try {
    // Сохраняем новую
    await invoke('save_translation_hotkey', { hotkey });
    currentHotkey.value = hotkey;

    alert(`Горячая клавиша установлена: ${hotkey}\n\nОбратите внимание: для активации горячей клавиши требуется перезапуск приложения.`);
  } catch (error) {
    console.error('Failed to save hotkey:', error);
    alert('Ошибка при сохранении горячей клавиши');
  }
}

// Захват скриншота через выбор области
async function captureScreenshot() {
  try {
    isProcessing.value = true;
    console.log('Starting area screenshot capture...');

    // Слушаем событие с выбранной областью
    const unlisten = await listen('area-selected', async (event: any) => {
      console.log('Received area-selected event:', event.payload);
      const { x, y, width, height, monitorIndex, monitorX, monitorY } = event.payload;

      try {
        // Окна уже закрыты в handle_area_selection
        console.log('Capturing area screenshot from monitor', monitorIndex);
        // Создаем скриншот выбранной области из нужного монитора
        const base64Image = await invoke<string>('capture_area_screenshot', {
          x,
          y,
          width,
          height,
          monitorIndex
        });
        console.log('Screenshot captured, length:', base64Image.length);

        // Рассчитываем абсолютные координаты на экране
        const absoluteX = monitorX + x;
        const absoluteY = monitorY + y;

        // Открываем popup окно в позиции выбранной области
        await invoke('open_translation_popup', {
          x: absoluteX,
          y: absoluteY,
          width,
          height,
          imageBase64: base64Image
        });
        console.log('Translation popup opened');
      } catch (error) {
        console.error('Failed to capture area screenshot:', error);
        alert('Ошибка при создании скриншота области: ' + error);
      } finally {
        isProcessing.value = false;
      }

      unlisten();
    });

    console.log('Listener registered, opening area selector...');

    // Открываем окно выбора области
    try {
      await invoke('open_area_selector');
      console.log('Area selector opened');
      isProcessing.value = false;
    } catch (error) {
      console.error('Failed to open area selector:', error);
      alert('Ошибка при открытии окна выбора области: ' + error);
      isProcessing.value = false;
      unlisten();
    }

  } catch (error) {
    console.error('Unexpected error in captureScreenshot:', error);
    alert('Неожиданная ошибка: ' + error);
    isProcessing.value = false;
  }
}

// Тестовый захват скриншота
async function testCapture() {
  await captureScreenshot();
}
</script>

<template>
  <div class="translations-page">
    <h1 class="page-title">Translations</h1>

    <div class="translations-content">
      <!-- Настройки API ключа -->
      <div class="settings-section">
        <h3>Настройки API</h3>
        <div class="input-group">
          <label>OpenAI API Key:</label>
          <input
            v-model="apiKey"
            @blur="handleApiKeyChange"
            type="password"
            placeholder="sk-..."
            class="api-input"
          />
        </div>
        <div class="input-group">
          <label>Anthropic API Key:</label>
          <input
            v-model="anthropicApiKey"
            @blur="handleAnthropicApiKeyChange"
            type="password"
            placeholder="sk-ant-..."
            class="api-input"
          />
        </div>
      </div>

      <!-- Настройка горячей клавиши -->
      <div class="hotkey-section">
        <h3>Горячая клавиша для скриншота</h3>
        <div class="hotkey-display">
          <span v-if="currentHotkey" class="hotkey-badge">{{ currentHotkey }}</span>
          <span v-else class="hotkey-badge empty">Не установлена</span>
        </div>
        <p class="hotkey-hint">
          Рекомендуется использовать комбинацию с модификаторами (Super/Ctrl/Shift/Alt)<br>
          Например: Super+PrintScreen, Ctrl+Shift+S
        </p>
        <button
          @click="startListeningForHotkey"
          :disabled="isListeningForHotkey"
          class="btn btn-primary"
        >
          {{ isListeningForHotkey ? 'Нажмите комбинацию клавиш...' : 'Выбрать горячую клавишу' }}
        </button>
        <button
          @click="testCapture"
          class="btn btn-secondary"
          :disabled="isProcessing"
        >
          Тестовый скриншот
        </button>
      </div>
    </div>

    <!-- Индикатор загрузки -->
    <div v-if="isProcessing" class="loading-overlay">
      <div class="spinner"></div>
      <p>Создание скриншота...</p>
    </div>
  </div>
</template>

<style scoped lang="stylus">
.translations-page
  display flex
  flex-direction column
  min-height 100vh
  padding 20px 25px

.page-title
  font-size 48px
  font-weight 700
  color #0052cc
  margin-bottom 30px
  text-align left

.translations-content
  display flex
  flex-direction column
  gap 30px

.settings-section,
.hotkey-section,
.result-section
  background #ffffff
  padding 25px
  border-radius 8px
  box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

  h3
    font-size 20px
    font-weight 600
    color #0052cc
    margin-bottom 15px

.input-group
  display flex
  flex-direction column
  gap 8px

  label
    font-size 14px
    font-weight 500
    color #333

.api-input
  padding 10px 15px
  border 2px solid #e0e0e0
  border-radius 6px
  font-size 14px
  transition border-color 0.2s ease

  &:focus
    outline none
    border-color #0052cc

.hotkey-display
  margin-bottom 15px

.hotkey-badge
  display inline-block
  padding 8px 16px
  background #0052cc
  color white
  border-radius 6px
  font-family monospace
  font-size 14px
  font-weight 600

  &.empty
    background #888

.hotkey-hint
  font-size 13px
  color #666
  margin-bottom 15px
  line-height 1.5

  br
    margin-bottom 5px

.btn
  padding 10px 20px
  border none
  border-radius 6px
  font-size 14px
  font-weight 500
  cursor pointer
  transition all 0.2s ease
  margin-right 10px

  &:disabled
    opacity 0.5
    cursor not-allowed

.btn-primary
  background #0052cc
  color white

  &:hover:not(:disabled)
    background #0747a6

.btn-secondary
  background #6c757d
  color white

  &:hover:not(:disabled)
    background #5a6268

.result-box
  padding 20px
  background #f8f9fa
  border-radius 6px
  border 1px solid #e0e0e0
  white-space pre-wrap
  word-break break-word
  margin-bottom 15px
  text-align left
  font-size 14px
  line-height 1.6

.dialog-overlay
  position fixed
  top 0
  left 0
  right 0
  bottom 0
  background rgba(0, 0, 0, 0.7)
  display flex
  align-items center
  justify-content center
  z-index 9999

.dialog
  background white
  padding 30px
  border-radius 12px
  max-width 90vw
  max-height 90vh
  overflow auto
  box-shadow 0 4px 20px rgba(0, 0, 0, 0.3)

  h2
    font-size 24px
    font-weight 600
    color #0052cc
    margin-bottom 20px
    text-align center

.dialog-preview
  margin-bottom 20px
  text-align center

  img
    max-width 100%
    max-height 60vh
    border-radius 8px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.2)

.dialog-actions
  display flex
  justify-content center
  gap 15px

.loading-overlay
  position fixed
  top 0
  left 0
  right 0
  bottom 0
  background rgba(255, 255, 255, 0.9)
  display flex
  flex-direction column
  align-items center
  justify-content center
  z-index 9998

  p
    margin-top 20px
    font-size 16px
    color #0052cc

.spinner
  width 50px
  height 50px
  border 4px solid #e0e0e0
  border-top 4px solid #0052cc
  border-radius 50%
  animation spin 1s linear infinite

@keyframes spin
  0%
    transform rotate(0deg)
  100%
    transform rotate(360deg)

@media (prefers-color-scheme: dark)
  .settings-section,
  .hotkey-section,
  .result-section
    background #333
    color #f6f6f6

  .api-input
    background #444
    color #f6f6f6
    border-color #555

  .result-box
    background #444
    border-color #555
    color #f6f6f6

  .dialog
    background #2f2f2f
    color #f6f6f6
</style>
