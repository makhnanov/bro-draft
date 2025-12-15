<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const STORAGE_KEY = 'recognition-active-tab';
const SEQUENCES_STORAGE_KEY = 'screen-recognition-sequences';
const AUDIO_DEVICE_STORAGE_KEY = 'selected-audio-device';

const activeTab = ref('voice');

// Voice recognition state
const audioDevices = ref<MediaDeviceInfo[]>([]);
const selectedDeviceId = ref<string>('');
const isListening = ref(false);
const audioLevel = ref(0);
let audioContext: AudioContext | null = null;
let analyser: AnalyserNode | null = null;
let microphone: MediaStreamAudioSourceNode | null = null;
let mediaStream: MediaStream | null = null;
let animationFrameId: number | null = null;

type ActionType = 'left-click' | 'right-click' | 'double-click' | 'middle-click' | 'text-input';

interface Step {
    image: string; // base64
    x: number;
    y: number;
    width: number;
    height: number;
    monitorIndex: number;
    monitorX: number;
    monitorY: number;
    matchPercentage?: number; // процент совпадения при последнем запуске
    actionType: ActionType; // тип действия
    textToType?: string; // текст для ввода (только для text-input)
    delayAfterMs: number; // задержка после действия в мс (по умолчанию 500)
}

interface Sequence {
    id: string;
    name: string;
    steps: Step[];
}

const sequences = ref<Sequence[]>([]);
const isRunning = ref(false);
let currentSequenceId: string | null = null;
let unlisten: (() => void) | null = null;

function setActiveTab(tab: string) {
    activeTab.value = tab;
    localStorage.setItem(STORAGE_KEY, tab);
}

// Voice recognition functions
async function loadAudioDevices() {
    try {
        console.log('Loading audio devices...');

        // Request permission first to get full device list with labels
        const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
        console.log('Got permission, stream:', stream);

        // Stop the temporary stream
        stream.getTracks().forEach(track => track.stop());

        // Now enumerate devices - we'll get full labels
        const devices = await navigator.mediaDevices.enumerateDevices();
        console.log('All devices:', devices);

        const audioInputs = devices.filter(device => device.kind === 'audioinput');
        console.log('Audio input devices found:', audioInputs.length, audioInputs);

        audioDevices.value = audioInputs;

        // Load saved device or select first one
        const savedDeviceId = localStorage.getItem(AUDIO_DEVICE_STORAGE_KEY);
        if (savedDeviceId && audioDevices.value.some(d => d.deviceId === savedDeviceId)) {
            selectedDeviceId.value = savedDeviceId;
        } else if (audioDevices.value.length > 0) {
            selectedDeviceId.value = audioDevices.value[0].deviceId;
        }

        console.log('Selected device:', selectedDeviceId.value);
    } catch (error) {
        console.error('Failed to enumerate audio devices:', error);
        const errorMessage = error instanceof Error ? error.message : String(error);
        alert('Ошибка доступа к устройствам: ' + errorMessage);
        // If permission denied, still try to get basic device list
        try {
            const devices = await navigator.mediaDevices.enumerateDevices();
            audioDevices.value = devices.filter(device => device.kind === 'audioinput');
        } catch (e) {
            console.error('Failed to get device list:', e);
        }
    }
}

async function startListening() {
    if (!selectedDeviceId.value) {
        alert('Пожалуйста, выберите устройство ввода');
        return;
    }

    try {
        // Request microphone access
        mediaStream = await navigator.mediaDevices.getUserMedia({
            audio: {
                deviceId: selectedDeviceId.value ? { exact: selectedDeviceId.value } : undefined
            }
        });

        // Create audio context and analyser
        audioContext = new AudioContext();
        analyser = audioContext.createAnalyser();
        analyser.fftSize = 256;
        analyser.smoothingTimeConstant = 0.8;

        microphone = audioContext.createMediaStreamSource(mediaStream);
        microphone.connect(analyser);

        isListening.value = true;
        updateAudioLevel();
    } catch (error) {
        console.error('Failed to start listening:', error);
        alert('Ошибка доступа к микрофону: ' + error);
    }
}

function stopListening() {
    if (animationFrameId !== null) {
        cancelAnimationFrame(animationFrameId);
        animationFrameId = null;
    }

    if (microphone) {
        microphone.disconnect();
        microphone = null;
    }

    if (analyser) {
        analyser = null;
    }

    if (audioContext) {
        audioContext.close();
        audioContext = null;
    }

    if (mediaStream) {
        mediaStream.getTracks().forEach(track => track.stop());
        mediaStream = null;
    }

    isListening.value = false;
    audioLevel.value = 0;
}

function updateAudioLevel() {
    if (!analyser || !isListening.value) return;

    const dataArray = new Uint8Array(analyser.frequencyBinCount);
    analyser.getByteFrequencyData(dataArray);

    // Calculate average volume
    const average = dataArray.reduce((a, b) => a + b) / dataArray.length;
    audioLevel.value = Math.min(100, (average / 255) * 100 * 2); // Scale to 0-100

    animationFrameId = requestAnimationFrame(updateAudioLevel);
}

function toggleListening() {
    if (isListening.value) {
        stopListening();
    } else {
        startListening();
    }
}

watch(selectedDeviceId, (newDeviceId) => {
    if (newDeviceId) {
        localStorage.setItem(AUDIO_DEVICE_STORAGE_KEY, newDeviceId);

        // Restart listening if currently active
        if (isListening.value) {
            stopListening();
            setTimeout(() => startListening(), 100);
        }
    }
});

// Fix double-encoded UTF-8 strings (fixes garbled Cyrillic)
function fixEncoding(str: string): string {
    try {
        // Check if string contains garbled UTF-8 characters
        if (!/[\u0080-\u00FF]/.test(str)) {
            return str; // Already correct or ASCII only
        }

        // Convert string to bytes as if it was Latin-1, then decode as UTF-8
        const bytes = new Uint8Array(str.length);
        for (let i = 0; i < str.length; i++) {
            bytes[i] = str.charCodeAt(i) & 0xFF;
        }

        // Decode as UTF-8
        const decoder = new TextDecoder('utf-8');
        return decoder.decode(bytes);
    } catch (e) {
        // If decoding fails, return original
        return str;
    }
}

// Get device name with proper encoding
function getDeviceName(device: MediaDeviceInfo): string {
    if (!device.label) {
        return `Устройство ${device.deviceId.substring(0, 8)}`;
    }

    try {
        // Try to fix encoding
        const fixed = fixEncoding(device.label);
        return fixed;
    } catch (e) {
        console.error('Failed to fix device name encoding:', e);
        return device.label;
    }
}

function createNewSequence() {
    const id = Date.now().toString();
    const newSequence: Sequence = {
        id,
        name: `Последовательность ${sequences.value.length + 1}`,
        steps: []
    };
    sequences.value.push(newSequence);
    saveSequences();
}

function deleteSequence(id: string) {
    sequences.value = sequences.value.filter(s => s.id !== id);
    saveSequences();
}

function deleteStep(sequenceId: string, stepIndex: number) {
    const sequence = sequences.value.find(s => s.id === sequenceId);
    if (sequence) {
        sequence.steps.splice(stepIndex, 1);
        saveSequences();
    }
}

async function addStep(sequenceId: string) {
    currentSequenceId = sequenceId;
    try {
        // Открываем селектор области
        await invoke('open_area_selector');
    } catch (error) {
        console.error('Failed to open area selector:', error);
        alert('Ошибка при открытии селектора области: ' + error);
    }
}

async function handleAreaSelected(event: any) {
    if (!currentSequenceId) return;

    const { x, y, width, height, monitorIndex, monitorX, monitorY } = event.payload;

    try {
        // Захватываем скриншот выбранной области
        const imageBase64 = await invoke<string>('capture_area_screenshot', {
            x,
            y,
            width,
            height,
            monitorIndex
        });

        // Добавляем шаг в последовательность
        const sequence = sequences.value.find(s => s.id === currentSequenceId);
        if (sequence) {
            sequence.steps.push({
                image: imageBase64,
                x: monitorX + x,
                y: monitorY + y,
                width,
                height,
                monitorIndex,
                monitorX,
                monitorY,
                actionType: 'left-click', // по умолчанию левый клик
                delayAfterMs: 500 // по умолчанию 500ms задержка
            });
            saveSequences();
        }
    } catch (error) {
        console.error('Failed to capture area:', error);
        alert('Ошибка при захвате области: ' + error);
    }

    currentSequenceId = null;
}

async function runSequence(sequence: Sequence) {
    if (sequence.steps.length === 0 || isRunning.value) return;

    isRunning.value = true;

    try {
        for (const step of sequence.steps) {
            // Захватываем текущий скриншот экрана
            const currentScreenshot = await captureCurrentScreen(step.monitorIndex);

            // Ищем изображение на экране
            const found = await findImageOnScreen(currentScreenshot, step.image, step.width, step.height);

            if (found) {
                // Сохраняем процент совпадения
                step.matchPercentage = found.match_percentage;
                saveSequences();

                // Вычисляем абсолютные координаты
                const absoluteX = step.monitorX + found.x + Math.floor(step.width / 2);
                const absoluteY = step.monitorY + found.y + Math.floor(step.height / 2);

                console.log(`Found image at monitor coords (${found.x}, ${found.y}) with ${found.match_percentage.toFixed(1)}% match`);

                // Выполняем действие в зависимости от типа
                await performAction(step, absoluteX, absoluteY);

                // Задержка после действия
                const delay = step.delayAfterMs || 500;
                await new Promise(resolve => setTimeout(resolve, delay));
            } else {
                // Сбрасываем процент совпадения если не нашли
                step.matchPercentage = undefined;
                saveSequences();
                alert(`Не удалось найти изображение для шага на экране`);
                break;
            }
        }
    } catch (error) {
        console.error('Failed to run sequence:', error);
        alert('Ошибка при выполнении последовательности: ' + error);
    } finally {
        isRunning.value = false;
    }
}

async function performAction(step: Step, x: number, y: number) {
    console.log(`Performing action: ${step.actionType} at (${x}, ${y})`);

    switch (step.actionType) {
        case 'left-click':
            await clickAt(x, y, 'left', 1);
            break;
        case 'right-click':
            await clickAt(x, y, 'right', 1);
            break;
        case 'double-click':
            await clickAt(x, y, 'left', 2);
            break;
        case 'middle-click':
            await clickAt(x, y, 'middle', 1);
            break;
        case 'text-input':
            if (step.textToType) {
                // Сначала кликаем, чтобы установить фокус
                await clickAt(x, y, 'left', 1);
                // Небольшая задержка перед вводом
                await new Promise(resolve => setTimeout(resolve, 200));
                // Вводим текст
                await typeText(step.textToType);
            }
            break;
    }
}

async function captureCurrentScreen(monitorIndex: number): Promise<string> {
    // Захватываем скриншот монитора напрямую без открытия окна
    const screenshot = await invoke<string>('capture_monitor_screenshot', { monitorIndex });
    return screenshot;
}

async function findImageOnScreen(screenBase64: string, targetBase64: string, targetWidth: number, targetHeight: number): Promise<{ x: number, y: number, match_percentage: number } | null> {
    // Эта функция будет вызывать Rust команду для поиска изображения
    try {
        const result = await invoke<{ x: number, y: number, match_percentage: number } | null>('find_image_on_screen', {
            screenImage: screenBase64,
            targetImage: targetBase64,
            targetWidth,
            targetHeight
        });
        return result;
    } catch (error) {
        console.error('Image search failed:', error);
        return null;
    }
}

async function clickAt(x: number, y: number, button: 'left' | 'right' | 'middle' = 'left', clickCount: number = 1) {
    try {
        await invoke('perform_click', { x, y, button, clickCount });
    } catch (error) {
        console.error('Click failed:', error);
    }
}

async function typeText(text: string) {
    try {
        await invoke('type_text', { text });
    } catch (error) {
        console.error('Type text failed:', error);
    }
}

function saveSequences() {
    localStorage.setItem(SEQUENCES_STORAGE_KEY, JSON.stringify(sequences.value));
}

function loadSequences() {
    const saved = localStorage.getItem(SEQUENCES_STORAGE_KEY);
    if (saved) {
        try {
            sequences.value = JSON.parse(saved);
        } catch (error) {
            console.error('Failed to load sequences:', error);
            sequences.value = [];
        }
    }
}

function getMatchBadgeClass(percentage: number): string {
    if (percentage >= 95) return 'match-excellent';
    if (percentage >= 85) return 'match-good';
    if (percentage >= 70) return 'match-fair';
    return 'match-poor';
}

onMounted(async () => {
    const savedTab = localStorage.getItem(STORAGE_KEY);
    if (savedTab && (savedTab === 'voice' || savedTab === 'screen')) {
        activeTab.value = savedTab;
    }

    loadSequences();
    await loadAudioDevices();

    // Слушаем событие выбора области
    unlisten = await listen('area-selected', handleAreaSelected);
});

onUnmounted(() => {
    if (unlisten) {
        unlisten();
    }

    // Stop listening when component unmounts
    if (isListening.value) {
        stopListening();
    }
});
</script>

<template>
    <div class="page">
        <h1 class="page-title">Recognition</h1>

        <!-- Табы -->
        <div class="tabs">
            <button
                :class="['tab', { active: activeTab === 'voice' }]"
                @click="setActiveTab('voice')"
            >
                <svg viewBox="0 0 24 24" class="tab-icon">
                    <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M19 11c0 3.87-3.13 7-7 7s-7-3.13-7-7M12 18v4m-4 0h8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Voice
            </button>
            <button
                :class="['tab', { active: activeTab === 'screen' }]"
                @click="setActiveTab('screen')"
            >
                <svg viewBox="0 0 24 24" class="tab-icon">
                    <rect x="2" y="3" width="20" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M8 21h8M12 17v4" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                </svg>
                Screen
            </button>
        </div>

        <!-- Контент Voice -->
        <div v-show="activeTab === 'voice'" class="tab-content">
            <div class="section">
                <h2 class="section-title">Голосовое распознавание</h2>
                <p class="section-description">Выберите устройство ввода и начните прослушивание</p>

                <div class="device-info" v-if="audioDevices.length > 0">
                    Найдено устройств: {{ audioDevices.length }}
                </div>

                <!-- Debug Info -->
                <div class="debug-panel" v-if="audioDevices.length > 0">
                    <div class="debug-header">Информация об устройствах (отладка):</div>
                    <div class="debug-device" v-for="(device, idx) in audioDevices" :key="idx">
                        <div><strong>{{ idx + 1 }}.</strong> {{ getDeviceName(device) }}</div>
                        <div class="debug-detail">Полное название: {{ device.label ? fixEncoding(device.label) : 'N/A' }}</div>
                        <div class="debug-detail">ID: {{ device.deviceId }}</div>
                        <div class="debug-detail">Тип: {{ device.kind }}</div>
                        <div class="debug-detail">Группа: {{ device.groupId || 'N/A' }}</div>
                    </div>
                </div>

                <!-- Audio Device Selection -->
                <div class="audio-controls">
                    <div class="control-group">
                        <div class="control-header">
                            <label class="control-label">Устройство ввода:</label>
                            <button class="btn-refresh" @click="loadAudioDevices" :disabled="isListening" title="Обновить список устройств">
                                <svg viewBox="0 0 24 24" class="refresh-icon">
                                    <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" fill="none"/>
                                </svg>
                            </button>
                        </div>
                        <select
                            v-model="selectedDeviceId"
                            class="device-select"
                            :disabled="isListening"
                        >
                            <option v-if="audioDevices.length === 0" value="">Нет доступных устройств</option>
                            <option
                                v-for="device in audioDevices"
                                :key="device.deviceId"
                                :value="device.deviceId"
                            >
                                {{ getDeviceName(device) }}
                            </option>
                        </select>
                    </div>

                    <button
                        class="btn-toggle-listening"
                        :class="{ active: isListening }"
                        @click="toggleListening"
                        :disabled="audioDevices.length === 0"
                    >
                        <svg v-if="!isListening" viewBox="0 0 24 24" class="btn-icon">
                            <path d="M12 14c1.66 0 3-1.34 3-3V5c0-1.66-1.34-3-3-3S9 3.34 9 5v6c0 1.66 1.34 3 3 3z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                            <path d="M19 11c0 3.87-3.13 7-7 7s-7-3.13-7-7M12 18v4m-4 0h8" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
                        </svg>
                        <svg v-else viewBox="0 0 24 24" class="btn-icon">
                            <rect x="6" y="4" width="12" height="16" rx="2" fill="currentColor"/>
                        </svg>
                        {{ isListening ? 'Остановить' : 'Начать прослушивание' }}
                    </button>
                </div>

                <!-- Audio Level Visualizer -->
                <div class="audio-visualizer">
                    <div class="visualizer-label">Уровень звука:</div>
                    <div class="visualizer-container">
                        <div
                            class="visualizer-bar"
                            :style="{ width: audioLevel + '%' }"
                            :class="{
                                active: isListening,
                                low: audioLevel < 30,
                                medium: audioLevel >= 30 && audioLevel < 70,
                                high: audioLevel >= 70
                            }"
                        ></div>
                    </div>
                    <div class="visualizer-value">{{ Math.round(audioLevel) }}%</div>
                </div>

                <!-- Status Info -->
                <div v-if="isListening" class="status-info">
                    <div class="status-indicator"></div>
                    <span>Прослушивание активно</span>
                </div>
            </div>
        </div>

        <!-- Контент Screen -->
        <div v-show="activeTab === 'screen'" class="tab-content">
            <div class="section">
                <h2 class="section-title">Распознавание экрана</h2>
                <p class="section-description">Создавайте последовательности для автоматического распознавания и клика по элементам на экране</p>

                <!-- Список последовательностей -->
                <div class="sequences-container">
                    <div class="sequences-header">
                        <h3>Последовательности</h3>
                        <button class="btn-primary" @click="createNewSequence">
                            <svg viewBox="0 0 24 24" class="btn-icon">
                                <path d="M12 5v14m7-7H5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                            </svg>
                            Создать последовательность
                        </button>
                    </div>

                    <!-- Список -->
                    <div v-if="sequences.length === 0" class="empty-state">
                        <svg viewBox="0 0 24 24" class="empty-icon">
                            <rect x="2" y="3" width="20" height="14" rx="2" fill="none" stroke="currentColor" stroke-width="2"/>
                            <path d="M8 21h8M12 17v4" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                        </svg>
                        <p>Нет созданных последовательностей</p>
                    </div>

                    <div v-else class="sequences-list">
                        <div v-for="sequence in sequences" :key="sequence.id" class="sequence-card">
                            <div class="sequence-header">
                                <div class="sequence-info">
                                    <h4>{{ sequence.name }}</h4>
                                    <span class="steps-count">{{ sequence.steps.length }} шагов</span>
                                </div>
                                <div class="sequence-actions">
                                    <button class="btn-icon-only" @click="runSequence(sequence)" :disabled="sequence.steps.length === 0 || isRunning">
                                        <svg viewBox="0 0 24 24">
                                            <path d="M8 5v14l11-7z" fill="currentColor"/>
                                        </svg>
                                    </button>
                                    <button class="btn-icon-only" @click="deleteSequence(sequence.id)">
                                        <svg viewBox="0 0 24 24">
                                            <path d="M3 6h18M8 6V4h8v2m-9 0v12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2V6" stroke="currentColor" stroke-width="2" fill="none" stroke-linecap="round"/>
                                        </svg>
                                    </button>
                                </div>
                            </div>

                            <!-- Шаги -->
                            <div class="steps-container">
                                <div v-if="sequence.steps.length === 0" class="no-steps">
                                    Нет добавленных шагов
                                </div>
                                <div v-else class="steps-list">
                                    <div v-for="(step, index) in sequence.steps" :key="index" class="step-item">
                                        <div class="step-number">{{ index + 1 }}</div>
                                        <div class="step-preview">
                                            <img :src="'data:image/png;base64,' + step.image" alt="Step preview" />
                                            <div v-if="step.matchPercentage !== undefined" class="match-badge" :class="getMatchBadgeClass(step.matchPercentage)">
                                                {{ step.matchPercentage.toFixed(1) }}%
                                            </div>
                                        </div>
                                        <div class="step-info">
                                            <div class="step-coords">{{ step.width }}x{{ step.height }}px</div>
                                            <div v-if="step.matchPercentage !== undefined" class="step-match">
                                                Совпадение: {{ step.matchPercentage.toFixed(1) }}%
                                            </div>
                                        </div>
                                        <div class="step-settings">
                                            <div class="setting-group">
                                                <label>Действие:</label>
                                                <select v-model="step.actionType" @change="saveSequences" class="action-select">
                                                    <option value="left-click">Левый клик</option>
                                                    <option value="right-click">Правый клик</option>
                                                    <option value="double-click">Двойной клик</option>
                                                    <option value="middle-click">Средний клик</option>
                                                    <option value="text-input">Ввод текста</option>
                                                </select>
                                            </div>
                                            <div v-if="step.actionType === 'text-input'" class="setting-group">
                                                <label>Текст:</label>
                                                <input
                                                    v-model="step.textToType"
                                                    @input="saveSequences"
                                                    type="text"
                                                    placeholder="Введите текст"
                                                    class="text-input"
                                                />
                                            </div>
                                            <div class="setting-group">
                                                <label>Задержка (мс):</label>
                                                <input
                                                    v-model.number="step.delayAfterMs"
                                                    @input="saveSequences"
                                                    type="number"
                                                    min="0"
                                                    step="100"
                                                    class="delay-input"
                                                />
                                            </div>
                                        </div>
                                        <button class="btn-icon-only btn-delete" @click="deleteStep(sequence.id, index)">
                                            <svg viewBox="0 0 24 24">
                                                <path d="M18 6L6 18M6 6l12 12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                            </svg>
                                        </button>
                                    </div>
                                </div>
                                <button class="btn-add-step" @click="addStep(sequence.id)">
                                    <svg viewBox="0 0 24 24" class="btn-icon">
                                        <path d="M12 5v14m7-7H5" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                                    </svg>
                                    Добавить шаг
                                </button>
                            </div>
                        </div>
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
    max-width 1200px
    margin 0 auto
    height 100vh
    overflow-y auto

.page-title
    font-size 36px
    font-weight 700
    color #0052cc
    margin-bottom 30px

.tabs
    display flex
    gap 8px
    margin-bottom 30px
    border-bottom 2px solid #DFE1E6

.tab
    display flex
    align-items center
    gap 8px
    padding 12px 24px
    background none
    border none
    border-bottom 3px solid transparent
    font-size 16px
    font-weight 600
    color #6B778C
    cursor pointer
    transition all 0.2s ease
    margin-bottom -2px

    &:hover
        color #0052cc
        background-color rgba(0, 82, 204, 0.05)

    &.active
        color #0052cc
        border-bottom-color #0052cc

.tab-icon
    width 20px
    height 20px

.tab-content
    animation fadeIn 0.3s ease

@keyframes fadeIn
    from
        opacity 0
        transform translateY(10px)
    to
        opacity 1
        transform translateY(0)

.section
    background white
    border-radius 12px
    padding 30px
    box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

.section-title
    font-size 24px
    font-weight 600
    color #172B4D
    margin-bottom 10px

.section-description
    font-size 14px
    color #6B778C
    margin-bottom 30px
    line-height 1.6

.device-info
    padding 12px 16px
    background #E3FCEF
    border-left 4px solid #00875A
    border-radius 6px
    color #006644
    font-size 14px
    font-weight 600
    margin-bottom 20px

.debug-panel
    padding 16px
    background #F4F5F7
    border-radius 8px
    margin-bottom 20px
    font-size 13px

.debug-header
    font-weight 600
    color #172B4D
    margin-bottom 12px

.debug-device
    padding 12px
    background white
    border-radius 6px
    margin-bottom 8px
    border-left 3px solid #0052cc

    &:last-child
        margin-bottom 0

.debug-detail
    font-size 12px
    color #6B778C
    margin-top 4px
    font-family monospace

// Voice controls
.audio-controls
    display flex
    flex-direction column
    gap 20px
    margin-bottom 30px

.control-group
    display flex
    flex-direction column
    gap 8px

.control-header
    display flex
    justify-content space-between
    align-items center
    gap 12px

.control-label
    font-size 14px
    font-weight 600
    color #172B4D

.btn-refresh
    display flex
    align-items center
    justify-content center
    width 32px
    height 32px
    padding 0
    background white
    border 2px solid #DFE1E6
    border-radius 6px
    cursor pointer
    transition all 0.2s ease

    &:hover
        border-color #0052cc
        background rgba(0, 82, 204, 0.05)

        .refresh-icon
            color #0052cc

    &:active
        transform scale(0.95)

    &:disabled
        opacity 0.5
        cursor not-allowed

        &:hover
            border-color #DFE1E6
            background white

            .refresh-icon
                color #6B778C

.refresh-icon
    width 18px
    height 18px
    color #6B778C
    transition color 0.2s ease

.device-select
    padding 12px 16px
    border 2px solid #DFE1E6
    border-radius 8px
    font-size 14px
    background white
    color #172B4D
    cursor pointer
    transition all 0.2s ease

    &:hover
        border-color #0052cc

    &:focus
        outline none
        border-color #0052cc
        box-shadow 0 0 0 3px rgba(0, 82, 204, 0.1)

    &:disabled
        opacity 0.5
        cursor not-allowed

.btn-toggle-listening
    display flex
    align-items center
    justify-content center
    gap 12px
    padding 16px 32px
    background #0052cc
    color white
    border none
    border-radius 8px
    font-size 16px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover
        background #0747a6
        transform translateY(-2px)
        box-shadow 0 4px 12px rgba(0, 82, 204, 0.3)

    &:active
        transform translateY(0)

    &.active
        background #DE350B

        &:hover
            background #BF2600

    &:disabled
        opacity 0.5
        cursor not-allowed

        &:hover
            transform none
            box-shadow none

    .btn-icon
        width 20px
        height 20px

// Audio visualizer
.audio-visualizer
    display flex
    align-items center
    gap 16px
    padding 20px
    background #F4F5F7
    border-radius 8px

.visualizer-label
    font-size 14px
    font-weight 600
    color #172B4D
    min-width 120px

.visualizer-container
    flex 1
    height 32px
    background white
    border-radius 16px
    overflow hidden
    box-shadow inset 0 2px 4px rgba(0, 0, 0, 0.1)

.visualizer-bar
    height 100%
    background #C1C7D0
    transition width 0.1s ease, background-color 0.3s ease
    border-radius 16px

    &.active.low
        background linear-gradient(90deg, #00875A 0%, #36B37E 100%)

    &.active.medium
        background linear-gradient(90deg, #FF8B00 0%, #FFAB00 100%)

    &.active.high
        background linear-gradient(90deg, #DE350B 0%, #FF5630 100%)

.visualizer-value
    font-size 16px
    font-weight 700
    color #172B4D
    min-width 50px
    text-align right

// Status info
.status-info
    display flex
    align-items center
    gap 12px
    padding 16px
    background #E3FCEF
    border-radius 8px
    border-left 4px solid #00875A
    font-size 14px
    font-weight 600
    color #006644

.status-indicator
    width 12px
    height 12px
    background #00875A
    border-radius 50%
    animation pulse 2s ease-in-out infinite

@keyframes pulse
    0%, 100%
        opacity 1
    50%
        opacity 0.5

.content-placeholder
    display flex
    flex-direction column
    align-items center
    justify-content center
    padding 60px 20px
    text-align center

.placeholder-icon
    width 80px
    height 80px
    color #C1C7D0
    margin-bottom 20px

.placeholder-text
    font-size 16px
    color #6B778C
    max-width 400px

// Новые стили для последовательностей
.sequences-container
    margin-top 20px
    max-height calc(100vh - 300px)
    overflow-y auto

.sequences-header
    display flex
    justify-content space-between
    align-items center
    margin-bottom 20px

    h3
        font-size 20px
        font-weight 600
        color #172B4D
        margin 0

.btn-primary
    display flex
    align-items center
    gap 8px
    padding 10px 20px
    background #0052cc
    color white
    border none
    border-radius 6px
    font-size 14px
    font-weight 600
    cursor pointer
    transition all 0.2s ease

    &:hover
        background #0747a6

    &:active
        transform scale(0.98)

.btn-icon
    width 16px
    height 16px

.empty-state
    display flex
    flex-direction column
    align-items center
    justify-content center
    padding 60px 20px
    text-align center
    color #6B778C

    .empty-icon
        width 64px
        height 64px
        margin-bottom 16px
        opacity 0.5

    p
        font-size 16px
        margin 0

.sequences-list
    display flex
    flex-direction column
    gap 20px

.sequence-card
    background #F4F5F7
    border-radius 8px
    padding 20px
    transition all 0.2s ease

    &:hover
        box-shadow 0 2px 8px rgba(0, 0, 0, 0.1)

.sequence-header
    display flex
    justify-content space-between
    align-items center
    margin-bottom 16px

.sequence-info
    display flex
    align-items center
    gap 12px

    h4
        font-size 18px
        font-weight 600
        color #172B4D
        margin 0

.steps-count
    font-size 13px
    color #6B778C
    background white
    padding 4px 12px
    border-radius 12px

.sequence-actions
    display flex
    gap 8px

.btn-icon-only
    width 36px
    height 36px
    display flex
    align-items center
    justify-content center
    background white
    border none
    border-radius 6px
    cursor pointer
    transition all 0.2s ease

    &:hover
        background #E4E5E7

    &:disabled
        opacity 0.5
        cursor not-allowed

    svg
        width 20px
        height 20px
        color #172B4D

.steps-container
    border-top 1px solid #DFE1E6
    padding-top 16px

.no-steps
    text-align center
    padding 20px
    color #6B778C
    font-size 14px

.steps-list
    display flex
    flex-direction column
    gap 12px
    margin-bottom 16px

.step-item
    display flex
    align-items flex-start
    gap 12px
    padding 12px
    background white
    border-radius 6px
    transition all 0.2s ease

    &:hover
        box-shadow 0 2px 4px rgba(0, 0, 0, 0.1)

.step-number
    width 28px
    height 28px
    display flex
    align-items center
    justify-content center
    background #0052cc
    color white
    border-radius 50%
    font-size 13px
    font-weight 600
    flex-shrink 0

.step-preview
    width 60px
    height 60px
    border-radius 4px
    overflow hidden
    background #F4F5F7
    flex-shrink 0
    position relative

    img
        width 100%
        height 100%
        object-fit cover

.match-badge
    position absolute
    top 4px
    right 4px
    padding 2px 6px
    border-radius 4px
    font-size 10px
    font-weight 700
    color white
    backdrop-filter blur(4px)
    box-shadow 0 1px 3px rgba(0, 0, 0, 0.3)

.match-excellent
    background rgba(0, 135, 90, 0.9)

.match-good
    background rgba(0, 102, 204, 0.9)

.match-fair
    background rgba(255, 140, 0, 0.9)

.match-poor
    background rgba(222, 53, 11, 0.9)

.step-info
    flex 1
    display flex
    flex-direction column
    gap 4px

.step-coords
    font-size 13px
    color #6B778C

.step-match
    font-size 12px
    color #0052cc
    font-weight 600
    margin-top 4px

.step-settings
    flex 2
    display flex
    flex-direction column
    gap 8px

.setting-group
    display flex
    align-items center
    gap 8px

    label
        font-size 12px
        color #6B778C
        font-weight 600
        min-width 90px

.action-select, .text-input, .delay-input
    flex 1
    padding 6px 10px
    border 1px solid #DFE1E6
    border-radius 4px
    font-size 13px
    background white
    color #172B4D
    transition all 0.2s ease

    &:focus
        outline none
        border-color #0052cc
        box-shadow 0 0 0 3px rgba(0, 82, 204, 0.1)

.delay-input
    width 100px
    flex none

.btn-delete
    &:hover
        background #FFEBE6

        svg
            color #DE350B

.btn-add-step
    display flex
    align-items center
    justify-content center
    gap 8px
    padding 12px
    background white
    border 2px dashed #DFE1E6
    border-radius 6px
    color #6B778C
    font-size 14px
    font-weight 600
    cursor pointer
    transition all 0.2s ease
    width 100%

    &:hover
        border-color #0052cc
        color #0052cc
        background rgba(0, 82, 204, 0.05)

    .btn-icon
        width 16px
        height 16px
</style>
