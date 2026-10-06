# 008 Licenza duale MIT OR Apache-2.0

## Contesto
L'intestazione del PRD indica "Open Source, MIT o Apache 2.0" senza scegliere.

## Decisione
Licenza duale `MIT OR Apache-2.0` (convenzione dell'ecosistema Rust), con `LICENSE-MIT` e `LICENSE-APACHE` a root. Titolare del copyright nel file MIT: Andrea Tozzi (2026). Alternative: solo MIT; solo Apache-2.0 (include una concessione esplicita di licenza sui brevetti).

## Conseguenze
- Aggiungere il campo licenza ai manifest (`package.json`, `Cargo.toml`).
- Se in futuro cambia: nuovo ADR che supera questo.

## Stato
Accettato (6 ott 2026, confermato dal proprietario). Titolare del copyright: Andrea Tozzi (deciso dal proprietario, 6 ott 2026); il progetto si presenta sempre come "vibecoded".
