// Prettier is configured to the style already in the tree (CLAUDE.md §6.2), not
// to Prettier's defaults: the point of adding it was to stop formatting drift,
// not to reformat 28 files into a different house style. Concretely that means
// single quotes, semicolons, 2-space indent and trailing commas, all of which
// the existing sources already use, and printWidth 110.
//
// 110 was measured, not picked: reformatting src/ costs +246/-52 lines at width
// 100 (Prettier explodes deliberately tabular data such as the Color Balance
// field table into one property per line), +109/-83 at 110, and +92/-133 at 120
// (wide enough that it starts joining line breaks the author chose). 110 is the
// width at which the formatter neither explodes nor collapses what is there,
// which is the width the code was actually written to — src/ line lengths run
// p90 = 75, p99 = 107.

/** @type {import('prettier').Config} */
export default {
  printWidth: 110,
  tabWidth: 2,
  useTabs: false,
  semi: true,
  singleQuote: true,
  trailingComma: 'all',
  plugins: ['prettier-plugin-svelte'],
  overrides: [{ files: '*.svelte', options: { parser: 'svelte' } }],
};
