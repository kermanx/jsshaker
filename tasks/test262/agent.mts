import 'dotenv/config'
import { generateObject } from '@xsai/generate-object'
import { shakeSingleModule } from 'jsshaker'
import { appendFile, readFile } from 'node:fs/promises'
import * as v from 'valibot'
import { diffLines } from 'diff'

function generateDiff(original: string, optimized: string): string {
  const changes = diffLines(original, optimized)
  const lines: string[] = []

  for (const change of changes) {
    const prefix = change.added ? '+' : change.removed ? '-' : ' '
    const text = change.value.replace(/\n$/, '')
    for (const line of text.split('\n')) {
      lines.push(prefix + ' ' + line)
    }
  }

  return lines.join('\n')
}

async function readLines(path: string): Promise<Set<string>> {
  try {
    const content = await readFile(path, 'utf-8')
    return new Set(content.split('\n').map(l => l.split('\t')[0].trim()).filter(l => l))
  } catch {
    return new Set()
  }
}

const SYSTEM_PROMPT = `You are an expert JavaScript analyzer for a code optimizer validation system.

Your task: Given ORIGINAL and OPTIMIZED JavaScript code, determine if the optimization preserves the original semantics.

IMPORTANT: The optimizer is NOT designed to handle the following features. If the original code relies on any of these, the optimization failure is EXPECTED and should be marked as "illegal_testcase":
- Modifying prototype chains or built-in objects, including adding/removing properties on Object.prototype, Function.prototype, JSON, etc.
- Using valueOf/toString with side effects
- Using eval, Function constructor, or other dynamic code execution
- Relying on property enumeration order
- Relying on function.toString() output or function.name, function.caller
- Special functions like \`$DONE\` is defined but never called
- **Relying on exception throwing behavior or try-catch side effects**: The original code intentionally depends on exceptions being thrown or caught for its logic. For example, assert.throws is used to verify that a function throws an error, but the optimized code no longer throws that error. This is a valid reason for optimization failure and should be marked as "illegal_testcase".

Respond with one of:
- "illegal_testcase": The original code uses features not supported by the optimizer
- "definitely_correct": The optimization is semantics-preserving
- "uncertain": Cannot determine with confidence
- "incorrect_optimization": The optimization changes observable behavior, even if it doesn't use illegal features

Return "reason" in Chinese, max 30 characters, should be informative.`

const ResponseSchema = v.object({
  reason: v.string(),
  verdict: v.picklist(['illegal_testcase', 'uncertain', 'incorrect_optimization',]),
})

const TIMEOUT_MS = 5_000
const SKIP_LLM = process.env.SKIP_LLM === '1' || process.argv.includes('--skip-llm')

async function worker(path: string): Promise<boolean> {
  const original = await readFile('./test262/test/' + path, 'utf-8')

  const normalized = shakeSingleModule(original, {
    preset: 'disabled',
    minify: false,
  }).output.code

  let optimized: string
  try {
    optimized = shakeSingleModule(original, {
      preset: 'safest',
      minify: false,
    }).output.code
  } catch (e) {
    console.error('Error during shaking:', e)
    return true // Success - skip this file
  }

  if (SKIP_LLM) {
    const entry = `## ${path}\n\n**Verdict**: incorrect_optimization\n**Reason**: (skipped LLM)\n\n### Original\n\n\`\`\`javascript\n${normalized}\n\`\`\`\n\n### Optimized\n\n\`\`\`javascript\n${optimized}\n\`\`\`\n\n---\n\n`
    await appendFile('./agent-declined.md', entry, 'utf-8')
    await appendFile('./agent-processed.txt', path + '\n', 'utf-8')
    console.log(`[DECLINE] ${path} - incorrect_optimization: (skipped LLM)`)
    return true
  }

  const controller = new AbortController()
  const timeout = setTimeout(() => controller.abort(), TIMEOUT_MS)

  try {
    const { object } = await generateObject({
      apiKey: process.env.LLM_API_KEY!,
      baseURL: process.env.LLM_BASE_URL!,
      messages: [
        {
          content: SYSTEM_PROMPT,
          role: 'system',
        },
        {
          content: `## ORIGINAL CODE:\n\`\`\`javascript\n${original}\n\`\`\`\n\n## OPTIMIZED CODE:\n\`\`\`javascript\n${optimized}\n\`\`\`\n\n## DIFF (- removed, + added):\n\`\`\`diff\n${generateDiff(normalized, optimized)}\n\`\`\``,
          role: 'user',
        },
      ],
      model: process.env.LLM_MODEL!,
      schema: ResponseSchema,
      abortSignal: controller.signal,
    })

    clearTimeout(timeout)

    const { verdict, reason } = object

    if (verdict === 'illegal_testcase') {
      await appendFile('./agent-accept.txt', path + '\t|\t' + reason + '\n', 'utf-8')
      console.log(`[ACCEPT] ${path} - ${verdict}: ${reason}`)
    } else {
      const entry = `## ${path}\n\n**Verdict**: ${verdict}\n**Reason**: ${reason}\n\n### Original\n\n\`\`\`javascript\n${normalized}\n\`\`\`\n\n### Optimized\n\n\`\`\`javascript\n${optimized}\n\`\`\`\n\n---\n\n`
      await appendFile('./agent-declined.md', entry, 'utf-8')
      console.log(`[DECLINE] ${path} - ${verdict}: ${reason}`)
    }

    await appendFile('./agent-processed.txt', path + '\n', 'utf-8')
    return true // Success
  } catch (e) {
    clearTimeout(timeout)
    if (e instanceof Error && e.name === 'AbortError') {
      console.log(`[TIMEOUT] ${path} - skipped, will retry on next run`)
    } else {
      console.error(`[ERROR] ${path}:`, e)
    }
    // Do not write to agent-processed.txt so it will be retried
    return false // Failed - needs retry
  }
}

const CONCURRENCY = 24

async function main() {
  const failed = (await readFile('./last-failed-list', 'utf-8')).split('\n').map(l => l.trim()).filter(l => l)
  const processed = await readLines('./agent-processed.txt')

  const pending = failed.filter(path => !processed.has(path))
  console.log(`Total: ${failed.length}, Processed: ${processed.size}, Pending: ${pending.length}`)

  if (pending.length === 0) {
    return false
  }

  let index = 0
  let completed = 0
  let hasError = false

  async function runNext(): Promise<void> {
    while (index < pending.length) {
      const path = pending[index++]
      console.log(`[${completed}/${pending.length}] Processing: ${path}`)
      const success = await worker(path)
      if (!success) {
        hasError = true
      }
      completed++
    }
  }

  await Promise.all(Array.from({ length: CONCURRENCY }, () => runNext()))
  return hasError
}

while (true) {
  const hasError = await main()
  if (!hasError) {
    console.log('Done!')
    break
  }
  console.log('Some tasks failed, retrying...')
}
