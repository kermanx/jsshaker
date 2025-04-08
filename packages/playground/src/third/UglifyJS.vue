<script lang="ts">
import { computed, ref, shallowRef, watch } from 'vue'
import { minify } from 'uglify-js'
import { debouncedInput, format } from '../states'
import Status from './Status.vue';

export const isAdvanced = ref(false);
export const output = computed(() => minify(debouncedInput.value, {

}))
export const errors = shallowRef<string[]>([]);

</script>

<template>
  <div flex>
    UglifyJS
    <div flex-grow />
    <Status :errors="output.error" :output="output.code" />
  </div>
</template>
