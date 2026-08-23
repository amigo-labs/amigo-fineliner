// ESLint runs WITHOUT type-aware rules, deliberately (ADR-020): the type pass is
// svelte-check's job and it already runs in CI, so `tseslint.configs.recommended`
// is used rather than `recommendedTypeChecked`. The payoff is that linting needs
// no tsconfig program, no Rust toolchain and no wasm-pack build — the CI lint job
// is a bare `pnpm install` away from running.

import js from '@eslint/js';
import globals from 'globals';
import tseslint from 'typescript-eslint';
import svelte from 'eslint-plugin-svelte';
import prettier from 'eslint-config-prettier';

export default tseslint.config(
  {
    // Generated output. src/lib/core/generated/ is committed and CI fails on a
    // dirty diff (ADR-014); src/lib/wasm/pkg/ is wasm-pack output.
    ignores: ['src/lib/core/generated/', 'src/lib/wasm/pkg/', 'dist/'],
  },
  js.configs.recommended,
  tseslint.configs.recommended,
  svelte.configs.recommended,

  // eslint-config-prettier last among the rule sets: it switches off every
  // stylistic rule Prettier already decides, so the two tools cannot disagree.
  prettier,
  svelte.configs.prettier,

  {
    languageOptions: {
      globals: { ...globals.browser },
    },
  },
  {
    // Svelte files carry TypeScript in <script lang="ts">, so the Svelte parser
    // needs the TS parser for the script blocks.
    files: ['**/*.svelte', '**/*.svelte.ts'],
    languageOptions: {
      parserOptions: { parser: tseslint.parser },
    },
  },
  {
    // Config files run in Node, not the browser.
    files: ['*.config.js', '*.config.ts'],
    languageOptions: { globals: { ...globals.node } },
  },
);
