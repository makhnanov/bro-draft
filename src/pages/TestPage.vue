<script setup lang="ts">
import { ref, onMounted, onUnmounted, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";
import SpeedTest from "@cloudflare/speedtest";

const temperature = ref(0);
const downloadSpeed = ref(0);
const uploadSpeed = ref(0);
const isSpeedTestError = ref(false);
const speedtestProgress = ref(0);
const isSpeedTestRunning = ref(false);
const currentTime = ref('');
const bitcoinPrice = ref(0);
const bitcoinProgress = ref(0);
const isDevelopmentMode = ref(true); // true = dev (20s), false = prod (10min)
let tempIntervalId: number | null = null;
let speedIntervalId: number | null = null;
let speedtestProgressIntervalId: number | null = null;
let timeIntervalId: number | null = null;
let bitcoinIntervalId: number | null = null;
let bitcoinProgressIntervalId: number | null = null;

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

function startSpeedtestProgress() {
  speedtestProgress.value = 0;
  const updateInterval = 300000; // 5 минут
  const progressStep = 100 / (updateInterval / 100); // обновляем прогресс каждые 100ms

  if (speedtestProgressIntervalId !== null) {
    clearInterval(speedtestProgressIntervalId);
  }

  speedtestProgressIntervalId = setInterval(() => {
    if (!isSpeedTestRunning.value) {
      speedtestProgress.value += progressStep;
      if (speedtestProgress.value >= 100) {
        speedtestProgress.value = 100;
      }
    }
  }, 100);
}

async function checkNetworkSpeed() {
  // Предотвращаем запуск если тест уже идет
  if (isSpeedTestRunning.value) {
    return;
  }

  try {
    isSpeedTestError.value = false;
    isSpeedTestRunning.value = true;
    speedtestProgress.value = 0;

    // Используем Cloudflare Speedtest API
    // Отключаем packet loss test, т.к. он требует WebRTC (недоступен в Tauri)
    const speedtest = new SpeedTest({
      measurements: [
        { type: 'latency', numPackets: 1 },
        { type: 'download', bytes: 1e7, count: 16 }, // 10MB x 16 запросов для высокой скорости
        { type: 'upload', bytes: 1e7, count: 12 }    // 10MB x 12 запросов
      ]
    });

    // Обработчик обновления во время теста
    speedtest.onResultsChange = ({ type }) => {
      if (type === 'download') {
        const downloadBps = speedtest.results.getDownloadBandwidth();
        if (downloadBps) {
          // Конвертируем bps в Mbps
          downloadSpeed.value = downloadBps / 1000000;
        }
      }

      if (type === 'upload') {
        const uploadBps = speedtest.results.getUploadBandwidth();
        if (uploadBps) {
          // Конвертируем bps в Mbps
          uploadSpeed.value = uploadBps / 1000000;
        }
      }
    };

    // Обработчик завершения теста
    speedtest.onFinish = (results) => {
      const downloadBps = results.getDownloadBandwidth();
      const uploadBps = results.getUploadBandwidth();

      console.log('Cloudflare Speedtest результаты:');
      console.log('Download (bps):', downloadBps);
      console.log('Upload (bps):', uploadBps);
      console.log('Download (Mbps):', downloadBps ? downloadBps / 1000000 : 0);
      console.log('Upload (Mbps):', uploadBps ? uploadBps / 1000000 : 0);

      // Конвертируем bps в Mbps
      downloadSpeed.value = downloadBps ? downloadBps / 1000000 : 0;
      uploadSpeed.value = uploadBps ? uploadBps / 1000000 : 0;
      isSpeedTestError.value = false;
      isSpeedTestRunning.value = false;

      // Перезапускаем прогресс-бар
      startSpeedtestProgress();
    };

    // Обработчик ошибки
    speedtest.onError = (error) => {
      console.error('Ошибка Cloudflare Speedtest:', error);
      downloadSpeed.value = 0;
      uploadSpeed.value = 0;
      isSpeedTestError.value = true;
      isSpeedTestRunning.value = false;

      // Перезапускаем прогресс-бар даже при ошибке
      startSpeedtestProgress();
    };

    // Запускаем тест
    await speedtest.play();

  } catch (error) {
    console.error('Ошибка speedtest:', error);
    downloadSpeed.value = 0;
    uploadSpeed.value = 0;
    isSpeedTestError.value = true;
    isSpeedTestRunning.value = false;
    startSpeedtestProgress();
  }
}

function onSpeedometerClick() {
  // Запускаем тест при клике
  checkNetworkSpeed();
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
    startSpeedtestProgress();
    speedIntervalId = setInterval(checkNetworkSpeed, 300000); // 5 минут
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
  if (speedtestProgressIntervalId !== null) {
    clearInterval(speedtestProgressIntervalId);
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
  <div class="test-page">
    <!-- Часы сверху -->
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

    <h1 class="page-title">Test Page</h1>

    <div class="speedometers-container">
      <!-- Спидометр температуры CPU -->
      <div class="speedometer-box">
        <h2 class="speedometer-title">CPU Temperature</h2>
        <div class="speedometer-wrapper">
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
      </div>

      <!-- Спидометр Download -->
      <div class="speedometer-box" @click="onSpeedometerClick" :style="{ cursor: isSpeedTestRunning ? 'wait' : 'pointer' }">
        <h2 class="speedometer-title">Download Speed</h2>
        <div class="speedometer-wrapper">
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
          <div class="speedtest-progress-bar">
            <div class="speedtest-progress-fill" :style="{ width: speedtestProgress + '%' }"></div>
          </div>
        </div>
      </div>

      <!-- Спидометр Upload -->
      <div class="speedometer-box" @click="onSpeedometerClick" :style="{ cursor: isSpeedTestRunning ? 'wait' : 'pointer' }">
        <h2 class="speedometer-title">Upload Speed</h2>
        <div class="speedometer-wrapper">
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
          <div class="speedtest-progress-bar">
            <div class="speedtest-progress-fill" :style="{ width: speedtestProgress + '%' }"></div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.test-page {
  padding: 20px;
  max-width: 1200px;
  margin: 0 auto;
}

.page-title {
  font-size: 32px;
  font-weight: 700;
  color: #0052cc;
  margin-bottom: 40px;
  text-align: center;
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

.speedometers-container {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
  gap: 30px;
  justify-items: center;
}

.speedometer-box {
  background-color: rgba(40, 40, 40, 0.9);
  padding: 20px;
  border-radius: 12px;
  backdrop-filter: blur(10px);
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.2);
  width: 100%;
  max-width: 250px;
  transition: transform 0.2s ease, box-shadow 0.2s ease;
}

.speedometer-box:hover {
  transform: scale(1.05);
  box-shadow: 0 6px 20px rgba(0, 0, 0, 0.3);
}

.speedometer-title {
  font-size: 14px;
  font-weight: 600;
  color: #888;
  text-align: center;
  margin-bottom: 10px;
  text-transform: uppercase;
  letter-spacing: 1px;
}

.speedometer-wrapper {
  display: flex;
  flex-direction: column;
  align-items: center;
}

.speedometer {
  width: 100%;
  height: auto;
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

.speedtest-progress-bar {
  width: 100%;
  height: 3px;
  background-color: rgba(59, 130, 246, 0.2);
  border-radius: 2px;
  overflow: hidden;
  margin-top: 10px;
}

.speedtest-progress-fill {
  height: 100%;
  background: linear-gradient(90deg, #3b82f6 0%, #60a5fa 100%);
  transition: width 0.1s linear;
  border-radius: 2px;
}
</style>
