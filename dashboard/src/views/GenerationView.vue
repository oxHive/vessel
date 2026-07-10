<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useGenerationsStore } from '../stores/generations'
import { api, PLATFORMS, type FeedbackSignal, type GenerationOutput } from '../api/client'

const route = useRoute()
const store = useGenerationsStore()
const copied = ref<Record<string, boolean>>({})
const feedback = ref<Record<string, FeedbackSignal | null>>({})
const drafts = ref<Record<string, string>>({})
const pendingNotes = ref<Record<string, string[]>>({})

const GENERAL = '_general'

const id = computed(() => route.params.id as string)

onMounted(() => {
  void store.fetchOne(id.value)
  store.subscribeToEvents(id.value)
})
onUnmounted(() => store.unsubscribe())

const gen = computed(() => store.current)
const reviewDone = computed(() => gen.value?.generation.review_state === 'done')

// Outputs arrive ordered platform, revision_number DESC — first per platform is latest.
const latestOutputs = computed(() => {
  const seen = new Set<string>()
  const latest: GenerationOutput[] = []
  for (const output of gen.value?.outputs ?? []) {
    if (!seen.has(output.platform)) {
      seen.add(output.platform)
      latest.push(output)
    }
  }
  return latest
})

// New revisions arrived — queued notes have been applied.
watch(
  () => gen.value?.outputs,
  () => {
    pendingNotes.value = {}
  },
)

const charCounts = computed(() => {
  const map: Record<string, { count: number; limit: number; over: boolean }> = {}
  for (const output of latestOutputs.value) {
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

async function sendRevision(key: string) {
  const note = (drafts.value[key] ?? '').trim()
  if (!note || !gen.value) return
  const platform = key === GENERAL ? undefined : key
  await api.postRevision(gen.value.generation.id, note, platform)
  pendingNotes.value[key] = [...(pendingNotes.value[key] ?? []), note]
  drafts.value[key] = ''
}

async function markDone() {
  if (!gen.value) return
  await api.markReviewDone(gen.value.generation.id)
  gen.value.generation.review_state = 'done'
}

function platformName(slug: string) {
  return PLATFORMS[slug]?.name ?? slug
}
</script>

<template>
  <div class="p-6 max-w-4xl">
    <div v-if="store.loading && !gen" class="text-neutral-500 text-sm">Loading…</div>

    <template v-else-if="gen">
      <div class="mb-6">
        <div class="flex items-center justify-between">
          <div class="flex items-center gap-3 mb-1">
            <code class="text-amber-400 font-mono text-lg">{{ gen.generation.tag }}</code>
            <span class="text-xs px-2 py-0.5 rounded bg-neutral-800 text-neutral-400 capitalize">
              {{ gen.generation.category }}
            </span>
          </div>
          <button
            v-if="!reviewDone"
            class="text-xs px-3 py-1 rounded font-medium bg-neutral-800 text-neutral-300 hover:bg-neutral-700 transition-colors"
            @click="markDone"
          >Done reviewing</button>
          <span v-else class="text-xs px-2 py-0.5 rounded bg-green-900 text-green-400">
            Review complete
          </span>
        </div>
        <p v-if="gen.generation.context_notes" class="text-neutral-500 text-sm">
          {{ gen.generation.context_notes }}
        </p>
      </div>

      <!-- Agent status -->
      <div
        v-if="store.agentReply"
        class="mb-4 text-sm text-neutral-400 bg-vessel-card border border-vessel-border rounded px-3 py-2"
      >
        <span class="text-amber-400 mr-2">agent</span>{{ store.agentReply }}
      </div>

      <div class="flex flex-col gap-4">
        <div
          v-for="output in latestOutputs"
          :key="output.id"
          class="bg-vessel-card border border-vessel-border rounded p-4"
        >
          <div class="flex items-center justify-between mb-3">
            <div class="flex items-center gap-2">
              <span class="text-sm font-medium text-neutral-300">{{ platformName(output.platform) }}</span>
              <span
                v-if="output.revision_number > 0"
                class="text-xs px-1.5 py-0.5 rounded bg-neutral-800 text-neutral-500 font-mono"
              >rev {{ output.revision_number }}</span>
            </div>
            <div class="flex items-center gap-2">
              <span
                v-if="charCounts[output.id]"
                class="text-xs font-mono"
                :class="charCounts[output.id].over ? 'text-red-400' : 'text-neutral-500'"
              >
                {{ charCounts[output.id].count }}/{{ charCounts[output.id].limit }}
              </span>
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

          <!-- Revision request -->
          <div v-if="!reviewDone" class="mt-3 pt-3 border-t border-vessel-border">
            <div v-if="pendingNotes[output.platform]?.length" class="flex flex-wrap gap-1 mb-2">
              <span
                v-for="(note, i) in pendingNotes[output.platform]"
                :key="i"
                class="text-xs px-2 py-0.5 rounded bg-amber-950 text-amber-400"
              >⏳ {{ note }}</span>
            </div>
            <div class="flex gap-2">
              <textarea
                v-model="drafts[output.platform]"
                rows="1"
                placeholder="Request a revision for this platform…"
                class="flex-1 text-sm bg-neutral-900 border border-vessel-border rounded px-2 py-1 text-neutral-300 resize-y"
                @keydown.enter.exact.prevent="sendRevision(output.platform)"
              />
              <button
                class="text-xs px-3 py-1 rounded font-medium bg-neutral-800 text-neutral-300 hover:bg-neutral-700 transition-colors"
                @click="sendRevision(output.platform)"
              >Send</button>
            </div>
          </div>
        </div>
      </div>

      <!-- Generation-level revision -->
      <div v-if="!reviewDone" class="mt-4 bg-vessel-card border border-vessel-border rounded p-4">
        <div class="text-sm font-medium text-neutral-300 mb-2">Revise all platforms</div>
        <div v-if="pendingNotes[GENERAL]?.length" class="flex flex-wrap gap-1 mb-2">
          <span
            v-for="(note, i) in pendingNotes[GENERAL]"
            :key="i"
            class="text-xs px-2 py-0.5 rounded bg-amber-950 text-amber-400"
          >⏳ {{ note }}</span>
        </div>
        <div class="flex gap-2">
          <textarea
            v-model="drafts[GENERAL]"
            rows="2"
            placeholder="e.g. shorter overall, drop the hashtags…"
            class="flex-1 text-sm bg-neutral-900 border border-vessel-border rounded px-2 py-1 text-neutral-300 resize-y"
          />
          <button
            class="text-xs px-3 py-1 rounded font-medium bg-amber-500 hover:bg-amber-600 text-black transition-colors self-end"
            @click="sendRevision(GENERAL)"
          >Send</button>
        </div>
      </div>
    </template>

    <div v-else class="text-neutral-500 text-sm">Generation not found.</div>
  </div>
</template>
