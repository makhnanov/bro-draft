<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const temperature = ref(0);
let intervalId: number | null = null;

// Вычисляем процент для прогресс-бара (0-100°C -> 0-100%)
const temperaturePercent = computed(() => {
  const minTemp = 0;
  const maxTemp = 100;
  const percent = ((temperature.value - minTemp) / (maxTemp - minTemp)) * 100;
  return Math.min(Math.max(percent, 0), 100);
});

// Вычисляем угол для SVG arc (спидометр от -135° до 135°, всего 270°)
const arcPath = computed(() => {
  const percent = temperaturePercent.value;
  const startAngle = -135;
  const endAngle = 135;
  const angle = startAngle + (percent / 100) * (endAngle - startAngle);

  const radius = 45;
  const centerX = 60;
  const centerY = 60;

  const startRad = (startAngle * Math.PI) / 180;
  const endRad = (angle * Math.PI) / 180;

  const x1 = centerX + radius * Math.cos(startRad);
  const y1 = centerY + radius * Math.sin(startRad);
  const x2 = centerX + radius * Math.cos(endRad);
  const y2 = centerY + radius * Math.sin(endRad);

  const largeArc = angle - startAngle > 180 ? 1 : 0;

  return `M ${x1} ${y1} A ${radius} ${radius} 0 ${largeArc} 1 ${x2} ${y2}`;
});

// Цвет в зависимости от температуры
const temperatureColor = computed(() => {
  const temp = temperature.value;
  if (temp < 50) return '#4ade80'; // зеленый
  if (temp < 70) return '#facc15'; // желтый
  if (temp < 85) return '#fb923c'; // оранжевый
  return '#ef4444'; // красный
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
    console.error('Ошибка:', error);
  }
}

async function closeApp() {
  await getCurrentWindow().close();
}

onMounted(() => {
  checkTemperature();
  intervalId = setInterval(checkTemperature, 1000);
});

onUnmounted(() => {
  if (intervalId !== null) {
    clearInterval(intervalId);
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

  <div class="speedometer-container">
    <svg viewBox="0 0 120 120" class="speedometer">
      <!-- Фоновая дуга (серая) -->
      <path
        d="M 18.18 91.82 A 45 45 0 1 1 101.82 91.82"
        fill="none"
        stroke="rgba(128, 128, 128, 0.2)"
        stroke-width="8"
        stroke-linecap="round"
      />

      <!-- Прогресс дуга (цветная) -->
      <path
        :d="arcPath"
        fill="none"
        :stroke="temperatureColor"
        stroke-width="8"
        stroke-linecap="round"
        class="progress-arc"
      />

      <!-- Температура в центре -->
      <text
        x="60"
        y="65"
        text-anchor="middle"
        class="temperature-text"
        :fill="temperatureColor"
      >
        {{ temperature.toFixed(1) }}°
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
  top: 16px;
  left: 16px;
  width: 120px;
  height: 120px;
  z-index: 999;
}

.speedometer {
  width: 100%;
  height: 100%;
  filter: drop-shadow(0 2px 8px rgba(0, 0, 0, 0.15));
}

.progress-arc {
  transition: stroke 0.3s ease;
}

.temperature-text {
  font-size: 20px;
  font-weight: 700;
  font-family: 'Inter', 'Arial', sans-serif;
  transition: fill 0.3s ease;
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