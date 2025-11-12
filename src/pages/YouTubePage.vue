<script setup lang="ts">
</script>

<template>
  <div class="youtube-page">
    <h1 class="page-title">YouTube</h1>
    <div class="youtube-content">
      <h2 class="youtube-subtitle">YouTube Integration</h2>
      <p class="youtube-text">Manage your YouTube content and playlists</p>
    </div>
  </div>
</template>

<style scoped>
.youtube-page {
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

.youtube-content {
  text-align: center;
}

.youtube-subtitle {
  font-size: 32px;
  font-weight: 600;
  color: #0052cc;
  margin-bottom: 20px;
  opacity: 0.8;
}

.youtube-text {
  font-size: 18px;
  color: #888;
  text-align: center;
}
</style>
