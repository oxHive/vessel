import { defineStore } from 'pinia'
import { ref } from 'vue'
import { api, type Project } from '../api/client'

export const useProjectsStore = defineStore('projects', () => {
  const projects = ref<Project[]>([])
  const loading = ref(false)
  const tagsCache = ref<Record<string, string[]>>({})

  async function fetchAll() {
    loading.value = true
    try {
      const data = await api.getProjects()
      projects.value = data.projects
    } finally {
      loading.value = false
    }
  }

  async function create(body: { profile_id: string; repo_path?: string; github_repo?: string; provider?: string }) {
    const { id } = await api.createProject(body)
    await fetchAll()
    return id
  }

  async function tagsFor(projectId: string): Promise<string[]> {
    if (tagsCache.value[projectId]) return tagsCache.value[projectId]
    const { tags } = await api.getProjectTags(projectId)
    tagsCache.value[projectId] = tags
    return tags
  }

  return { projects, loading, fetchAll, create, tagsFor }
})
