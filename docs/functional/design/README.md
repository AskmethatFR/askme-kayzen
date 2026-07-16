# Kaizen — application de suivi d'habitudes

> Un suivi d'habitudes volontairement **bête et doux**. Pensé pour la simplicité
> Kaizen (petits pas, 1 % à la fois) et pour être reposant, notamment en contexte
> de neurodivergence.

Ce dossier fige le produit pour le réécrire en **Rust** (Dioxus), écran par écran.

## Sommaire

| Fichier | Contenu |
| --- | --- |
| [`01-principes.md`](01-principes.md) | Les règles Kaizen non négociables |
| [`02-modele-donnees.md`](../../technical/design/02-modele-donnees.md) | Le modèle de données (structs Rust) |
| [`03-ecrans.md`](03-ecrans.md) | Les écrans, leurs gestes et transitions |
| [`04-gestes-kaizen.md`](04-gestes-kaizen.md) | Les sept gestes Kaizen + la boucle |
| [`05-style-graphique.md`](05-style-graphique.md) | Couleurs, typographie, icônes, composants + aperçu des écrans |
| [`06-mise-en-place-rust.md`](../../technical/design/06-mise-en-place-rust.md) | Tuto de création du projet Rust (Dioxus) |
| [`images/`](images/) | Captures des six écrans du prototype |

## En une phrase

Une habitude **démarre minuscule** → **grandit** (ou s'allège) → **s'ancre** quand elle
est stable → rejoint les *Ancrées* (suivi léger, hors quotidien) → **libère une place**
pour en commencer une nouvelle. Le tout sans streak, sans rouge d'alerte, sans notification
culpabilisante.

## Périmètre technique

- **100 % local**, pas de serveur, pas de compte : les données restent sur l'appareil.
- Cible **web + bureau + mobile** avec le même code Rust (Dioxus), ou UI web + logique Rust (Tauri).
- Le seul coût récurrent pour publier sur l'App Store est le compte développeur Apple
  (99 $/an), indépendant du langage.
