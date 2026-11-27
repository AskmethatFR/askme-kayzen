today-date = { $weekday ->
    [0] Monday
    [1] Tuesday
    [2] Wednesday
    [3] Thursday
    [4] Friday
    [5] Saturday
   *[6] Sunday
} { $day } { $month ->
    [1] January
    [2] February
    [3] March
    [4] April
    [5] May
    [6] June
    [7] July
    [8] August
    [9] September
    [10] October
    [11] November
   *[12] December
}
today-greeting = Hello.
today-empty-lede-1 = Nothing yet. And that's perfectly fine.
today-empty-lede-2 = One tiny habit is enough to start.
today-add-cta = + Add a tiny habit
today-lede = One small step is enough for today.
today-eyebrow-active = Your small steps
today-habit-meta = { $goal } times a day
today-done-aria = Done today · { $title }
today-mark-done-aria = Mark as done · { $title }
today-tally = { $done ->
    [0] Nothing required today.
   *[other] { $done } of { $total } · that's already something.
}
today-paused-link = { $count } paused · no pressure
paused-heading = Paused
paused-empty-note = Nothing here. Everything is back in your daily life.
not-found-title = This page doesn't exist.
staircase-aria = { $goal ->
    [one] Your last seven days, current goal { $goal } time a day
   *[other] Your last seven days, current goal { $goal } times a day
}
detail-back-to-today = Back to today
habit-detail-active-dose = { $goal ->
    [one] { $goal } time a day
   *[other] { $goal } times a day
}
habit-detail-paused-dose = { $goal ->
    [one] paused · { $goal } time a day
   *[other] paused · { $goal } times a day
}
habit-detail-anchored-dose = { $goal ->
    [one] anchored · { $goal } time a day
   *[other] anchored · { $goal } times a day
}
adjust-goal-eyebrow = Adjust, at your own pace
grow-goal-label = Move to { $goal } times
grow-goal-aria = Move to { $goal } times · { $title }
lighten-goal-label = { $goal ->
    [one] Lighten to { $goal } time
   *[other] Lighten to { $goal } times
}
lighten-goal-aria = { $goal ->
    [one] Lighten to { $goal } time · { $title }
   *[other] Lighten to { $goal } times · { $title }
}
start-ritual-label = Start
habit-detail-mark-done-label = It's done
habit-detail-mark-done-aria = It's done · { $title }
habit-detail-done-aria = It's done · today's goal reached · { $title }
habit-detail-undo-label = Remove one
habit-detail-today-count = { $count ->
    [one] { $count } time today
   *[other] { $count } times today
}
habit-detail-undo-aria = Remove one · { $title }
pause-habit-label = Pause, no guilt
pause-habit-aria = Pause, no guilt · { $title }
anchor-habit-label = Anchor it · it's become natural
anchor-habit-aria = Anchor it · it's become natural · { $title }
resume-habit-label = Resume it
resume-habit-aria = Resume it · { $title }
habit-not-found-message = This habit is no longer on your list.
habit-not-found-back-link = Back to today
week-heading = This week
week-times-practised = { $count ->
    [one] time practised
   *[other] times practised
}
week-message-fresh-start = A perfect start. Everything is still ahead.
week-message-resting = This week is resting. It's here whenever you're ready, no rush.
week-message-growing = You're moving forward, at your own pace.
week-rhythm-aria = Your rhythm over the last seven days
weekday-short = { $weekday ->
    [0] Mo
    [1] Tu
    [2] We
    [3] Th
    [4] Fr
    [5] Sa
   *[6] Su
}
week-curve-aria = { $title }'s journey, from { $starting_goal } to { $current_goal } times a day, { $practised_days ->
    [one] { $practised_days } day practised
   *[other] { $practised_days } days practised
}
week-habit-goal = { $changed ->
    [yes] { $starting_goal } to { $current_goal } times a day
   *[no] { $goal } times a day
}
ritual-not-found-message = This habit is no longer on your list.
ritual-not-found-back-link = Back to today
ritual-paused-message = This habit is resting right now. It's here whenever you're ready, no rush.
ritual-anchored-message = This habit has become natural. It has left your daily life.
ritual-complete-aria = Done · { $title }
ritual-complete-label = Done
ritual-stop-aria = Stop, that's okay · { $title }
ritual-stop-label = Stop, that's okay
ritual-gentle-word = You gave yourself this moment. That's already a lot.
anchored-heading = Anchored
anchored-readmit-aria = Bring it back into my daily life · { $title }
anchored-readmit-label = Bring it back into my daily life
anchored-count-tally = { $count } · became natural
anchored-daily-life-tally = You're following { $count } / { $max } habits in parallel
anchored-empty-note = None yet. A habit that has become natural will settle here.
anchored-refusal-full = Your daily life is full · anchor another one first to bring this one back
anchored-refusal-duplicate = It's already in your daily life
add-habit-heading = A new tiny habit
add-habit-lede = A gentle goal: { $goal } times a day. Less is already something; more, even better.
add-habit-ideas-eyebrow = A few ideas, ready to go
add-habit-idea-meta = { $goal } times a day
add-habit-idea-add-aria = Add "{ $label }"
add-habit-own-eyebrow = Or your own
add-habit-name-input-label = Habit name
add-habit-full-note = Your daily life is full · to add one, anchor one first
add-habit-refusal-no-name = Give it a name to get started
add-habit-submit = Add, at { $goal } times a day
idea-drink-water = Drink a glass of water
idea-put-away-an-object = Put an object away
idea-write-a-line = Write a line
idea-stretch = Stretch
idea-breathe-deeply = Breathe deeply
idea-read-a-page = Read a page
idea-walk-a-minute = Walk for a minute
idea-make-the-bed = Make your bed
idea-note-a-gratitude = Note a gratitude
idea-close-your-eyes-a-minute = Close your eyes for a minute
idea-water-a-plant = Water a plant
idea-drink-tea = Drink a tea
idea-look-out-the-window = Look out the window
idea-stand-tall-a-minute = Stand tall for a minute
idea-listen-to-a-song = Listen to a song
idea-tidy-your-desk = Tidy your desk
idea-meditate-a-minute = Meditate for a minute
idea-smile-at-someone = Smile at someone
idea-say-thanks = Say thanks
idea-get-some-air = Get some air
data-unavailable-title = Sorry.
data-unavailable-lede-1 = This device doesn't offer a safe place to keep your habits.
data-unavailable-lede-2 = Nothing was written. You can close the app.
bottom-nav-aria = Main navigation
bottom-nav-today = Today
bottom-nav-week = Week
bottom-nav-anchored = Anchored
bottom-nav-today-aria = Today · navigation
bottom-nav-week-aria = Week · navigation
bottom-nav-anchored-aria = Anchored · navigation
