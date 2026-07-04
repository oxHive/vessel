<script setup lang="ts">
import { onMounted, ref } from 'vue'
import { useProfilesStore } from '../stores/profiles'

const store = useProfilesStore()
onMounted(() => store.fetchAll())

const FORMALITY = ['casual', 'balanced', 'professional']
const HUMOR = ['none', 'subtle', 'present']
const DEPTH = ['low', 'medium', 'high']
const PROMO = ['understated', 'balanced', 'direct']

// New profile form
const showForm = ref(false)
const newName = ref('')
const newFormality = ref('balanced')
const newHumor = ref('subtle')
const newDepth = ref('medium')
const newPromo = ref('balanced')
const saving = ref(false)

async function create() {
  if (!newName.value) return
  saving.value = true
  try {
    await store.create({
      name: newName.value,
      formality: newFormality.value,
      humor: newHumor.value,
      technical_depth: newDepth.value,
      self_promotion: newPromo.value,
    })
    newName.value = ''
    showForm.value = false
  } finally {
    saving.value = false
  }
}
</script>

<template>
  <div class="p-6 max-w-3xl">
    <div class="flex items-center justify-between mb-6">
      <h1 class="text-xl font-semibold">Profiles</h1>
      <button
        class="px-4 py-2 bg-amber-500 hover:bg-amber-600 text-black text-sm font-medium rounded transition-colors"
        @click="showForm = !showForm"
      >New Profile</button>
    </div>

    <!-- Create form -->
    <div v-if="showForm" class="mb-6 p-4 bg-vessel-card border border-vessel-border rounded flex flex-col gap-3">
      <input
        v-model="newName"
        placeholder="Profile name"
        class="w-full bg-neutral-900 border border-vessel-border rounded px-3 py-2 text-sm text-neutral-300 placeholder-neutral-600 focus:outline-none focus:border-amber-500"
      />
      <div class="grid grid-cols-2 gap-3">
        <label class="text-xs text-neutral-500">
          Formality
          <select v-model="newFormality" class="mt-1 w-full bg-neutral-900 border border-vessel-border rounded px-2 py-1.5 text-sm text-neutral-300 focus:outline-none focus:border-amber-500">
            <option v-for="o in FORMALITY" :key="o" :value="o">{{ o }}</option>
          </select>
        </label>
        <label class="text-xs text-neutral-500">
          Humor
          <select v-model="newHumor" class="mt-1 w-full bg-neutral-900 border border-vessel-border rounded px-2 py-1.5 text-sm text-neutral-300 focus:outline-none focus:border-amber-500">
            <option v-for="o in HUMOR" :key="o" :value="o">{{ o }}</option>
          </select>
        </label>
        <label class="text-xs text-neutral-500">
          Technical depth
          <select v-model="newDepth" class="mt-1 w-full bg-neutral-900 border border-vessel-border rounded px-2 py-1.5 text-sm text-neutral-300 focus:outline-none focus:border-amber-500">
            <option v-for="o in DEPTH" :key="o" :value="o">{{ o }}</option>
          </select>
        </label>
        <label class="text-xs text-neutral-500">
          Self-promotion
          <select v-model="newPromo" class="mt-1 w-full bg-neutral-900 border border-vessel-border rounded px-2 py-1.5 text-sm text-neutral-300 focus:outline-none focus:border-amber-500">
            <option v-for="o in PROMO" :key="o" :value="o">{{ o }}</option>
          </select>
        </label>
      </div>
      <button
        class="self-start px-4 py-2 bg-amber-500 hover:bg-amber-600 text-black text-sm font-medium rounded transition-colors disabled:opacity-50"
        :disabled="saving"
        @click="create"
      >{{ saving ? 'Creating…' : 'Create' }}</button>
    </div>

    <div v-if="store.loading" class="text-neutral-500 text-sm">Loading…</div>
    <div v-else-if="store.profiles.length === 0" class="text-neutral-500 text-sm py-8 text-center">No profiles yet.</div>
    <div v-else class="flex flex-col gap-3">
      <div
        v-for="profile in store.profiles"
        :key="profile.id"
        class="p-4 bg-vessel-card border border-vessel-border rounded"
      >
        <p class="text-sm font-medium text-neutral-200 mb-2">{{ profile.name }}</p>
        <div class="flex flex-wrap gap-3 text-xs text-neutral-500">
          <span>Formality: <span class="text-neutral-300">{{ profile.formality }}</span></span>
          <span>Humor: <span class="text-neutral-300">{{ profile.humor }}</span></span>
          <span>Depth: <span class="text-neutral-300">{{ profile.technical_depth }}</span></span>
          <span>Self-promo: <span class="text-neutral-300">{{ profile.self_promotion }}</span></span>
        </div>
      </div>
    </div>
  </div>
</template>
