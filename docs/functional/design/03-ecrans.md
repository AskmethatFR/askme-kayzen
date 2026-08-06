# 3 · Les écrans

L'app tient en **six écrans**, un seul visible à la fois (`App.screen`).

## Aujourd'hui — l'écran d'accueil

Masthead (date + « Kaizen »), une phrase d'encouragement en italique, puis la liste des
habitudes actives (**3 max visibles, 5 max au total**). Chaque ligne : icône, nom,
« [déclencheur] · N min », et une grosse cible ronde.

La liste est **triée par déclencheur** pour épouser le fil de la journée.

**Gestes**
- Taper la cible → validé (l'encre se remplit + petit éclat cyan/magenta).
- Taper la ligne → détail.
- « + Ajouter une toute petite habitude » en bas.
- Liens : « Voir comment je grandis · cette semaine » ; « Mes habitudes ancrées · N »
  (si au moins une ancrée).
- Pied de page : « X sur Y · c'est déjà quelque chose ».
- Zone « En pause · aucune pression » sous la liste (habitudes en pause, reprenables d'un tap).

## Cette semaine — l'amélioration continue

Le cœur « Kaizen » : montrer qu'on grandit, jamais culpabiliser.
- Un **grand chiffre** = minutes gagnées depuis les débuts (somme de `current − steps[0]`).
- Par habitude : l'évolution « 2 → 4 min » avec une mini-courbe (une barre par étape).
- Le **rythme des 7 derniers jours** en points neutres.
- **Réflexion hebdo** (hansei) : une question douce + réponse d'un mot… ou rien.
- **Le mot de la semaine** : une phrase courte, non chiffrée.

*Ton :* une habitude sans progrès lit « un début parfait », jamais « 0 ».

## Détail — une habitude

- Titre + résumé (« passée de 2 à 4 min, tout doucement »).
- **Déclencheur** : champ texte libre + chips « après une habitude ancrée » (habit stacking).
- **Escalier de pratique** — une barre par jour sur les **7 derniers jours**. Le jour fait
  est plein, à la hauteur de l'objectif actif ce jour-là ; le jour non fait est **la même
  barre en opacité faible** — jamais un trou, jamais du rouge. On y lit deux choses à la
  fois : si on continue (la suite des barres) et si l'effort monte, descend ou tient (leur
  profil). *(Amendé 2026-07-27 par l'owner — voir `[[lifecycle-backlog]]` slice 3b. Remplace
  « une barre par étape de `steps` » : l'escalier dessine la pratique, pas l'intention.)*
- ~~**Calendrier** en points, sans chiffres.~~ **Supprimé du détail** (2026-07-27) — l'escalier
  de pratique porte déjà fait/pas-fait par jour, plus la hauteur d'effort que les points
  n'avaient pas. Deux dessins pour la même information contredisent la sobriété de l'écran.
- Bouton **« Faire ma minute »** (ouvre le rituel).
- Zone **« Ajuster, à votre rythme »** : « Passer à N+1 min » (grandir) et « Alléger à N−1 min ».
- **Ancrer** — à l'initiative de l'utilisateur, quand il la sent acquise (aucune détection de stabilité, aucune suggestion) → la marque acquise, libère une place.
- **« Mettre en pause, sans culpabilité »**.

## Rituel d'une minute (Timer)

Minuteur doux de 60 s : anneau de progression, cercle qui « respire », décompte `m:ss`.
- « J'ai terminé » → valide l'habitude et revient à l'accueil.
- « Arrêter, ce n'est pas grave » / « Fermer » → revient au détail, sans validation.

## Ajouter — une nouvelle petite habitude

- Quelques idées prêtes (boire un verre d'eau, ranger un objet, écrire une ligne, s'étirer),
  toutes à 1 min. Taper une idée → ajoutée.
- Sinon un champ libre + « Ajouter, à 1 min par jour ».
- **Si 5 habitudes actives** : l'écran se bloque en douceur et invite à en ancrer une pour
  libérer une place.
- *Règle :* aucune configuration d'objectif. On commence toujours tout petit.

## Ancrées — les acquises (archive)

Les habitudes ancrées, hors quotidien. Icône **sceau magenta** (pas de texte).
- Suivi léger : les 7 derniers jours en points.
- « La remettre dans mon quotidien » (désarchive, si une place est libre).
- Pied : « Vous suivez N / 5 habitudes en parallèle. »

## Actions & transitions

```text
toggle(id)      → habit.done_today = !done_today
open(id)        → screen = Detail(id)
back()          → screen = Today
increase(id)    → steps.push(current + 1)
lighten(id)     → steps.push(max(1, current - 1))   // muda : enlever la friction
pause(id)       → habit.paused = true ;  screen = Détail (AMENDÉ — voir ci-dessous)
resume(id)      → habit.paused = false
add(name, icon) → habits.push(Habit { steps: vec![1], .. }) ; screen = Today
week()          → screen = Week   // tout le récap est calculé, jamais stocké
ritual(id)      → screen = Timer ; minuteur 60 s ;  à la fin → done_today = true
anchor(id)      → habit.anchored = true  ; screen = Today  // archive + libère une place
unanchor(id)    → habit.anchored = false           // la remettre dans le quotidien
set_trigger(id, texte) → habit.trigger = Some(texte)
```

## Règles de parallélisme

- Au plus **5 habitudes actives** — ~~`active = !paused && !anchored`~~ **AMENDÉ, voir ci-dessous** :
  le plafond compte les habitudes **non ancrées**. Une habitude en pause garde sa place.
- L'écran *Ajouter* se bloque au-delà et invite à en ancrer une.
- La liste du jour est **triée par déclencheur** : sans notification, `trigger` sert
  d'*intention* qui épouse la journée plutôt que de rappel push.

## Amendements — pause et reprise, livrés en slice 5 (2026-08-06)

Ce nœud est le dessin d'origine du designer. Deux de ses lignes ont été tranchées
autrement depuis, et le code suit les décisions ci-dessous, pas la lettre au-dessus.

**Le plafond compte les non-ancrées, pas les non-pausées.** La formule
`active = !paused && !anchored` était la lecture littérale ; Q1 du
`[[lifecycle-backlog]]` l'a amendée dès l'approbation du modèle de cycle de vie :
**une habitude en pause garde son siège**. Sans quoi reprendre pourrait échouer — on
mettrait une habitude au repos et on découvrirait, en voulant la reprendre, que la
place a été donnée à une autre. Un produit *sans culpabilité* ne pose pas ce piège.
La sixième demande reste donc refusée tant qu'une habitude en pause occupe une place.
Épinglé par `[[pause-resume]]` S3.

**Mettre en pause ne renvoie pas à Aujourd'hui.** Le dessin d'origine disait
`pause(id) → screen = Today`. Il précède la décision du propriétaire de faire du
détail d'une habitude en pause un **écran de repos** à part entière : son escalier de
pratique, et « La reprendre ». Renvoyer à Aujourd'hui cacherait l'écran qu'on vient
de dessiner et éloignerait l'utilisateur de son geste d'annulation. L'écran se relit
donc sur place. Épinglé par `[[pause-resume]]` S4.

**Ce que le détail d'une habitude en pause n'offre plus.** Ni « Faire ma minute », ni
« Passer à N+1 min », ni « Alléger à N−1 min ». Une pause est un vrai repos : rien à
pratiquer, rien à ajuster. Le domaine, lui, n'interdit rien — c'est l'écran qui cesse
de proposer, jamais la règle qui se met à refuser (même logique que Q3 pour *marquer
comme fait*).

**L'icône `ph-pause`** (`[[design-style-graphique]]`) n'est pas honorée : l'application
n'embarque aucune police d'icônes, et en câbler une pour un seul bouton serait une
dépendance hors sujet. Le bouton porte son texte, comme « Passer à N min ».
