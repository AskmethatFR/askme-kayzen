# 6 · Style graphique — Galets

> **Remplace [`05-style-graphique.md`](05-style-graphique.md) (« Broadsheet »)** depuis
> l'issue #55 (2026-09). Les deux règles d'accent tranchées sous Broadsheet (#30, #32) sont
> reprises ici telles quelles : elles ne dépendaient pas du système visuel.

Le design **Galets** : un papier sable chaud, un titrage en **Fraunces** sur un texte courant
en **Figtree**, et des **formes de galet** — légèrement irrégulières — pour tout ce que le
pouce touche. Le cyan d'imprimerie reste l'accent, en petites touches délibérées. Le rituel
passe en **fond sombre**, pour se distinguer du reste de l'app comme un moment à part.

Seul le visuel a changé : aucun geste, aucun écran, aucun texte.

## Couleurs

Toutes les couleurs sont des variables `var(--color-*)` dans `:root` (`app/assets/main.css`).
Ne jamais coder un hex en dur.

| Rôle | Variable | Hex | Usage |
| --- | --- | --- | --- |
| Papier | `--color-paper` | `#F4F1EA` | fond des écrans |
| Carte | `--color-card` | `#FBF9F4` | surfaces posées sur le papier |
| Encre | `--color-ink` | `#2A2622` | texte |
| Encre douce | `--color-ink-soft` | `#6B645C` | texte secondaire |
| Filet | `--color-rule` | `#DCD5C8` | bordures discrètes |
| Accent | `--color-accent` | `#0088b0` | aplats : l'interactif, le fait d'avoir pratiqué |
| Accent texte | `--color-accent-text` | `#00708F` | texte en accent, boutons qui portent du texte |
| Accent survol | `--color-accent-hover` | `#005A73` | état survolé / pressé |
| Teinte | `--color-tint` / `--color-on-tint` | `#DDEEF3` / `#00506A` | fonds teintés et leur texte |
| Ancrée | `--color-anchored` / `--color-anchored-tint` | `#7A5E24` / `#EFE3C8` | habitudes acquises (ocre) |

**Rituel (scopé par `.screen.ritual`)** : fond `#12303A`, piste de l'anneau `#1C4552`,
progression et souffle `#2E6B7E`, texte `#F4F1EA` / `#A9BEC4`. Le fond sombre s'étend à
`html` et `body` via `:has(.screen.ritual)`, pour qu'aucune bande claire n'apparaisse au
défilement.

**Règle — pourquoi deux cyans :** `#0088b0` n'atteint que **3,6:1** sur le papier. Il reste
l'accent des aplats (cibles, barres, points), où 3:1 suffit à un élément graphique ; tout
texte en accent prend `#00708F` (**5,0:1**), qui passe AA.

**Règle — l'accent ne dit qu'une chose (issue #30, reprise de Broadsheet) :** en dehors de
l'interactif, le cyan signifie **« pratiqué »**, et rien d'autre. Avant de peindre en accent :
*est-ce que cet élément affirme que l'utilisateur a pratiqué ?* — non ⇒ tonalité neutre.

**Règle — la couleur ne porte jamais un signal toute seule (issue #32, reprise de
Broadsheet) :** tout signal chromatique se double d'un indice de forme — présence ou absence
d'un élément, jamais deux nuances du même. La cible « fait » garde son tampon
(`::before`/`::after`) en plus de sa couleur. Les points de rythme et l'escalier du détail
gardent le défaut signalé sous Broadsheet ; il attend toujours sa passe d'accessibilité.

### Contrastes mesurés

| Paire | Ratio |
| --- | --- |
| Encre / papier | 13,3:1 |
| Encre douce / papier | 5,2:1 |
| Accent texte / papier | 5,0:1 |
| Carte sur accent texte (bouton) | 5,4:1 |
| Texte sur teinte | 7,5:1 |
| Ocre sur teinte ocre | 4,8:1 |
| Texte rituel / fond rituel | 12,3:1 (doux : 7,2:1) |
| Progression de l'anneau / fond rituel | 2,3:1 — **décoratif, assumé** |

L'anneau du rituel ne porte aucune information que le compte à rebours ne donne pas déjà en
chiffres : il est décoratif, et reste à la couleur de la maquette.

## Typographie

- **Fraunces** (`--font-display`) pour les titres : salutation, grands chiffres du récap,
  titre de la semaine.
- **Figtree** (`--font-body`) pour tout le reste : texte courant, boutons, champs.
- Les deux polices sont **embarquées** (`app/assets/fonts/`, variables, licence OFL) et
  préchargées par `app/src/main.rs` — l'app n'appelle plus Google Fonts et s'affiche
  identique hors ligne, Android compris. La liste `BUNDLED_FONTS` y est la source unique :
  un test vérifie que chaque `@font-face` de `main.css` désigne une police de cette liste.
- L'italique garde son rôle émotionnel (encouragements, synthèses).

## Formes

| Variable | Valeur | Usage |
| --- | --- | --- |
| `--radius-pebble` | `48% 52% 46% 54% / 55% 47% 53% 45%` | cible, points de rythme, ajout, souffle du rituel |
| `--radius-pill` | `30px` | boutons |
| `--radius-field` | `22px` | champs de saisie |
| `--radius-bar` | `4px` | sommet des barres de courbe |

Les ombres restent douces (`--shadow-sm` / `--shadow-md`), dérivées de l'encre.

## Ce qui ne change pas

Animations (`kzStamp`, `kzRing`, `kzUp`, `kzBreathe`, `kzTick` — ADR-0015), icônes et motifs
d'interaction décrits dans [`05-style-graphique.md`](05-style-graphique.md), sauf là où ce
document les contredit.

## Aperçu des écrans

Captures de l'app (web, largeur mobile).

### Aujourd'hui
![Écran Aujourd'hui](images/01-aujourdhui.png)

### Détail d'une habitude
![Écran Détail](images/02-detail.png)

### Rituel
![Écran Rituel](images/03-rituel.png)

### Cette semaine
![Écran Cette semaine](images/04-semaine.png)

### Ancrées (acquises)
![Écran Ancrées](images/05-ancrees.png)

### Ajouter
![Écran Ajouter](images/06-ajouter.png)
