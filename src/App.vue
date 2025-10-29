<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const temperature = ref(0);
const downloadSpeed = ref(0);
const uploadSpeed = ref(0);
const isSpeedTestError = ref(false);
const currentTime = ref('');
const isSidebarCollapsed = ref(false);
const bitcoinPrice = ref(0);
const bitcoinProgress = ref(0);
const isDevelopmentMode = ref(true); // true = dev (20s), false = prod (10min)
let tempIntervalId: number | null = null;
let speedIntervalId: number | null = null;
let timeIntervalId: number | null = null;
let bitcoinIntervalId: number | null = null;
let bitcoinProgressIntervalId: number | null = null;

function toggleSidebar() {
  isSidebarCollapsed.value = !isSidebarCollapsed.value;
}

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

// Цвет для скорости (синий если ок, красный если ошибка)
const speedColor = computed(() => {
  return isSpeedTestError.value ? '#ef4444' : '#3b82f6'; // красный или синий
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

    if (downloadMatch && uploadMatch) {
      downloadSpeed.value = parseFloat(downloadMatch[1]);
      uploadSpeed.value = parseFloat(uploadMatch[1]);
      isSpeedTestError.value = false;
    } else {
      downloadSpeed.value = 0;
      uploadSpeed.value = 0;
      isSpeedTestError.value = true;
    }
  } catch (error) {
    console.error('Ошибка speedtest:', error);
    downloadSpeed.value = 0;
    uploadSpeed.value = 0;
    isSpeedTestError.value = true;
  }
}

function updateTime() {
  const now = new Date();
  const hours = String((now.getHours() - 1 + 24) % 24).padStart(2, '0');
  const minutes = String(now.getMinutes()).padStart(2, '0');
  const seconds = String(now.getSeconds()).padStart(2, '0');
  currentTime.value = `${hours}:${minutes}:${seconds}`;
}

async function fetchBitcoinPrice() {
  try {
    const response = await fetch('https://api.coingecko.com/api/v3/simple/price?ids=bitcoin&vs_currencies=usd');
    const data = await response.json();
    bitcoinPrice.value = data.bitcoin.usd;
  } catch (error) {
    console.error('Ошибка получения курса Bitcoin:', error);
  }
}

function toggleDevelopmentMode() {
  isDevelopmentMode.value = !isDevelopmentMode.value;
  restartBitcoinUpdates();
}

function startBitcoinProgress() {
  bitcoinProgress.value = 0;
  const updateInterval = isDevelopmentMode.value ? 20000 : 600000; // 20s или 10 минут
  const progressStep = 100 / (updateInterval / 100); // обновляем прогресс каждые 100ms

  if (bitcoinProgressIntervalId !== null) {
    clearInterval(bitcoinProgressIntervalId);
  }

  bitcoinProgressIntervalId = setInterval(() => {
    bitcoinProgress.value += progressStep;
    if (bitcoinProgress.value >= 100) {
      bitcoinProgress.value = 100;
    }
  }, 100);
}

function restartBitcoinUpdates() {
  // Очищаем старые интервалы
  if (bitcoinIntervalId !== null) {
    clearInterval(bitcoinIntervalId);
  }
  if (bitcoinProgressIntervalId !== null) {
    clearInterval(bitcoinProgressIntervalId);
  }

  // Получаем курс сразу
  fetchBitcoinPrice();
  startBitcoinProgress();

  // Устанавливаем новый интервал
  const updateInterval = isDevelopmentMode.value ? 20000 : 600000; // 20s или 10 минут
  bitcoinIntervalId = setInterval(() => {
    fetchBitcoinPrice();
    startBitcoinProgress();
  }, updateInterval);
}

async function closeApp() {
  await getCurrentWindow().close();
}

onMounted(() => {
  // Запускаем время сразу
  updateTime();
  timeIntervalId = setInterval(updateTime, 1000);

  // Запускаем температуру сразу
  checkTemperature();
  tempIntervalId = setInterval(checkTemperature, 1000);

  // Запускаем speedtest с задержкой 2 секунды, чтобы UI успел загрузиться
  setTimeout(() => {
    checkNetworkSpeed();
    speedIntervalId = setInterval(checkNetworkSpeed, 30000);
  }, 2000);

  // Запускаем обновление Bitcoin
  restartBitcoinUpdates();
});

onUnmounted(() => {
  if (timeIntervalId !== null) {
    clearInterval(timeIntervalId);
  }
  if (tempIntervalId !== null) {
    clearInterval(tempIntervalId);
  }
  if (speedIntervalId !== null) {
    clearInterval(speedIntervalId);
  }
  if (bitcoinIntervalId !== null) {
    clearInterval(bitcoinIntervalId);
  }
  if (bitcoinProgressIntervalId !== null) {
    clearInterval(bitcoinProgressIntervalId);
  }
});
</script>

<template>
  <!-- Боковое меню в стиле JIRA -->
  <div :class="['sidebar', { collapsed: isSidebarCollapsed }]">
    <button class="sidebar-toggle" @click="toggleSidebar">
      <svg viewBox="0 0 24 24" class="toggle-icon">
        <line x1="3" y1="12" x2="21" y2="12" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <line x1="3" y1="6" x2="21" y2="6" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        <line x1="3" y1="18" x2="21" y2="18" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
      </svg>
    </button>

    <nav class="sidebar-nav">
      <a href="#" class="nav-item active">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <circle cx="12" cy="7" r="4" fill="none" stroke="currentColor" stroke-width="2"/>
        </svg>
        <span class="nav-text">Aliases</span>
      </a>
      <a href="#" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <circle cx="12" cy="12" r="3" fill="none" stroke="currentColor" stroke-width="2"/>
        </svg>
        <span class="nav-text">Recognition</span>
      </a>
      <a href="#" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <circle cx="12" cy="12" r="10" fill="none" stroke="currentColor" stroke-width="2"/>
          <polyline points="12 6 12 12 16 14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
        </svg>
        <span class="nav-text">Watcher</span>
      </a>
      <a href="#" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <polyline points="16 3 21 3 21 8" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <polyline points="8 21 3 21 3 16" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <line x1="21" y1="3" x2="14" y2="10" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <line x1="3" y1="21" x2="10" y2="14" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span class="nav-text">Converters</span>
      </a>
      <a href="#" class="nav-item">
        <svg viewBox="0 0 24 24" class="nav-icon">
          <rect x="2" y="6" width="20" height="12" rx="2" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"/>
          <path d="M12 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <path d="M17 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
          <path d="M7 12h.01" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
        </svg>
        <span class="nav-text">Automatization</span>
      </a>
    </nav>
  </div>

  <!-- Часы сверху посередине -->
  <div class="clock">
    {{ currentTime }}
  </div>

  <!-- Bitcoin виджет -->
  <div class="bitcoin-widget" @click="toggleDevelopmentMode">
    <div class="bitcoin-header">
      <svg viewBox="0 0 24 24" class="bitcoin-icon">
        <path d="M23.638 14.904c-1.602 6.43-8.113 10.34-14.542 8.736C2.67 22.05-1.244 15.525.362 9.105 1.962 2.67 8.475-1.243 14.9.358c6.43 1.605 10.342 8.115 8.738 14.548v-.002zm-6.35-4.613c.24-1.59-.974-2.45-2.64-3.03l.54-2.153-1.315-.33-.525 2.107c-.345-.087-.705-.167-1.064-.25l.526-2.127-1.32-.33-.54 2.165c-.285-.067-.565-.132-.84-.2l-1.815-.45-.35 1.407s.975.225.955.236c.535.136.63.486.615.766l-1.477 5.92c-.075.166-.24.406-.614.314.015.02-.96-.24-.96-.24l-.66 1.51 1.71.426.93.242-.54 2.19 1.32.327.54-2.17c.36.1.705.19 1.05.273l-.51 2.154 1.32.33.545-2.19c2.24.427 3.93.257 4.64-1.774.57-1.637-.03-2.58-1.217-3.196.854-.193 1.5-.76 1.68-1.93h.01zm-3.01 4.22c-.404 1.64-3.157.75-4.05.53l.72-2.9c.896.23 3.757.67 3.33 2.37zm.41-4.24c-.37 1.49-2.662.735-3.405.55l.654-2.64c.744.18 3.137.524 2.75 2.084v.006z" fill="currentColor"/>
      </svg>
      <span class="bitcoin-label">BTC</span>
      <span class="dev-mode-badge" v-if="isDevelopmentMode">DEV</span>
    </div>
    <div class="bitcoin-price">${{ bitcoinPrice.toLocaleString('en-US', { minimumFractionDigits: 0, maximumFractionDigits: 0 }) }}</div>
    <div class="bitcoin-progress-bar">
      <div class="bitcoin-progress-fill" :style="{ width: bitcoinProgress + '%' }"></div>
    </div>
  </div>

  <button class="close-button" @click="closeApp">
    <svg viewBox="0 0 24 24" class="close-icon">
      <line x1="18" y1="6" x2="6" y2="18" stroke="white" stroke-width="2" stroke-linecap="round"/>
      <line x1="6" y1="6" x2="18" y2="18" stroke="white" stroke-width="2" stroke-linecap="round"/>
    </svg>
  </button>

<!-- Speedometers-->
  <div style="margin-left: 250px;">
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
  </div>
</template>

<style scoped>
/* Боковое меню в стиле JIRA */
.sidebar {
  position: fixed;
  left: 0;
  top: 0;
  bottom: 0;
  width: 240px;
  background: linear-gradient(180deg, #0747a6 0%, #0052cc 100%);
  box-shadow: 2px 0 8px rgba(0, 0, 0, 0.15);
  transition: width 0.3s ease;
  z-index: 999;
  display: flex;
  flex-direction: column;
}

.sidebar.collapsed {
  width: 64px;
}

.sidebar-toggle {
  width: 100%;
  height: 64px;
  background: transparent;
  border: none;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
  color: #ffffff;
  transition: background-color 0.2s ease;
  border-bottom: 1px solid rgba(255, 255, 255, 0.1);
}

.sidebar-toggle:hover {
  background-color: rgba(255, 255, 255, 0.1);
}

.toggle-icon {
  width: 24px;
  height: 24px;
}

.sidebar-nav {
  flex: 1;
  padding: 16px 0;
  overflow-y: auto;
}

.nav-item {
  display: flex;
  align-items: center;
  padding: 12px 20px;
  color: rgba(255, 255, 255, 0.8);
  text-decoration: none;
  font-size: 14px;
  font-weight: 500;
  transition: all 0.2s ease;
  position: relative;
  white-space: nowrap;
}

.nav-item:hover {
  background-color: rgba(255, 255, 255, 0.1);
  color: #ffffff;
}

.nav-item.active {
  background-color: rgba(255, 255, 255, 0.15);
  color: #ffffff;
}

.nav-item.active::before {
  content: '';
  position: absolute;
  left: 0;
  top: 0;
  bottom: 0;
  width: 3px;
  background-color: #ffffff;
}

.nav-icon {
  width: 20px;
  height: 20px;
  min-width: 20px;
  margin-right: 12px;
}

.sidebar.collapsed .nav-icon {
  margin-right: 0;
}

.nav-text {
  opacity: 1;
  transition: opacity 0.2s ease;
}

.sidebar.collapsed .nav-text {
  opacity: 0;
  width: 0;
  overflow: hidden;
}

.clock {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(-50%);
  font-size: 48px;
  font-weight: 700;
  font-family: 'Inter', 'Arial', sans-serif;
  color: #ffffff;
  text-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  z-index: 1000;
  letter-spacing: 2px;
  background-color: rgba(60, 60, 60, 0.85);
  padding: 12px 32px;
  border-radius: 12px;
  backdrop-filter: blur(10px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
}

.bitcoin-widget {
  position: fixed;
  top: 16px;
  left: 50%;
  transform: translateX(180px);
  background-color: rgba(40, 40, 40, 0.9);
  padding: 12px 20px;
  border-radius: 12px;
  backdrop-filter: blur(10px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  z-index: 1000;
  cursor: pointer;
  transition: all 0.2s ease;
  min-width: 180px;
}

.bitcoin-widget:hover {
  background-color: rgba(50, 50, 50, 0.95);
  transform: translateX(180px) scale(1.02);
}

.bitcoin-header {
  display: flex;
  align-items: center;
  gap: 8px;
  margin-bottom: 8px;
}

.bitcoin-icon {
  width: 20px;
  height: 20px;
  color: #f7931a;
}

.bitcoin-label {
  font-size: 14px;
  font-weight: 600;
  color: #ffffff;
  font-family: 'Inter', 'Arial', sans-serif;
}

.dev-mode-badge {
  margin-left: auto;
  font-size: 10px;
  font-weight: 700;
  color: #000;
  background-color: #facc15;
  padding: 2px 6px;
  border-radius: 4px;
}

.bitcoin-price {
  font-size: 24px;
  font-weight: 700;
  color: #ffffff;
  font-family: 'Inter', 'Arial', sans-serif;
  margin-bottom: 8px;
}

.bitcoin-progress-bar {
  width: 100%;
  height: 4px;
  background-color: rgba(255, 255, 255, 0.2);
  border-radius: 2px;
  overflow: hidden;
}

.bitcoin-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #f7931a 0%, #ffb84d 100%);
  transition: width 0.1s linear;
  border-radius: 2px;
}

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
  width: 100px;
  height: 70px;
  z-index: 999;
}

.temp-speedometer {
  top: 16px;
  left: 16px;
  background-color: rgba(40, 40, 40, 0.9);
  padding: 8px;
  border-radius: 12px;
  backdrop-filter: blur(10px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  margin-top: 12px;
  width: 116px;
  height: 86px;
  display: flex;
  align-items: center;
  justify-content: center;
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