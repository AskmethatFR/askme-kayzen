# 4 · Les six gestes Kaizen

1. **Le rituel de la minute** (kaizen 一分間) — « Commencer ma pratique » : un minuteur
   doux avec respiration, réglé sur **l'objectif de l'habitude**. Le principe n'est pas la
   durée, c'est de ne pas casser la chaîne intérieure. Anti-échec par design : à zéro le
   minuteur s'arrête et attend, il ne valide rien tout seul.
   *(Amendé 2026-08-20 — issue #13. Le nom du geste reste : 一分間 nomme la philosophie
   du départ minuscule, pas le réglage du chronomètre. Ce qui change est la mécanique —
   le minuteur portait 60 s pour toutes, il porte désormais la dose de chacune. Le libellé
   du bouton avait déjà cessé d'annoncer une durée en #12.)*
2. **Le prochain tout petit pas** — « Passer à N+1 min », **à l'initiative seule de
   l'utilisateur**, quand *lui* le décide. Le système ne détecte rien et ne suggère rien.
   *(Amendé 2026-07-23 — plus de « proposé par le système » ; voir
   `[[adr-0008-goal-based-dose-user-paced-progression]]`.)*
3. **Enlever la friction** (muda) — « Alléger à N−1 min ». *Alléger n'est pas reculer, c'est
   enlever ce qui freine.*
4. **Aller voir le réel** (gemba) — relier l'habitude à un déclencheur écrit librement
   (`trigger`) : un moment (« après le café »), une habitude ancrée (**habit stacking** :
   « après Bouger un peu »), ou un repère de vie hors app (« après la promenade du chien »).
   Champ texte libre + chips pour les habitudes déjà ancrées. Une ancrée est automatique :
   elle déclenche **sans notification**.
5. **Standardiser avant d'améliorer** — quand l'utilisateur **sent** une habitude acquise, il
   la marque *ancrée* (à son initiative, aucune détection de stabilité) : elle rejoint les
   acquises et sert de base (déclencheur) pour en faire grandir d'autres.
6. **Le mot de la semaine** — une phrase courte, non chiffrée, qui clôt le récap.
   *(Amendé 2026-08-21 — issue #23. La réflexion hebdo (hansei) était le geste 6 : une
   question douce et une réponse d'un mot. Elle est retirée — le récap semaine est
   **informatif et en lecture seule**, il ne recueille aucune réponse de l'utilisateur.
   Observer sans juger reste porté par le récap lui-même, qui ne chiffre ni ne reproche.)*

## La boucle

Ces gestes forment une boucle vertueuse :

```text
démarrer sur un objectif → grandir / alléger à son rythme → ancrer quand on la sent acquise
        ↑                                          │
        └──────── libère une place ◀── archiver ◀──┘
```

Une habitude ancrée quitte le quotidien (suivi léger dans *Ancrées*), libère une des 5 places
actives, et peut servir de point d'accroche (habit stacking) pour la suivante.

## Sur le « quand » sans notification

Le déclencheur ne sert **pas** de rappel push mais d'*intention* (implementation intention).
Comme la liste du jour est triée par déclencheur, l'app épouse le fil de la journée. Les
meilleurs ancrages sont souvent des repères de vie déjà automatiques (« après la promenade
du chien », « en démarrant le travail »), d'où le champ texte **libre**.
