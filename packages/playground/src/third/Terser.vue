<script setup lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'
import { minify } from 'terser'
import { debouncedInput, format } from '../states'
import Status from './Status.vue';
import DiffToggle from './DiffToggle.vue';

const output = ref<string>("");
const formatted = computed(() => format(output.value));
const error = shallowRef<Error | null>(null);

watch([debouncedInput], async ([input]) => {
  try {
    const result = await minify(input, {
      // compress: true,
    });
    console.log(result.code);
    output.value = result.code || "// undefined";
    error.value = null;
  } catch (e) {
    output.value = "";
    error.value = e as Error;
  }
}, { immediate: true });
</script>

<template>
  <div flex>
    Terser
    <div flex-grow />
    <Status :error :output="formatted" />
    <DiffToggle name="UglifyJS" :code="formatted" />
  </div>
</template>
