import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type Generation, type GenerationOutput } from '../api/client'

export const useGenerationsStore = defineStore('generations', () => {
  const generations = ref<Generation[]>([])
  const current = ref<{ generation: Generation; outputs: GenerationOutput[] } | null>(null)
  const loading = ref(false)

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

  return { generations, current, loading, fetchAll, fetchOne }
})
