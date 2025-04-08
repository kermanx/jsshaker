<script setup lang="ts">
import { computed } from "vue";
import { diffWith, diffCode } from "../states"

const props = defineProps<{
  name: string
  code: string
}>()

function onToggle() {
  if (diffWith.value === props.name) {
    diffWith.value = "none";
    diffCode.value = null;
  } else {
    diffWith.value = props.name;
    diffCode.value = computed(() => props.code);
  }
}
</script>

<template>
  <button class="px-2 mx-2 select-none" :class="diffWith === props.name ? 'bg-blue' : 'bg-dark'" @click="onToggle">
    Diff
  </button>
</template>
