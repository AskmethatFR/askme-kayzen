# 3 · Les écrans

L'app tient en **six écrans**, un seul visible à la fois (`App.screen`).

## Aujourd'hui — l'écran d'accueil

Masthead (date + « Kaizen »), une phrase d'encouragement en italique, puis la liste des
habitudes actives (**3 max visibles, 5 max au total**). Chaque ligne : icône, nom,
« [déclencheur] · N min », et une grosse cible ronde.

La liste est **triée par déclencheur** pour épouser le fil de la journée.

**Gestes**
- Taper la cible → validé (la cible se remplit d'accent cyan + petit éclat cyan).
- Taper la ligne → détail.
- « + Ajouter une toute petite habitude » en bas.
- Liens : « Voir comment je grandis · cette semaine » ; « Mes habitudes ancrées · N »
  (si au moins une ancrée).
- Pied de page : « X sur Y · c'est déjà quelque chose ».
- Zone « En pause · aucune pression » sous la liste (habitudes en pause, reprenables d'un tap).

## Cette semaine — l'amélioration continue

Le cœur « Kaizen » : montrer qu'on grandit, jamais culpabiliser.
- Un **grand chiffre** = les minutes **vécues** : la somme, sur toutes les habitudes, de
  l'objectif en vigueur chaque jour pratiqué. *(Corrigé 2026-08-20 — issue #22. Remplace
  « minutes gagnées depuis les débuts (somme de `current − steps[0]`) » : un gain sur
  l'objectif de départ affiche « 0 » à qui pratique fidèlement sans jamais grandir, ce que
  le ton de l'écran interdit. Même règle que le récap du détail, voir `[[habit-stats]]`.)*
- Par habitude : l'évolution « 2 → 4 min » avec une mini-courbe, **une barre par étape
  d'objectif**. Les hauteurs sont **relatives à la ligne** : la plus grande étape de la
  ligne remplit la courbe, les autres suivent en proportion.
  *(Amendé 2026-08-22 — issue #32.)* La courbe porte désormais **deux canaux** : la
  **hauteur** dit la trajectoire d'objectif, la **couleur** dit si l'habitude a été
  pratiquée dans la fenêtre glissante. Elle disait auparavant « l'intention, pas la
  pratique » — vrai de la hauteur, faux depuis que la couleur parle. Une habitude
  pratiquée porte l'accent et un socle sous sa courbe ; une habitude non pratiquée garde
  la tonalité neutre et **ne reçoit rien de plus** : pas de compteur, pas de marque
  d'absence. Le socle existe parce que la teinte seule ne se voit pas en niveaux de gris
  ni en vision daltonienne.
  Une habitude 2 → 3 et une habitude 30 → 32 dessinent donc la même forme, et c'est voulu :
  la ligne annonce une progression, pas un volume d'effort. *(Tranché 2026-08-21 par
  l'owner — issue #22. Remplace des hauteurs absolues, qui débordaient de la courbe dès
  7 minutes d'objectif et rendaient trois barres identiques sur une habitude déjà grande.
  L'escalier du détail garde ses hauteurs absolues : lui profile l'effort, la comparaison
  y est le sujet.)*
- Le **rythme des 7 derniers jours** en points neutres : une fenêtre glissante finissant
  aujourd'hui, un point par jour du plus ancien au plus récent, allumé dès qu'**au moins
  une** habitude a été pratiquée ce jour-là. Jamais un trou — un jour sans pratique est le
  même point, en veille.
- **Le mot de la semaine** : une phrase courte, non chiffrée, dérivée du rythme.

*Ton :* une habitude sans progrès lit « un début parfait », jamais « 0 ».

*Lecture seule (tranché 2026-08-21 par l'owner — issue #23) :* cet écran est
**entièrement dérivé** — aucun champ, aucun bouton, rien à y stocker. La réflexion hebdo
(hansei) y figurait comme seul élément écrivant ; elle est **retirée du produit**, pas
déplacée ailleurs. Le récap informe, il ne recueille rien.

## Détail — une habitude

- Titre + résumé (« passée de 2 à 4 min, tout doucement »).
- **Déclencheur** : champ texte libre + chips « après une habitude ancrée » (habit stacking).
- **Escalier de pratique** — une barre par jour sur les **7 derniers jours**. Le jour fait
  est plein, le jour non fait est **la même barre en opacité faible** — jamais un trou,
  jamais du rouge. On y lit deux choses à la fois : si on continue (la suite des barres) et
  si l'effort monte, descend ou tient (leur profil). Chaque barre tient sa hauteur de
  l'objectif actif ce jour-là, **relativement au plus haut objectif de la fenêtre** : le jour
  le plus haut remplit l'escalier, les autres suivent en proportion. Le profil est donc
  intact — la normalisation préserve les rapports — mais deux semaines d'une même habitude,
  l'une à 5 min et l'autre à 30, dessinent le même escalier ; on n'en voit jamais qu'une à la
  fois. *(Amendé 2026-07-27 par l'owner — voir `[[lifecycle-backlog]]` slice 3b. Remplace
  « une barre par étape de `steps` » : l'escalier dessine la pratique, pas l'intention.
  Hauteurs relatives tranchées 2026-08-21 par l'owner : en absolu, une barre débordait de
  l'escalier dès 10 minutes d'objectif.)*
- ~~**Calendrier** en points, sans chiffres.~~ **Supprimé du détail** (2026-07-27) — l'escalier
  de pratique porte déjà fait/pas-fait par jour, plus la hauteur d'effort que les points
  n'avaient pas. Deux dessins pour la même information contredisent la sobriété de l'écran.
- Bouton **« Commencer ma pratique »** (ouvre le rituel) — le libellé n'annonce aucune
  durée : la dose vient de l'objectif de l'habitude, pas du bouton, et le rituel n'a
  jamais été 1 min pour toutes. *(Corrigé 2026-08-19 — issue #12. Ce que le rituel
  chronomètre reste ouvert, voir issue #13.)*
- Zone **« Ajuster, à votre rythme »** : « Passer à N+1 min » (grandir) et « Alléger à N−1 min ».
- **Ancrer** — à l'initiative de l'utilisateur, quand il la sent acquise (aucune détection de stabilité, aucune suggestion) → la marque acquise, libère une place.
- **« Mettre en pause, sans culpabilité »**.

## Rituel — la pratique (Timer)

Minuteur doux : anneau de progression, cercle qui « respire », décompte `m:ss`. **Il
chronomètre l'objectif de l'habitude**, pas une durée fixe — une habitude à 5 min ouvre
un rituel de 5 min. *(Tranché 2026-08-20 par l'owner — issue #13, question laissée
ouverte par #12. Remplace « minuteur doux de 60 s » : le rituel n'a jamais été une minute
pour toutes, et le libellé du bouton ayant cessé d'annoncer une durée, la dose ne pouvait
venir que de l'habitude.)*
- « J'ai terminé » → valide l'habitude et revient à l'accueil.
- « Arrêter, ce n'est pas grave » / « Fermer » → revient au détail, sans validation.
- **À zéro le minuteur s'arrête et attend.** Rien ne se valide tout seul : la validation
  est un geste, jamais une échéance. *(Tranché 2026-08-20 — issue #13.)*

*Ton :* le décompte se lit à l'écoulé, pas au tic-tac — un écran masqué qui revient
affiche le temps réellement passé, jamais un compteur figé.

## Ajouter — une nouvelle petite habitude

- Quelques idées prêtes (boire un verre d'eau, ranger un objet, écrire une ligne, s'étirer),
  toutes à 1 min. Taper une idée → ajoutée.
- Sinon un champ libre + « Ajouter, à 1 min par jour ».
- **Si 5 habitudes actives** : l'écran se bloque en douceur et invite à en ancrer une pour
  libérer une place.
- *Règle :* aucune configuration d'objectif. On commence toujours tout petit.

## Ancrées — les acquises (archive)

Les habitudes ancrées, hors quotidien.
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
ritual(id)      → screen = Timer ; minuteur = objectif de l'habitude (AMENDÉ — voir ci-dessous)
anchor(id)      → habit.anchored = true  ; screen = Détail (AMENDÉ — voir ci-dessous)  // libère le siège ET le titre, par suppression de l'entrée
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

**Ce que le détail d'une habitude en pause n'offre plus.** Ni « Commencer ma pratique », ni
« Passer à N+1 min », ni « Alléger à N−1 min ». Une pause est un vrai repos : rien à
pratiquer, rien à ajuster. Le domaine, lui, n'interdit rien — c'est l'écran qui cesse
de proposer, jamais la règle qui se met à refuser (même logique que Q3 pour *marquer
comme fait*).

**L'icône `ph-pause`** (`[[design-style-graphique]]`) n'est pas honorée : l'application
n'embarque aucune police d'icônes, et en câbler une pour un seul bouton serait une
dépendance hors sujet. Le bouton porte son texte, comme « Passer à N min ».

## Amendements — ancrage, livré en slice 6 (2026-08-11)

**Précision sur le plafond — la décision Q1 ne bouge pas.** L'amendement ci-dessus
disait *« le plafond compte les habitudes non ancrées »*, ce qui reste l'effet
observé. Mais le mécanisme qui le produit n'est pas un filtre de comptage : ancrer
une habitude **retire son entrée du board** (`HabitBoard::release`), ce qui libère
le siège *et* le titre du même geste. Une habitude en pause, elle, garde bel et bien
son entrée — Q1 tient exactement tel qu'approuvé, une habitude en pause garde sa
place. Ce qui change ici est seulement le mot pour l'ancrage : « compte les non
ancrées » était le raccourci fonctionnel, l'implémentation l'obtient en supprimant
l'entrée, pas en la filtrant à la lecture.

**Le rituel ne valide plus à l'échéance.** Le dessin d'origine disait
`ritual(id) → minuteur 60 s ; à la fin → done_today = true`. Les deux moitiés sont
tombées le 2026-08-20 (issue #13) : le minuteur porte l'objectif de l'habitude, et
atteindre zéro ne valide rien — il s'arrête et attend « J'ai terminé ». Valider à
l'échéance ferait de la pratique une épreuve à réussir, quand le geste #1 dit
« anti-échec par design » : on ne peut pas rater un rituel qu'on a choisi d'arrêter.

**Ancrer ne renvoie pas à Aujourd'hui non plus.** Même décision et même raison
que pour la mise en pause en slice 5 : le dessin d'origine disait
`anchor(id) → habit.anchored = true ; screen = Today`. Le détail se relit sur
place, en un écran « ancrée » sobre — titre, objectif, escalier de pratique,
**aucun geste**. Renvoyer à Aujourd'hui cacherait l'écran qu'on vient de dessiner.

**L'écran Ancrées ne livre que la liste et le compte.** Les points des 7 derniers
jours du dessin d'origine sont **différés** — aucun scénario ne les demande, et tant
qu'aucun écran n'offre de marquer fait une habitude ancrée, les points figeraient au
jour de l'ancrage et rejoueraient indéfiniment un historique pré-ancrage. Le pied
« Vous suivez N / 5 habitudes en parallèle » revient à la slice 7 *(livré 2026-08-18)*,
où le refus à board plein devient le sujet — et « La remettre dans mon quotidien »
(désarchive) est devenu un geste de cet écran, refusable si le quotidien est complet
ou si le titre a été repris.

**Copie approuvée :** bouton du détail « L'ancrer · elle est devenue naturelle » ;
bandeau ancrée « ancrée · {N} min » ; lien d'Aujourd'hui « Mes habitudes ancrées ·
{N} » (affiché seulement si N ≥ 1) ; écran Ancrées : les titres + « {N} · devenues
naturelles ».
