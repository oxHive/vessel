import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type Profile } from '../api/client'

export const useProfilesStore = defineStore('profiles', () => {
  const profiles = ref<Profile[]>([])
  const loading = ref(false)

  async function fetchAll() {
    loading.value = true
    try {
      const data = await api.getProfiles()
      profiles.value = data.profiles
    } finally {
      loading.value = false
    }
  }

  async function create(body: { name: string; formality?: string; humor?: string; technical_depth?: string; self_promotion?: string }) {
    const { id } = await api.createProfile(body)
    await fetchAll()
    return id
  }

  async function update(id: string, body: Partial<Omit<Profile, 'id' | 'created_at' | 'updated_at'>>) {
    await api.updateProfile(id, body)
    await fetchAll()
  }

  return { profiles, loading, fetchAll, create, update }
})
