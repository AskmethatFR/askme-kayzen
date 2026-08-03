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
pause(id)       → habit.paused = true ;  screen = Today
resume(id)      → habit.paused = false
add(name, icon) → habits.push(Habit { steps: vec![1], .. }) ; screen = Today
week()          → screen = Week   // tout le récap est calculé, jamais stocké
ritual(id)      → screen = Timer ; minuteur 60 s ;  à la fin → done_today = true
anchor(id)      → habit.anchored = true  ; screen = Today  // archive + libère une place
unanchor(id)    → habit.anchored = false           // la remettre dans le quotidien
set_trigger(id, texte) → habit.trigger = Some(texte)
```

## Règles de parallélisme

- Au plus **5 habitudes actives** (`active = !paused && !anchored`).
- L'écran *Ajouter* se bloque au-delà et invite à en ancrer une.
- La liste du jour est **triée par déclencheur** : sans notification, `trigger` sert
  d'*intention* qui épouse la journée plutôt que de rappel push.
