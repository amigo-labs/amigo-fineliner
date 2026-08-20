# NEXT-PROMPTS — amigo-fineliner

Prompts zum Weiterarbeiten in Claude Code. Stand: 20.08.2026.

## Stand

Das Repo ist funktional fertig: M1–M12 durch, `fineliner-effects` und
Adjustments geliefert, 282 Tests, Release-Automation und Cloudflare-Deploy
live seit Juli 2026. Es steht seit dem 20.08.2026 auf **Maintenance-Hold** —
kein Feature-Work, Dependabot bleibt an.

Der Hold ist **kein Archiv**. Die aktuelle Überlegung ist, dass Fineliner ein
Modus einer gemeinsamen App wird (siehe `NEXT-PROMPTS.md` in amigo-pincel).
Alles hier ist entweder Schuldenabbau oder Vorbereitung darauf.

---

## Erledigt (PR „Chore/maintenance hold followups")

Die ersten drei Prompts dieser Liste sind abgearbeitet und committet:

1. **Spec-Divergenz aufgelöst.** `docs/specs/fineliner.md` §13.2 beschreibt
   WebP jetzt als lossless-only, §17 hat das `quality`-Argument von
   `export_webp` verloren, DL-008 ist der Spec-seitige Eintrag. ADR-018 in
   CLAUDE.md ist die Entscheidung, ADR-007 damit endgültig geschlossen.
2. **Die zwei offenen Entscheidungen sind entschieden.** Arrow-Shape → Phase 2,
   kein Non-Goal (ADR-019). ESLint + Prettier → genehmigt und gelandet
   (ADR-020): acht Dev-Dependencies, `pnpm lint` ist `eslint .`, `pnpm format` /
   `pnpm format:check` sind neu, `ui/src` ist Prettier-formatiert bei
   printWidth 110, und der CI-UI-Job führt `pnpm lint` + `pnpm format:check`
   vor `pnpm check` aus. STATUS.md „Open questions" ist damit leer.
3. **Zeilenenden.** `.gitattributes` pinnt `* text=auto eol=lf` repo-weit,
   `*.sh` explizit; die Konvention steht in CLAUDE.md §6.4.

---

## Offene Punkte, die keine Prompts sind

Zwei Dinge kann Claude Code nicht erledigen:

1. **Cloudflare-Dashboard.** Die Workers-Builds-Git-Integration baut
   PR-Branches als „production". Die Einschränkung auf `main` ist eine
   Dashboard-Einstellung, nicht in `wrangler.jsonc` ausdrückbar — das steht in
   STATUS.md als Human-Action-Item. Einmal klicken.
2. **Der visuelle Durchlauf.** STATUS.md vermerkt an mehreren Milestones „not
   yet done by a human: visual browser run" — unter anderem für das
   Adjustments-Menü aus M12. Einmal `pnpm dev`, Bild öffnen, durchklicken.

---

## 1 — Was diese Hülle in eine gemeinsame App einbringt

Der einzige noch offene Prompt. Die Gegenseite zum Design-Prompt in
amigo-pincel. Sinnvoll erst, wenn der Modus-Entwurf dort steht — oder parallel,
wenn du beide Seiten gleichzeitig beurteilen willst.

```
Wir überlegen, dieses Projekt und amigo-pincel unter eine gemeinsame UI zu
legen, als eine App mit zwei Modi (Pixel / Paint). Die Rust-Cores bleiben
getrennt. Fineliner wäre der Paint-Modus.

Mach eine Inventur dieser Hülle, als Dokument, kein Code. Für jedes Modul in
ui/src/lib/: ist es modusneutral (gehört in die gemeinsame Hülle), oder
paint-spezifisch (gehört hinter den Modus)? Interessant sind besonders
Layer-Panel, Effekt-Dialoge und das Adjustments-Menü — die sind reicher als
alles auf der Pixel-Seite, und der Effekt-Dialog hat ein generisches
Feldsystem, das vielleicht modusneutral tragfähig ist.

Nimm die Release-Automation mit auf: .github/workflows/release.yml und der
Cloudflare-Pfad über scripts/cf-build.sh funktionieren hier und fehlen im
Schwesterprojekt vollständig. Beschreib, was davon eine gemeinsame App
übernehmen könnte und was am Repo-Layout hängt.

Nenn am Ende die drei Dinge, die dieses Repo hat und das andere nicht — und
die drei, die umgekehrt fehlen (fs-Adapter und IndexedDB-Substrat sind dort
gebaut, hier nicht).
```

---

## Nicht anfassen

Kein Feature-Work, solange der Hold gilt. 9D Free Transform ist ausdrücklich
Phase 2, die Arrow-Shape ebenfalls (ADR-019). Der grafische Curves-Editor, die
Channel-Mixer-UI und der Color-Balance-Layout-Feinschliff sind als
Phase-2-Nachzügler dokumentiert und über die WASM-API ohnehin erreichbar.
