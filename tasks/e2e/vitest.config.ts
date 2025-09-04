import { defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    include: [
      '**\/*.{test,spec}.?(c|m)[jt]s?(x)',
      '**\/{test,spec}.?(c|m)[jt]s?(x)',
    ],
    browser: {
      enabled: true,
      name: 'chromium',
      provider: 'playwright',
      providerOptions: {},
    },
  }
})
