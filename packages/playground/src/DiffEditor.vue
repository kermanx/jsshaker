<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watchEffect } from 'vue'
import * as monaco from 'monaco-editor'
import { onInputUpdate } from './states';

const props = defineProps<{
  lang: 'javascript'
  options?: Partial<monaco.editor.IStandaloneEditorConstructionOptions>
  original: string
  modified: string
}>()

const container = ref<HTMLElement | null>(null)

onMounted(async () => {
  const originalModel = monaco.editor.createModel(props.original, props.lang)
  const modifiedModel = monaco.editor.createModel(props.modified, props.lang)

  const editor = monaco.editor.createDiffEditor(container.value!, {
    originalEditable: false,
    language: props.lang,
    readOnly: true,
    automaticLayout: true,
    lineNumbersMinChars: 3,
    wordWrap: 'on',
    wordWrapColumn: 80,
    padding: {
      top: 16,
    },
    tabSize: 2,
    minimap: {
      enabled: false,
    },
    ...props.options,
  })
  editor.setModel({
    original: originalModel,
    modified: modifiedModel,
  });

  watchEffect(() => {
    originalModel.setValue(props.original)
  })
  watchEffect(() => {
    modifiedModel.setValue(props.modified)
  })

  const index = onInputUpdate.length;
  onInputUpdate.push(async () => {
    await nextTick()
    await nextTick()
    originalModel.setValue(props.original)
    modifiedModel.setValue(props.modified)
  })
  onUnmounted(() => {
    onInputUpdate[index] = () => { }
  })
})
</script>

<template>
  <div ref="container" />
</template>
