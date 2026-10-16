# Easycut — CLAUDE.md

> Regole ordinate per priorità. In conflitto, vince la regola più in alto.
> "STOP" = blocca e chiedi conferma esplicita. "MAI" = non negoziabile.

---

## PRODOTTO: CONFINE NON NEGOZIABILE

**Easycut** è un'app desktop per tagliare, comprimere, cambiare risoluzione e formato dei video nel modo più efficiente possibile. "Efficiente" = veloce, a basso consumo, senza perdere qualità.

- Locale, privato e gratuito: **nessun cloud, nessun upload**, nessun account.
- App nativa per macOS, Windows e Linux. Mobile e web sono **fuori dalla prima versione**.
- Non introdurre rete, telemetria, analitica, login, pubblicità o auto-update senza conferma esplicita dell'utente.
- I video dell'utente non vengono mai copiati o modificati in place: l'output è sempre un nuovo file.

---

## STOP-THE-LINE

Non eseguire mai queste azioni senza conferma esplicita dell'utente:

- **Git**: `commit`, `push`, `reset --hard`, `rebase`, o qualsiasi comando che modifica history o remote. Proponi il comando e attendi "sì".
- **Attribuzione AI nei commit**: MAI. Nessun riferimento a Claude, Anthropic o altri assistenti nei messaggi di commit, nelle PR o nei metadati Git. Niente `Co-Authored-By` con un'AI, niente "Generated with". Il messaggio descrive solo la modifica.
- **Dipendenze**: aggiunta, rimozione o aggiornamento di crate in `Cargo.toml`.
- **Struttura**: modifiche ai confini tra `easycut-core`, `easycut-cli` ed `easycut-app`.
- **Pipeline FFmpeg**: cambi a come si costruiscono i comandi, alla scelta degli encoder, allo stream copy o alla gestione dei processi.
- **Licenze e distribuzione**: includere FFmpeg, x264/x265 o altri codec nel pacchetto (GPL e brevetti H.264/H.265).
- **Privacy e scope**: qualunque funzione di rete, upload, analitica o sincronizzazione.
- **Avvio manuale dell'app GUI**: non eseguire `cargo run -p easycut-app` senza richiesta esplicita. `cargo build`, `cargo check`, `cargo clippy`, `cargo test` e la CLI su file di prova sono autorizzati.
- **File dell'utente**: MAI eliminare o sovrascrivere video dell'utente. Le prove si fanno su copie o file generati.

Le modifiche esclusivamente di layout/stile egui non richiedono conferma, purché non tocchino chiamate al core.

---

## LINGUA

Italiano in tutto: risposte, commenti nel codice, documentazione e commit proposti.
Eccezione: termini tecnici standard (crate, trait, stream copy, keyframe, ecc.) restano in inglese.

---

## STACK

- **Rust** per tutto: motore e interfaccia.
- **egui/eframe** per la UI. L'API di `eframe 0.29` è il punto più a rischio dello scheletro.
- **FFmpeg** come processo esterno per i codec. Più avanti si potrà passare ai binding libav: il resto del codice non deve dipendere da come FFmpeg viene invocato.
- **Tauri scartato**: non ripensarlo senza una nuova decisione.

---

## ARCHITETTURA

Workspace Cargo con tre parti:

```
easycut-core/  -> motore: probe, tagli, transcodifica, scelta encoder. Zero dipendenze UI.
easycut-cli/   -> riga di comando, sottile, chiama solo easycut-core.
easycut-app/   -> finestra egui, sottile e sostituibile, chiama solo easycut-core.
```

### Regole non negoziabili

**Core indipendente dalla UI**

- CLI e GUI chiamano le **stesse funzioni** di `easycut-core`. Nessuna logica di business in `easycut-cli` o `easycut-app`.
- `easycut-core` non dipende da egui né da `clap`.

**FFmpeg dietro un confine**

- Tutto ciò che sa come invocare FFmpeg vive in un solo modulo di `easycut-core`. Il resto lavora con tipi propri (job, preset, codec), non con stringhe di argomenti.
- MAI costruire comandi concatenando stringhe da input utente: argomenti passati come lista, mai tramite shell.

**Error handling**

- `Result<T, E>` con tipi di errore definiti in `easycut-core`. MAI `unwrap()`/`expect()` in codice non di test, tranne invarianti documentate.
- Nessun panic deve arrivare all'UI: gli errori diventano messaggi leggibili.
- L'output di FFmpeg va catturato (stderr) e incluso negli errori.

**Operazioni lunghe**

- Mai bloccare il thread UI: i job girano su thread/task separati con progresso e annullamento.
- Un job annullato o fallito non lascia file parziali: pulisci l'output incompleto.

**Single source of truth**

- Un solo punto che decide l'encoder da usare, un solo posto che definisce i preset. Se ne trovi due, segnalalo e non unificarli senza conferma.

---

## EFFICIENZA: PRINCIPI DEL PRODOTTO

1. **Stream copy per i tagli** quando possibile: istantaneo e senza perdita, ma preciso solo al keyframe. Dichiara sempre all'utente quando il taglio è "al keyframe" e non al frame.
2. **Encoder hardware quando disponibili**: VideoToolbox (macOS), NVENC/QSV/AMF (Windows/Linux). In caso di fallimento, **ripiega** su libx264/libx265 senza far fallire il job.
3. **Taglio preciso** (futuro): ricodifica solo i frame agli estremi, copia il resto.
4. Evita ricodifiche inutili: se formato, codec e risoluzione richiesti coincidono con l'origine, usa stream copy.
5. Qualità prima della dimensione: nessun default che degradi il video in silenzio.

---

## STATO DEL LAVORO

- Esiste (o esisteva) uno scheletro creato fuori da questo Mac: `easycut-core`, `easycut-cli`, `easycut-app`. **Non è mai stato compilato.** Se la cartella è vuota, va ricreato.
- Prima di tutto: far compilare il workspace e provarlo con un video vero.

### Prossimi passi (in ordine)

1. Far compilare il progetto e provarlo con un video vero.
2. Anteprima video con timeline trascinabile.
3. Taglio preciso: ricodificare solo i frame agli estremi.
4. Altri formati e codec (WebM, AV1), preset, coda di lavori, drag & drop.
5. Pacchetti per macOS, Windows e Linux con FFmpeg incluso.

### Rischi noti

- **Licenze/brevetti**: x264 e x265 sono GPL; H.264/H.265 hanno brevetti. Da valutare prima di distribuire.
- **FFmpeg nel pacchetto**: includerlo per le tre piattaforme è la parte più fastidiosa del packaging.
- **Linux**: VAAPI non ancora gestito.

---

## DIMENSIONE DEI FILE

| Soglia | Azione |
|---|---|
| <=500 righe | Obiettivo. |
| 501-800 righe | Accettabile, nessuna azione. |
| >800 righe | **Refactor obbligatorio**, oppure chiedi una deroga esplicita. |

Il refactoring segue la responsabilità singola.

---

## INTESTAZIONE FILE

Ogni file `.rs` creato o modificato inizia con:

```rust
// File: easycut-core/src/percorso/esatto.rs
```

---

## TEST-DRIVEN DEVELOPMENT

Approccio TDD quando il comportamento è testabile in modo sensato.

- Scrivi prima un test che descriva il comportamento atteso e verificalo rosso.
- Implementa il minimo per portarlo verde; refactor solo a test verdi e senza ampliare lo scope.
- Per i bug, aggiungi prima un test di regressione.
- Il core è testabile senza FFmpeg reale: separa la **costruzione degli argomenti** (test unitari puri) dall'**esecuzione** (test di integrazione con brevi clip generate, mai video dell'utente).
- Se un test anticipato non è praticabile (UI egui esplorativa, integrazione hardware), dichiara il motivo e applica la verifica più vicina possibile.
- Prima di dichiarare un lavoro finito: `cargo fmt`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`.

---

## UI

- Interfaccia minimale e chiara; funziona a diverse dimensioni di finestra e scala del display (HiDPI) senza testo troncato o controlli irraggiungibili.
- Mostra sempre: file di origine, cosa verrà fatto, progresso, possibilità di annullare, percorso di output.
- Tema chiaro/scuro seguendo il sistema.

---

## COMPORTAMENTO

### Pensa prima di scrivere

- Dichiara le assunzioni prima di implementare.
- Se esistono più interpretazioni plausibili, elencale: non scegliere in silenzio.
- Se esiste un approccio più semplice, proponilo prima.
- Se la richiesta è ambigua, fermati e chiedi.

### Semplicità

- Scrivi il minimo codice che risolve il problema. Zero "per il futuro".
- Zero astrazioni per codice usato una sola volta, zero configurabilità non richiesta.
- Se scrivi 200 righe dove ne bastavano 50, riscrivi.

### Modifiche chirurgiche

- Tocca solo ciò che la richiesta richiede.
- MAI migliorare formattazione, commenti o stile del codice adiacente senza motivo.
- MAI rifattorizzare codice non rotto.
- Rimuovi solo gli orfani creati dalle tue modifiche. Codice morto preesistente: segnalalo, non cancellarlo.

### Quando chiedere vs procedere

| Situazione | Azione |
|---|---|
| Layout/stile egui soltanto | Procedi |
| Typo o rinomina locale e circoscritta | Procedi |
| Pipeline FFmpeg, scelta encoder, gestione processi | STOP — chiedi |
| Nuovo crate o cambio di struttura del workspace | STOP — chiedi |
| Licenze, packaging, distribuzione | STOP — chiedi |
| Trade-off non banale | Elenca opzioni — chiedi |
| Comportamento ambiguo | Fermati — chiedi |
