import type { PartialDictionary } from "..";

/** Traduzione della comunità (ADR 022): le chiavi mancanti ripiegano sull'inglese. */
export const shell: PartialDictionary["shell"] = {
  update: {
    available: "WinDozCal {version} está disponible (instalada {current}).",
    failed: "Error en la actualización: {error}",
    downloading: "Descargando…",
    install: "Instalar y reiniciar",
    later: "Más tarde",
  },
  header: {
    toggleSidebar: "Mostrar u ocultar la barra lateral",
    previous: "Periodo anterior",
    next: "Periodo siguiente",
    today: "Hoy",
    searchAria: "Buscar eventos (Ctrl+K)",
    search: "Buscar eventos",
    views: "Vista",
    newEvent: "Nuevo evento",
  },
  sidebar: {
    hideChannel: "Ocultar todo el canal",
    showChannel: "Mostrar todo el canal",
    authRenewTitle: "Hay que renovar el inicio de sesión: abre Ajustes > Cuentas",
    authRenew: "Hay que renovar el inicio de sesión",
    syncError: "Error de sincronización",
    addLocalCalendar: "Añadir calendario local",
    calendarNamePlaceholder: "Nombre del calendario",
    toggleHint: "Clic: mostrar u ocultar · Clic derecho: color",
    colorOf: "Color de {name}",
    colorHeading: "Color · {name}",
    settings: "Ajustes",
  },
  search: {
    title: "Buscar eventos",
    placeholder: "Busca por título, lugar, descripción o invitado",
    untitled: "(sin título)",
    empty: "No se han encontrado eventos.",
  },
  colorPicker: {
    hex: "Color hexadecimal",
    use: "Usar",
  },
  move: {
    failed: "No se pudo mover: {error}",
    question: "Mover \"{title}\" a {start}–{end}: aplicar a",
    note: "Mover toda la serie o los eventos siguientes descarta los cambios ya hechos en las repeticiones afectadas.",
  },
};
