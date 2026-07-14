#!/usr/bin/env python3
"""Generate site/{es,fr,de,it}/index.html from site/index.html.

Every replacement is whitespace-flexible (newline+indent in the source HTML
matches any whitespace run) and MUST match exactly once, otherwise the script
aborts so no page ships with untranslated English left behind.
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SRC = (ROOT / "index.html").read_text(encoding="utf-8")

LANGS = ["es", "fr", "de", "it"]

# (source, {lang: target}) — source whitespace runs match \s+.
T = [
    ('<html lang="en">', {
        "es": '<html lang="es">', "fr": '<html lang="fr">',
        "de": '<html lang="de">', "it": '<html lang="it">'}),
    ("<title>Screen for me — the fastest capture-to-share loop for macOS &amp; Linux</title>", {
        "es": "<title>Screen for me — el ciclo de captura a compartir más rápido para macOS y Linux</title>",
        "fr": "<title>Screen for me — la boucle capture-partage la plus rapide pour macOS et Linux</title>",
        "de": "<title>Screen for me — der schnellste Weg vom Screenshot zum Teilen für macOS &amp; Linux</title>",
        "it": "<title>Screen for me — il ciclo cattura-condivisione più rapido per macOS e Linux</title>"}),
    ('content="Screen for me is a free, open-source screenshot app for macOS and Linux. Capture, annotate, and drag your screenshot straight into any app — in seconds."', {
        "es": 'content="Screen for me es una aplicación de capturas de pantalla gratuita y de código abierto para macOS y Linux. Captura, anota y arrastra tu captura directamente a cualquier aplicación, en segundos."',
        "fr": 'content="Screen for me est une application de capture d\'écran gratuite et open source pour macOS et Linux. Capturez, annotez et glissez votre capture directement dans n\'importe quelle application, en quelques secondes."',
        "de": 'content="Screen for me ist eine kostenlose Open-Source-Screenshot-App für macOS und Linux. Aufnehmen, beschriften und den Screenshot direkt in jede App ziehen — in Sekunden."',
        "it": 'content="Screen for me è un\'app di screenshot gratuita e open source per macOS e Linux. Cattura, annota e trascina lo screenshot direttamente in qualsiasi app, in pochi secondi."'}),
    ('content="A free, open-source screenshot app for macOS and Linux. The fastest capture-to-share loop."', {
        "es": 'content="Una aplicación de capturas de pantalla gratuita y de código abierto para macOS y Linux. El ciclo de captura a compartir más rápido."',
        "fr": 'content="Une application de capture d\'écran gratuite et open source pour macOS et Linux. La boucle capture-partage la plus rapide."',
        "de": 'content="Eine kostenlose Open-Source-Screenshot-App für macOS und Linux. Der schnellste Weg vom Screenshot zum Teilen."',
        "it": 'content="Un\'app di screenshot gratuita e open source per macOS e Linux. Il ciclo cattura-condivisione più rapido."'}),
    ('<nav class="top-nav" aria-label="Site">', {
        "es": '<nav class="top-nav" aria-label="Sitio">',
        "fr": '<nav class="top-nav" aria-label="Site">',
        "de": '<nav class="top-nav" aria-label="Website">',
        "it": '<nav class="top-nav" aria-label="Sito">'}),
    ('<a href="#features">Features</a>', {
        "es": '<a href="#features">Características</a>',
        "fr": '<a href="#features">Fonctionnalités</a>',
        "de": '<a href="#features">Funktionen</a>',
        "it": '<a href="#features">Funzionalità</a>'}),
    ('<a href="#open-source">Open source</a>', {
        "es": '<a href="#open-source">Código abierto</a>',
        "fr": '<a href="#open-source">Open source</a>',
        "de": '<a href="#open-source">Open Source</a>',
        "it": '<a href="#open-source">Open source</a>'}),
    ('aria-label="Screen for me on GitHub"', {
        "es": 'aria-label="Screen for me en GitHub"',
        "fr": 'aria-label="Screen for me sur GitHub"',
        "de": 'aria-label="Screen for me auf GitHub"',
        "it": 'aria-label="Screen for me su GitHub"'}),
    ("<h1>Capture. Mark it up.<br>Drag it anywhere.</h1>", {
        "es": "<h1>Captura. Anótala.<br>Arrástrala a cualquier parte.</h1>",
        "fr": "<h1>Capturez. Annotez.<br>Glissez-la où vous voulez.</h1>",
        "de": "<h1>Aufnehmen. Beschriften.<br>Überallhin ziehen.</h1>",
        "it": "<h1>Cattura. Annota.<br>Trascinala ovunque.</h1>"}),
    ("""Screen for me is a screenshot app for macOS and Linux built around one idea:
      the moment between taking a capture and using it should be seconds, not a trip through your Downloads folder.""", {
        "es": "Screen for me es una aplicación de capturas de pantalla para macOS y Linux construida alrededor de una idea: el momento entre hacer una captura y usarla debería durar segundos, no un viaje por tu carpeta de descargas.",
        "fr": "Screen for me est une application de capture d'écran pour macOS et Linux construite autour d'une idée : entre la capture et son utilisation, il devrait s'écouler quelques secondes, pas un détour par votre dossier Téléchargements.",
        "de": "Screen for me ist eine Screenshot-App für macOS und Linux, gebaut um eine Idee: Zwischen Aufnahme und Verwendung sollten Sekunden liegen, kein Umweg über den Downloads-Ordner.",
        "it": "Screen for me è un'app di screenshot per macOS e Linux costruita attorno a un'idea: tra la cattura e il suo utilizzo dovrebbero passare secondi, non un giro nella cartella Download."}),
    ("Download for macOS", {
        "es": "Descargar para macOS", "fr": "Télécharger pour macOS",
        "de": "Für macOS laden", "it": "Scarica per macOS"}),
    (">View on GitHub</a>", {
        "es": ">Ver en GitHub</a>", "fr": ">Voir sur GitHub</a>",
        "de": ">Auf GitHub ansehen</a>", "it": ">Guarda su GitHub</a>"}),
    ("Free &amp; open source · MIT licence · No account, no cloud", {
        "es": "Gratis y de código abierto · Licencia MIT · Sin cuenta, sin nube",
        "fr": "Gratuit et open source · Licence MIT · Pas de compte, pas de cloud",
        "de": "Kostenlos &amp; Open Source · MIT-Lizenz · Kein Konto, keine Cloud",
        "it": "Gratuita e open source · Licenza MIT · Nessun account, nessun cloud"}),
    ('data-media-alt="The Screen for me quick-access overlay floating over a desktop, mid-capture"', {
        "es": 'data-media-alt="El panel de acceso rápido de Screen for me flotando sobre un escritorio, en plena captura"',
        "fr": 'data-media-alt="Le panneau d\'accès rapide de Screen for me flottant au-dessus d\'un bureau, en pleine capture"',
        "de": 'data-media-alt="Das Schnellzugriff-Panel von Screen for me über einem Schreibtisch, mitten in einer Aufnahme"',
        "it": 'data-media-alt="Il pannello di accesso rapido di Screen for me sopra una scrivania, durante una cattura"'}),
    ('aria-label="A dimmed desktop with an area selection in progress and the Screen for me quick-access overlay in the corner"', {
        "es": 'aria-label="Un escritorio atenuado con una selección de área en curso y el panel de acceso rápido de Screen for me en la esquina"',
        "fr": 'aria-label="Un bureau assombri avec une sélection de zone en cours et le panneau d\'accès rapide de Screen for me dans le coin"',
        "de": 'aria-label="Ein abgedunkelter Schreibtisch mit laufender Bereichsauswahl und dem Schnellzugriff-Panel von Screen for me in der Ecke"',
        "it": 'aria-label="Una scrivania oscurata con una selezione di area in corso e il pannello di accesso rapido di Screen for me nell\'angolo"'}),
    ("<h2>The whole loop, in the time it takes to say it</h2>", {
        "es": "<h2>Todo el ciclo, en lo que tardas en contarlo</h2>",
        "fr": "<h2>Toute la boucle, le temps de le dire</h2>",
        "de": "<h2>Der ganze Ablauf, schneller als man ihn erklärt</h2>",
        "it": "<h2>Tutto il ciclo, nel tempo di dirlo</h2>"}),
    ("<h3>Press the shortcut</h3>", {
        "es": "<h3>Pulsa el atajo</h3>", "fr": "<h3>Appuyez sur le raccourci</h3>",
        "de": "<h3>Kurzbefehl drücken</h3>", "it": "<h3>Premi la scorciatoia</h3>"}),
    ("<p>Area, window, or full screen: from anywhere, over any app. The menu bar works too.</p>", {
        "es": "<p>Área, ventana o pantalla completa: desde cualquier lugar, sobre cualquier aplicación. La barra de menús también funciona.</p>",
        "fr": "<p>Zone, fenêtre ou écran entier : depuis n'importe où, par-dessus n'importe quelle application. La barre de menus fonctionne aussi.</p>",
        "de": "<p>Bereich, Fenster oder Vollbild: von überall, über jeder App. Die Menüleiste geht auch.</p>",
        "it": "<p>Area, finestra o schermo intero: da qualsiasi punto, sopra qualsiasi app. Funziona anche dalla barra dei menu.</p>"}),
    ("<h3>The overlay appears</h3>", {
        "es": "<h3>Aparece el panel</h3>", "fr": "<h3>Le panneau apparaît</h3>",
        "de": "<h3>Das Panel erscheint</h3>", "it": "<h3>Appare il pannello</h3>"}),
    ("<p>A small panel slips into the corner with your capture: copy it, save it, or open the editor.</p>", {
        "es": "<p>Un pequeño panel aparece en la esquina con tu captura: cópiala, guárdala o abre el editor.</p>",
        "fr": "<p>Un petit panneau se glisse dans le coin avec votre capture : copiez-la, enregistrez-la ou ouvrez l'éditeur.</p>",
        "de": "<p>Ein kleines Panel erscheint in der Ecke mit deiner Aufnahme: kopieren, sichern oder den Editor öffnen.</p>",
        "it": "<p>Un piccolo pannello compare nell'angolo con la tua cattura: copiala, salvala o apri l'editor.</p>"}),
    ("<h3>Drag it straight in</h3>", {
        "es": "<h3>Arrástrala directamente</h3>", "fr": "<h3>Glissez-la directement</h3>",
        "de": "<h3>Direkt hineinziehen</h3>", "it": "<h3>Trascinala direttamente</h3>"}),
    ("<p>Pull the thumbnail out of the overlay and drop it into Slack, an email, a doc — the app you were already in.</p>", {
        "es": "<p>Saca la miniatura del panel y suéltala en Slack, un correo, un documento — la aplicación en la que ya estabas.</p>",
        "fr": "<p>Tirez la vignette hors du panneau et déposez-la dans Slack, un e-mail, un document — l'application où vous étiez déjà.</p>",
        "de": "<p>Zieh die Miniatur aus dem Panel und lass sie in Slack, einer E-Mail oder einem Dokument los — der App, in der du ohnehin gerade warst.</p>",
        "it": "<p>Prendi la miniatura dal pannello e lasciala in Slack, un'email, un documento — l'app in cui eri già.</p>"}),
    ('data-media-video="assets/demo.mp4"', {
        "es": 'data-media-video="assets/demo-es.mp4"',
        "fr": 'data-media-video="assets/demo-fr.mp4"',
        "de": 'data-media-video="assets/demo-de.mp4"',
        "it": 'data-media-video="assets/demo-it.mp4"'}),
    ('data-media-alt="Screen recording of the full Screen for me capture-to-share loop"', {
        "es": 'data-media-alt="Grabación de pantalla del ciclo completo de captura a compartir de Screen for me"',
        "fr": 'data-media-alt="Enregistrement d\'écran de la boucle capture-partage complète de Screen for me"',
        "de": 'data-media-alt="Bildschirmaufnahme des kompletten Aufnehmen-und-Teilen-Ablaufs von Screen for me"',
        "it": 'data-media-alt="Registrazione dello schermo dell\'intero ciclo cattura-condivisione di Screen for me"'}),
    ("<p>Two-minute walkthrough, coming with the first release</p>", {
        "es": "<p>Vídeo de dos minutos, disponible con la primera versión</p>",
        "fr": "<p>Démonstration de deux minutes, disponible avec la première version</p>",
        "de": "<p>Zwei-Minuten-Rundgang, verfügbar mit dem ersten Release</p>",
        "it": "<p>Video di due minuti, in arrivo con la prima release</p>"}),
    ("<h2>Capture exactly what you mean</h2>", {
        "es": "<h2>Captura exactamente lo que quieres</h2>",
        "fr": "<h2>Capturez exactement ce que vous voulez</h2>",
        "de": "<h2>Nimm genau das auf, was du meinst</h2>",
        "it": "<h2>Cattura esattamente ciò che intendi</h2>"}),
    ("""<p>Draw an area with pixel-precise crosshairs, pick a single window, or grab the whole screen.
           A self-timer covers the menus and tooltips that vanish the moment you press a key.</p>""", {
        "es": "<p>Dibuja un área con precisión de píxel, elige una sola ventana o captura toda la pantalla. El temporizador cubre los menús y tooltips que desaparecen en cuanto pulsas una tecla.</p>",
        "fr": "<p>Tracez une zone au pixel près, choisissez une seule fenêtre ou capturez tout l'écran. Le retardateur couvre les menus et infobulles qui disparaissent dès que vous appuyez sur une touche.</p>",
        "de": "<p>Zieh einen Bereich pixelgenau auf, wähl ein einzelnes Fenster oder nimm den ganzen Bildschirm auf. Der Selbstauslöser erwischt Menüs und Tooltips, die verschwinden, sobald du eine Taste drückst.</p>",
        "it": "<p>Disegna un'area con precisione al pixel, scegli una singola finestra o cattura l'intero schermo. L'autoscatto copre i menu e i tooltip che spariscono appena premi un tasto.</p>"}),
    ("</span> Area</li>", {
        "es": "</span> Área</li>", "fr": "</span> Zone</li>",
        "de": "</span> Bereich</li>", "it": "</span> Area</li>"}),
    ("</span> Window</li>", {
        "es": "</span> Ventana</li>", "fr": "</span> Fenêtre</li>",
        "de": "</span> Fenster</li>", "it": "</span> Finestra</li>"}),
    ("</span> Full screen</li>", {
        "es": "</span> Pantalla completa</li>", "fr": "</span> Écran entier</li>",
        "de": "</span> Vollbild</li>", "it": "</span> Schermo intero</li>"}),
    ("""<p class="feature-note">Shortcuts are global and configurable. On Linux, it's <kbd>Ctrl</kbd> instead of <kbd>⌘</kbd>.</p>""", {
        "es": '<p class="feature-note">Los atajos son globales y configurables. En Linux es <kbd>Ctrl</kbd> en lugar de <kbd>⌘</kbd>.</p>',
        "fr": '<p class="feature-note">Les raccourcis sont globaux et configurables. Sous Linux, c\'est <kbd>Ctrl</kbd> au lieu de <kbd>⌘</kbd>.</p>',
        "de": '<p class="feature-note">Kurzbefehle sind global und konfigurierbar. Unter Linux ist es <kbd>Ctrl</kbd> statt <kbd>⌘</kbd>.</p>',
        "it": '<p class="feature-note">Le scorciatoie sono globali e configurabili. Su Linux è <kbd>Ctrl</kbd> invece di <kbd>⌘</kbd>.</p>'}),
    ('data-media-alt="Area capture in progress: crosshair selection with live pixel dimensions"', {
        "es": 'data-media-alt="Captura de área en curso: selección con cruceta y dimensiones en píxeles en directo"',
        "fr": 'data-media-alt="Capture de zone en cours : sélection au viseur avec dimensions en pixels en direct"',
        "de": 'data-media-alt="Laufende Bereichsaufnahme: Fadenkreuz-Auswahl mit Live-Pixelmaßen"',
        "it": 'data-media-alt="Cattura di area in corso: selezione con mirino e dimensioni in pixel in tempo reale"'}),
    ('aria-label="Crosshair area selection with a live 640 by 400 pixel readout"', {
        "es": 'aria-label="Selección de área con cruceta y lectura en directo de 640 por 400 píxeles"',
        "fr": 'aria-label="Sélection de zone au viseur avec un affichage en direct de 640 par 400 pixels"',
        "de": 'aria-label="Bereichsauswahl mit Fadenkreuz und Live-Anzeige von 640 mal 400 Pixeln"',
        "it": 'aria-label="Selezione di area con mirino e lettura in tempo reale di 640 per 400 pixel"'}),
    ("<h2>The overlay that ends the Downloads-folder detour</h2>", {
        "es": "<h2>El panel que acaba con el desvío por la carpeta de descargas</h2>",
        "fr": "<h2>Le panneau qui met fin au détour par les Téléchargements</h2>",
        "de": "<h2>Das Panel, das den Umweg über den Downloads-Ordner beendet</h2>",
        "it": "<h2>Il pannello che elimina la deviazione nella cartella Download</h2>"}),
    ("""<p>Every capture lands in a small glass panel in the corner of your screen, stacked so burst captures
           don't get lost. Copy to the clipboard, save where you want, or grab the thumbnail and
           <strong>drag it directly into any app</strong> that accepts an image.</p>""", {
        "es": "<p>Cada captura aterriza en un pequeño panel de cristal en la esquina de tu pantalla, apiladas para que las ráfagas de capturas no se pierdan. Copia al portapapeles, guarda donde quieras o coge la miniatura y <strong>arrástrala directamente a cualquier aplicación</strong> que acepte una imagen.</p>",
        "fr": "<p>Chaque capture atterrit dans un petit panneau de verre au coin de votre écran, empilées pour que les rafales ne se perdent pas. Copiez dans le presse-papiers, enregistrez où vous voulez, ou attrapez la vignette et <strong>glissez-la directement dans n'importe quelle application</strong> qui accepte une image.</p>",
        "de": "<p>Jede Aufnahme landet in einem kleinen Glas-Panel in der Bildschirmecke, gestapelt, damit Serienaufnahmen nicht verloren gehen. Kopiere in die Zwischenablage, sichere, wohin du willst, oder nimm die Miniatur und <strong>zieh sie direkt in jede App</strong>, die ein Bild akzeptiert.</p>",
        "it": "<p>Ogni cattura atterra in un piccolo pannello di vetro nell'angolo dello schermo, impilate così le raffiche non si perdono. Copia negli appunti, salva dove vuoi, oppure prendi la miniatura e <strong>trascinala direttamente in qualsiasi app</strong> che accetti un'immagine.</p>"}),
    ('<p class="feature-note">The overlay gets out of the way on its own. Captures stay on your machine, in a folder you control.</p>', {
        "es": '<p class="feature-note">El panel se aparta solo. Las capturas se quedan en tu equipo, en una carpeta que tú controlas.</p>',
        "fr": '<p class="feature-note">Le panneau s\'efface de lui-même. Vos captures restent sur votre machine, dans un dossier que vous contrôlez.</p>',
        "de": '<p class="feature-note">Das Panel räumt sich selbst weg. Aufnahmen bleiben auf deinem Rechner, in einem Ordner, den du bestimmst.</p>',
        "it": '<p class="feature-note">Il pannello si toglie di mezzo da solo. Le catture restano sul tuo computer, in una cartella che controlli tu.</p>'}),
    ('data-media-alt="The quick-access overlay: capture thumbnail with Copy, Save and Edit actions"', {
        "es": 'data-media-alt="El panel de acceso rápido: miniatura de la captura con acciones de copiar, guardar y editar"',
        "fr": 'data-media-alt="Le panneau d\'accès rapide : vignette de la capture avec les actions copier, enregistrer et modifier"',
        "de": 'data-media-alt="Das Schnellzugriff-Panel: Aufnahme-Miniatur mit Aktionen zum Kopieren, Sichern und Bearbeiten"',
        "it": 'data-media-alt="Il pannello di accesso rapido: miniatura della cattura con azioni copia, salva e modifica"'}),
    ('aria-label="Quick-access overlay panel with a capture thumbnail, a stack badge showing 3 captures, and Copy, Save, Edit buttons"', {
        "es": 'aria-label="Panel de acceso rápido con una miniatura de captura, una insignia que muestra 3 capturas apiladas y botones de copiar, guardar y editar"',
        "fr": 'aria-label="Panneau d\'accès rapide avec une vignette de capture, un badge indiquant 3 captures empilées et des boutons copier, enregistrer et modifier"',
        "de": 'aria-label="Schnellzugriff-Panel mit Aufnahme-Miniatur, einem Stapel-Badge mit 3 Aufnahmen und Schaltflächen zum Kopieren, Sichern und Bearbeiten"',
        "it": 'aria-label="Pannello di accesso rapido con miniatura della cattura, un badge che mostra 3 catture impilate e pulsanti copia, salva e modifica"'}),
    ("<h2>Annotate without breaking stride</h2>", {
        "es": "<h2>Anota sin perder el ritmo</h2>",
        "fr": "<h2>Annotez sans casser votre élan</h2>",
        "de": "<h2>Beschriften, ohne den Faden zu verlieren</h2>",
        "it": "<h2>Annota senza perdere il ritmo</h2>"}),
    ("""<p>Arrows, boxes, text, highlights, and pixelate for the parts that aren't anyone's business.
           Crop, undo, redo. Your last tool, colour, and stroke are remembered, so the second
           annotation starts faster than the first.</p>""", {
        "es": "<p>Flechas, cajas, texto, resaltados y pixelado para lo que no es asunto de nadie. Recorta, deshaz, rehaz. Tu última herramienta, color y trazo se recuerdan, así que la segunda anotación empieza más rápido que la primera.</p>",
        "fr": "<p>Flèches, cadres, texte, surlignage, et pixellisation pour ce qui ne regarde personne. Recadrez, annulez, rétablissez. Votre dernier outil, couleur et épaisseur sont mémorisés : la deuxième annotation démarre plus vite que la première.</p>",
        "de": "<p>Pfeile, Rahmen, Text, Markierungen und Verpixeln für das, was niemanden etwas angeht. Zuschneiden, rückgängig, wiederherstellen. Dein letztes Werkzeug, Farbe und Strichstärke werden gemerkt — die zweite Anmerkung startet also schneller als die erste.</p>",
        "it": "<p>Frecce, riquadri, testo, evidenziazioni e pixel per ciò che non riguarda nessuno. Ritaglia, annulla, ripeti. Ultimo strumento, colore e tratto vengono ricordati, così la seconda annotazione parte più veloce della prima.</p>"}),
    ("<p>Exports are <strong>native resolution</strong>, always: what you share is exactly as sharp as what you captured.</p>", {
        "es": "<p>Las exportaciones son siempre a <strong>resolución nativa</strong>: lo que compartes es exactamente tan nítido como lo que capturaste.</p>",
        "fr": "<p>Les exports sont toujours en <strong>résolution native</strong> : ce que vous partagez est exactement aussi net que ce que vous avez capturé.</p>",
        "de": "<p>Exporte sind immer in <strong>nativer Auflösung</strong>: Was du teilst, ist exakt so scharf wie das, was du aufgenommen hast.</p>",
        "it": "<p>Le esportazioni sono sempre a <strong>risoluzione nativa</strong>: ciò che condividi è nitido esattamente quanto ciò che hai catturato.</p>"}),
    ('data-media-alt="The annotation editor: toolbar with drawing tools, an arrow and pixelated region on a capture"', {
        "es": 'data-media-alt="El editor de anotaciones: barra de herramientas de dibujo, una flecha y una zona pixelada sobre una captura"',
        "fr": 'data-media-alt="L\'éditeur d\'annotations : barre d\'outils de dessin, une flèche et une zone pixellisée sur une capture"',
        "de": 'data-media-alt="Der Anmerkungs-Editor: Werkzeugleiste, ein Pfeil und ein verpixelter Bereich auf einer Aufnahme"',
        "it": 'data-media-alt="L\'editor di annotazioni: barra degli strumenti di disegno, una freccia e una zona pixelata su una cattura"'}),
    ('aria-label="The annotation editor with a toolbar, a yellow arrow annotation, and a pixelated region on the capture"', {
        "es": 'aria-label="El editor de anotaciones con una barra de herramientas, una flecha amarilla y una zona pixelada sobre la captura"',
        "fr": 'aria-label="L\'éditeur d\'annotations avec une barre d\'outils, une flèche jaune et une zone pixellisée sur la capture"',
        "de": 'aria-label="Der Anmerkungs-Editor mit Werkzeugleiste, einem gelben Pfeil und einem verpixelten Bereich auf der Aufnahme"',
        "it": 'aria-label="L\'editor di annotazioni con barra degli strumenti, una freccia gialla e una zona pixelata sulla cattura"'}),
    ("<h2>Capture pages taller than your screen</h2>", {
        "es": "<h2>Captura páginas más altas que tu pantalla</h2>",
        "fr": "<h2>Capturez des pages plus hautes que votre écran</h2>",
        "de": "<h2>Nimm Seiten auf, die höher sind als dein Bildschirm</h2>",
        "it": "<h2>Cattura pagine più alte del tuo schermo</h2>"}),
    ("""<p>Scrolling capture scrolls the page for you, grabs it in strips, and stitches them into one
           seamless full-length image. Point it at a chat thread, a long doc, a whole webpage.</p>""", {
        "es": "<p>La captura con desplazamiento recorre la página por ti, la captura en franjas y las une en una sola imagen continua. Apúntala a un hilo de chat, un documento largo, una página web entera.</p>",
        "fr": "<p>La capture avec défilement fait défiler la page pour vous, la saisit par bandes et les assemble en une seule image continue. Pointez-la vers un fil de discussion, un long document, une page web entière.</p>",
        "de": "<p>Die scrollende Aufnahme scrollt die Seite für dich, nimmt sie in Streifen auf und fügt sie zu einem nahtlosen Bild zusammen. Richte sie auf einen Chatverlauf, ein langes Dokument, eine ganze Webseite.</p>",
        "it": "<p>La cattura con scorrimento scorre la pagina per te, la acquisisce a strisce e le unisce in un'unica immagine continua. Puntala su una chat, un documento lungo, un'intera pagina web.</p>"}),
    ('<p class="feature-note">Scrolling capture is macOS-only for now.</p>', {
        "es": '<p class="feature-note">La captura con desplazamiento es solo para macOS por ahora.</p>',
        "fr": '<p class="feature-note">La capture avec défilement est réservée à macOS pour l\'instant.</p>',
        "de": '<p class="feature-note">Die scrollende Aufnahme gibt es vorerst nur für macOS.</p>',
        "it": '<p class="feature-note">La cattura con scorrimento per ora è solo per macOS.</p>'}),
    ('data-media-alt="Scrolling capture in progress: the recording pill over a long page being stitched"', {
        "es": 'data-media-alt="Captura con desplazamiento en curso: el indicador de grabación sobre una página larga mientras se une"',
        "fr": 'data-media-alt="Capture avec défilement en cours : la pastille d\'enregistrement sur une longue page en cours d\'assemblage"',
        "de": 'data-media-alt="Laufende scrollende Aufnahme: die Aufnahme-Pille über einer langen Seite, die zusammengefügt wird"',
        "it": 'data-media-alt="Cattura con scorrimento in corso: la pillola di registrazione sopra una pagina lunga in fase di unione"'}),
    ('aria-label="A long page scrolling behind the recording pill while Screen for me stitches it into one image"', {
        "es": 'aria-label="Una página larga desplazándose tras el indicador de grabación mientras Screen for me la une en una sola imagen"',
        "fr": 'aria-label="Une longue page défilant derrière la pastille d\'enregistrement pendant que Screen for me l\'assemble en une seule image"',
        "de": 'aria-label="Eine lange Seite scrollt hinter der Aufnahme-Pille, während Screen for me sie zu einem Bild zusammenfügt"',
        "it": 'aria-label="Una pagina lunga che scorre dietro la pillola di registrazione mentre Screen for me la unisce in un\'unica immagine"'}),
    ('<section class="etc" aria-label="More details">', {
        "es": '<section class="etc" aria-label="Más detalles">',
        "fr": '<section class="etc" aria-label="Plus de détails">',
        "de": '<section class="etc" aria-label="Weitere Details">',
        "it": '<section class="etc" aria-label="Altri dettagli">'}),
    ("<li><strong>Lives in the menu bar.</strong> No Dock icon, no window until you need one.</li>", {
        "es": "<li><strong>Vive en la barra de menús.</strong> Sin icono en el Dock, sin ventanas hasta que las necesitas.</li>",
        "fr": "<li><strong>Vit dans la barre de menus.</strong> Pas d'icône dans le Dock, pas de fenêtre avant d'en avoir besoin.</li>",
        "de": "<li><strong>Lebt in der Menüleiste.</strong> Kein Dock-Symbol, kein Fenster, bis du eines brauchst.</li>",
        "it": "<li><strong>Vive nella barra dei menu.</strong> Nessuna icona nel Dock, nessuna finestra finché non ti serve.</li>"}),
    ("<li><strong>Capture history.</strong> Every screenshot kept and browsable until you say otherwise.</li>", {
        "es": "<li><strong>Historial de capturas.</strong> Cada captura se guarda y se puede consultar hasta que tú digas lo contrario.</li>",
        "fr": "<li><strong>Historique des captures.</strong> Chaque capture est conservée et consultable jusqu'à ce que vous en décidiez autrement.</li>",
        "de": "<li><strong>Aufnahmeverlauf.</strong> Jeder Screenshot bleibt erhalten und durchsuchbar, bis du es anders willst.</li>",
        "it": "<li><strong>Cronologia catture.</strong> Ogni screenshot resta salvato e consultabile finché non decidi altrimenti.</li>"}),
    ("<li><strong>Self-timer.</strong> Three seconds to open the menu you actually wanted to capture.</li>", {
        "es": "<li><strong>Temporizador.</strong> Tres segundos para abrir el menú que querías capturar.</li>",
        "fr": "<li><strong>Retardateur.</strong> Trois secondes pour ouvrir le menu que vous vouliez vraiment capturer.</li>",
        "de": "<li><strong>Selbstauslöser.</strong> Drei Sekunden, um das Menü zu öffnen, das du eigentlich aufnehmen wolltest.</li>",
        "it": "<li><strong>Autoscatto.</strong> Tre secondi per aprire il menu che volevi davvero catturare.</li>"}),
    ("<li><strong>Speaks your language.</strong> English, Español, Français, Deutsch, Italiano.</li>", {
        "es": "<li><strong>Habla tu idioma.</strong> English, Español, Français, Deutsch, Italiano.</li>",
        "fr": "<li><strong>Parle votre langue.</strong> English, Español, Français, Deutsch, Italiano.</li>",
        "de": "<li><strong>Spricht deine Sprache.</strong> English, Español, Français, Deutsch, Italiano.</li>",
        "it": "<li><strong>Parla la tua lingua.</strong> English, Español, Français, Deutsch, Italiano.</li>"}),
    ("<li><strong>Light and dark.</strong> Windows follow your system appearance.</li>", {
        "es": "<li><strong>Claro y oscuro.</strong> Las ventanas siguen la apariencia de tu sistema.</li>",
        "fr": "<li><strong>Clair et sombre.</strong> Les fenêtres suivent l'apparence de votre système.</li>",
        "de": "<li><strong>Hell und dunkel.</strong> Fenster folgen deinem System-Erscheinungsbild.</li>",
        "it": "<li><strong>Chiaro e scuro.</strong> Le finestre seguono l'aspetto del sistema.</li>"}),
    ("<li><strong>Small and fast.</strong> Built with Tauri and Rust: a native app, not a browser in a trench coat.</li>", {
        "es": "<li><strong>Pequeña y rápida.</strong> Construida con Tauri y Rust: una aplicación nativa, no un navegador disfrazado.</li>",
        "fr": "<li><strong>Petit et rapide.</strong> Construit avec Tauri et Rust : une application native, pas un navigateur déguisé.</li>",
        "de": "<li><strong>Klein und schnell.</strong> Gebaut mit Tauri und Rust: eine native App, kein verkleideter Browser.</li>",
        "it": "<li><strong>Piccola e veloce.</strong> Costruita con Tauri e Rust: un'app nativa, non un browser travestito.</li>"}),
    ("<h2>Free. Open. Yours.</h2>", {
        "es": "<h2>Gratis. Abierto. Tuyo.</h2>",
        "fr": "<h2>Gratuit. Ouvert. À vous.</h2>",
        "de": "<h2>Kostenlos. Offen. Deins.</h2>",
        "it": "<h2>Gratuita. Aperta. Tua.</h2>"}),
    ("""<p class="oss-lead">Screen for me is MIT-licensed and completely free — no trial, no Pro tier, no
      “unlock export” button. The entire source code is on GitHub: read it, build it yourself,
      fix the bug that annoys you, or just open an issue and tell us about it.</p>""", {
        "es": '<p class="oss-lead">Screen for me tiene licencia MIT y es completamente gratis — sin versión de prueba, sin plan Pro, sin botón de «desbloquear exportación». Todo el código fuente está en GitHub: léelo, compílalo tú mismo, arregla el fallo que te molesta o simplemente abre una issue y cuéntanoslo.</p>',
        "fr": '<p class="oss-lead">Screen for me est sous licence MIT et entièrement gratuit — pas d\'essai, pas de version Pro, pas de bouton « débloquer l\'export ». Tout le code source est sur GitHub : lisez-le, compilez-le vous-même, corrigez le bug qui vous agace, ou ouvrez simplement une issue pour nous en parler.</p>',
        "de": '<p class="oss-lead">Screen for me ist MIT-lizenziert und komplett kostenlos — keine Testversion, kein Pro-Tarif, kein „Export freischalten“-Knopf. Der gesamte Quellcode liegt auf GitHub: lies ihn, bau ihn selbst, behebe den Bug, der dich nervt, oder eröffne einfach ein Issue und erzähl uns davon.</p>',
        "it": '<p class="oss-lead">Screen for me ha licenza MIT ed è completamente gratuita — nessuna prova, nessun piano Pro, nessun pulsante «sblocca l\'esportazione». Tutto il codice sorgente è su GitHub: leggilo, compilalo da te, sistema il bug che ti infastidisce o apri semplicemente una issue e raccontacelo.</p>'}),
    ("Star it on GitHub", {
        "es": "Dale una estrella en GitHub", "fr": "Mettre une étoile sur GitHub",
        "de": "Auf GitHub einen Stern geben", "it": "Metti una stella su GitHub"}),
    (">Download the latest release</a>", {
        "es": ">Descargar la última versión</a>", "fr": ">Télécharger la dernière version</a>",
        "de": ">Neueste Version laden</a>", "it": ">Scarica l'ultima release</a>"}),
    ('aria-label="Footer">', {
        "es": 'aria-label="Pie de página">', "fr": 'aria-label="Pied de page">',
        "de": 'aria-label="Fußzeile">', "it": 'aria-label="Piè di pagina">'}),
    (">Releases</a>", {
        "es": ">Versiones</a>", "fr": ">Versions</a>",
        "de": ">Releases</a>", "it": ">Release</a>"}),
    (">MIT licence</a>", {
        "es": ">Licencia MIT</a>", "fr": ">Licence MIT</a>",
        "de": ">MIT-Lizenz</a>", "it": ">Licenza MIT</a>"}),
    ("Made by Mario &amp; Jorge Alvarez · Built with Tauri", {
        "es": "Desarrollado por Mario &amp; Jorge Alvarez · Construida con Tauri",
        "fr": "Développé par Mario &amp; Jorge Alvarez · Construit avec Tauri",
        "de": "Entwickelt von Mario &amp; Jorge Alvarez · Gebaut mit Tauri",
        "it": "Sviluppato da Mario &amp; Jorge Alvarez · Costruito con Tauri"}),
]

# path rewrites for subdirectory pages (count = expected occurrences)
PATHS = [
    ('href="assets/icon.png"', 'href="../assets/icon.png"', 1),
    ('src="assets/icon.png"', 'src="../assets/icon.png"', 2),
    ('href="site.css"', 'href="../site.css"', 1),
    ('src="site.js"', 'src="../site.js"', 1),
]

# canonical / og:url are per-language; the hreflang alternates are absolute and
# identical on every page, so they pass through untouched.
CANONICAL = [
    ('<link rel="canonical" href="https://screenforme.app/">',
     '<link rel="canonical" href="https://screenforme.app/{lang}/">'),
    ('<meta property="og:url" content="https://screenforme.app/">',
     '<meta property="og:url" content="https://screenforme.app/{lang}/">'),
]

FOOTER_LANG_SRC = """  <nav class="footer-lang" aria-label="Language">
    <span aria-current="page">EN</span>
    <a href="es/" lang="es" hreflang="es" aria-label="Español">ES</a>
    <a href="fr/" lang="fr" hreflang="fr" aria-label="Français">FR</a>
    <a href="de/" lang="de" hreflang="de" aria-label="Deutsch">DE</a>
    <a href="it/" lang="it" hreflang="it" aria-label="Italiano">IT</a>
  </nav>"""

LANG_NAMES = {"en": "English", "es": "Español", "fr": "Français", "de": "Deutsch", "it": "Italiano"}
NAV_LABEL = {"es": "Idioma", "fr": "Langue", "de": "Sprache", "it": "Lingua"}


def footer_lang_block(lang):
    lines = [f'  <nav class="footer-lang" aria-label="{NAV_LABEL[lang]}">']
    for code in ["en"] + LANGS:
        if code == lang:
            lines.append(f'    <span aria-current="page">{code.upper()}</span>')
        else:
            href = "../" if code == "en" else f"../{code}/"
            lines.append(f'    <a href="{href}" lang="{code}" hreflang="{code}" aria-label="{LANG_NAMES[code]}">{code.upper()}</a>')
    lines.append("  </nav>")
    return "\n".join(lines)


def flex_pattern(src):
    parts = re.split(r"\s+", src)
    return re.compile(r"\s+".join(re.escape(p) for p in parts))


errors = []
for lang in LANGS:
    html = SRC
    for src, targets in T:
        pat = flex_pattern(src)
        n = len(pat.findall(html))
        if n != 1:
            errors.append(f"[{lang}] expected 1 match, got {n}: {src[:70]!r}")
            continue
        html = pat.sub(lambda m: targets[lang], html, count=1)
    for src, dst, count in PATHS:
        n = html.count(src)
        if n != count:
            errors.append(f"[{lang}] path {src!r}: expected {count}, got {n}")
        html = html.replace(src, dst)
    for src, dst in CANONICAL:
        n = html.count(src)
        if n != 1:
            errors.append(f"[{lang}] canonical {src!r}: expected 1, got {n}")
        html = html.replace(src, dst.format(lang=lang))
    for block_src, block_dst in [
        (FOOTER_LANG_SRC, footer_lang_block(lang)),
    ]:
        pat = flex_pattern(block_src)
        if len(pat.findall(html)) != 1:
            errors.append(f"[{lang}] block not found: {block_src[:60]!r}")
        else:
            html = pat.sub(lambda m: block_dst, html, count=1)
    if not errors:
        out = ROOT / lang / "index.html"
        out.parent.mkdir(exist_ok=True)
        out.write_text(html, encoding="utf-8")
        print(f"wrote {out}")

if errors:
    print("\n".join(errors))
    sys.exit(1)
print("all pages generated, no unmatched strings")
