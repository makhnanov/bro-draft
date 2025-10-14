<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const temperature = ref(0);
const downloadSpeed = ref(0);
const uploadSpeed = ref(0);
let tempIntervalId: number | null = null;
let speedIntervalId: number | null = null;

// Вычисляем процент для прогресс-бара температуры (0-100°C -> 0-100%)
const temperaturePercent = computed(() => {
  const minTemp = 0;
  const maxTemp = 100;
  const percent = ((temperature.value - minTemp) / (maxTemp - minTemp)) * 100;
  return Math.min(Math.max(percent, 0), 100);
});

// Вычисляем процент для скорости загрузки (0-500 Mbps -> 0-100%)
const downloadPercent = computed(() => {
  const minSpeed = 0;
  const maxSpeed = 500;
  const percent = ((downloadSpeed.value - minSpeed) / (maxSpeed - minSpeed)) * 100;
  return Math.min(Math.max(percent, 0), 100);
});

// Вычисляем процент для скорости отдачи (0-500 Mbps -> 0-100%)
const uploadPercent = computed(() => {
  const minSpeed = 0;
  const maxSpeed = 500;
  const percent = ((uploadSpeed.value - minSpeed) / (maxSpeed - minSpeed)) * 100;
  return Math.min(Math.max(percent, 0), 100);
});

// Вычисляем progress для stroke-dasharray
const tempDashProgress = computed(() => {
  const radius = 45;
  const circumference = Math.PI * radius;
  const progress = (temperaturePercent.value / 100) * circumference;
  return { progress, circumference };
});

const downloadDashProgress = computed(() => {
  const radius = 45;
  const circumference = Math.PI * radius;
  const progress = (downloadPercent.value / 100) * circumference;
  return { progress, circumference };
});

const uploadDashProgress = computed(() => {
  const radius = 45;
  const circumference = Math.PI * radius;
  const progress = (uploadPercent.value / 100) * circumference;
  return { progress, circumference };
});

// Цвет в зависимости от температуры
const temperatureColor = computed(() => {
  const temp = temperature.value;
  if (temp < 50) return '#4ade80'; // зеленый
  if (temp < 70) return '#facc15'; // желтый
  if (temp < 85) return '#fb923c'; // оранжевый
  return '#ef4444'; // красный
});

// Цвет для скорости (одинаковый для обоих)
const speedColor = computed(() => {
  return '#3b82f6'; // синий
});

async function checkTemperature() {
  try {
    const result = await invoke("get_cpu_temperature") as string;
    const lines = result.split('\n').filter(line => line.trim() !== '');
    const dataLines = lines.filter(line =>
      !line.includes('Температура процессора:') &&
      !line.includes('Не найдено') &&
      !line.includes('Доступные датчики:')
    );

    if (dataLines.length > 0) {
      const firstLine = dataLines[0].trim();
      const tempMatch = firstLine.match(/(\d+\.?\d*)°C/);
      if (tempMatch) {
        temperature.value = parseFloat(tempMatch[1]);
      }
    }
  } catch (error) {
    console.error('Ошибка температуры:', error);
  }
}

async function checkNetworkSpeed() {
  try {
    const result = await invoke("get_network_speed") as string;
    // Формат speedtest-cli --simple:
    // Ping: 10.00 ms
    // Download: 100.00 Mbit/s
    // Upload: 50.00 Mbit/s

    const downloadMatch = result.match(/Download:\s+(\d+\.?\d*)\s+Mbit\/s/);
    const uploadMatch = result.match(/Upload:\s+(\d+\.?\d*)\s+Mbit\/s/);

    if (downloadMatch) {
      downloadSpeed.value = parseFloat(downloadMatch[1]);
    }
    if (uploadMatch) {
      uploadSpeed.value = parseFloat(uploadMatch[1]);
    }
  } catch (error) {
    console.error('Ошибка speedtest:', error);
  }
}

async function closeApp() {
  await getCurrentWindow().close();
}

onMounted(() => {
  checkTemperature();
  tempIntervalId = setInterval(checkTemperature, 1000);

  // Запускаем speedtest сразу и каждые 30 секунд
  checkNetworkSpeed();
  speedIntervalId = setInterval(checkNetworkSpeed, 30000);
});

onUnmounted(() => {
  if (tempIntervalId !== null) {
    clearInterval(tempIntervalId);
  }
  if (speedIntervalId !== null) {
    clearInterval(speedIntervalId);
  }
});
</script>

<template>
  <button class="close-button" @click="closeApp">
    <svg viewBox="0 0 24 24" class="close-icon">
      <line x1="18" y1="6" x2="6" y2="18" stroke="white" stroke-width="2" stroke-linecap="round"/>
      <line x1="6" y1="6" x2="18" y2="18" stroke="white" stroke-width="2" stroke-linecap="round"/>
    </svg>
  </button>

  <!-- Спидометр температуры CPU -->
  <div class="speedometer-container temp-speedometer">
    <svg viewBox="0 0 120 80" class="speedometer">
      <path
        d="M 15 70 A 45 45 0 0 1 105 70"
        fill="none"
        stroke="rgba(128, 128, 128, 0.2)"
        stroke-width="10"
        stroke-linecap="round"
      />
      <path
        d="M 15 70 A 45 45 0 0 1 105 70"
        fill="none"
        :stroke="temperatureColor"
        stroke-width="10"
        stroke-linecap="round"
        class="progress-arc"
        :stroke-dasharray="`${tempDashProgress.progress} ${tempDashProgress.circumference}`"
      />
      <text
        x="60"
        y="62"
        text-anchor="middle"
        class="temperature-text"
        :fill="temperatureColor"
      >
        {{ temperature.toFixed(1) }}°
      </text>
    </svg>
  </div>

  <!-- Спидометр Download -->
  <div class="speedometer-container download-speedometer">
    <svg viewBox="0 0 120 80" class="speedometer">
      <path
        d="M 15 70 A 45 45 0 0 1 105 70"
        fill="none"
        stroke="rgba(128, 128, 128, 0.2)"
        stroke-width="10"
        stroke-linecap="round"
      />
      <path
        d="M 15 70 A 45 45 0 0 1 105 70"
        fill="none"
        :stroke="speedColor"
        stroke-width="10"
        stroke-linecap="round"
        class="progress-arc"
        :stroke-dasharray="`${downloadDashProgress.progress} ${downloadDashProgress.circumference}`"
      />
      <text
        x="60"
        y="62"
        text-anchor="middle"
        class="speed-text"
        :fill="speedColor"
      >
        {{ downloadSpeed.toFixed(0) }}
      </text>
      <text
        x="60"
        y="76"
        text-anchor="middle"
        class="label-text"
        fill="#888"
      >
        ↓
      </text>
    </svg>
  </div>

  <!-- Спидометр Upload -->
  <div class="speedometer-container upload-speedometer">
    <svg viewBox="0 0 120 80" class="speedometer">
      <path
        d="M 15 70 A 45 45 0 0 1 105 70"
        fill="none"
        stroke="rgba(128, 128, 128, 0.2)"
        stroke-width="10"
        stroke-linecap="round"
      />
      <path
        d="M 15 70 A 45 45 0 0 1 105 70"
        fill="none"
        :stroke="speedColor"
        stroke-width="10"
        stroke-linecap="round"
        class="progress-arc"
        :stroke-dasharray="`${uploadDashProgress.progress} ${uploadDashProgress.circumference}`"
      />
      <text
        x="60"
        y="62"
        text-anchor="middle"
        class="speed-text"
        :fill="speedColor"
      >
        {{ uploadSpeed.toFixed(0) }}
      </text>
      <text
        x="60"
        y="76"
        text-anchor="middle"
        class="label-text"
        fill="#888"
      >
        ↑
      </text>
    </svg>
  </div>
</template>

<style scoped>
.close-button {
  position: fixed;
  top: 16px;
  right: 16px;
  width: 40px;
  height: 40px;
  background-color: #dc3545;
  border: none;
  border-radius: 8px 8px 8px 0px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 0;
  transition: all 0.2s ease;
  box-shadow: 0 2px 8px rgba(220, 53, 69, 0.3);
  z-index: 1000;
}

.close-button:hover {
  background-color: #c82333;
  box-shadow: 0 4px 12px rgba(220, 53, 69, 0.5);
  transform: scale(1.05);
}

.close-button:active {
  transform: scale(0.95);
}

.close-icon {
  width: 20px;
  height: 20px;
}

.speedometer-container {
  position: fixed;
  width: 100px;
  height: 70px;
  z-index: 999;
}

.temp-speedometer {
  top: 16px;
  left: 16px;
}

.download-speedometer {
  top: 16px;
  left: 126px;
}

.upload-speedometer {
  top: 16px;
  left: 236px;
}

.speedometer {
  width: 100%;
  height: 100%;
  filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.15));
}

.progress-arc {
  transition: stroke 0.3s ease, stroke-dasharray 0.3s ease;
}

.temperature-text {
  font-size: 20px;
  font-weight: 700;
  font-family: 'Inter', 'Arial', sans-serif;
  transition: fill 0.3s ease;
}

.speed-text {
  font-size: 18px;
  font-weight: 700;
  font-family: 'Inter', 'Arial', sans-serif;
  transition: fill 0.3s ease;
}

.label-text {
  font-size: 14px;
  font-weight: 600;
}

</style>
<style>
* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}

:root {
  font-family: Inter, Avenir, Helvetica, Arial, sans-serif;
  font-size: 16px;
  color: #0f0f0f;
  background-color: #f6f6f6;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
}

html, body {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }
}
</style>