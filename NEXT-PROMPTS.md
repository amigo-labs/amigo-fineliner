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

Gerade gelandet, noch nicht committet: `STATUS.md`, `CLAUDE.md` (ADR-018),
`wrangler.jsonc`.

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

## 1 — Die Spec-Divergenz auflösen

Der wichtigste Punkt. ADR-018 lässt eine Spec-Anforderung dauerhaft fallen;
die Spec fordert sie weiter.

```
ADR-018 in CLAUDE.md hat die WebP-Frage als lossless-final geschlossen: die
No-System-Deps-Regel schlägt die Spec-Anforderung, weil der pure-Rust
`image`-Crate nur lossless encodiert. Spec §13.2 in docs/specs/fineliner.md
verlangt aber weiterhin lossy WebP mit Qualität 1–100, und die Spec ist laut
README die source of truth für Verhalten.

CLAUDE.md §2.1 verlangt, dass die Spec bei einer Design-Divergenz nachgezogen
wird, und §11 führt "Spec references updated if design shifted" in der
Definition of Done. Zieh die Spec nach: §13.2 so ändern, dass sie lossless
beschreibt, mit Verweis auf ADR-018 und auf die Bedingung, unter der das
revidiert würde (ein tragfähiger pure-Rust Lossy-Encoder). Danach das
Human-Action-Item in STATUS.md schließen.
```

## 2 — Die zwei offenen Entscheidungen

```
In STATUS.md stehen unter "Open questions" zwei Entscheidungen offen, die seit
Monaten liegen. Beide sind klein, beide blockieren nichts, beide sollten
entschieden statt weitergetragen werden.

1) Arrow-Shape (spec §9.2): nicht in CLAUDE.md's M10-Shape-Liste. Phase 1 ist
geschlossen, die Frage ist also, ob die Arrow in Phase 2 gehört oder als
Non-Goal festgehalten wird.

2) ESLint + Prettier für ui/: neue Dev-Dependencies, brauchen laut CLAUDE.md
§9 ausdrückliche Genehmigung. Prüf, ob sie faktisch schon in ui/package.json
stehen — falls ja, ist das eine nachzutragende Genehmigung und keine
Entscheidung mehr.

Arbeite beides begründet aus, schreib die Entscheidungen ins Decision Log bzw.
in die ADR-Liste, und räum die "Open questions" entsprechend auf. Wenn du eine
Frage nicht ohne mich entscheiden kannst, sag das statt zu raten.
```

## 3 — Zeilenenden

Gleiche Baustelle wie im Schwesterrepo.

```
Dieses Repo hat kein .gitattributes und core.autocrlf=false, während das
Working Tree CRLF ist und die committeten Blobs LF sind. Folge: `git status`
zeigt ~120 Dateien als geändert, die reines Zeilenende-Rauschen sind (gleich
viele Insertions wie Deletions pro Datei), und jeder Checkout auf einer
anderen Plattform erzeugt den Diff neu.

Leg ein .gitattributes an, das das dauerhaft löst, normalisiere den Index in
EINEM separaten Commit, der nichts anderes anfasst, und schreib in CLAUDE.md
fest, welche Konvention jetzt gilt.
```

## 4 — Was diese Hülle in eine gemeinsame App einbringt

Die Gegenseite zum Design-Prompt in amigo-pincel. Sinnvoll erst, wenn der
Modus-Entwurf dort steht — oder parallel, wenn du beide Seiten gleichzeitig
beurteilen willst.

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
Phase 2. Der grafische Curves-Editor, die Channel-Mixer-UI und der
Color-Balance-Layout-Feinschliff sind als Phase-2-Nachzügler dokumentiert und
über die WASM-API ohnehin erreichbar.
