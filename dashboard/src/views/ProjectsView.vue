<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useProjectsStore } from '../stores/projects'
import { useProfilesStore } from '../stores/profiles'
import { useSettingsStore } from '../stores/settings'

const projectsStore = useProjectsStore()
const profilesStore = useProfilesStore()
const settingsStore = useSettingsStore()

onMounted(() => Promise.all([projectsStore.fetchAll(), profilesStore.fetchAll()]))

// New project form
const showForm = ref(false)
const newRepoPath = ref('')
const newProfileId = ref('')
const saving = ref(false)

async function addProject() {
  if (!newRepoPath.value || !newProfileId.value) return
  saving.value = true
  try {
    await projectsStore.create({ profile_id: newProfileId.value, repo_path: newRepoPath.value, provider: 'local' })
    newRepoPath.value = ''
    newProfileId.value = ''
    showForm.value = false
  } finally {
    saving.value = false
  }
}

// GitHub token per project
const tokenInputs = ref<Record<string, string>>({})
const tokenSaving = ref<Record<string, boolean>>({})

async function saveToken(projectId: string) {
  const token = tokenInputs.value[projectId]
  if (!token) return
  tokenSaving.value[projectId] = true
  try {
    await settingsStore.storeToken(projectId, token)
    tokenInputs.value[projectId] = ''
  } finally {
    tokenSaving.value[projectId] = false
  }
}

function profileName(id: string) {
  return profilesStore.profiles.find(p => p.id === id)?.name ?? id
}
</script>

<template>
  <div class="p-6 max-w-3xl">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-semibold">Projects</h1>
      <button
        class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-black text-sm font-medium rounded transition-colors"
        @click="showForm = !showForm"
      >Add Project</button>
    </div>

    <!-- Add form -->
    <div v-if="showForm" class="mb-6 p-4 bg-vessel-card border border-vessel-border rounded flex flex-col gap-3">
      <input
        v-model="newRepoPath"
        placeholder="/absolute/path/to/repo"
        class="w-full bg-neutral-900 border border-vessel-border rounded px-3 py-2 text-sm text-neutral-300 placeholder-neutral-600 focus:outline-none focus:border-amber-500"
      />
      <select
        v-model="newProfileId"
        class="w-full bg-neutral-900 border border-vessel-border rounded px-3 py-2 text-sm text-neutral-300 focus:outline-none focus:border-amber-500"
      >
        <option value="">Select profile…</option>
        <option v-for="p in profilesStore.profiles" :key="p.id" :value="p.id">{{ p.name }}</option>
      </select>
      <button
        class="self-start px-4 py-2 bg-amber-500 hover:bg-amber-600 text-black text-sm font-medium rounded transition-colors disabled:opacity-50"
        :disabled="saving"
        @click="addProject"
      >{{ saving ? 'Adding…' : 'Add' }}</button>
    </div>

    <div v-if="projectsStore.loading" class="text-neutral-500 text-sm">Loading…</div>
    <div v-else-if="projectsStore.projects.length === 0" class="text-neutral-500 text-sm py-8 text-center">
      No projects yet.
    </div>
    <div v-else class="flex flex-col gap-3">
      <div
        v-for="proj in projectsStore.projects"
        :key="proj.id"
        class="p-4 bg-vessel-card border border-vessel-border rounded"
      >
        <div class="flex items-start justify-between mb-3">
          <div>
            <p class="text-sm text-neutral-300 font-mono">{{ proj.repo_path ?? proj.github_repo ?? proj.id }}</p>
            <p class="text-xs text-neutral-600 mt-0.5">Profile: {{ profileName(proj.profile_id) }}</p>
          </div>
          <span class="text-xs px-2 py-0.5 rounded bg-neutral-800 text-neutral-500">{{ proj.provider }}</span>
        </div>
        <!-- GitHub token -->
        <div class="flex gap-2">
          <input
            v-model="tokenInputs[proj.id]"
            type="password"
            placeholder="GitHub token (optional)"
            class="flex-1 bg-neutral-900 border border-vessel-border rounded px-3 py-1.5 text-xs text-neutral-300 placeholder-neutral-600 focus:outline-none focus:border-amber-500"
          />
          <button
            class="px-3 py-1.5 text-xs rounded bg-neutral-800 hover:bg-neutral-700 text-neutral-300 transition-colors disabled:opacity-50"
            :disabled="tokenSaving[proj.id] || !tokenInputs[proj.id]"
            @click="saveToken(proj.id)"
          >{{ tokenSaving[proj.id] ? 'Saving…' : 'Save token' }}</button>
        </div>
      </div>
    </div>
  </div>
</template>
