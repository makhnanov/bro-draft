<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const textToType = ref('');
const isPlaying = ref(false);
const countdown = ref(0);

// Click sequence recording
const isRecording = ref(false);
const isPlayingClicks = ref(false);
const clickSequence = ref<Array<{x: number, y: number, monitor: number, button: string}>>([]);
const clickCountdown = ref(0);
const clickInterval = ref(2000); // интервал в миллисекундах
const repeatCount = ref(1); // количество повторений

async function startTyping() {
    if (!textToType.value.trim()) {
        alert('Пожалуйста, введите текст для воспроизведения');
        return;
    }

    isPlaying.value = true;

    // Обратный отсчет 5 секунд
    for (let i = 5; i > 0; i--) {
        countdown.value = i;
        await new Promise(resolve => setTimeout(resolve, 1000));
    }

    countdown.value = 0;

    // Эмуляция нажатий клавиш
    try {
        await invoke('type_text', { text: textToType.value });
        console.log('Typing completed successfully');
    } catch (error) {
        console.error('Error typing text:', error);
        alert('Ошибка при эмуляции клавиш: ' + error);
    } finally {
        isPlaying.value = false;
    }
}

async function startRecording() {
    try {
        clickSequence.value = [];
        isRecording.value = true;
        await invoke('start_click_recording');
        console.log('Recording started');
    } catch (error) {
        console.error('Error starting recording:', error);
        alert('Ошибка при запуске записи: ' + error);
        isRecording.value = false;
    }
}

async function stopRecording() {
    try {
        const sequence = await invoke<Array<{x: number, y: number, monitor: number, button: string}>>('stop_click_recording');
        // Удаляем последний клик (это клик по кнопке "Завершить запись")
        if (sequence.length > 0) {
            sequence.pop();
        }
        clickSequence.value = sequence;
        isRecording.value = false;
        console.log('Recording stopped, clicks:', sequence.length);
    } catch (error) {
        console.error('Error stopping recording:', error);
        alert('Ошибка при остановке записи: ' + error);
        isRecording.value = false;
    }
}

async function playClickSequence() {
    if (clickSequence.value.length === 0) {
        alert('Нет записанной последовательности кликов');
        return;
    }

    isPlayingClicks.value = true;

    // Обратный отсчет 3 секунды
    for (let i = 3; i > 0; i--) {
        clickCountdown.value = i;
        await new Promise(resolve => setTimeout(resolve, 1000));
    }

    clickCountdown.value = 0;

    try {
        await invoke('play_click_sequence', {
            clicks: clickSequence.value,
            intervalMs: clickInterval.value,
            repeatCount: repeatCount.value
        });
        console.log('Click sequence played');
    } catch (error) {
        console.error('Error playing clicks:', error);
        alert('Ошибка при воспроизведении: ' + error);
    } finally {
        isPlayingClicks.value = false;
    }
}
</script>

<template>
    <div class="page">
        <h1 class="page-title">Automatization</h1>

        <div class="section">
            <h2 class="section-title">Эмуляция нажатий клавиш</h2>
            <p class="section-description">Введите текст, который нужно воспроизвести. После нажатия кнопки у вас будет 5 секунд, чтобы переключиться на нужное окно.</p>

            <textarea
                v-model="textToType"
                class="text-input"
                placeholder="Введите текст для эмуляции..."
                rows="6"
                :disabled="isPlaying"
            ></textarea>

            <div v-if="countdown > 0" class="countdown">
                {{ countdown }}
            </div>

            <button
                @click="startTyping"
                class="play-button"
                :disabled="isPlaying"
            >
                {{ isPlaying ? 'Воспроизведение...' : 'Воспроизвести' }}
            </button>
        </div>

        <div class="section">
            <h2 class="section-title">Запись последовательности кликов</h2>
            <p class="section-description">
                Запишите последовательность кликов мыши для автоматического воспроизведения.
                Каждый клик будет воспроизводиться с интервалом в 1 секунду.
            </p>

            <div class="interval-control">
                <label>
                    <b>Интервал:</b>
                </label>
                <div class="interval-buttons">
                    <button @click="clickInterval = Math.max(100, clickInterval - 100)" class="interval-btn">-</button>
                    <span class="interval-value"
                          style="min-width: 90px">{{ (clickInterval / 1000).toFixed(1) }} сек</span>
                    <button @click="clickInterval = Math.min(10000, clickInterval + 100)" class="interval-btn">+</button>
                </div>
            </div>

            <div class="interval-control">
                <label><b>Повторений:</b></label>
                <div class="interval-buttons">
                    <button @click="repeatCount = Math.max(1, repeatCount - 1)" class="interval-btn">-</button>
                    <input
                        type="number"
                        v-model.number="repeatCount"
                        min="1"
                        max="1000"
                        class="repeat-input"
                        style="min-width: 90px"
                    />
                    <button @click="repeatCount = Math.min(1000, repeatCount + 1)" class="interval-btn">+</button>
                </div>
            </div>

            <div class="click-controls">
                <button
                    v-if="!isRecording"
                    @click="startRecording"
                    class="record-button"
                    :disabled="isPlayingClicks"
                >
                    🔴 Начать запись
                </button>
                <button
                    v-else
                    @click="stopRecording"
                    class="stop-button"
                >
                    ⏹ Завершить запись
                </button>

                <button
                    @click="playClickSequence"
                    class="play-button"
                    :disabled="isRecording || isPlayingClicks || clickSequence.length === 0"
                >
                    {{ isPlayingClicks ? 'Воспроизведение...' : '▶ Воспроизвести' }}
                </button>
            </div>

            <div v-if="clickCountdown > 0" class="countdown">
                {{ clickCountdown }}
            </div>

            <div v-if="isRecording" class="recording-indicator">
                <span class="recording-dot"></span>
                Запись... (кликайте на экране)
            </div>

            <div v-if="clickSequence.length > 0" class="click-info">
                <strong>Записано кликов:</strong> {{ clickSequence.length }}
                <div class="click-list">
                    <div v-for="(click, index) in clickSequence" :key="index" class="click-item">
                        {{ index + 1 }}. x: {{ click.x }}, y: {{ click.y }}
                        <span :class="['button-badge', click.button]">{{ click.button === 'right' ? 'ПКМ' : 'ЛКМ' }}</span>
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<style scoped lang="stylus">
.page
    display flex
    flex-direction column
    padding 40px
    max-width 900px
    margin 0 auto

.page-title
    font-size 36px
    font-weight 700
    color #0052cc
    margin-bottom 40px

.section
    background white
    border-radius 12px
    padding 30px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)
    margin-bottom 30px

.section-title
    font-size 24px
    font-weight 600
    color #172B4D
    margin-bottom 10px

.section-description
    font-size 14px
    color #6B778C
    margin-bottom 20px
    line-height 1.6

.text-input
    width 100%
    padding 12px 16px
    border 2px solid #DFE1E6
    border-radius 8px
    font-size 14px
    font-family inherit
    resize vertical
    transition border-color 0.2s ease
    margin-bottom 20px

    &:focus
        outline none
        border-color #0052cc

    &:disabled
        background-color #F4F5F7
        cursor not-allowed

.countdown
    position fixed
    top 50%
    left 50%
    transform translate(-50%, -50%)
    font-size 120px
    font-weight 700
    color #0052cc
    z-index 10000
    text-shadow 0 4px 8px rgba(0, 0, 0, 0.2)
    animation pulse 1s ease-in-out

@keyframes pulse
    0%
        transform translate(-50%, -50%) scale(1)
        opacity 1
    50%
        transform translate(-50%, -50%) scale(1.1)
        opacity 0.9
    100%
        transform translate(-50%, -50%) scale(1)
        opacity 1

.play-button
    padding 12px 32px
    background-color #0052cc
    color white
    border none
    border-radius 8px
    font-size 16px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover:not(:disabled)
        background-color #0747a6
        transform translateY(-2px)
        box-shadow 0 4px 12px rgba(0, 82, 204, 0.3)

    &:active:not(:disabled)
        transform translateY(0)

    &:disabled
        background-color #DFE1E6
        color #A5ADBA
        cursor not-allowed

.settings-row
    display flex
    gap 16px
    margin-bottom 20px

.interval-control
    display flex
    align-items center
    gap 16px
    padding 12px 16px
    background-color #f4f5f7
    border-radius 8px
    margin-bottom 12px

    label
        font-weight 500
        color #172B4D
        min-width 106px

.interval-buttons
    display flex
    align-items center
    gap 8px

.interval-btn
    width 32px
    height 32px
    border none
    border-radius 6px
    background-color #0052cc
    color white
    font-size 18px
    font-weight bold
    cursor pointer
    transition all 0.2s ease

    &:hover
        background-color #0747a6

.interval-value
    min-width 60px
    text-align center
    font-weight 600
    color #172B4D

.repeat-input
    width 80px
    padding 8px 12px
    border 2px solid #DFE1E6
    border-radius 6px
    font-size 14px
    font-weight 600
    text-align center
    color #172B4D

    &:focus
        outline none
        border-color #0052cc

    &::-webkit-inner-spin-button,
    &::-webkit-outer-spin-button
        opacity 1

.click-controls
    display flex
    justify-content space-between
    margin-bottom 20px

.record-button
    padding 12px 24px
    background-color #de350b
    color white
    border none
    border-radius 8px
    font-size 16px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover:not(:disabled)
        background-color #bf2600
        transform translateY(-2px)
        box-shadow 0 4px 12px rgba(222, 53, 11, 0.3)

    &:disabled
        background-color #DFE1E6
        color #A5ADBA
        cursor not-allowed

.stop-button
    padding 12px 24px
    background-color #ff5630
    color white
    border none
    border-radius 8px
    font-size 16px
    font-weight 600
    cursor pointer
    transition all 0.2s ease
    animation pulse-red 1.5s ease-in-out infinite

    &:hover
        background-color #de350b
        transform translateY(-2px)

@keyframes pulse-red
    0%, 100%
        box-shadow 0 0 0 0 rgba(255, 86, 48, 0.4)
    50%
        box-shadow 0 0 0 10px rgba(255, 86, 48, 0)

.recording-indicator
    display flex
    align-items center
    gap 10px
    padding 12px 16px
    background-color #ffebe6
    border-radius 8px
    color #de350b
    font-weight 500
    margin-bottom 20px

.recording-dot
    width 12px
    height 12px
    background-color #de350b
    border-radius 50%
    animation blink 1s ease-in-out infinite

@keyframes blink
    0%, 100%
        opacity 1
    50%
        opacity 0.3

.click-info
    background-color #f4f5f7
    border-radius 8px
    padding 16px

.click-list
    margin-top 12px
    max-height 200px
    overflow-y auto

.click-item
    padding 6px 0
    font-size 13px
    color #6B778C
    border-bottom 1px solid #DFE1E6
    display flex
    align-items center
    gap 8px

    &:last-child
        border-bottom none

.button-badge
    padding 2px 6px
    border-radius 4px
    font-size 11px
    font-weight 600

    &.left
        background-color #e3fcef
        color #006644

    &.right
        background-color #ffebe6
        color #de350b
</style>
