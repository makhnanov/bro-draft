<script setup lang="ts">
import { ref, onMounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const screenshot = ref<string | null>(null);
const translationResult = ref('');
const recognitionResult = ref('');
const isProcessing = ref(false);
const apiKey = ref('');

onMounted(async () => {
  // Загружаем API ключ
  try {
    const savedApiKey = await invoke<string | null>('get_openai_api_key');
    if (savedApiKey) {
      apiKey.value = savedApiKey;
    }
  } catch (error) {
    console.error('Failed to load API key:', error);
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
  } catch (error) {
    console.error('Failed to recognize:', error);
    alert('Ошибка при распознавании: ' + error);
  } finally {
    isProcessing.value = false;
  }
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
    <div class="popup-header" data-tauri-drag-region>
      <h2 data-tauri-drag-region>Выбранная область</h2>
      <button class="close-btn" @click="closePopup">×</button>
    </div>

    <div class="popup-content">
      <div class="preview-section">
        <img v-if="screenshot" :src="`data:image/png;base64,${screenshot}`" alt="Screenshot" />
        <div v-else class="no-image">Загрузка изображения...</div>
      </div>

      <div v-if="recognitionResult" class="result-section">
        <h3>Распознанный текст:</h3>
        <div class="result-box">{{ recognitionResult }}</div>
      </div>

      <div v-if="translationResult" class="result-section">
        <h3>Перевод:</h3>
        <div class="result-box">{{ translationResult }}</div>
      </div>
    </div>

    <div class="popup-actions">
      <button
        @click="translateScreenshot"
        class="btn btn-primary"
        :disabled="isProcessing || !screenshot"
      >
        {{ isProcessing ? 'Обработка...' : 'Распознать и перевести' }}
      </button>
      <button
        @click="recognizeOnly"
        class="btn btn-secondary"
        :disabled="isProcessing || !screenshot"
      >
        Просто распознать
      </button>
      <button @click="closePopup" class="btn btn-close">
        Закрыть
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

.popup-header
  display flex
  justify-content space-between
  align-items center
  padding 15px 20px
  background linear-gradient(180deg, #0747a6 0%, #0052cc 100%)
  color white
  -webkit-app-region drag

  h2
    margin 0
    font-size 18px
    font-weight 600

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
  overflow auto
  display flex
  flex-direction column
  gap 10px

.preview-section
  display flex
  justify-content center

  img
    border-radius 4px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.2)

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
  flex-direction column
  gap 8px
  padding 15px 20px
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
