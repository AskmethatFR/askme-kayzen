# 5 · Style graphique

Le design suit le système **Broadsheet** : du newsprint pour le web — Source Serif 4 noir
sur papier blanc, avec les accents d'imprimerie (cyan et magenta) en petites touches
délibérées, comme du spot color. Hiérarchie par l'échelle du serif et le blanc, **pas de
boîtes ni de filets**.

## Couleurs

| Rôle | Hex | Usage |
| --- | --- | --- |
| Fond papier | `#f3f2f2` | fond de tous les écrans |
| Encre | `#201e1d` | texte |
| Cyan (accent) | `#0088b0` | tout ce qui est interactif |
| Magenta (accent 2) | `#d6006c` | second spot, rare — l'ancrage, les acquises |

Chaque rôle a une rampe tonale 100–900 (générée en OKLCH). Utiliser les pas clairs (100–300)
pour les fonds teintés et bordures, 500 comme base, 700–900 pour le texte sur fond teinté.
Ne jamais coder un hex en dur : passer par les variables `var(--color-*)`.

**Règle :** ne pas utiliser les deux accents dans un même petit composant.

## Typographie

- **Source Serif 4** partout — titres et corps. Pas de sans-serif pour l'UI : le serif *est*
  le chrome.
- L'**italique vraie** (pas oblique synthétique) porte l'émotion : phrases d'encouragement,
  synthèses, mot de la semaine.
- Densité 1.25× et rayon 2px sont déjà dans les variables `--space-*` / `--radius-*`.

## Icônes

**Phosphor**, poids **duotone** partout. Icônes utilisées :

| Écran / élément | Icône |
| --- | --- |
| Lire | `ph-book-open` |
| Bouger un peu | `ph-person-simple-walk` |
| Respirer | `ph-wind` |
| Rituel de la minute | `ph-timer` |
| Grandir (+1 min) | `ph-plant` |
| Alléger (−1 min) | `ph-feather` |
| Ancrer / acquises | `ph-seal-check`, `ph-anchor-simple` |
| Récap semaine | `ph-chart-line-up` |
| Ajouter | `ph-plus-circle` |
| Validation | `ph-check` |
| Pause | `ph-pause` |
| Idées d'ajout | `ph-drop`, `ph-broom`, `ph-pencil-simple`, `ph-flower-lotus`, `ph-seedling` |

## Motifs d'interaction

- Espacer les sections par du **blanc**, jamais des filets ou des cartes.
- Validation d'une habitude : la cible ronde se remplit d'encre (`kzStamp`) avec deux
  anneaux qui se diffusent (`kzRing`) en cyan puis magenta — récompense visuelle satisfaisante.
- Transition d'écran douce : léger fondu + montée (`kzUp`).
- Rituel : cercle central qui « respire » lentement (`kzBreathe`, 8 s).
- Chaque élément interactif : `:hover` teinté et état pressé (un pas de rampe au-delà de la
  base), focus clavier `outline: 2px solid var(--color-accent)`.

## Composants Broadsheet utilisés

- `.btn` / `.btn-primary` / `.btn-secondary` / `.btn-block` — actions.
- `.tag` / `.tag-accent` / `.tag-accent-2` — petites étiquettes (ex. « +2 min », « ancrée »).
- `.field` + `.input` — champs de saisie (nom d'habitude, déclencheur, réflexion).
- Rampes `--color-*`, ombres `--shadow-sm/md/lg`.

## Animations (référence CSS)

```css
@keyframes kzStamp   { 0%{transform:scale(.35);opacity:0} 55%{transform:scale(1.12)} 100%{transform:scale(1);opacity:1} }
@keyframes kzRing    { 0%{transform:scale(.55);opacity:.55} 100%{transform:scale(2);opacity:0} }
@keyframes kzUp      { from{opacity:0;transform:translateY(8px)} to{opacity:1;transform:none} }
@keyframes kzBreathe { 0%,100%{transform:scale(.86);opacity:.7} 50%{transform:scale(1.08);opacity:1} }
```

En Rust/Dioxus, ces `@keyframes` se placent dans la feuille de style globale de l'app ;
les couleurs et l'anneau de progression du rituel se calculent côté logique (offset =
`circonférence × (1 − restant / total)`).

## Aperçu des écrans

Captures du prototype (référence visuelle 1:1 pour la réécriture).

### Aujourd'hui
![Écran Aujourd'hui](images/01-aujourdhui.png)

### Détail d'une habitude
![Écran Détail](images/02-detail.png)

### Rituel d'une minute
![Écran Rituel](images/03-rituel.png)

### Cette semaine
![Écran Cette semaine](images/04-semaine.png)

### Ancrées (acquises)
![Écran Ancrées](images/05-ancrees.png)

### Ajouter
![Écran Ajouter](images/06-ajouter.png)
