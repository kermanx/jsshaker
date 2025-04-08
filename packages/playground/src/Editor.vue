<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watchEffect } from 'vue'
import * as monaco from 'monaco-editor'
import { diffCode, onInputUpdate } from './states';

const props = defineProps<{
  lang: 'javascript' | 'rust' | 'markdown'
  readonly?: boolean,
  options?: Partial<monaco.editor.IStandaloneEditorConstructionOptions>
  diff?: boolean
}>()

const value = defineModel<string>({ required: true })

const container = ref<HTMLElement | null>(null)

onMounted(() => {
  if (props.diff) {
    setupDiffEditor()
  } else {
    setupNormalEditor()
  }
})

function setupNormalEditor() {
  const editor = monaco.editor.create(container.value!, {
    value: value.value,
    language: props.lang,
    readOnly: props.readonly,
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

  if (props.readonly) {
    watchEffect(() => {
      editor.setValue(value.value)
    })
  } else {
    editor.onDidChangeModelContent(() => {
      value.value = editor.getValue()
    })
  }

  const index = onInputUpdate.length;
  onInputUpdate.push(async () => {
    await nextTick()
    editor.setValue(value.value)
  })
  onUnmounted(() => {
    onInputUpdate[index] = () => { }
    editor.dispose()
  })
}

function setupDiffEditor() {
  const editor = monaco.editor.createDiffEditor(container.value!, {
    readOnly: props.readonly,
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

  const originalModel = monaco.editor.createModel(value.value, props.lang)
  const modifiedModel = monaco.editor.createModel(diffCode.value!.value, props.lang)

  editor.setModel({
    original: originalModel,
    modified: modifiedModel,
  })

  watchEffect(() => {
    originalModel.setValue(value.value)
  })
  watchEffect(() => {
    modifiedModel.setValue(diffCode.value!.value)
  })


  onUnmounted(() => {
    editor.dispose()
  })
}
</script>

<template>
  <div ref="container" />
</template>
