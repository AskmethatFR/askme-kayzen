# Habit Progression — Evidence Study & Settled Rule

> Functional node — owner: pm · status: current · updated: 2026-07-16
> Deep-research run (46 results, adversarially verified claims) settling the progression fork:
> automatic vs suggested vs manual difficulty increase. Decision taken by the human on 2026-07-16.

## The settled rule

**Progression is a suggestion, never a mutation.** The app automatically *detects* stability
from the completion history and *suggests* "Passer à N+1 min"; the dose changes **only**
through an explicit user gesture (grow / lighten). This preserves the designer's
non-negotiables ([[design-principes-kaizen]], gesture 2 of [[design-gestes-kaizen]]:
"proposé, jamais imposé") while honoring the requirement that repeated completions drive
difficulty.

**Starting policy thresholds (conservative, tunable — policy values, not invariants):**

| Suggestion | Threshold |
|---|---|
| Growth ("Passer à N+1") | done ≥ 10 of last 14 days AND current step held ≥ 14 days |
| Anchor | done ≥ 10 of last 14 days (designer's own rule, [[design-ecrans]]) |

> Note for implementation: growth and anchor share the 10-of-14 completion clause — the
> step-held clause differentiates them, but their interplay (can both fire at once?) is an
> open point for the implementation spec.

Suggestions are recomputed from history on every read — nothing stored, no proposal entity,
no decline tracking, no expiry, no re-nag mechanics (anti-guilt by design).

## Why (evidence, verified against primary sources)

1. **Habit formation is slow and wildly variable.** Lally 2010 (EJSP, N=96): median 66 days
   to 95% automaticity, range 18–254. Singh 2024 meta-analysis (20 studies, N=2,601):
   median 59–66 days, means 106–154, range 4–335; realistic horizon 2–5 months. The 21-day
   figure is a myth (Maltz 1960 anecdote). → No fixed short threshold is defensible;
   stability windows are weeks, not days.
2. **Missing one day does not harm formation.** Lally 2010, verified verbatim (p.1007):
   automaticity gain 0.79 (two days done) vs 0.55 (one miss between) — "a missed
   opportunity did not materially affect the habit formation process". Scope: single-day
   misses; week-long lapses do harm (Armitage 2005). → Streak mechanics are scientifically
   unjustified; rolling X-of-last-Y windows are the right shape.
3. **Autonomy matters.** Self-selected behaviours form stronger habits than assigned ones
   (Singh 2024, verbatim). Fogg (Tiny Habits): escalate slowly; the tiny version must
   always count as success — automatic escalation would turn tiny completions into failures.
4. **Industry is unanimous: no major app auto-escalates demand.** Habitica: difficulty 100%
   manual (automation confined to feedback colour/reward). Anti-Habit: manual two-tier
   (full + "survival minimum"). (Not Boring) Habits: cumulative non-consecutive count.
   Loop Habit Tracker: EWMA strength score over full history, lapse-resilient (~80% at one
   month of perfect dailies). Content analysis of 40 activity apps: 0/40 support goal
   re-evaluation, only 25% tailor difficulty. Documented backlash against imposed pressure
   (streaks, escalating expectations): field test where 8/10 apps "broke" the tester the
   same way; 53% of mHealth apps uninstalled within 30 days with autonomy loss cited.
5. **Honest counter-evidence (kept, not hidden).** RCTs (Adams 2013/2017, Zhou 2018) show
   *fully automatic adaptive* goals (60th percentile of last 9 days, bidirectional — goals
   can decrease) OUTPERFORMED static goals for adherence in step-count interventions. So
   "science forbids automatic" is false. It supports *bidirectional adaptation*, not forced
   monotonic escalation — and it concerns step targets, not tiny-habit durations. The SDT
   argument against automatic escalation is theory, not direct empirical evidence (three
   adversarial verdicts refuted the stronger reading). Kept as a minority path: if
   suggestion-based progression ever underperforms, evidence-backed bidirectional
   auto-adaptation is the researched alternative — it would amend the designer docs.
6. **Refuted approaches (do not resurrect):** per-individual automaticity-asymptote
   detection (model fails ~half of individuals and needs self-report data the app lacks);
   "complexity escalation weakens habit formation" (overreach of a cross-sectional
   between-behaviour comparison).

## Sources (primary, verified)

- Lally et al. 2010, *How are habits formed*, Eur. J. Soc. Psychol. 40:998–1009, doi:10.1002/ejsp.674
- Singh et al. 2024, *Time to Form a Habit*, Healthcare 12(23):2488 (PMC11641623)
- Keller et al. 2021, BJHP, doi:10.1111/bjhp.12504 (asymptote-model fit critique)
- Adams et al. 2017, BMC Public Health (PMC5372290); Adams et al. 2013, PLOS ONE (adaptive-goal RCTs)
- BJ Fogg, *Tiny Habits* (Action Line, tiny-version-counts principle)
- Villalobos-Zúñiga & Cherubini 2020, IJHCS (SDT feature taxonomy — theoretical)
- Morsink et al. 2022, J. Attention Disorders (SDT & ADHD — hypothesis-level)
- Industry: Habitica wiki (mechanics), Loop Habit Tracker FAQ (uhabits #689), 40-app content analysis

Technical counterpart: [[adr-0005-progression-suggestion-policy]] (policy-not-invariant
modeling, DDD grounding).
