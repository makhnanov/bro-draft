<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const isStreaming = ref(false);
const streamUrl = ref('');
const localIp = ref('');
const port = ref(8080);
const errorMessage = ref('');

// Получаем локальный IP при монтировании компонента
onMounted(async () => {
  try {
    const ip = await invoke<string>('get_local_ip');
    localIp.value = ip;
  } catch (err) {
    console.error('Failed to get local IP:', err);
    errorMessage.value = 'Failed to get local IP address';
  }
});

async function startStreaming() {
  try {
    errorMessage.value = '';

    // Запускаем стриминг сервер на бэкенде
    // Backend будет сам захватывать экран
    const result = await invoke<{ ip: string; port: number }>('start_screen_streaming', {
      port: port.value,
      screenIndex: 0 // Первый экран
    });

    streamUrl.value = `http://${result.ip}:${result.port}`;
    localIp.value = result.ip;
    port.value = result.port;
    isStreaming.value = true;
  } catch (err: any) {
    console.error('Failed to start streaming:', err);
    errorMessage.value = `Failed to start streaming: ${err}`;
    isStreaming.value = false;
  }
}

async function stopStreaming() {
  try {
    // Останавливаем сервер на бэкенде
    await invoke('stop_screen_streaming');

    isStreaming.value = false;
    streamUrl.value = '';
    errorMessage.value = '';
  } catch (err) {
    console.error('Failed to stop streaming:', err);
    errorMessage.value = `Failed to stop streaming: ${err}`;
  }
}

onUnmounted(() => {
  if (isStreaming.value) {
    stopStreaming();
  }
});
</script>

<template>
  <div class="streaming-page">
    <h1 class="page-title">Screen Streaming</h1>

    <div class="streaming-container">
      <div class="control-panel">
        <div class="info-section" v-if="localIp">
          <h3>Network Information</h3>
          <div class="info-item">
            <span class="info-label">Local IP:</span>
            <span class="info-value">{{ localIp }}</span>
          </div>
          <div class="info-item">
            <span class="info-label">Port:</span>
            <span class="info-value">{{ port }}</span>
          </div>
        </div>

        <div class="stream-status" v-if="isStreaming">
          <div class="status-indicator active"></div>
          <div class="status-info">
            <h3>Streaming Active (WebSocket)</h3>
            <p class="stream-url">{{ streamUrl }}</p>
            <p class="instruction">Open in your browser to view the stream:</p>
            <p class="instruction-url">http://{{ localIp }}:{{ port }}/</p>
          </div>
        </div>

        <div class="error-message" v-if="errorMessage">
          <svg viewBox="0 0 24 24" class="error-icon">
            <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/>
            <line x1="12" y1="8" x2="12" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
            <line x1="12" y1="16" x2="12.01" y2="16" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          </svg>
          <span>{{ errorMessage }}</span>
        </div>

        <div class="button-group">
          <button
            v-if="!isStreaming"
            @click="startStreaming"
            class="btn btn-primary"
          >
            <svg viewBox="0 0 24 24" class="btn-icon">
              <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/>
              <polygon points="10 8 16 12 10 16" fill="currentColor"/>
            </svg>
            Start Streaming
          </button>

          <button
            v-if="isStreaming"
            @click="stopStreaming"
            class="btn btn-danger"
          >
            <svg viewBox="0 0 24 24" class="btn-icon">
              <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/>
              <rect x="9" y="9" width="6" height="6" fill="currentColor"/>
            </svg>
            Stop Streaming
          </button>
        </div>

        <div class="instructions" v-if="!isStreaming">
          <h3>How to use:</h3>
          <ol>
            <li>Click "Start Streaming" button</li>
            <li>Select the screen or window you want to share</li>
            <li>Share the displayed URL with viewers</li>
            <li>Viewers can access the stream in their browser</li>
            <li>Click "Stop Streaming" when done</li>
          </ol>
        </div>
      </div>

      <div class="preview-panel" v-if="isStreaming">
        <h3>Stream Preview</h3>
        <div class="preview-info">
          <p>Your screen is being streamed to:</p>
          <code class="preview-url">{{ streamUrl }}</code>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped lang="stylus">
.streaming-page
  padding 40px
  max-width 1200px
  margin 0 auto
  width 100%
  box-sizing border-box

.page-title
  font-size 42px
  font-weight 700
  color #0052cc
  margin-bottom 40px
  text-align center

.streaming-container
  display flex
  flex-direction column
  gap 30px

.control-panel
  background white
  border-radius 12px
  padding 30px
  box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

.info-section
  margin-bottom 30px
  padding 20px
  background #f5f8fa
  border-radius 8px

  h3
    font-size 18px
    font-weight 600
    color #0052cc
    margin-bottom 15px

.info-item
  display flex
  justify-content space-between
  align-items center
  padding 10px 0
  border-bottom 1px solid #e0e0e0

  &:last-child
    border-bottom none

.info-label
  font-weight 600
  color #666

.info-value
  font-family monospace
  font-size 16px
  color #0052cc
  background white
  padding 5px 10px
  border-radius 4px

.stream-status
  display flex
  align-items center
  gap 20px
  padding 20px
  background linear-gradient(135deg, #e8f5e9 0%, #c8e6c9 100%)
  border-radius 8px
  margin-bottom 20px

.status-indicator
  width 20px
  height 20px
  border-radius 50%
  background #4caf50
  box-shadow 0 0 10px rgba(76, 175, 80, 0.5)
  animation pulse 2s infinite
  flex-shrink 0

@keyframes pulse
  0%, 100%
    opacity 1
    transform scale(1)
  50%
    opacity 0.7
    transform scale(1.1)

.status-info
  flex 1

  h3
    font-size 18px
    font-weight 600
    color #2e7d32
    margin-bottom 8px

.stream-url
  font-family monospace
  font-size 16px
  color #1565c0
  background white
  padding 8px 12px
  border-radius 4px
  margin-bottom 8px
  word-break break-all

.instruction
  font-size 14px
  color #666
  margin 0

.instruction-url
  font-family monospace
  font-size 18px
  color #0052cc
  background white
  padding 10px 15px
  border-radius 4px
  margin-top 10px
  font-weight bold

.error-message
  display flex
  align-items center
  gap 12px
  padding 15px
  background #ffebee
  border-left 4px solid #f44336
  border-radius 4px
  color #c62828
  margin-bottom 20px

.error-icon
  width 24px
  height 24px
  flex-shrink 0

.button-group
  display flex
  gap 15px
  margin-bottom 20px

.btn
  flex 1
  display flex
  align-items center
  justify-content center
  gap 10px
  padding 16px 24px
  font-size 16px
  font-weight 600
  border none
  border-radius 8px
  cursor pointer
  transition all 0.2s ease

  &:hover
    transform translateY(-2px)
    box-shadow 0 4px 12px rgba(0, 0, 0, 0.2)

  &:active
    transform translateY(0)

.btn-primary
  background linear-gradient(135deg, #0052cc 0%, #0747a6 100%)
  color white

  &:hover
    background linear-gradient(135deg, #0747a6 0%, #0052cc 100%)

.btn-danger
  background linear-gradient(135deg, #f44336 0%, #d32f2f 100%)
  color white

  &:hover
    background linear-gradient(135deg, #d32f2f 0%, #f44336 100%)

.btn-icon
  width 20px
  height 20px

.instructions
  padding 20px
  background #f5f8fa
  border-radius 8px

  h3
    font-size 18px
    font-weight 600
    color #0052cc
    margin-bottom 15px

  ol
    margin-left 20px

    li
      padding 8px 0
      color #666
      line-height 1.6

.preview-panel
  background white
  border-radius 12px
  padding 30px
  box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

  h3
    font-size 18px
    font-weight 600
    color #0052cc
    margin-bottom 15px

.preview-info
  padding 20px
  background #f5f8fa
  border-radius 8px

  p
    margin-bottom 10px
    color #666

.preview-url
  display block
  font-family monospace
  font-size 16px
  color #0052cc
  background white
  padding 12px
  border-radius 4px
  word-break break-all

@media (prefers-color-scheme: dark)
  .control-panel, .preview-panel
    background #1e1e1e

  .info-section, .instructions, .preview-info
    background #2a2a2a

  .info-value, .stream-url, .preview-url
    background #1e1e1e

  .info-label, .instructions li, .preview-info p
    color #ccc
</style>
