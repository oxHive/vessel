<script setup lang="ts">
import { onMounted, ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useGenerationsStore } from '../stores/generations'
import { api, PLATFORMS, type FeedbackSignal } from '../api/client'

const route = useRoute()
const store = useGenerationsStore()
const copied = ref<Record<string, boolean>>({})
const feedback = ref<Record<string, FeedbackSignal | null>>({})

const id = computed(() => route.params.id as string)

onMounted(() => store.fetchOne(id.value))

const gen = computed(() => store.current)

const charCounts = computed(() => {
  const map: Record<string, { count: number; limit: number; over: boolean }> = {}
  for (const output of gen.value?.outputs ?? []) {
    const limit = PLATFORMS[output.platform]?.charLimit
    if (limit) {
      map[output.id] = { count: output.content.length, limit, over: output.content.length > limit }
    }
  }
  return map
})

async function copy(platform: string, content: string) {
  await navigator.clipboard.writeText(content)
  copied.value[platform] = true
  setTimeout(() => { copied.value[platform] = false }, 2000)
}

async function sendFeedback(platform: string, signal: FeedbackSignal) {
  if (!gen.value) return
  await api.postFeedback(gen.value.generation.id, platform, signal)
  feedback.value[platform] = signal
}

function platformName(slug: string) {
  return PLATFORMS[slug]?.name ?? slug
}
</script>

<template>
  <div class="p-6 max-w-4xl">
    <div v-if="store.loading" class="text-neutral-500 text-sm">Loading…</div>

    <template v-else-if="gen">
      <div class="mb-6">
        <div class="flex items-center gap-3 mb-1">
          <code class="text-amber-400 font-mono text-lg">{{ gen.generation.tag }}</code>
          <span class="text-xs px-2 py-0.5 rounded bg-neutral-800 text-neutral-400 capitalize">
            {{ gen.generation.category }}
          </span>
        </div>
        <p v-if="gen.generation.context_notes" class="text-neutral-500 text-sm">
          {{ gen.generation.context_notes }}
        </p>
      </div>

      <div class="flex flex-col gap-4">
        <div
          v-for="output in gen.outputs"
          :key="output.id"
          class="bg-vessel-card border border-vessel-border rounded p-4"
        >
          <div class="flex items-center justify-between mb-3">
            <span class="text-sm font-medium text-neutral-300">{{ platformName(output.platform) }}</span>
            <div class="flex items-center gap-2">
              <!-- Char count -->
              <span
                v-if="charCounts[output.id]"
                class="text-xs font-mono"
                :class="charCounts[output.id].over ? 'text-red-400' : 'text-neutral-500'"
              >
                {{ charCounts[output.id].count }}/{{ charCounts[output.id].limit }}
              </span>
              <!-- Feedback -->
              <button
                class="text-xs px-2 py-0.5 rounded transition-colors"
                :class="feedback[output.platform] === 'liked' ? 'bg-green-900 text-green-400' : 'text-neutral-600 hover:text-green-400'"
                @click="sendFeedback(output.platform, 'liked')"
              >👍</button>
              <button
                class="text-xs px-2 py-0.5 rounded transition-colors"
                :class="feedback[output.platform] === 'disliked' ? 'bg-red-900 text-red-400' : 'text-neutral-600 hover:text-red-400'"
                @click="sendFeedback(output.platform, 'disliked')"
              >👎</button>
              <!-- Copy -->
              <button
                class="text-xs px-3 py-1 rounded font-medium transition-colors"
                :class="copied[output.platform] ? 'bg-green-600 text-white' : 'bg-amber-500 hover:bg-amber-600 text-black'"
                @click="copy(output.platform, output.content)"
              >
                {{ copied[output.platform] ? 'Copied!' : 'Copy' }}
              </button>
            </div>
          </div>
          <pre class="text-sm text-neutral-300 whitespace-pre-wrap font-sans leading-relaxed">{{ output.content }}</pre>
        </div>
      </div>
    </template>

    <div v-else class="text-neutral-500 text-sm">Generation not found.</div>
  </div>
</template>
