<script setup lang="ts">
import { RouterLink, RouterView } from 'vue-router'
</script>

<template>
  <div class="flex h-screen bg-vessel-bg text-neutral-100">
    <!-- Sidebar -->
    <aside class="w-52 shrink-0 border-r border-vessel-border flex flex-col py-6 px-4 gap-1">
      <div class="mb-6 px-2">
        <span class="text-amber-500 font-bold text-lg tracking-tight">Vessel</span>
      </div>
      <RouterLink to="/" class="nav-link">Home</RouterLink>
      <RouterLink to="/new" class="nav-link">New Post</RouterLink>
      <RouterLink to="/projects" class="nav-link">Projects</RouterLink>
      <RouterLink to="/profiles" class="nav-link">Profiles</RouterLink>
      <RouterLink to="/settings" class="nav-link">Settings</RouterLink>
    </aside>

    <!-- Main -->
    <main class="flex-1 overflow-y-auto">
      <RouterView />
    </main>
  </div>
</template>

<style scoped>
.nav-link {
  @apply px-3 py-2 rounded text-sm text-neutral-400 hover:text-neutral-100 hover:bg-vessel-card transition-colors;
}
.nav-link.router-link-active {
  @apply text-amber-400 bg-vessel-card;
}
</style>
