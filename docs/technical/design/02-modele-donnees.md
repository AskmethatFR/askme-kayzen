# 2 · Modèle de données (Rust)

```rust
struct Habit {
    id: String,
    name: String,
    icon: String,        // nom d'icône Phosphor : "ph-book-open"
    steps: Vec<u32>,      // minutes, du départ à aujourd'hui : [2, 3, 4]
    done_today: bool,
    paused: bool,
    calendar: Vec<bool>, // ~21 derniers jours (fait / vide)
    trigger: Option<String>, // déclencheur libre : "après la promenade du chien"
    anchored: bool,      // ancrée = archivée (sort du quotidien, libère une place)
}

struct App {
    screen: Screen,            // Today | Week | Detail(id) | Timer(id) | Add | Archive
    habits: Vec<Habit>,
    draft_name: String,        // saisie de l'écran "Ajouter"
}

impl Habit {
    fn current(&self) -> u32 { *self.steps.last().unwrap() }
}
```

## Notes de conception

- **La « dose du jour » n'est jamais stockée** : c'est toujours `steps.last()`. La
  croissance est l'historique lui-même (`steps` = `[2, 3, 4]` signifie « passée de 2 à 4 min »).
- **Alléger** pousse simplement une étape plus basse : `steps.push(max(1, current - 1))`.
  Alléger n'est donc pas un cas particulier, juste un pas de plus dans l'historique.
- **Tout le récap hebdo est dérivé** de `steps` et `calendar` — aucun champ à stocker en plus.
- `trigger` est un simple texte libre (voir [gestes Kaizen](../../functional/design/04-gestes-kaizen.md)) : un moment,
  une habitude ancrée, ou un repère de vie hors app.
- `anchored = true` retire l'habitude du quotidien et la place dans l'écran *Ancrées*.

## Persistance (local, sans serveur)

```toml
# Cargo.toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- Bureau / mobile : sérialiser `Vec<Habit>` en JSON dans un fichier de données app.
- Web : le même JSON dans `localStorage`.
- Alternative : `rusqlite` si tu veux t'exercer sur SQLite (surdimensionné pour ce besoin).
