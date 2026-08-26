# `store/` — tout ce qui part vers la Play Console

Ce dossier contient les artefacts de publication et rien d'autre : aucun code applicatif,
aucun élément dont l'application dépende à l'exécution. Les icônes du lanceur, elles, vivent
dans `app/android/res/` — à côté du manifest, qui est déjà le seul fichier Android tenu à la
main dans le dépôt.

## Contenu

| Chemin | Rôle |
| --- | --- |
| `play-console-answers.md` | Chaque déclaration de la console, pré-remplie, avec la liste des bloquants |
| `icon/icon.html` | Source de l'icône — le K de Source Serif 4 suivi du tampon cyan |
| `icon/build_icons.py` | Génère toutes les densités du lanceur + l'icône 512×512 |
| `icon/ic_launcher-512.png` | Icône de la fiche store |
| `listing/build_feature_graphic.py` | Génère le graphique 1024×500 |
| `listing/build_screenshots.py` | Capture les 6 écrans, en français et en anglais, depuis la vraie application |
| `listing/copy.fr.md`, `listing/copy.en.md` | Titre, descriptions, et les autres champs de la fiche |
| `verify_assets.py` | Vérifie chaque fichier contre ce que Play refuse |

## Régénérer

Tout est produit par script : rien ici n'est un fichier binaire retouché à la main, et une
correction de marque, de police ou d'écran se répercute par une commande, pas par une
séance de retouche.

```sh
python3 store/icon/build_icons.py              # icônes lanceur + 512×512
python3 store/listing/build_feature_graphic.py # graphique 1024×500
dx build --platform web                        # prérequis des captures
python3 store/listing/build_screenshots.py     # 12 captures 1080×1920
python3 store/verify_assets.py                 # contrôle final
```

Les trois générateurs passent par Chrome sans interface, présent sur la machine de
développement. Ils ne tournent pas en intégration continue : ces fichiers changent à la
vitesse de la marque, pas à celle du code.

## Ce qui n'est pas encore branché

Les icônes de `app/android/res/` sont **inertes** tant que la build ne les recopie pas :
`dx` 0.7.9 régénère `res/` sous `target/` à chaque build et ne câble pas `[android].icon`.
Les pages de `site/` sont **hors ligne** tant que la branche `gh-pages` n'existe pas — or
l'URL de politique de confidentialité doit répondre avant toute soumission.

`play-console-answers.md` tient la liste complète de ces bloquants, avec la raison pour
laquelle chacun bloque.
