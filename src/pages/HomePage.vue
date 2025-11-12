<script setup lang="ts">
</script>

<template>
  <div class="home-page">
    <h1 class="page-title">Home</h1>
    <div class="welcome-content">
      <h2 class="welcome-subtitle">Welcome to BRO</h2>
      <p class="welcome-text">Use the sidebar to navigate to different sections</p>
    </div>
  </div>
</template>

<style scoped>
.home-page {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  min-height: 60vh;
  padding: 20px;
}

.page-title {
  font-size: 48px;
  font-weight: 700;
  color: #0052cc;
  margin-bottom: 40px;
  text-align: center;
}

.welcome-content {
  text-align: center;
}

.welcome-subtitle {
  font-size: 32px;
  font-weight: 600;
  color: #0052cc;
  margin-bottom: 20px;
  opacity: 0.8;
}

.welcome-text {
  font-size: 18px;
  color: #888;
  text-align: center;
}
</style>
