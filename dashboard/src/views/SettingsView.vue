<script setup lang="ts">
import { onMounted } from 'vue'
import { useSettingsStore } from '../stores/settings'

const store = useSettingsStore()
onMounted(() => store.fetch())
</script>

<template>
  <div class="p-6 max-w-2xl">
    <h1 class="text-xl font-semibold mb-6">Settings</h1>

    <div v-if="store.loading" class="text-neutral-500 text-sm">Loading…</div>

    <template v-else-if="store.settings">
      <section class="mb-8">
        <h2 class="text-sm font-medium text-neutral-400 uppercase tracking-wider mb-3">Server</h2>
        <div class="p-4 bg-vessel-card border border-vessel-border rounded flex flex-col gap-2 text-sm">
          <div class="flex justify-between">
            <span class="text-neutral-500">Dashboard port</span>
            <code class="text-amber-400">{{ store.settings.port }}</code>
          </div>
          <div class="flex justify-between">
            <span class="text-neutral-500">Database</span>
            <code class="text-neutral-400 text-xs">{{ store.settings.db_path }}</code>
          </div>
          <div class="flex justify-between">
            <span class="text-neutral-500">Version</span>
            <code class="text-neutral-400">{{ store.settings.version }}</code>
          </div>
        </div>
      </section>

      <section class="mb-8">
        <h2 class="text-sm font-medium text-neutral-400 uppercase tracking-wider mb-3">HiveMind</h2>
        <div class="p-4 bg-vessel-card border border-vessel-border rounded flex flex-col gap-2 text-sm">
          <div class="flex justify-between items-center">
            <span class="text-neutral-500">Port</span>
            <code class="text-amber-400">{{ store.settings.hivemind_port }}</code>
          </div>
          <div class="flex justify-between items-center">
            <span class="text-neutral-500">Status</span>
            <span
              class="text-xs px-2 py-0.5 rounded font-medium"
              :class="store.settings.hivemind_available ? 'bg-green-900 text-green-400' : 'bg-neutral-800 text-neutral-500'"
            >
              {{ store.settings.hivemind_available ? 'Connected' : 'Not running' }}
            </span>
          </div>
          <p v-if="!store.settings.hivemind_available" class="text-xs text-neutral-600">
            Run <code class="text-amber-400">hivemind up</code> to enable project context in generations.
          </p>
        </div>
      </section>

      <section>
        <h2 class="text-sm font-medium text-neutral-400 uppercase tracking-wider mb-3">MCP Config</h2>
        <div class="p-4 bg-vessel-card border border-vessel-border rounded">
          <p class="text-xs text-neutral-500 mb-2">Add to Claude Code MCP settings:</p>
          <pre class="text-xs text-amber-400 font-mono">{{ '{\n  "mcpServers": {\n    "vessel": {\n      "command": "vessel",\n      "args": ["mcp"]\n    }\n  }\n}' }}</pre>
        </div>
      </section>
    </template>
  </div>
</template>
