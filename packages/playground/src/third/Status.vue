<script setup lang="ts">
import { computed } from 'vue';
import { output } from '../states'
const props = defineProps<{
  error: string[] | Error | null | undefined,
  output: String,
}>()
const diff = computed(() => props.output.length - output.value.length)
const error = computed(() => {
  if (props.error instanceof Error) {
    return props.error.message;
  }
  if (Array.isArray(props.error)) {
    return props.error.join('\n');
  }
  return '';
});
</script>

<template>
  <div v-if="error" class="text-red" :title="error">
    Error
  </div>
  <div v-else :class="diff === 0 ? 'text-gray' : diff > 0 ? 'text-green' : 'text-red'">
    <span v-if="diff > 0">+{{ diff }}</span>
    <span v-else-if="diff < 0">{{ diff }}</span>
    <span v-else>0</span>
    <span class="text-sm op-80">B</span>
  </div>
</template>
