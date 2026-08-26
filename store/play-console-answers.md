# Play Console — fiche de réponses

Ce document pré-remplit chaque déclaration de la Play Console pour Kayzen. Le passage dans
la console doit être du recopiage, pas de la décision improvisée : une réponse inventée sur
place est ce qui rend la fiche Data Safety incohérente avec la politique de confidentialité,
et cette incohérence est la première cause de signalement.

Toutes les réponses ci-dessous décrivent l'application **telle qu'elle sera au moment de la
soumission**, c'est-à-dire une fois les prérequis bloquants de la dernière section levés.

---

## 0 · État du compte

| Point | État |
| --- | --- |
| Type de compte | **Organisation** (entreprise) |
| Test fermé 12 testeurs / 14 jours | **Non applicable** — l'obligation ne vise que les comptes personnels créés après le 13 novembre 2023. Un compte organisation peut publier directement en production après review |
| Compte marchand | **Non requis** — application gratuite, aucun achat intégré, aucun abonnement |

Un test interne reste recommandé malgré l'exemption : c'est aussi la cible du pipeline CD
décrit par l'issue #28, et le premier AAB doit de toute façon être déposé **à la main** —
l'API Play ne sait pas créer une application.

---

## 1 · Identité de l'application

| Champ | Valeur | Source |
| --- | --- | --- |
| Nom du package | `com.askmethat.kayzen` | `app/Dioxus.toml` — **immuable après publication** |
| Nom sur le Play Store (fr-FR) | `Kayzen — habitudes douces` | `store/listing/copy.fr.md` |
| Nom sur le Play Store (en-US) | `Kayzen — Gentle Habits` | `store/listing/copy.en.md` |
| Libellé du lanceur | `Kayzen` | `app/android/AndroidManifest.xml` — **à corriger, il dit encore `Kaizen`** |
| Locale par défaut | `fr-FR` | seconde locale : `en-US` |
| Signature | **Play App Signing** : Google détient la clé d'application, le dépôt ne détient qu'une clé d'upload | décision issue #28 |

---

## 2 · Exigences techniques

| Exigence | Échéance | État |
| --- | --- | --- |
| `targetSdk` ≥ 36 (Android 16) | obligatoire pour toute **nouvelle soumission** depuis le 31 août 2026 | ✅ `target_sdk = 36` dans `app/Dioxus.toml` |
| `compileSdk` 36 | — | ✅ `compile_sdk = 36` |
| `minSdk` | — | ✅ 24 |
| Alignement mémoire 16 Ko | obligatoire depuis le 1er novembre 2025 | ⛔ **à faire** — issue #28 slice S2a ; le NDK r25c épinglé n'aligne pas, il faut `-Wl,-z,max-page-size=16384` |
| Format d'upload | AAB obligatoire | ⛔ **à faire** — issue #28 slices S2a / S2b |
| `versionCode` incrémental | à chaque upload | ⛔ **à faire** — le template `dx` 0.7.9 le code en dur à `1` |

Une extension au 1er novembre 2026 peut être demandée dans la console si l'API 36 devait
poser problème ; le repli documenté par #28 est de publier en 35 sous extension.

---

## 3 · Contenu de l'application (« App content »)

### Politique de confidentialité

**https://askmethatfr.github.io/askme-kayzen/privacy.fr.html** (anglais :
`.../privacy.en.html`), publiée depuis `site/` par la branche orpheline `gh-pages`.

> **La page doit répondre en 200 avant la soumission, et le rester.** Un 404 sur l'URL de
> politique déclarée est une cause de suspension, pas un avertissement.

### Data safety

| Question | Réponse |
| --- | --- |
| L'application collecte-t-elle ou partage-t-elle les types de données requis ? | **Non** |
| Les données sont-elles chiffrées en transit ? | Sans objet — aucune donnée n'est transmise |
| L'utilisateur peut-il demander la suppression de ses données ? | Sans objet — aucune donnée n'est collectée ; l'effacement se fait en désinstallant l'application ou en vidant son stockage |
| Des SDK tiers collectent-ils des données ? | **Non** — aucun SDK d'analytics, de publicité ou de rapport de plantage n'est intégré |

Justification : la persistance est un fichier local dans le stockage privé de l'application
(`adr-0016-snapshot-store-persistence`, `adr-0017-platform-location-adapter`), et
`android:allowBackup="false"` interdit même la sauvegarde automatique de Google.

> ⛔ **Cette réponse n'est vraie qu'une fois les deux imports Google Fonts supprimés** — voir
> la dernière section. Tant qu'ils sont là, l'application émet une requête vers Google au
> démarrage, ce qui transmet l'adresse IP et le User-Agent de l'utilisateur, et « aucune
> donnée collectée » devient une déclaration fausse.

### Publicités

**Non**, l'application ne contient aucune publicité.

### Accès à l'application (« App access »)

**Toutes les fonctionnalités sont accessibles sans restriction.** Aucun compte, aucun
identifiant, aucun code d'accès à fournir au relecteur.

### Classification du contenu (IARC)

Questionnaire à remplir dans la console, catégorie **« Utilitaire, productivité,
communication ou autre »**. Aucune violence, aucun contenu sexuel, aucun jeu d'argent,
aucune substance, aucun contenu généré par les utilisateurs, aucun partage de localisation,
aucun achat. Classification attendue : **PEGI 3 / Everyone**.

### Public cible et contenu

**18 ans et plus.**

Cocher une tranche d'âge inférieure à 13 ans ferait entrer l'application dans le programme
**Families**, avec ses exigences supplémentaires (politique de confidentialité renforcée,
revue dédiée, conformité COPPA), pour aucun bénéfice ici : l'application ne vise pas les
enfants et ne collecte rien.

### Déclarations sectorielles

| Déclaration | Réponse |
| --- | --- |
| Application gouvernementale | Non |
| Fonctionnalités financières | Non |
| Applications de santé | Non |
| Application d'actualités | Non |
| Suivi COVID-19 / contacts | Non |

> Sur la **catégorie**, choisir **Productivité** (ou à défaut Style de vie) et **éviter
> « Santé et remise en forme »** : ce rangement expose l'application aux règles Play sur les
> applications de santé alors qu'elle ne traite aucune donnée de santé, ne se connecte pas à
> Health Connect, et ne prétend à aucun effet thérapeutique.

---

## 4 · Fiche principale

| Élément | Fichier |
| --- | --- |
| Icône 512×512 (PNG 32 bits avec alpha, < 1 Mo) | `store/icon/ic_launcher-512.png` |
| Feature graphic 1024×500 (sans canal alpha) | `store/listing/feature-graphic-1024x500.png` |
| Captures téléphone fr-FR (6 × 1080×1920) | `store/listing/screenshots/fr/` |
| Captures téléphone en-US (6 × 1080×1920) | `store/listing/screenshots/en/` |
| Titre, descriptions courte et complète | `store/listing/copy.fr.md`, `store/listing/copy.en.md` |
| E-mail de contact | `ateixeira@askmethat.fr` |

Aucune capture 7 pouces ni 10 pouces n'est fournie : l'application est pensée pour le
téléphone. Conséquence assumée — la fiche portera la mention indiquant qu'elle n'est pas
optimisée pour les tablettes.

---

## 5 · Diffusion

| Champ | Valeur |
| --- | --- |
| Prix | Gratuite |
| Achats intégrés | Aucun |
| Pays | France et pays anglophones au minimum ; la fiche existe en fr-FR et en-US |
| Publicités | Aucune |

---

## 6 · Bloquants avant soumission

Rien de ce qui suit n'est optionnel. Chaque ligne rend, tant qu'elle n'est pas levée, une
des déclarations ci-dessus fausse ou une exigence technique non satisfaite.

| # | Bloquant | Pourquoi il bloque |
| --- | --- | --- |
| 1 | `@import` Google Fonts dans `app/assets/main.css` | Requête sortante vers Google à chaque lancement → « aucune donnée collectée » devient faux |
| 2 | `@import` Google Fonts (police Inter) dans l'`index.html` généré par `dx` | **Deuxième fuite, indépendante de la première** : elle vient du gabarit `dx` 0.7.9, pas de notre feuille de style. La retirer demande de fournir notre propre `index.html`. Sans cela, supprimer le point 1 ne suffit pas |
| 3 | Icône non injectée dans la build | `res/` est régénéré sous `target/` à chaque build et `[android].icon` n'est pas câblé dans `dx` 0.7.9 : les fichiers de `app/android/res/` sont inertes tant qu'un mécanisme ne les copie pas |
| 4 | `android:label` encore à `Kaizen` | Le lanceur afficherait un autre nom que la fiche (`app/android/AndroidManifest.xml:39,46`, et `title` dans `app/Dioxus.toml:6`) |
| 4bis | Le bandeau de l'application affiche `Kaizen` en dur | `app/src/views/today.rs:28` — la marque vue à l'écran contredit la fiche, **et elle est visible sur toutes les captures** : les captures sont à régénérer après la correction (`python3 store/listing/build_screenshots.py`) |
| ~~5~~ | ~~Branche `gh-pages` inexistante~~ | **Levé** — pages en ligne, `/docs` vérifié non servi (404) |
| 6 | AAB aligné 16 Ko et signé | Issue #28, slices S2a et S2b |
| 7 | `versionCode` dérivé du tag | Issue #28 ; en dur à `1` dans le gabarit, donc aucune mise à jour ne pourrait être publiée ensuite |

Les points 1 à 5 sont l'objet des tickets de la phase technique ; les points 6 et 7 sont
déjà couverts par l'issue #28.

> **Sur le point 4bis** — le choix entre `Kaizen` (le concept japonais dont le produit tire
> son principe) et `Kayzen` (le nom du produit, du paquet et du dépôt) appartient au
> propriétaire. Tout le reste de la publication est aligné sur **Kayzen** : le paquet
> `com.askmethat.kayzen`, la fiche, l'icône, le graphique, les pages légales. Laisser
> `Kaizen` à l'écran est possible, mais c'est alors un écart volontaire à assumer, pas un
> oubli.

---

## 7 · Ordre des opérations

1. Lever les bloquants 1 à 4 (code). Le 5 est levé : le site est en ligne.
2. Vérifier que l'URL de politique répond toujours en 200.
3. Produire l'AAB aligné et signé (#28 S2a, S2b).
4. Créer l'application dans la Play Console pour `com.askmethat.kayzen`, activer Play App Signing.
5. Déposer le premier AAB **à la main** en test interne.
6. Remplir la fiche principale et les déclarations « App content » avec ce document.
7. Vérifier une dernière fois le triangle **fiche Data Safety ⇔ politique publiée ⇔
   comportement du binaire** : c'est lui qui casse en premier.
8. Promouvoir en production — geste humain, jamais automatisé (#28).
