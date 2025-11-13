<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const apiKey = ref('');
const currentHotkey = ref<string | null>(null);
const isListeningForHotkey = ref(false);
const screenshot = ref<string | null>(null);
const showActionDialog = ref(false);
const translationResult = ref('');
const isProcessing = ref(false);

// Автосохранение API ключа при изменении
async function handleApiKeyChange() {
  if (apiKey.value) {
    try {
      await invoke('save_openai_api_key', { apiKey: apiKey.value });
      console.log('API key saved');
    } catch (error) {
      console.error('Failed to save API key:', error);
    }
  }
}

// Загружаем сохраненную горячую клавишу и API ключ при монтировании
onMounted(async () => {
  try {
    const savedHotkey = await invoke<string | null>('get_translation_hotkey');
    if (savedHotkey) {
      currentHotkey.value = savedHotkey;
    }

    // Загружаем сохраненный API ключ
    const savedApiKey = await invoke<string | null>('get_openai_api_key');
    if (savedApiKey) {
      apiKey.value = savedApiKey;
    }

    // Слушаем событие от Rust, когда горячая клавиша нажата
    window.addEventListener('translation-hotkey-pressed', () => {
      console.log('Translation hotkey event received!');
      captureScreenshot();
    });
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
      const { x, y, width, height, monitorIndex } = event.payload;

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
        screenshot.value = base64Image;
        showActionDialog.value = true;
        console.log('Dialog shown');
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

// Отправка в ChatGPT для перевода
async function translateScreenshot() {
  if (!apiKey.value) {
    alert('Пожалуйста, введите API ключ OpenAI');
    return;
  }

  if (!screenshot.value) {
    alert('Сначала нужно сделать скриншот');
    return;
  }

  try {
    isProcessing.value = true;

    // Сохраняем API ключ, если он изменился
    await invoke('save_openai_api_key', { apiKey: apiKey.value });

    const prompt = 'Распознай текст на этом изображении и переведи его на русский язык. Верни только переведенный текст.';

    const result = await invoke<string>('send_to_chatgpt', {
      apiKey: apiKey.value,
      imageBase64: screenshot.value,
      prompt: prompt
    });

    translationResult.value = result;
    showActionDialog.value = false;
  } catch (error) {
    console.error('Failed to translate:', error);
    alert('Ошибка при отправке в ChatGPT: ' + error);
  } finally {
    isProcessing.value = false;
  }
}

// Закрыть диалог
function closeDialog() {
  showActionDialog.value = false;
  screenshot.value = null;
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
        <h3>Настройки</h3>
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

      <!-- Результат перевода -->
      <div v-if="translationResult" class="result-section">
        <h3>Результат перевода:</h3>
        <div class="result-box">
          {{ translationResult }}
        </div>
        <button @click="translationResult = ''" class="btn btn-secondary">
          Очистить
        </button>
      </div>
    </div>

    <!-- Диалог выбора действия -->
    <div v-if="showActionDialog" class="dialog-overlay" @click="closeDialog">
      <div class="dialog" @click.stop>
        <h2>Выбранная область</h2>
        <div class="dialog-preview">
          <img v-if="screenshot" :src="`data:image/png;base64,${screenshot}`" alt="Screenshot" />
        </div>
        <div class="dialog-actions">
          <button
            @click="translateScreenshot"
            class="btn btn-primary"
            :disabled="isProcessing"
          >
            {{ isProcessing ? 'Обработка...' : 'Распознать и перевести' }}
          </button>
          <button @click="closeDialog" class="btn btn-secondary">
            Отмена
          </button>
        </div>
      </div>
    </div>

    <!-- Индикатор загрузки -->
    <div v-if="isProcessing && !showActionDialog" class="loading-overlay">
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
  max-width 800px

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
