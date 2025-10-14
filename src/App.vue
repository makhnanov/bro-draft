<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const temperatureMsg = ref("");
let intervalId: number | null = null;

async function checkTemperature() {
  try {
    const result = await invoke("get_cpu_temperature") as string;
    // Извлекаем все строки после заголовка
    const lines = result.split('\n').filter(line => line.trim() !== '');

    // Убираем заголовок "Температура процессора:"
    const dataLines = lines.filter(line =>
      !line.includes('Температура процессора:') &&
      !line.includes('Не найдено') &&
      !line.includes('Доступные датчики:')
    );

    if (dataLines.length > 0) {
      // Извлекаем только температуру (число с °C)
      const firstLine = dataLines[0].trim();
      const tempMatch = firstLine.match(/(\d+\.?\d*)°C/);
      if (tempMatch) {
        temperatureMsg.value = `${tempMatch[1]}°C`;
      } else {
        temperatureMsg.value = firstLine;
      }
    } else {
      temperatureMsg.value = result;
    }
  } catch (error) {
    temperatureMsg.value = `Ошибка: ${error}`;
  }
}

async function closeApp() {
  await getCurrentWindow().close();
}

onMounted(() => {
  // Запускаем сразу при загрузке
  checkTemperature();
  // Обновляем каждую секунду
  intervalId = setInterval(checkTemperature, 1000);
});

onUnmounted(() => {
  // Очищаем интервал при размонтировании компонента
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
  <main class="container">
    <div class="temperature-section">
      <pre class="temperature-output">{{ temperatureMsg }}</pre>
    </div>
  </main>
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

.temperature-section {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
}

.temperature-output {
  background-color: rgba(0, 0, 0, 0.05);
  padding: 2em;
  border-radius: 12px;
  min-width: 400px;
  text-align: center;
  font-family: 'Courier New', monospace;
  font-size: 1.2em;
  line-height: 1.6;
  white-space: pre-wrap;
  word-wrap: break-word;
  box-shadow: 0 4px 15px rgba(0, 0, 0, 0.1);
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
  line-height: 24px;
  font-weight: 400;
  color: #0f0f0f;
  background-color: #f6f6f6;
  font-synthesis: none;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  -webkit-text-size-adjust: 100%;
  overflow: hidden;
}

html, body {
  width: 100%;
  height: 100%;
  overflow: hidden;
}

.container {
  margin: 0;
  padding: 0;
  width: 100%;
  height: 100vh;
  display: flex;
  flex-direction: column;
  justify-content: center;
  align-items: center;
  text-align: center;
  overflow: hidden;
}

@media (prefers-color-scheme: dark) {
  :root {
    color: #f6f6f6;
    background-color: #2f2f2f;
  }

  .temperature-output {
    background-color: rgba(255, 255, 255, 0.05);
    color: #f6f6f6;
  }
}
</style>