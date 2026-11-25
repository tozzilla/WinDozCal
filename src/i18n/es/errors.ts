import type { PartialDictionary } from "..";

/** Traduzione della comunità (ADR 022): le chiavi mancanti ripiegano sull'inglese. */
export const errors: PartialDictionary["errors"] = {
  database: "Error de la base de datos local",
  io: "Error de lectura o escritura en el disco",
  tauri: "Error de la aplicación",
  not_found: "Elemento no encontrado",
  invalid_input: "Datos no válidos",
  not_implemented: "Aún no disponible",
  credentials: "No se pudo acceder a las credenciales de Windows",
  configuration: "Falta configuración en esta compilación",
  provider: "El servidor ha rechazado la solicitud",
  auth_required: "La cuenta necesita iniciar sesión de nuevo",
  network: "Error de red",
  rate_limited: "Demasiadas solicitudes al servidor, inténtalo de nuevo en breve",
  sync_cursor_expired: "La sincronización debe reiniciarse desde cero",
  internal: "Error interno",
};
