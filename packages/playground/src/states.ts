import { compressToBase64, decompressFromBase64 } from 'lz-string'
import { computed, ref, shallowRef, watch, watchEffect } from 'vue'
import { DEMO } from './examples';

export const onInputUpdate: (() => void)[] = []
export const input = ref('')
export const preset = ref('recommended')
export const doMinify = ref(false)
export const alwaysInline = ref(false)

export const debouncedInput = ref('')
let debounceTimeout = Number.NaN
watch(input, (input) => {
  clearInterval(debounceTimeout)
  debounceTimeout = setTimeout(() => {
    debouncedInput.value = input
  }, 300)
})

export function load(reset = false) {
  let parsed
  if (!reset && window.location.hash) {
    try {
      parsed = JSON.parse(decompressFromBase64(window.location.hash.slice(1)) || '{}')
    }
    catch (e) { console.error(e) }
  }
  parsed ||= {}
  debouncedInput.value = input.value = parsed.input ?? DEMO
  onInputUpdate.forEach(fn => fn())
  preset.value = parsed.preset ?? (parsed.doTreeShake != null ? (parsed.doTreeShake ? 'recommended' : 'disabled') : 'recommended')
  doMinify.value = parsed.doMinify ?? false
  alwaysInline.value = parsed.alwaysInline ?? false
  save()
}

function save() {
  window.location.hash = compressToBase64(JSON.stringify({
    input: input.value,
    preset: preset.value,
    doMinify: doMinify.value,
    alwaysInline: alwaysInline.value,
  }))
}

load()
watchEffect(save)

let library = shallowRef<typeof import('jsshaker') | null>(null)
function treeShake(...args: Parameters<(typeof import('jsshaker'))['shakeSingleModule']>) {
  if (!library.value) {
    import('jsshaker').then(lib => {
      library.value = lib
    }).catch(err => {
      console.error(err)
      library.value = {
        shakeSingleModule: () => ({ output: `Failed to load library.\n${err}`, diagnostics: [] }),
        ...({} as any)
      }
    })
    return { output: 'Loading library...', diagnostics: [] }
  }
  try {
    return library.value.shakeSingleModule(...args)
  }
  catch (e) {
    console.error(e)
    const diagnostics = [String(e)] as any
    diagnostics.isError = true
    return { diagnostics, output: '' }
  }
}

const copyOnly = computed(() => treeShake(debouncedInput.value, {
  preset: 'disabled',
  minify: false,
  alwaysInlineLiteral: false,
}))
const minifiedOnly = computed(() => treeShake(debouncedInput.value, {
  preset: 'disabled',
  minify: true,
  alwaysInlineLiteral: false,
}))
const treeShakedOnly = computed(() => treeShake(debouncedInput.value, {
  preset: preset.value as any,
  minify: false,
  alwaysInlineLiteral: alwaysInline.value,
}))
const treeShakedMinified = computed(() => treeShake(treeShakedOnly.value.output, {
  preset: 'disabled',
  minify: true,
  alwaysInlineLiteral: false,
}))

const result = computed(() => {
  return {
    diagnostics: treeShakedOnly.value.diagnostics,
    output: doMinify.value ? treeShakedMinified.value.output : treeShakedOnly.value.output,
  }
})
export const copyOutput = computed(() => (doMinify.value ? minifiedOnly.value.output : copyOnly.value.output).trim() || `// Empty output or error`)
export const output = computed(() => result.value.output.trim() || `// Empty output or error`)
export const onlyMinifiedSize = computed(() => minifiedOnly.value.output.length)
export const treeShakedUnminifiedSize = computed(() => treeShakedOnly.value.output.length)
export const treeShakedMinifiedSize = computed(() => treeShakedMinified.value.output.length)
export const treeShakeRate = computed(() => 100 * treeShakedMinifiedSize.value / onlyMinifiedSize.value);
export const diagnostics = computed(() => {
  hideDiagnostics.value = false
  return result.value.diagnostics
})
export const hideDiagnostics = ref(false)
