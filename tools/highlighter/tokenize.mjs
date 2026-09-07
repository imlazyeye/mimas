import { createHighlighterCore } from 'npm:@shikijs/core'
import { createJavaScriptRegexEngine } from 'npm:@shikijs/engine-javascript'
import { readFileSync } from 'node:fs'

const arg = (k, d) => { const i = process.argv.indexOf(k); return i > -1 ? process.argv[i + 1] : d }
const grammar = JSON.parse(readFileSync(arg('--grammar'), 'utf-8'))
const theme = arg('--theme', 'github-dark')
const sources = JSON.parse(await new Response(Deno.stdin.readable).text())

// Our only JS-side job is running the TextMate grammar over the code; shiki.mim
// turns these tokens into HTML or SVG. Fine-grained core + the JS regex engine
// skips the oniguruma WASM load, the slowest part of cold start.
const hl = await createHighlighterCore({
  themes: [import(`npm:@shikijs/themes/${theme}`)],
  langs: [grammar],
  engine: createJavaScriptRegexEngine(),
})

// Emit one entry per scope-run: { content, color, fontStyle, scopes }. The SVG
// path uses color/fontStyle, the HTML path uses scopes — one shape feeds both.
const out = sources.map(src => {
  const { tokens, bg, fg } = hl.codeToTokens(src.replace(/\n$/, ''), { lang: grammar.name, theme, includeExplanation: true })
  const lines = tokens.map(line => line.flatMap(tok =>
    (tok.explanation ?? [{ content: tok.content, scopes: [] }]).map(seg => ({
      content: seg.content,
      color: tok.color,
      fontStyle: tok.fontStyle,
      scopes: seg.scopes.map(s => s.scopeName),
    }))
  ))
  return { lines, bg, fg }
})
process.stdout.write(JSON.stringify(out))
