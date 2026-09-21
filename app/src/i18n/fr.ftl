today-date = { $weekday ->
    [0] Lundi
    [1] Mardi
    [2] Mercredi
    [3] Jeudi
    [4] Vendredi
    [5] Samedi
   *[6] Dimanche
} { $day } { $month ->
    [1] janvier
    [2] février
    [3] mars
    [4] avril
    [5] mai
    [6] juin
    [7] juillet
    [8] août
    [9] septembre
    [10] octobre
    [11] novembre
   *[12] décembre
}
today-greeting = Bonjour.
today-empty-lede-1 = Rien pour l'instant. Et c'est très bien.
today-empty-lede-2 = Une seule toute petite habitude suffit pour commencer.
today-add-cta = + Ajouter une toute petite habitude
today-lede = Un seul petit pas suffit pour aujourd'hui.
today-eyebrow-active = Vos petits pas
today-habit-meta = chaque jour · { $minutes } min
today-done-aria = Fait aujourd'hui · { $title }
today-mark-done-aria = Marquer comme fait · { $title }
today-tally = { $done } sur { $total } · c'est déjà quelque chose.
today-paused-link = { $count } en pause · aucune pression
paused-heading = En pause
paused-empty-note = Rien ici. Tout est de retour dans votre quotidien.
not-found-title = Cette page n'existe pas.
not-found-today-link = Aujourd'hui
staircase-aria = Vos sept derniers jours, objectif actuel { $goal } minutes
recap-days-done-label = { $count ->
    [one] réalisé
   *[other] réalisés
}
recap-empty-days-label = { $count ->
    [one] autre jour
   *[other] autres jours
}
recap-minutes-label = { $count ->
    [one] minute de pratique accumulée
   *[other] minutes de pratique accumulées
}
recap-growths-label = fois grandie
recap-lightenings-label = fois allégée
recap-message-fresh-start = Un début parfait. Tout est encore devant.
recap-message-resting = Elle se repose en ce moment. Elle vous attend, sans presser.
recap-message-growing = Vous la faites vivre, à votre rythme.
masthead-back-to-today = ← Aujourd'hui
detail-back-to-today = Retour à aujourd'hui
habit-detail-active-dose = chaque jour · { $goal } min
habit-detail-paused-dose = en pause · { $goal } min
habit-detail-anchored-dose = ancrée · { $goal } min
adjust-goal-eyebrow = Ajuster, à votre rythme
grow-goal-label = Passer à { $goal } min
grow-goal-aria = Passer à { $goal } min · { $title }
lighten-goal-label = Alléger à { $goal } min
lighten-goal-aria = Alléger à { $goal } min · { $title }
start-ritual-label = Commencer ma pratique
pause-habit-label = Mettre en pause, sans culpabilité
pause-habit-aria = Mettre en pause, sans culpabilité · { $title }
anchor-habit-label = L'ancrer · elle est devenue naturelle
anchor-habit-aria = L'ancrer · elle est devenue naturelle · { $title }
resume-habit-label = La reprendre
resume-habit-aria = La reprendre · { $title }
habit-not-found-message = Cette habitude n'est plus sur votre liste.
habit-not-found-back-link = Retour à Aujourd'hui
week-heading = Cette semaine
week-minutes-practised = { $count ->
    [0] minutes de pratique accumulées
    [one] minute de pratique accumulée
   *[other] minutes de pratique accumulées
}
week-message-fresh-start = Un début parfait. Tout est encore devant.
week-message-resting = Cette semaine se repose. Elle vous attend, sans presser.
week-message-growing = Vous avancez, à votre rythme.
week-rhythm-aria = Votre rythme sur les sept derniers jours
week-curve-aria = Trajectoire de { $title }, de { $starting_goal } à { $current_goal } minutes, { $practised_days ->
    [one] { $practised_days } jour pratiqué
   *[other] { $practised_days } jours pratiqués
}
week-habit-goal = { $changed ->
    [yes] { $starting_goal } → { $current_goal } min
   *[no] { $goal } min
}
ritual-not-found-message = Cette habitude n'est plus sur votre liste.
ritual-not-found-back-link = Retour à Aujourd'hui
ritual-paused-message = Cette habitude se repose en ce moment. Elle vous attend, sans presser.
ritual-anchored-message = Cette habitude est devenue naturelle. Elle a quitté votre quotidien.
ritual-complete-aria = J'ai terminé · { $title }
ritual-complete-label = J'ai terminé
ritual-stop-aria = Arrêter, ce n'est pas grave · { $title }
ritual-stop-label = Arrêter, ce n'est pas grave
ritual-timer-aria = Minuteur · { $title }
ritual-gentle-word = Vous avez pris ce moment pour vous. C'est déjà beaucoup.
anchored-heading = Ancrées
anchored-readmit-aria = La remettre dans mon quotidien · { $title }
anchored-readmit-label = La remettre dans mon quotidien
anchored-count-tally = { $count } · devenues naturelles
anchored-daily-life-tally = Vous suivez { $count } / { $max } habitudes en parallèle
anchored-refusal-full = Le quotidien est complet · pour la remettre, ancrez-en une autre d'abord
anchored-refusal-duplicate = Elle est déjà dans votre quotidien
add-habit-heading = Une nouvelle petite habitude
add-habit-lede = Un objectif doux : 5 minutes par jour. Moins, c'est déjà quelque chose ; plus, tant mieux.
add-habit-ideas-eyebrow = Quelques idées déjà prêtes
add-habit-idea-meta = { $minutes } min par jour
add-habit-idea-add-aria = Ajouter « { $label } »
add-habit-own-eyebrow = Ou la vôtre
add-habit-name-input-label = Nom de l'habitude
add-habit-submit = Ajouter, à { $minutes } min par jour
idea-drink-water = Boire un verre d'eau
idea-put-away-an-object = Ranger un objet
idea-write-a-line = Écrire une ligne
idea-stretch = S'étirer
idea-breathe-deeply = Respirer profondément
idea-read-a-page = Lire une page
idea-walk-a-minute = Marcher une minute
idea-make-the-bed = Faire son lit
idea-note-a-gratitude = Noter une gratitude
idea-close-your-eyes-a-minute = Fermer les yeux une minute
idea-water-a-plant = Arroser une plante
idea-drink-tea = Boire un thé
idea-look-out-the-window = Regarder par la fenêtre
idea-stand-tall-a-minute = Se tenir droit une minute
idea-listen-to-a-song = Écouter une chanson
idea-tidy-your-desk = Ranger son bureau
idea-meditate-a-minute = Méditer une minute
idea-smile-at-someone = Sourire à quelqu'un
idea-say-thanks = Dire merci
idea-get-some-air = Prendre l'air
data-unavailable-title = Désolé.
data-unavailable-lede-1 = Cet appareil ne propose pas d'endroit sûr où garder vos habitudes.
data-unavailable-lede-2 = Rien n'a été écrit. Vous pouvez fermer l'application.
bottom-nav-aria = Navigation principale
bottom-nav-today = Aujourd'hui
bottom-nav-week = Semaine
bottom-nav-anchored = Ancrées
bottom-nav-today-aria = Aujourd'hui · navigation
bottom-nav-week-aria = Semaine · navigation
bottom-nav-anchored-aria = Ancrées · navigation
