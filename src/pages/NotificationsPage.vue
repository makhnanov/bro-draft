<script setup lang="ts">
</script>

<template>
  <div class="notifications-page">
    <h1 class="page-title">Notifications</h1>
    <div class="notifications-content">
      <p class="notifications-text">No notifications yet</p>
    </div>
  </div>
</template>

<style scoped lang="stylus">
.notifications-page
  display flex
  flex-direction column
  align-items center
  justify-content center
  min-height 60vh
  padding 20px

.page-title
  font-size 48px
  font-weight 700
  color #0052cc
  margin-bottom 40px
  text-align center

.notifications-content
  text-align center

.notifications-text
  font-size 18px
  color #888
  text-align center
</style>
