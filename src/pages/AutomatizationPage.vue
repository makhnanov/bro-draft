<script setup lang="ts">
import { ref } from 'vue';
import { invoke } from '@tauri-apps/api/core';

const textToType = ref('');
const isPlaying = ref(false);
const countdown = ref(0);

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
</style>
