<script lang="ts">
import { ref, shallowRef, watch } from 'vue'
import { debouncedInput, format } from '../states'

const isAdvanced = ref(false);
const output = ref<string>("");
const errors = shallowRef<string[]>([]);

watch([debouncedInput, isAdvanced], async (input, isAdvanced) => {
  const response = await fetch("https://jscompressor.treblereel.dev/compile", {
    "headers": {
      "accept": "*/*",
      "accept-language": "zh-CN,zh;q=0.9,en;q=0.8,en-GB;q=0.7,en-US;q=0.6",
      "content-type": "application/json",
      "priority": "u=1, i",
      "sec-ch-ua": "\"Microsoft Edge\";v=\"135\", \"Not-A.Brand\";v=\"8\", \"Chromium\";v=\"135\"",
      "sec-ch-ua-mobile": "?0",
      "sec-ch-ua-platform": "\"Windows\"",
      "sec-fetch-dest": "empty",
      "sec-fetch-mode": "cors",
      "sec-fetch-site": "same-origin"
    },
    "body": JSON.stringify({
      "payload": input,
      "compilationLevel": isAdvanced ? "Advanced" : "Whitespace only",
      "warningLevel": "DEFAULT",
      "outputFileName": "default.js",
      "formatting": {
        "prettyPrint": false,
        "printInputDelimiter": false
      },
      "externalScripts": {
        "urls": []
      }
    }),
    "method": "POST",
    "mode": "cors",
    "credentials": "omit"
  });
  const data = await response.json();
  errors.value = data.errors;
  if (data.compiledCode) {
    output.value = format(data.compiledCode);
  } else {
    output.value = "";
  }
}, { immediate: true });
</script>

<template>
  <div flex>
    Closure Compiler
    <div flex-grow />
    <Status :third="cc.output.value" />
  </div>
</template>