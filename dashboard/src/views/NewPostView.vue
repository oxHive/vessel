<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useProjectsStore } from '../stores/projects'

const projectsStore = useProjectsStore()

onMounted(() => projectsStore.fetchAll())

// Step state
const step = ref<'category' | 'project' | 'tag' | 'notes' | 'command'>('category')

const category = ref<string>('')
const selectedProjectId = ref<string>('')
const selectedTag = ref<string>('')
const contextNotes = ref<string>('')
const tags = ref<string[]>([])
const loadingTags = ref(false)
const copied = ref(false)

const CATEGORIES = ['release', 'update', 'milestone', 'announcement']

async function selectProject(id: string) {
  selectedProjectId.value = id
  loadingTags.value = true
  tags.value = await projectsStore.tagsFor(id)
  loadingTags.value = false
  step.value = 'tag'
}

function selectTag(tag: string) {
  selectedTag.value = tag
  step.value = 'notes'
}

function proceed() {
  step.value = 'command'
}

const selectedProject = computed(() =>
  projectsStore.projects.find(p => p.id === selectedProjectId.value)
)

const slashCommand = computed(() => {
  const proj = selectedProject.value
  if (!proj) return ''
  const repoArg = proj.repo_path ?? proj.github_repo ?? ''
  let cmd = `/vessel-generate repo_path="${repoArg}" tag="${selectedTag.value}" category="${category.value}"`
  if (contextNotes.value.trim()) {
    cmd += ` context_notes="${contextNotes.value.trim().replace(/"/g, '\\"')}"`
  }
  return cmd
})

async function copyCommand() {
  await navigator.clipboard.writeText(slashCommand.value)
  copied.value = true
  setTimeout(() => { copied.value = false }, 2000)
}
</script>

<template>
  <div class="p-6 max-w-2xl">
    <h1 class="text-xl font-semibold mb-6">New Post</h1>

    <!-- Step: Category -->
    <div v-if="step === 'category'">
      <p class="text-sm text-neutral-400 mb-4">What kind of post?</p>
      <div class="flex flex-wrap gap-2">
        <button
          v-for="cat in CATEGORIES"
          :key="cat"
          class="px-4 py-2 rounded border border-vessel-border text-sm capitalize hover:border-amber-500 hover:text-amber-400 transition-colors"
          @click="() => { category = cat; step = 'project' }"
        >{{ cat }}</button>
      </div>
    </div>

    <!-- Step: Project -->
    <div v-else-if="step === 'project'">
      <p class="text-sm text-neutral-400 mb-4">Select project</p>
      <div v-if="projectsStore.loading" class="text-neutral-500 text-sm">Loading…</div>
      <div v-else-if="projectsStore.projects.length === 0" class="text-neutral-500 text-sm">
        No projects yet. Add one in <a href="/projects" class="text-amber-400 hover:underline">Projects</a>.
      </div>
      <div v-else class="flex flex-col gap-2">
        <button
          v-for="proj in projectsStore.projects"
          :key="proj.id"
          class="px-4 py-3 bg-vessel-card border border-vessel-border rounded text-left text-sm hover:border-amber-500 transition-colors"
          @click="selectProject(proj.id)"
        >
          <span class="text-neutral-300">{{ proj.repo_path ?? proj.github_repo ?? proj.id }}</span>
          <span class="ml-2 text-xs text-neutral-600">{{ proj.provider }}</span>
        </button>
      </div>
    </div>

    <!-- Step: Tag -->
    <div v-else-if="step === 'tag'">
      <p class="text-sm text-neutral-400 mb-4">Select tag</p>
      <div v-if="loadingTags" class="text-neutral-500 text-sm">Loading tags…</div>
      <div v-else-if="tags.length === 0" class="text-neutral-500 text-sm">No git tags found in this repo.</div>
      <div v-else class="flex flex-wrap gap-2">
        <button
          v-for="tag in tags"
          :key="tag"
          class="px-3 py-1.5 rounded border border-vessel-border font-mono text-sm text-amber-400 hover:border-amber-500 transition-colors"
          @click="selectTag(tag)"
        >{{ tag }}</button>
      </div>
    </div>

    <!-- Step: Context notes -->
    <div v-else-if="step === 'notes'">
      <p class="text-sm text-neutral-400 mb-4">Add context notes (optional)</p>
      <textarea
        v-model="contextNotes"
        rows="4"
        placeholder="e.g. Highlight the new retry logic. Tone it down on the self-promotion."
        class="w-full bg-vessel-card border border-vessel-border rounded px-3 py-2 text-sm text-neutral-300 placeholder-neutral-600 focus:outline-none focus:border-amber-500 resize-none"
      />
      <div class="flex gap-3 mt-4">
        <button
          class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-black text-sm font-medium rounded transition-colors"
          @click="proceed"
        >Generate command</button>
        <button
          class="px-4 py-2 text-neutral-400 hover:text-neutral-300 text-sm transition-colors"
          @click="proceed"
        >Skip</button>
      </div>
    </div>

    <!-- Step: Slash command -->
    <div v-else-if="step === 'command'">
      <p class="text-sm text-neutral-400 mb-4">Copy this command and run it in Claude Code:</p>
      <div class="bg-vessel-card border border-vessel-border rounded p-4 font-mono text-sm text-amber-400 break-all mb-4">
        {{ slashCommand }}
      </div>
      <button
        class="px-4 py-2 text-sm font-medium rounded transition-colors"
        :class="copied ? 'bg-green-600 text-white' : 'bg-amber-500 hover:bg-amber-600 text-black'"
        @click="copyCommand"
      >{{ copied ? 'Copied!' : 'Copy command' }}</button>
      <p class="mt-4 text-xs text-neutral-600">
        After running, the generation will appear in <a href="/" class="text-amber-400 hover:underline">Home</a>.
      </p>
    </div>

    <!-- Breadcrumb -->
    <div class="mt-8 flex items-center gap-2 text-xs text-neutral-600">
      <span :class="step !== 'category' ? 'text-amber-500' : ''">{{ category || 'category' }}</span>
      <span>›</span>
      <span :class="['tag', 'notes', 'command'].includes(step) ? 'text-amber-500' : ''">
        {{ selectedProject?.repo_path?.split('/').pop() ?? selectedProject?.github_repo ?? 'project' }}
      </span>
      <span>›</span>
      <span :class="['notes', 'command'].includes(step) ? 'text-amber-500' : ''">{{ selectedTag || 'tag' }}</span>
    </div>
  </div>
</template>
