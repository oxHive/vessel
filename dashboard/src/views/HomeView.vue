<script setup lang="ts">
import { onMounted } from 'vue'
import { useRouter } from 'vue-router'
import { useGenerationsStore } from '../stores/generations'

const router = useRouter()
const store = useGenerationsStore()

onMounted(() => store.fetchAll())

function formatDate(ts: number) {
  return new Date(ts * 1000).toLocaleDateString('en-US', { month: 'short', day: 'numeric', year: 'numeric' })
}
</script>

<template>
  <div class="p-6 max-w-4xl">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-semibold">Generations</h1>
      <button
        class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-black text-sm font-medium rounded transition-colors"
        @click="router.push('/new')"
      >
        New Post
      </button>
    </div>

    <div v-if="store.loading" class="text-neutral-500 text-sm">Loading…</div>

    <div v-else-if="store.generations.length === 0" class="text-neutral-500 text-sm py-12 text-center">
      No generations yet. Run <code class="bg-vessel-card px-1 py-0.5 rounded text-amber-400">/vessel-generate</code> in Claude Code to get started.
    </div>

    <div v-else class="flex flex-col gap-2">
      <div
        v-for="gen in store.generations"
        :key="gen.id"
        class="flex items-center gap-4 px-4 py-3 bg-vessel-card border border-vessel-border rounded cursor-pointer hover:border-neutral-600 transition-colors"
        @click="router.push(`/generation/${gen.id}`)"
      >
        <span class="text-xs px-2 py-0.5 rounded bg-neutral-800 text-neutral-400 capitalize">{{ gen.category }}</span>
        <code class="text-amber-400 text-sm font-mono">{{ gen.tag }}</code>
        <span class="text-neutral-500 text-xs">{{ gen.project_id }}</span>
        <span class="ml-auto text-neutral-600 text-xs">{{ formatDate(gen.created_at) }}</span>
      </div>
    </div>
  </div>
</template>
