# 6 · Mise en place du projet Rust (Dioxus)

**Dioxus** = une UI écrite *entièrement en Rust* (syntaxe proche de React), qui cible le web,
le bureau *et* le mobile. Idéal pour progresser et réutiliser le même code.

## 1. Installer Rust puis la CLI Dioxus

```bash
# Rust (si pas déjà là)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# la CLI Dioxus
cargo install dioxus-cli
```

## 2. Créer le projet et le lancer dans le navigateur

```bash
dx new kaizen        # assistant : choisir "Web"
cd kaizen
dx serve             # ouvre http://localhost:8080, rechargement à chaud
```

## 3. Un premier composant — la liste du jour

```rust
use dioxus::prelude::*;

fn main() { dioxus::launch(App); }

fn App() -> Element {
    let mut habits = use_signal(|| vec![
        ("Lire", 4u32, false),
        ("Bouger un peu", 3, true),
        ("Respirer", 2, false),
    ]);

    rsx! {
        h1 { "Bonjour." }
        for (i, (name, min, done)) in habits().into_iter().enumerate() {
            div { style: "display:flex; gap:16px; padding:20px 0;",
                div { flex: "1",
                    div { "{name}" }
                    div { "aujourd'hui · {min} min" }
                }
                button {
                    onclick: move |_| habits.write()[i].2 = !done,
                    if done { "✓" } else { "○" }
                }
            }
        }
    }
}
```

## 4. Persister les habitudes (local, sans serveur)

```toml
# Cargo.toml — dériver serde sur les structs, écrire un petit JSON
serde = { version = "1", features = ["derive"] }
serde_json = "1"
# bureau/mobile : écrire dans un fichier ;  web : localStorage
```

Voir [`02-modele-donnees.md`](02-modele-donnees.md) pour les structs à sérialiser.

## 5. Passer sur le téléphone (plus tard)

```bash
dx serve --platform ios       # nécessite un Mac + Xcode
dx serve --platform android   # nécessite le SDK / NDK Android
```

Le même code Rust tourne partout. Le seul vrai coût reste le compte développeur Apple
(99 $/an) pour publier sur l'App Store — indépendant du langage.

## Variante — garder l'UI web

Garder le prototype HTML comme UI et mettre la logique en Rust derrière avec **Tauri v2** :

```bash
cargo create-tauri-app
```

Moins de Rust côté écran, mais tu réutilises le front tel quel.

## Ordre de construction suggéré (approche Kaizen 😉)

1. Écran **Aujourd'hui** en dur (données codées) + validation.
2. Le **modèle de données** + persistance JSON locale.
3. Écran **Détail** (croissance, calendrier, +1 / −1 min).
4. Le **rituel** (minuteur 60 s).
5. Écran **Ajouter** (idées prêtes + champ libre, plafond à 5).
6. Écran **Cette semaine** (tout dérivé).
7. Écran **Ancrées** (archive + désarchivage).
8. Le **déclencheur** (`trigger`) + tri de la liste du jour.

Cinq minutes par jour, un écran à la fois. 🌱
