import { createContext, useContext } from "react";

// All user-facing strings in the promo, per locale. Locales mirror the app's
// catalogs (locales/*.json in the repo root); product terms (Area/Window/
// Copy/Save/Pixelate…) reuse the app's own translations so the video matches
// the real UI.
export type Locale = "en-GB" | "es" | "fr" | "de" | "it";

export interface Copy {
  /** Scene 1 — headline words pop in one by one. */
  hookWords: string[];
  /** Scene 2 */
  captureHeadline: string;
  captureCaption: string;
  /** Scene 3 */
  modesHeadline: string;
  modeTitles: [string, string, string]; // area, window, full screen
  modesCaption: string;
  /** Scene 4 */
  overlayHeadline: string;
  overlayButtons: [string, string, string, string]; // copy, save, finder, edit
  overlayButtonFontSize: number; // long labels (fr/de) need a smaller size
  chatPlaceholder: string;
  overlayCaption: string;
  /** Scene 5 */
  editorHeadline: string;
  editorCaption: string;
  /** Scene 6 */
  scrollHeadline: string;
  capturingPill: string;
  capturingPillWidth: number; // pill + traced border share this width
  scrollCaption: string;
  timerCaption: string;
  /** Scene 7 */
  trustLines: [string, string, string];
  /** Scene 8 */
  finaleFree: string;
  finaleScan: string;
}

export const COPY: Record<Locale, Copy> = {
  "en-GB": {
    hookWords: ["Your", "screenshots", "deserve", "better."],
    captureHeadline:
      "Screen for me — capture, annotate, share. In seconds.",
    captureCaption: "The fastest capture-to-share loop on your Mac.",
    modesHeadline: "Area. Window. Full screen.",
    modeTitles: ["Area", "Window", "Full screen"],
    modesCaption:
      "From the menu bar or a global shortcut — you’re never more than one keystroke away.",
    overlayHeadline: "Captured. Now it’s already where you need it.",
    overlayButtons: ["Copy", "Save", "Finder", "Edit"],
    overlayButtonFontSize: 24,
    chatPlaceholder: "Message…",
    overlayCaption: "Copy, save, or drag it straight into any app.",
    editorHeadline: "Annotate like you mean it.",
    editorCaption:
      "Arrows · shapes · text · counter steps · pixelate · crop — undo/redo, native-resolution export.",
    scrollHeadline: "Capture the whole page.",
    capturingPill: "Capturing…",
    capturingPillWidth: 196,
    scrollCaption: "Capture entire scrolling pages — stitched into one image.",
    timerCaption: "Or put it on a timer.",
    trustLines: [
      "Customisable shortcuts.",
      "English · Español · Français · Deutsch · Italiano.",
      "Native on macOS & Linux.",
    ],
    finaleFree: "100% Free",
    finaleScan: "Scan to download.",
  },
  es: {
    hookWords: ["Tus", "capturas", "merecen", "algo", "mejor."],
    captureHeadline:
      "Screen for me — captura, anota, comparte. En segundos.",
    captureCaption: "El flujo de captura a compartir más rápido de tu Mac.",
    modesHeadline: "Área. Ventana. Pantalla completa.",
    modeTitles: ["Área", "Ventana", "Pantalla completa"],
    modesCaption:
      "Desde la barra de menús o con un atajo global: siempre a una tecla de distancia.",
    overlayHeadline: "Capturado. Ya está justo donde lo necesitas.",
    overlayButtons: ["Copiar", "Guardar", "Finder", "Editar"],
    overlayButtonFontSize: 22,
    chatPlaceholder: "Mensaje…",
    overlayCaption:
      "Cópialo, guárdalo o arrástralo directamente a cualquier aplicación.",
    editorHeadline: "Anota en serio.",
    editorCaption:
      "Flechas · formas · texto · contador · pixelar · recorte — deshacer/rehacer y exportación nativa.",
    scrollHeadline: "Captura la página entera.",
    capturingPill: "Capturando…",
    capturingPillWidth: 212,
    scrollCaption:
      "Captura páginas enteras con desplazamiento, unidas en una sola imagen.",
    timerCaption: "O con el temporizador.",
    trustLines: [
      "Atajos personalizables.",
      "English · Español · Français · Deutsch · Italiano.",
      "Nativo en macOS y Linux.",
    ],
    finaleFree: "100% gratis",
    finaleScan: "Escanea para descargar.",
  },
  fr: {
    hookWords: ["Vos", "captures", "méritent", "mieux."],
    captureHeadline:
      "Screen for me — capturez, annotez, partagez. En un instant.",
    captureCaption: "La boucle capture-partage la plus rapide sur votre Mac.",
    modesHeadline: "Zone. Fenêtre. Écran entier.",
    modeTitles: ["Zone", "Fenêtre", "Écran entier"],
    modesCaption:
      "Depuis la barre des menus ou un raccourci global — jamais à plus d’une touche.",
    overlayHeadline: "Capturé. Déjà là où vous en avez besoin.",
    overlayButtons: ["Copier", "Enregistrer", "Finder", "Modifier"],
    overlayButtonFontSize: 19,
    chatPlaceholder: "Message…",
    overlayCaption:
      "Copiez, enregistrez ou glissez-la directement dans n’importe quelle app.",
    editorHeadline: "Des annotations dignes de ce nom.",
    editorCaption:
      "Flèches · formes · texte · compteur · pixelliser · rognage — annuler/rétablir, export pleine résolution.",
    scrollHeadline: "Capturez la page entière.",
    capturingPill: "Capture en cours…",
    capturingPillWidth: 288,
    scrollCaption:
      "Capturez des pages entières en défilement — assemblées en une seule image.",
    timerCaption: "Ou avec le retardateur.",
    trustLines: [
      "Raccourcis personnalisables.",
      "English · Español · Français · Deutsch · Italiano.",
      "Natif sur macOS et Linux.",
    ],
    finaleFree: "100 % gratuit",
    finaleScan: "Scannez pour télécharger.",
  },
  de: {
    hookWords: ["Ihre", "Screenshots", "verdienen", "Besseres."],
    captureHeadline:
      "Screen for me — aufnehmen, kommentieren, teilen. In Sekunden.",
    captureCaption:
      "Der schnellste Weg von der Aufnahme zum Teilen auf Ihrem Mac.",
    modesHeadline: "Bereich. Fenster. Vollbild.",
    modeTitles: ["Bereich", "Fenster", "Vollbild"],
    modesCaption:
      "Über die Menüleiste oder ein globales Tastenkürzel — nie mehr als einen Tastendruck entfernt.",
    overlayHeadline: "Aufgenommen. Schon da, wo Sie es brauchen.",
    overlayButtons: ["Kopieren", "Sichern", "Finder", "Bearbeiten"],
    overlayButtonFontSize: 19,
    chatPlaceholder: "Nachricht…",
    overlayCaption: "Kopieren, sichern oder direkt in jede App ziehen.",
    editorHeadline: "Anmerkungen, die sitzen.",
    editorCaption:
      "Pfeile · Formen · Text · Zähler · Verpixeln · Zuschneiden — Widerrufen/Wiederholen, nativer Export.",
    scrollHeadline: "Die ganze Seite aufnehmen.",
    capturingPill: "Aufnahme läuft…",
    capturingPillWidth: 262,
    scrollCaption:
      "Ganze scrollende Seiten aufnehmen — zusammengefügt zu einem Bild.",
    timerCaption: "Oder mit Selbstauslöser.",
    trustLines: [
      "Anpassbare Tastenkürzel.",
      "English · Español · Français · Deutsch · Italiano.",
      "Nativ auf macOS und Linux.",
    ],
    finaleFree: "100 % kostenlos",
    finaleScan: "Zum Herunterladen scannen.",
  },
  it: {
    hookWords: ["I", "tuoi", "screenshot", "meritano", "di", "meglio."],
    captureHeadline:
      "Screen for me — cattura, annota, condividi. In pochi secondi.",
    captureCaption:
      "Dalla cattura alla condivisione: il flusso più rapido sul tuo Mac.",
    modesHeadline: "Area. Finestra. Schermo intero.",
    modeTitles: ["Area", "Finestra", "Schermo intero"],
    modesCaption:
      "Dalla barra dei menu o con una scorciatoia globale — mai a più di un tasto di distanza.",
    overlayHeadline: "Catturato. Già dove ti serve.",
    overlayButtons: ["Copia", "Salva", "Finder", "Modifica"],
    overlayButtonFontSize: 22,
    chatPlaceholder: "Messaggio…",
    overlayCaption: "Copia, salva o trascina direttamente in qualsiasi app.",
    editorHeadline: "Annota sul serio.",
    editorCaption:
      "Frecce · forme · testo · contatore · pixella · ritaglia — annulla/ripristina, esportazione nativa.",
    scrollHeadline: "Cattura l’intera pagina.",
    capturingPill: "Cattura in corso…",
    capturingPillWidth: 278,
    scrollCaption:
      "Cattura intere pagine a scorrimento, unite in un’unica immagine.",
    timerCaption: "Oppure con l’autoscatto.",
    trustLines: [
      "Scorciatoie personalizzabili.",
      "English · Español · Français · Deutsch · Italiano.",
      "Nativo su macOS e Linux.",
    ],
    finaleFree: "100% gratuito",
    finaleScan: "Scansiona per scaricare.",
  },
};

export const LOCALES = Object.keys(COPY) as Locale[];

const CopyContext = createContext<Copy>(COPY["en-GB"]);
export const CopyProvider = CopyContext.Provider;
export const useCopy = (): Copy => useContext(CopyContext);
