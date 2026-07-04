import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type Settings } from '../api/client'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings | null>(null)
  const loading = ref(false)

  async function fetch() {
    loading.value = true
    try {
      settings.value = await api.getSettings()
    } finally {
      loading.value = false
    }
  }

  async function storeToken(project_id: string, token: string) {
    await api.storeGithubToken(project_id, token)
  }

  async function deleteToken(project_id: string) {
    await api.deleteGithubToken(project_id)
  }

  return { settings, loading, fetch, storeToken, deleteToken }
})
