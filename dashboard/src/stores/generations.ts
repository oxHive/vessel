import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type Generation, type GenerationOutput } from '../api/client'

export const useGenerationsStore = defineStore('generations', () => {
  const generations = ref<Generation[]>([])
  const current = ref<{ generation: Generation; outputs: GenerationOutput[] } | null>(null)
  const loading = ref(false)
  const agentReply = ref<string | null>(null)

  let events: EventSource | null = null

  async function fetchAll() {
    loading.value = true
    try {
      const data = await api.getGenerations()
      generations.value = data.generations
    } finally {
      loading.value = false
    }
  }

  async function fetchOne(id: string) {
    loading.value = true
    try {
      current.value = await api.getGeneration(id)
    } finally {
      loading.value = false
    }
  }

  function subscribeToEvents(id: string) {
    unsubscribe()
    events = new EventSource(`/api/v1/generations/${id}/events`)
    events.addEventListener('outputs-updated', () => {
      void fetchOne(id)
    })
    events.addEventListener('agent-reply', (e) => {
      agentReply.value = JSON.parse((e as MessageEvent).data).message
    })
    events.addEventListener('review-done', () => {
      if (current.value) current.value.generation.review_state = 'done'
    })
    // EventSource reconnects automatically; re-fetch on (re)open to close any
    // gap while disconnected. Fires once on initial connect too — harmless.
    events.onopen = () => {
      void fetchOne(id)
    }
  }

  function unsubscribe() {
    events?.close()
    events = null
  }

  return {
    generations,
    current,
    loading,
    agentReply,
    fetchAll,
    fetchOne,
    subscribeToEvents,
    unsubscribe,
  }
})
