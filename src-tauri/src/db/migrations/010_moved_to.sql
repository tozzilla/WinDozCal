-- Spostamento tra calendari (ADR 020): l'originale in `pending_delete` ricorda la copia nel
-- calendario di destinazione. Il sync cancella l'originale sul server solo quando la copia c'e'
-- gia', cosi' un errore nella creazione non fa sparire l'evento da entrambi i calendari.
-- Nessuna foreign key: la copia puo' essere cancellata prima del push.
ALTER TABLE events ADD COLUMN moved_to TEXT;
