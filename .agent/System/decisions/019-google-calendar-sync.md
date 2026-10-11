# 019 Sincronizzazione Google Calendar

## Contesto
PRD §13: Google Calendar API, OAuth 2.0 per app desktop nel browser di sistema, sync incrementale con sync token, CRUD con partecipanti, ricorrenze e promemoria. Il 12 ott 2026 il proprietario ha creato il progetto Google Cloud `windozcal` con un client OAuth di tipo "Desktop app". Il piano della Fase 1 lasciava al proprietario la scelta di come distribuire il client in un repository pubblico. Comportamenti dell'API verificati il 12 ott 2026 sulle pagine di riferimento ufficiali (`events.list`, `events.instances`, `events.patch`, errori, OAuth per app native).

## Decisione
- Autenticazione: Authorization Code + PKCE (S256), browser di sistema, redirect loopback `http://127.0.0.1:{porta}` effimera, `access_type=offline` e `prompt=consent` (il refresh token arriva anche quando si ricollega). Scope `openid email profile https://www.googleapis.com/auth/calendar`. Solo il refresh token in Credential Manager (ADR 005); alla disconnessione si revoca il token presso Google (best effort).
- Client: `WINDOZCAL_GOOGLE_CLIENT_ID` e `WINDOZCAL_GOOGLE_CLIENT_SECRET` iniettati in build (`option_env!`) o letti a runtime per lo sviluppo, come il client Microsoft; mai nel repository (in locale in `.env.local`, ignorato da git). Google tratta il secret di un client desktop come non confidenziale: finisce nell'eseguibile delle release. **Ipotesi da confermare dal proprietario.**
- Pull: `events.list` per calendario con `singleEvents=false` e `showDeleted=true`; il cursore in `sync_state` è il `nextSyncToken`. `410 fullSyncRequired` => full sync; un `410 deleted` su una risorsa è invece "già cancellata" (non deve far ripartire il full sync).
- Serie: il master porta la regola (`recurrence`; si conservano RRULE ed EXDATE, RDATE ed EXRULE si scartano perché il modello locale non li ha). Le istanze cancellate diventano `EXDATE`, quelle modificate eccezioni con `original_start` nel formato delle occorrenze espanse (ADR 013). Nel full sync il gruppo della serie è già nella lista; nel sync incrementale per ogni serie toccata si rilegge il master e il gruppo con lo stesso `iCalUID`. Un evento cancellato senza `recurringEventId` (singolo o serie intera) è una cancellazione.
- Push: `POST`/`PATCH`/`DELETE` sempre con `sendUpdates=none` (WinDozCal non invia email ai partecipanti); `PATCH` con `If-Match` sull'ETag; partecipanti e promemoria sostituiscono gli array sul server; promemoria inviati come override espliciti (`useDefault: false`, al massimo 5). Un'eccezione locale modifica l'istanza trovata con `events.instances?originalStart=`. Le EXDATE locali senza fuso ricevono il `TZID` dell'evento.
- Promemoria in lettura: `useDefault: true` usa i `defaultReminders` del calendario restituiti da `events.list`.
- Errori: 401 => rinnovo e poi `auth_required`; 403 `rateLimitExceeded`/`userRateLimitExceeded`/`quotaExceeded` e 429 => `RateLimited`; 5xx => errore di rete (si riprova al ciclo successivo); 412 `conditionNotMet` => errore sull'evento.

## Conseguenze
- `conference_url` si legge (`hangoutLink` o `conferenceData`) ma non si invia: creare un link Meet richiede `conferenceData`, fuori perimetro. Un link aggiunto in locale a un evento Google si perde al pull successivo (server wins, ADR 007).
- Colore, icona e riempimento degli eventi restano locali (ADR 017); la mappatura con `colorId` è un seguito.
- Con la schermata di consenso in modalità Testing Google fa scadere i refresh token dopo 7 giorni: l'account torna `auth_required` e si riconnette. Per l'uso quotidiano serve pubblicare l'app (senza verifica Google compare l'avviso "app non verificata").
- Il full sync scarica tutta la storia del calendario (nessuna finestra temporale: `timeMin` non è compatibile con il sync token).
- Modificare un evento di cui non si è organizzatori può fallire (403 `forbiddenForNonOrganizer`): l'evento passa a `error`, senza perdita locale.

## Stato
Accettato (12 ott 2026), con l'ipotesi sulla distribuzione del client da confermare.
