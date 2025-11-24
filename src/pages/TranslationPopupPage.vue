<script setup lang="ts">
import { ref, onMounted, computed } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const screenshot = ref<string | null>(null);
const translationResult = ref('');
const recognitionResult = ref('');
const isProcessing = ref(false);
const apiKey = ref('');
const anthropicApiKey = ref('');
const clickMarker = ref<{ x: number; y: number; answer: string } | null>(null);
const autoOpenLinks = ref(false);
const popupContent = ref<HTMLElement | null>(null);

// Функция для изменения размера окна под контент
async function resizeWindowToFit() {
  await new Promise(resolve => setTimeout(resolve, 50)); // Ждём рендеринг

  const popup = document.querySelector('.translation-popup') as HTMLElement;
  if (!popup) return;

  // Вычисляем нужную высоту
  const headerHeight = 54;
  const contentEl = popup.querySelector('.popup-content') as HTMLElement;
  const actionsEl = popup.querySelector('.popup-actions') as HTMLElement;

  if (!contentEl || !actionsEl) return;

  // Временно убираем overflow для точного измерения
  const oldOverflow = contentEl.style.overflow;
  contentEl.style.overflow = 'visible';

  const contentHeight = contentEl.scrollHeight;
  const actionsHeight = actionsEl.offsetHeight;

  contentEl.style.overflow = oldOverflow;

  const newHeight = headerHeight + contentHeight + actionsHeight + 2;

  // Ограничиваем максимальную высоту (80% экрана)
  const maxHeight = window.screen.height * 0.8;
  const finalHeight = Math.min(newHeight, maxHeight);

  try {
    // Получаем текущий размер окна
    const currentSize = await invoke<{ width: number; height: number }>('get_window_size');

    // Анимируем изменение размера
    const startHeight = currentSize.height;
    const duration = 200; // мс
    const startTime = performance.now();

    const animate = async (currentTime: number) => {
      const elapsed = currentTime - startTime;
      const progress = Math.min(elapsed / duration, 1);

      // Easing функция для плавности
      const easeOutCubic = 1 - Math.pow(1 - progress, 3);
      const currentHeight = startHeight + (finalHeight - startHeight) * easeOutCubic;

      await invoke('set_window_size', {
        width: currentSize.width,
        height: Math.round(currentHeight)
      });

      if (progress < 1) {
        requestAnimationFrame(animate);
      }
    };

    requestAnimationFrame(animate);
  } catch (error) {
    console.error('Failed to resize window:', error);
  }
}

// Функция для определения, является ли текст ссылкой
function isLink(text: string): boolean {
  if (!text) return false;
  const trimmed = text.trim();
  // Одно слово без пробелов и с точками = ссылка
  // Или начинается с http:// или https://
  if (trimmed.includes(' ')) return false;
  if (trimmed.startsWith('http://') || trimmed.startsWith('https://')) return true;
  // Проверяем наличие точки и минимальную длину домена
  const parts = trimmed.split('.');
  if (parts.length >= 2 && parts[parts.length - 1].length >= 2) {
    return true;
  }
  return false;
}

// Вычисляемое свойство для определения, есть ли ссылка в результате
const detectedLink = computed(() => {
  if (recognitionResult.value && isLink(recognitionResult.value)) {
    return recognitionResult.value.trim();
  }
  return null;
});

// Открыть ссылку в браузере
async function openLinkInBrowser() {
  if (!detectedLink.value) return;
  try {
    await invoke('open_url_in_browser', { url: detectedLink.value });
  } catch (error) {
    console.error('Failed to open URL:', error);
    alert('Ошибка при открытии ссылки: ' + error);
  }
}

// Сохранить настройку автооткрытия
async function saveAutoOpenSetting() {
  try {
    await invoke('save_auto_open_links', { enabled: autoOpenLinks.value });
  } catch (error) {
    console.error('Failed to save auto open setting:', error);
  }
}

// Перетаскивание окна вручную
let isDragging = false;
let dragStartX = 0;
let dragStartY = 0;
let windowStartX = 0;
let windowStartY = 0;

async function startDrag(event: MouseEvent) {
  if ((event.target as HTMLElement).closest('.close-btn')) {
    return;
  }
  event.preventDefault();

  isDragging = true;
  dragStartX = event.screenX;
  dragStartY = event.screenY;

  try {
    const [x, y] = await invoke<[number, number]>('get_translation_popup_position');
    windowStartX = x;
    windowStartY = y;
  } catch (error) {
    console.error('Failed to get position:', error);
    return;
  }

  document.addEventListener('mousemove', onDrag);
  document.addEventListener('mouseup', stopDrag);
}

async function onDrag(event: MouseEvent) {
  if (!isDragging) return;

  const deltaX = event.screenX - dragStartX;
  const deltaY = event.screenY - dragStartY;

  await invoke('move_translation_popup', {
    x: windowStartX + deltaX,
    y: windowStartY + deltaY
  });
}

function stopDrag() {
  isDragging = false;
  document.removeEventListener('mousemove', onDrag);
  document.removeEventListener('mouseup', stopDrag);
}

onMounted(async () => {
  // Загружаем API ключи
  try {
    const savedApiKey = await invoke<string | null>('get_openai_api_key');
    if (savedApiKey) {
      apiKey.value = savedApiKey;
    }

    const savedAnthropicKey = await invoke<string | null>('get_anthropic_api_key');
    if (savedAnthropicKey) {
      anthropicApiKey.value = savedAnthropicKey;
    }

    // Загружаем настройку автооткрытия ссылок
    const savedAutoOpen = await invoke<boolean | null>('get_auto_open_links');
    if (savedAutoOpen !== null) {
      autoOpenLinks.value = savedAutoOpen;
    }
  } catch (error) {
    console.error('Failed to load settings:', error);
  }

  // Получаем данные скриншота из state
  try {
    const imageData = await invoke<string | null>('get_popup_screenshot');
    if (imageData) {
      screenshot.value = imageData;
      console.log('Screenshot loaded, length:', imageData.length);
    } else {
      console.error('No screenshot data available');
    }
  } catch (error) {
    console.error('Failed to get screenshot:', error);
  }
});

async function translateScreenshot() {
  if (!apiKey.value) {
    alert('API ключ OpenAI не настроен');
    return;
  }

  if (!screenshot.value) {
    return;
  }

  try {
    isProcessing.value = true;

    const prompt = 'Распознай текст на этом изображении и переведи его на русский язык. Верни только переведенный текст.';

    const result = await invoke<string>('send_to_chatgpt', {
      apiKey: apiKey.value,
      imageBase64: screenshot.value,
      prompt: prompt
    });

    translationResult.value = result;
    await resizeWindowToFit();
  } catch (error) {
    console.error('Failed to translate:', error);
    alert('Ошибка при отправке в ChatGPT: ' + error);
  } finally {
    isProcessing.value = false;
  }
}

async function recognizeOnly() {
  if (!apiKey.value) {
    alert('API ключ OpenAI не настроен');
    return;
  }

  if (!screenshot.value) {
    return;
  }

  try {
    isProcessing.value = true;

    const prompt = 'Распознай текст на этом изображении. Верни только распознанный текст без перевода.';

    const result = await invoke<string>('send_to_chatgpt', {
      apiKey: apiKey.value,
      imageBase64: screenshot.value,
      prompt: prompt
    });

    recognitionResult.value = result;
    await resizeWindowToFit();

    // Если включено автооткрытие и результат - ссылка, открываем сразу
    if (autoOpenLinks.value && isLink(result)) {
      await invoke('open_url_in_browser', { url: result.trim() });
    }
  } catch (error) {
    console.error('Failed to recognize:', error);
    alert('Ошибка при распознавании: ' + error);
  } finally {
    isProcessing.value = false;
  }
}

async function solveTask() {
  if (!anthropicApiKey.value) {
    alert('API ключ Anthropic не настроен. Добавьте его в Настройках.');
    return;
  }

  if (!screenshot.value) {
    return;
  }

  try {
    isProcessing.value = true;

    const prompt = `Посмотри на это изображение. Это задача или тест. Определи правильный ответ и укажи координаты X и Y элемента на который нужно кликнуть чтобы выбрать этот ответ.

Верни ответ СТРОГО в формате JSON:
{"answer": "краткое описание ответа", "x": число, "y": число}

Где x и y - координаты в пикселях относительно левого верхнего угла изображения.`;

    const result = await invoke<string>('send_to_claude', {
      apiKey: anthropicApiKey.value,
      imageBase64: screenshot.value,
      prompt: prompt
    });

    console.log('Claude response:', result);

    // Парсим JSON из ответа
    const jsonMatch = result.match(/\{[^}]+\}/);
    if (!jsonMatch) {
      alert('Не удалось получить координаты из ответа:\n\n' + result);
      return;
    }

    const parsed = JSON.parse(jsonMatch[0]);
    console.log('Parsed response:', parsed);

    if (typeof parsed.x !== 'number' || typeof parsed.y !== 'number') {
      alert('Некорректные координаты в ответе:\n\n' + result);
      return;
    }

    console.log(`Claude coordinates: ${parsed.x}, ${parsed.y}`);
    console.log(`Answer: ${parsed.answer}`);

    // Показываем маркер на изображении
    clickMarker.value = {
      x: parsed.x,
      y: parsed.y,
      answer: parsed.answer || 'Правильный ответ'
    };

    // Ждём 1.5 секунды чтобы пользователь увидел маркер
    await new Promise(resolve => setTimeout(resolve, 1500));

    // Получаем координаты где был сделан скриншот на экране
    const [screenX, screenY] = await invoke<[number, number]>('get_popup_screen_position');

    // Абсолютные координаты = позиция скриншота на экране + относительные координаты от Claude
    const absoluteX = screenX + parsed.x;
    const absoluteY = screenY + parsed.y;

    console.log(`Screenshot position: ${screenX}, ${screenY}`);
    console.log(`Absolute click position: ${absoluteX}, ${absoluteY}`);

    // Закрываем окно и выполняем клик через Rust (в отдельном потоке)
    await invoke('solve_and_click', {
      x: absoluteX,
      y: absoluteY,
      answer: parsed.answer || 'Правильный ответ'
    });

  } catch (error) {
    console.error('Failed to solve:', error);
    alert('Ошибка при решении: ' + error);
  } finally {
    isProcessing.value = false;
  }
}

async function confirmClick() {
  if (!clickMarker.value) return;

  try {
    // Получаем координаты где был сделан скриншот на экране
    const [screenX, screenY] = await invoke<[number, number]>('get_popup_screen_position');

    // Абсолютные координаты = позиция скриншота на экране + относительные координаты от Claude
    const absoluteX = screenX + clickMarker.value.x;
    const absoluteY = screenY + clickMarker.value.y;

    console.log(`Screenshot position: ${screenX}, ${screenY}`);
    console.log(`Absolute click position: ${absoluteX}, ${absoluteY}`);

    // Закрываем окно
    await invoke('close_translation_popup');

    // Ждём 1 секунду перед началом движения курсора
    await new Promise(resolve => setTimeout(resolve, 1000));

    // Выполняем клик
    await invoke('play_click_sequence', {
      clicks: [{
        x: absoluteX,
        y: absoluteY,
        monitor: 0,
        button: 'left'
      }],
      intervalMs: 100,
      repeatCount: 1
    });

  } catch (error) {
    console.error('Failed to click:', error);
    alert('Ошибка при клике: ' + error);
  }
}

function cancelClick() {
  clickMarker.value = null;
}

async function closePopup() {
  try {
    await invoke('close_translation_popup');
  } catch (error) {
    console.error('Failed to close popup:', error);
  }
}
</script>

<template>
  <div class="translation-popup">
    <div class="popup-header" @mousedown="startDrag">
      <h2>Выбранная область</h2>
      <button class="close-btn" @click.stop="closePopup">×</button>
    </div>

    <div class="popup-content">
      <div class="preview-section">
        <div class="image-container" v-if="screenshot">
          <img :src="`data:image/png;base64,${screenshot}`" alt="Screenshot" />
          <div
            v-if="clickMarker"
            class="click-marker"
            :style="{ left: clickMarker.x + 'px', top: clickMarker.y + 'px' }"
          >
            <div class="marker-dot"></div>
            <div class="marker-label">{{ clickMarker.answer }}</div>
          </div>
        </div>
        <div v-else class="no-image">Загрузка изображения...</div>
      </div>

      <div v-if="recognitionResult" class="result-section">
        <h3>Распознанный текст:</h3>
        <div class="result-box">{{ recognitionResult }}</div>

        <!-- UI для ссылок -->
        <div v-if="detectedLink" class="link-actions">
          <button @click="openLinkInBrowser" class="btn btn-link">
            Открыть ссылку
          </button>
          <label class="auto-open-checkbox">
            <input
              type="checkbox"
              v-model="autoOpenLinks"
              @change="saveAutoOpenSetting"
            />
            Автоматически открывать ссылки
          </label>
        </div>
      </div>

      <div v-if="translationResult" class="result-section">
        <h3>Перевод:</h3>
        <div class="result-box">{{ translationResult }}</div>
      </div>
    </div>

    <div class="popup-actions" v-if="clickMarker">
      <button @click="cancelClick" class="btn btn-secondary">
        Отмена
      </button>
      <button @click="confirmClick" class="btn btn-solve">
        Кликнуть
      </button>
    </div>
    <div class="popup-actions" v-else>
      <button
        @click="recognizeOnly"
        class="btn btn-secondary"
        :disabled="isProcessing || !screenshot"
      >
        {{ isProcessing ? 'Обработка...' : 'Распознать' }}
      </button>
      <button
        @click="translateScreenshot"
        class="btn btn-primary"
        :disabled="isProcessing || !screenshot"
      >
        {{ isProcessing ? 'Обработка...' : 'Перевести' }}
      </button>
      <button
        @click="solveTask"
        class="btn btn-solve"
        :disabled="isProcessing || !screenshot"
      >
        {{ isProcessing ? 'Обработка...' : 'Решить' }}
      </button>
    </div>
  </div>
</template>

<style scoped lang="stylus">
.translation-popup
  display flex
  flex-direction column
  height 100vh
  background rgba(255, 255, 255, 0.6)
  font-family -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif

// Отключаем все анимации
*
  animation none !important
  transition none !important

.popup-header
  display flex
  justify-content space-between
  align-items center
  padding 15px 20px
  background linear-gradient(180deg, #0747a6 0%, #0052cc 100%)
  color white
  -webkit-app-region drag
  cursor move

  h2
    margin 0
    font-size 18px
    font-weight 600
    -webkit-app-region drag
    cursor move

.close-btn
  background transparent
  border none
  color white
  font-size 24px
  cursor pointer
  padding 0 5px
  -webkit-app-region no-drag

  &:hover
    opacity 0.8

.popup-content
  flex 1
  padding 15px
  overflow hidden
  display flex
  flex-direction column
  gap 10px

.preview-section
  display flex
  justify-content center

  .image-container
    position relative
    display inline-block

  img
    border-radius 4px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.2)

.click-marker
  position absolute
  transform translate(-50%, -50%)
  pointer-events none
  z-index 10

  .marker-dot
    width 20px
    height 20px
    background rgba(255, 0, 0, 0.8)
    border 3px solid white
    border-radius 50%
    box-shadow 0 0 10px rgba(0, 0, 0, 0.5)

  .marker-label
    position absolute
    top 25px
    left 50%
    transform translateX(-50%)
    background rgba(0, 0, 0, 0.8)
    color white
    padding 4px 8px
    border-radius 4px
    font-size 12px
    white-space nowrap
    max-width 200px
    overflow hidden
    text-overflow ellipsis

.result-section
  h3
    font-size 14px
    font-weight 600
    color #0052cc
    margin-bottom 10px

.result-box
  padding 15px
  background #f8f9fa
  border-radius 6px
  border 1px solid #e0e0e0
  white-space pre-wrap
  word-break break-word
  font-size 14px
  line-height 1.5

.popup-actions
  display flex
  flex-direction row
  gap 10px
  padding 15px
  border-top 1px solid #e0e0e0

.btn
  flex 1
  padding 10px 15px
  border none
  border-radius 6px
  font-size 14px
  font-weight 500
  cursor pointer
  transition all 0.2s ease

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

.btn-close
  background #dc3545
  color white

  &:hover:not(:disabled)
    background #c82333

.btn-solve
  background #28a745
  color white

  &:hover:not(:disabled)
    background #218838

.btn-link
  background #17a2b8
  color white

  &:hover:not(:disabled)
    background #138496

.link-actions
  margin-top 15px
  display flex
  flex-direction column
  gap 10px

.auto-open-checkbox
  display flex
  align-items center
  gap 8px
  font-size 13px
  color #6B778C
  cursor pointer

  input[type="checkbox"]
    width 16px
    height 16px
    cursor pointer

.no-image
  padding 40px
  text-align center
  color #666
  font-style italic

@media (prefers-color-scheme: dark)
  .translation-popup
    background rgba(47, 47, 47, 0.6)
    color #f6f6f6

  .result-box
    background #444
    border-color #555
    color #f6f6f6
</style>

<style lang="stylus">
// Глобальные стили для прозрачности окна
html, body
  background transparent !important
</style>
