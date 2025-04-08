<script setup lang="ts">
import Editor from './Editor.vue';
import { alwaysInline, debouncedInput, diagnostics, doMinify, hideDiagnostics, input, load, onlyMinifiedSize, output, preset, treeShakedMinifiedSize, treeShakedUnminifiedSize, treeShakeRate, diffCode } from './states';
import Terser from './third/Terser.vue';
</script>

<template>
  <div py-2 md:py-4 fixed inset-0 flex flex-col>
    <div px-4 flex flex-wrap md:flex-nowrap gap-x-2 pb-2>
      <h1 text-xl md:text-3xl font-bold md:pb-2 select-none flex flex-wrap items-center gap-x-2>
        <img src="/favicon.ico" h-1em bg-gray-200 rounded-lg>
        <div @click="load(true)">
          Tree Shaker
        </div>
        <div text-sm self-end flex items-center gap-1 op-80>
          Experimental Tree Shaker for JS Based on Oxc
          <a i-carbon-logo-github flex-grow inline-block w-1.2em h-1.2em hover:op-80
            href="https://github.com/KermanX/tree-shaker" target="_blank" />
        </div>
      </h1>
      <div flex-grow md:w-0 />
      <div flex w-fit md:flex-col h-min md:h-0 z-10 gap-x-4 font-mono items-end mr-2 mt-1 md:mt--2 leading-5>
        <label flex align-center gap-1 select-none>
          <span op-80>
            Preset:
          </span>
          <select v-model="preset" class="w-26 text-sm black bg-transparent mb--.2">
            <option value="smallest">Smallest</option>
            <option value="recommended">Recommended</option>
            <option value="safest">Safest</option>
            <option value="disabled">Disabled</option>
          </select>
        </label>
        <label flex align-center gap-1 select-none>
          <span op-80>
            Always inline:
          </span>
          <input v-model="alwaysInline" type="checkbox">
        </label>
        <label flex align-center gap-1 select-none>
          <span op-80>
            Minify by Oxc:
          </span>
          <input v-model="doMinify" type="checkbox">
        </label>
      </div>
    </div>
    <div flex-grow h-0 flex flex-col md:flex-row gap-x-2 gap-y-2>
      <div flex-grow h-0 md:h-full md:w-0 flex flex-col>
        <div flex items-center>
          <h2 md:text-xl pb-2 pl-4 select-none>
            Input
            <span text-sm op80 font-mono>
              (Raw: {{ debouncedInput.length }}B, Minified: {{ onlyMinifiedSize }}B)
            </span>
          </h2>
        </div>
        <Editor v-model="input" lang="javascript" class="flex-grow h-0 max-h-full" />
      </div>
      <div flex-grow h-0 md:h-full md:w-0 flex flex-col>
        <h2 md:text-xl pb-2 pl-4 select-none flex items-center>
          Output
          <span text-sm font-mono self-end ml-2 mb--1>
            <span op80>(Raw: {{ treeShakedUnminifiedSize }}B,
              Minified: {{ treeShakedMinifiedSize }}B, </span>
            <math display="inline">
              <mfrac>
                <mi>Output Minified</mi>
                <mi>Input Minified</mi>
              </mfrac>
            </math>={{ treeShakeRate.toFixed(2) }}%<span op80>)</span>
          </span>
          <div flex-grow />
        </h2>
        <div flex-grow relative max-h-full flex flex-col>
          <Editor v-model="output" lang="javascript" readonly :diff="diffCode != null" w-full flex-grow :key="String(diffCode != null)" />
          <div bg-dark-600 w-full p-2 flex flex-col>
            <Terser />
          </div>
          <div z-20 absolute left-1 right-2 bottom--2 children:p-2 children:px-3 children:b-2 children:rounded flex
            flex-col gap-2>
            <div v-if="diagnostics.length" v-show="!hideDiagnostics" relative bg-op-80
              :class="diagnostics.isError ? 'text-red-200 bg-red-900 b-red-500' : 'text-yellow-200 bg-yellow-900 b-yellow-500'">
              <h3 text-lg pb-1>
                {{ diagnostics.isError ? 'Error' : 'Warning' }}
              </h3>
              <div font-mono max-h-8em overflow-y-auto>
                <p v-for="d, i in diagnostics" :key="i" style="text-indent: -1em" ml-1em>
                  {{ d }}
                </p>
              </div>
              <button absolute right-3 top-3 w-6 h-6 b-none i-carbon-close @click="hideDiagnostics = true" />
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
