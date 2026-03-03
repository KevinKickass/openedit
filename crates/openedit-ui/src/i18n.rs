//! Internationalization (i18n) support for OpenEdit.
//!
//! Provides a simple HashMap-based translation system with per-locale string tables.
//! The active locale is stored in a thread-local and can be changed at runtime via
//! [`set_locale`]. Translated strings are retrieved with [`t`], which falls back to
//! English when a key is missing in the active locale.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

/// Supported locales.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Locale {
    English,
    German,
    French,
    Spanish,
    Japanese,
    Chinese,
    Korean,
    Portuguese,
    Russian,
}

impl Locale {
    /// All available locales, in display order.
    pub fn all() -> &'static [Locale] {
        &[
            Locale::English,
            Locale::German,
            Locale::French,
            Locale::Spanish,
            Locale::Japanese,
            Locale::Chinese,
            Locale::Korean,
            Locale::Portuguese,
            Locale::Russian,
        ]
    }

    /// The human-readable name of the locale (in its own language).
    pub fn display_name(self) -> &'static str {
        match self {
            Locale::English => "English",
            Locale::German => "Deutsch",
            Locale::French => "Fran\u{00E7}ais",
            Locale::Spanish => "Espa\u{00F1}ol",
            Locale::Japanese => "\u{65E5}\u{672C}\u{8A9E}",
            Locale::Chinese => "\u{4E2D}\u{6587}",
            Locale::Korean => "\u{D55C}\u{AD6D}\u{C5B4}",
            Locale::Portuguese => "Portugu\u{00EA}s",
            Locale::Russian => "\u{0420}\u{0443}\u{0441}\u{0441}\u{043A}\u{0438}\u{0439}",
        }
    }

    /// Short identifier used for config serialization (e.g. "en", "de").
    pub fn id(self) -> &'static str {
        match self {
            Locale::English => "en",
            Locale::German => "de",
            Locale::French => "fr",
            Locale::Spanish => "es",
            Locale::Japanese => "ja",
            Locale::Chinese => "zh",
            Locale::Korean => "ko",
            Locale::Portuguese => "pt",
            Locale::Russian => "ru",
        }
    }

    /// Parse a locale from its short id. Returns `None` for unknown ids.
    pub fn from_id(id: &str) -> Option<Locale> {
        match id {
            "en" => Some(Locale::English),
            "de" => Some(Locale::German),
            "fr" => Some(Locale::French),
            "es" => Some(Locale::Spanish),
            "ja" => Some(Locale::Japanese),
            "zh" => Some(Locale::Chinese),
            "ko" => Some(Locale::Korean),
            "pt" => Some(Locale::Portuguese),
            "ru" => Some(Locale::Russian),
            _ => None,
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

impl Default for Locale {
    fn default() -> Self {
        Locale::English
    }
}

// ── Thread-local active locale ──────────────────────────────────────────

thread_local! {
    static CURRENT_LOCALE: RefCell<Locale> = RefCell::new(Locale::English);
}

/// Set the active locale for the current thread.
pub fn set_locale(locale: Locale) {
    CURRENT_LOCALE.with(|cell| {
        *cell.borrow_mut() = locale;
    });
}

/// Get the active locale for the current thread.
pub fn get_locale() -> Locale {
    CURRENT_LOCALE.with(|cell| *cell.borrow())
}

// ── Translation tables ──────────────────────────────────────────────────

/// Holds translation strings for a single locale.
struct TranslationTable {
    map: HashMap<&'static str, &'static str>,
}

impl TranslationTable {
    fn new(entries: &[(&'static str, &'static str)]) -> Self {
        Self {
            map: entries.iter().copied().collect(),
        }
    }

    fn get(&self, key: &str) -> Option<&'static str> {
        self.map.get(key).copied()
    }
}

/// Container for all locale translation tables.
struct Translations {
    tables: HashMap<Locale, TranslationTable>,
}

impl Translations {
    fn new() -> Self {
        let mut tables = HashMap::new();
        tables.insert(Locale::English, TranslationTable::new(ENGLISH));
        tables.insert(Locale::German, TranslationTable::new(GERMAN));
        tables.insert(Locale::French, TranslationTable::new(FRENCH));
        tables.insert(Locale::Spanish, TranslationTable::new(SPANISH));
        tables.insert(Locale::Japanese, TranslationTable::new(JAPANESE));
        Self { tables }
    }

    fn get<'a>(&self, locale: Locale, key: &'a str) -> &'a str {
        // Try the requested locale first, then fall back to English.
        if let Some(table) = self.tables.get(&locale) {
            if let Some(val) = table.get(key) {
                return val;
            }
        }
        // Fallback to English
        if let Some(table) = self.tables.get(&Locale::English) {
            if let Some(val) = table.get(key) {
                return val;
            }
        }
        // If even English doesn't have it, return the key itself.
        // This is safe because the returned &str lives at least as long as the
        // key reference passed in, and our 'static translation values trivially
        // satisfy any shorter lifetime.
        key
    }
}

// Singleton translations (initialized on first use).
fn translations() -> &'static Translations {
    use std::sync::OnceLock;
    static INSTANCE: OnceLock<Translations> = OnceLock::new();
    INSTANCE.get_or_init(Translations::new)
}

/// Look up a translated string for the current locale.
///
/// Falls back to English if the key is not found in the active locale.
/// Falls back to returning the key itself if not found in English either.
///
/// # Example
/// ```
/// use openedit_ui::i18n::{t, set_locale, Locale};
/// set_locale(Locale::German);
/// assert_eq!(t("menu.file"), "Datei");
/// ```
pub fn t(key: &str) -> &str {
    let locale = get_locale();
    translations().get(locale, key)
}

// ── English translations (complete) ─────────────────────────────────────

static ENGLISH: &[(&str, &str)] = &[
    // Menu bar top-level items
    ("menu.file", "File"),
    ("menu.edit", "Edit"),
    ("menu.view", "View"),
    ("menu.selection", "Selection"),
    ("menu.go", "Go"),
    ("menu.terminal", "Terminal"),
    ("menu.settings", "Settings"),
    ("menu.help", "Help"),
    // File menu
    ("file.new", "New File"),
    ("file.open", "Open File"),
    ("file.open_folder", "Open Folder"),
    ("file.save", "Save"),
    ("file.save_as", "Save As"),
    ("file.close_tab", "Close Tab"),
    ("file.exit", "Exit"),
    // Edit menu
    ("edit.undo", "Undo"),
    ("edit.redo", "Redo"),
    ("edit.cut", "Cut"),
    ("edit.copy", "Copy"),
    ("edit.paste", "Paste"),
    ("edit.find", "Find"),
    ("edit.find_in_files", "Find in Files"),
    ("edit.replace", "Replace"),
    ("edit.select_all", "Select All"),
    ("edit.toggle_comment", "Toggle Comment"),
    // View menu
    ("view.command_palette", "Command Palette"),
    ("view.toggle_sidebar", "Toggle Sidebar"),
    ("view.toggle_terminal", "Toggle Terminal"),
    ("view.minimap", "Minimap"),
    ("view.line_numbers", "Line Numbers"),
    ("view.word_wrap", "Word Wrap"),
    ("view.markdown_preview", "Markdown Preview"),
    ("view.zen_mode", "Zen Mode"),
    ("view.zoom_in", "Zoom In"),
    ("view.zoom_out", "Zoom Out"),
    // Selection menu
    ("selection.add_cursor_above", "Add Cursor Above"),
    ("selection.add_cursor_below", "Add Cursor Below"),
    ("selection.select_all_occurrences", "Select All Occurrences"),
    ("selection.add_next_occurrence", "Add Next Occurrence"),
    // Go menu
    ("go.go_to_line", "Go to Line"),
    ("go.go_to_file", "Go to File"),
    ("go.go_to_definition", "Go to Definition"),
    ("go.go_to_references", "Go to References"),
    ("go.rename_symbol", "Rename Symbol"),
    // Terminal menu
    ("terminal.new", "New Terminal"),
    ("terminal.send_selection", "Send Selection to Terminal"),
    // Settings menu
    ("settings.theme", "Theme"),
    ("settings.vim_mode", "Vim Mode"),
    ("settings.font_size", "Font Size:"),
    ("settings.tab_size", "Tab Size"),
    ("settings.auto_save", "Auto Save"),
    ("settings.language", "Language"),
    ("settings.open_themes_folder", "Open Themes Folder..."),
    ("settings.create_theme", "Create Theme from Current..."),
    ("settings.reload_themes", "Reload Themes"),
    // Help menu
    ("help.about", "About OpenEdit"),
    ("help.shortcuts", "Keyboard Shortcuts"),
    // Status bar
    ("status.line", "Ln"),
    ("status.col", "Col"),
    ("status.chars_selected", "chars"),
    ("status.lines_selected", "lines selected"),
    ("status.read_only", "READ-ONLY"),
    ("status.recording", "REC"),
    ("status.plain_text", "Plain Text"),
    // Tab bar
    ("tab.close", "Close"),
    ("tab.close_others", "Close Others"),
    ("tab.close_all", "Close All"),
    ("tab.close_to_right", "Close to the Right"),
    ("tab.copy_path", "Copy Path"),
    ("tab.reveal_in_file_manager", "Reveal in File Manager"),
    ("tab.new_tab", "New Tab"),
    // Dialogs
    ("dialog.save", "Save"),
    ("dialog.dont_save", "Don't Save"),
    ("dialog.cancel", "Cancel"),
    ("dialog.ok", "OK"),
    ("dialog.yes", "Yes"),
    ("dialog.no", "No"),
    ("dialog.close", "Close"),
    ("dialog.reload", "Reload"),
    ("dialog.keep", "Keep"),
    ("dialog.unsaved_title", "Unsaved Changes"),
    ("dialog.unsaved_message", "Do you want to save changes before closing?"),
    ("dialog.file_changed_title", "File Changed"),
    ("dialog.file_changed_message", "This file has been modified externally. Reload?"),
    // Search panel
    ("search.find", "Find:"),
    ("search.replace_with", "Replace:"),
    ("search.previous", "Previous"),
    ("search.next", "Next"),
    ("search.replace", "Replace"),
    ("search.replace_all", "Replace All"),
    ("search.case_sensitive", "Case Sensitive"),
    ("search.whole_word", "Whole Word"),
    ("search.regex", "Regex"),
    // Command palette
    ("command_palette.hint", "Type a command..."),
    // Settings: Change Language command
    ("cmd.settings.change_language", "Settings: Change Language"),
];

// ── German translations (complete) ──────────────────────────────────────

static GERMAN: &[(&str, &str)] = &[
    // Menu bar
    ("menu.file", "Datei"),
    ("menu.edit", "Bearbeiten"),
    ("menu.view", "Ansicht"),
    ("menu.selection", "Auswahl"),
    ("menu.go", "Gehe zu"),
    ("menu.terminal", "Terminal"),
    ("menu.settings", "Einstellungen"),
    ("menu.help", "Hilfe"),
    // File menu
    ("file.new", "Neue Datei"),
    ("file.open", "Datei \u{00F6}ffnen"),
    ("file.open_folder", "Ordner \u{00F6}ffnen"),
    ("file.save", "Speichern"),
    ("file.save_as", "Speichern unter"),
    ("file.close_tab", "Tab schlie\u{00DF}en"),
    ("file.exit", "Beenden"),
    // Edit menu
    ("edit.undo", "R\u{00FC}ckg\u{00E4}ngig"),
    ("edit.redo", "Wiederholen"),
    ("edit.cut", "Ausschneiden"),
    ("edit.copy", "Kopieren"),
    ("edit.paste", "Einf\u{00FC}gen"),
    ("edit.find", "Suchen"),
    ("edit.find_in_files", "In Dateien suchen"),
    ("edit.replace", "Ersetzen"),
    ("edit.select_all", "Alles ausw\u{00E4}hlen"),
    ("edit.toggle_comment", "Kommentar umschalten"),
    // View menu
    ("view.command_palette", "Befehlspalette"),
    ("view.toggle_sidebar", "Seitenleiste umschalten"),
    ("view.toggle_terminal", "Terminal umschalten"),
    ("view.minimap", "Minimap"),
    ("view.line_numbers", "Zeilennummern"),
    ("view.word_wrap", "Zeilenumbruch"),
    ("view.markdown_preview", "Markdown-Vorschau"),
    ("view.zen_mode", "Zen-Modus"),
    ("view.zoom_in", "Vergr\u{00F6}\u{00DF}ern"),
    ("view.zoom_out", "Verkleinern"),
    // Selection menu
    ("selection.add_cursor_above", "Cursor dar\u{00FC}ber hinzuf\u{00FC}gen"),
    ("selection.add_cursor_below", "Cursor darunter hinzuf\u{00FC}gen"),
    ("selection.select_all_occurrences", "Alle Vorkommen ausw\u{00E4}hlen"),
    ("selection.add_next_occurrence", "N\u{00E4}chstes Vorkommen hinzuf\u{00FC}gen"),
    // Go menu
    ("go.go_to_line", "Gehe zu Zeile"),
    ("go.go_to_file", "Gehe zu Datei"),
    ("go.go_to_definition", "Gehe zu Definition"),
    ("go.go_to_references", "Gehe zu Referenzen"),
    ("go.rename_symbol", "Symbol umbenennen"),
    // Terminal menu
    ("terminal.new", "Neues Terminal"),
    ("terminal.send_selection", "Auswahl an Terminal senden"),
    // Settings menu
    ("settings.theme", "Design"),
    ("settings.vim_mode", "Vim-Modus"),
    ("settings.font_size", "Schriftgr\u{00F6}\u{00DF}e:"),
    ("settings.tab_size", "Tabgr\u{00F6}\u{00DF}e"),
    ("settings.auto_save", "Automatisch speichern"),
    ("settings.language", "Sprache"),
    ("settings.open_themes_folder", "Design-Ordner \u{00F6}ffnen\u{2026}"),
    ("settings.create_theme", "Design aus aktuellem erstellen\u{2026}"),
    ("settings.reload_themes", "Designs neu laden"),
    // Help menu
    ("help.about", "\u{00DC}ber OpenEdit"),
    ("help.shortcuts", "Tastaturk\u{00FC}rzel"),
    // Status bar
    ("status.line", "Zl"),
    ("status.col", "Sp"),
    ("status.chars_selected", "Zeichen"),
    ("status.lines_selected", "Zeilen ausgew\u{00E4}hlt"),
    ("status.read_only", "SCHREIBGESCH\u{00DC}TZT"),
    ("status.recording", "AUFN"),
    ("status.plain_text", "Nur Text"),
    // Tab bar
    ("tab.close", "Schlie\u{00DF}en"),
    ("tab.close_others", "Andere schlie\u{00DF}en"),
    ("tab.close_all", "Alle schlie\u{00DF}en"),
    ("tab.close_to_right", "Rechts davon schlie\u{00DF}en"),
    ("tab.copy_path", "Pfad kopieren"),
    ("tab.reveal_in_file_manager", "Im Dateimanager anzeigen"),
    ("tab.new_tab", "Neuer Tab"),
    // Dialogs
    ("dialog.save", "Speichern"),
    ("dialog.dont_save", "Nicht speichern"),
    ("dialog.cancel", "Abbrechen"),
    ("dialog.ok", "OK"),
    ("dialog.yes", "Ja"),
    ("dialog.no", "Nein"),
    ("dialog.close", "Schlie\u{00DF}en"),
    ("dialog.reload", "Neu laden"),
    ("dialog.keep", "Behalten"),
    ("dialog.unsaved_title", "Ungespeicherte \u{00C4}nderungen"),
    ("dialog.unsaved_message", "M\u{00F6}chten Sie die \u{00C4}nderungen vor dem Schlie\u{00DF}en speichern?"),
    ("dialog.file_changed_title", "Datei ge\u{00E4}ndert"),
    ("dialog.file_changed_message", "Diese Datei wurde extern ge\u{00E4}ndert. Neu laden?"),
    // Search panel
    ("search.find", "Suchen:"),
    ("search.replace_with", "Ersetzen:"),
    ("search.previous", "Vorheriges"),
    ("search.next", "N\u{00E4}chstes"),
    ("search.replace", "Ersetzen"),
    ("search.replace_all", "Alle ersetzen"),
    ("search.case_sensitive", "Gro\u{00DF}/Klein"),
    ("search.whole_word", "Ganzes Wort"),
    ("search.regex", "Regex"),
    // Command palette
    ("command_palette.hint", "Befehl eingeben\u{2026}"),
    // Settings: Change Language command
    ("cmd.settings.change_language", "Einstellungen: Sprache \u{00E4}ndern"),
];

// ── French translations (partial — menus, dialogs, common items) ────────

static FRENCH: &[(&str, &str)] = &[
    // Menu bar
    ("menu.file", "Fichier"),
    ("menu.edit", "\u{00C9}dition"),
    ("menu.view", "Affichage"),
    ("menu.selection", "S\u{00E9}lection"),
    ("menu.go", "Aller"),
    ("menu.terminal", "Terminal"),
    ("menu.settings", "Param\u{00E8}tres"),
    ("menu.help", "Aide"),
    // File menu
    ("file.new", "Nouveau fichier"),
    ("file.open", "Ouvrir un fichier"),
    ("file.open_folder", "Ouvrir un dossier"),
    ("file.save", "Enregistrer"),
    ("file.save_as", "Enregistrer sous"),
    ("file.close_tab", "Fermer l'onglet"),
    ("file.exit", "Quitter"),
    // Edit menu
    ("edit.undo", "Annuler"),
    ("edit.redo", "R\u{00E9}tablir"),
    ("edit.cut", "Couper"),
    ("edit.copy", "Copier"),
    ("edit.paste", "Coller"),
    ("edit.find", "Rechercher"),
    ("edit.find_in_files", "Rechercher dans les fichiers"),
    ("edit.replace", "Remplacer"),
    ("edit.select_all", "Tout s\u{00E9}lectionner"),
    ("edit.toggle_comment", "Basculer le commentaire"),
    // View menu
    ("view.command_palette", "Palette de commandes"),
    ("view.toggle_sidebar", "Basculer la barre lat\u{00E9}rale"),
    ("view.toggle_terminal", "Basculer le terminal"),
    ("view.minimap", "Minimap"),
    ("view.line_numbers", "Num\u{00E9}ros de ligne"),
    ("view.word_wrap", "Retour \u{00E0} la ligne"),
    ("view.markdown_preview", "Aper\u{00E7}u Markdown"),
    ("view.zen_mode", "Mode Zen"),
    ("view.zoom_in", "Zoom avant"),
    ("view.zoom_out", "Zoom arri\u{00E8}re"),
    // Selection menu
    ("selection.add_cursor_above", "Ajouter un curseur au-dessus"),
    ("selection.add_cursor_below", "Ajouter un curseur en dessous"),
    ("selection.select_all_occurrences", "S\u{00E9}lectionner toutes les occurrences"),
    ("selection.add_next_occurrence", "Ajouter l'occurrence suivante"),
    // Go menu
    ("go.go_to_line", "Aller \u{00E0} la ligne"),
    ("go.go_to_file", "Aller au fichier"),
    ("go.go_to_definition", "Aller \u{00E0} la d\u{00E9}finition"),
    ("go.go_to_references", "Aller aux r\u{00E9}f\u{00E9}rences"),
    ("go.rename_symbol", "Renommer le symbole"),
    // Terminal menu
    ("terminal.new", "Nouveau terminal"),
    ("terminal.send_selection", "Envoyer la s\u{00E9}lection au terminal"),
    // Settings menu
    ("settings.theme", "Th\u{00E8}me"),
    ("settings.vim_mode", "Mode Vim"),
    ("settings.font_size", "Taille de police\u{00A0}:"),
    ("settings.tab_size", "Taille de tabulation"),
    ("settings.auto_save", "Sauvegarde automatique"),
    ("settings.language", "Langue"),
    ("settings.open_themes_folder", "Ouvrir le dossier des th\u{00E8}mes\u{2026}"),
    ("settings.create_theme", "Cr\u{00E9}er un th\u{00E8}me depuis l'actuel\u{2026}"),
    ("settings.reload_themes", "Recharger les th\u{00E8}mes"),
    // Help menu
    ("help.about", "\u{00C0} propos d'OpenEdit"),
    ("help.shortcuts", "Raccourcis clavier"),
    // Status bar
    ("status.line", "Ln"),
    ("status.col", "Col"),
    ("status.chars_selected", "caract\u{00E8}res"),
    ("status.lines_selected", "lignes s\u{00E9}lectionn\u{00E9}es"),
    ("status.read_only", "LECTURE SEULE"),
    ("status.recording", "ENR"),
    ("status.plain_text", "Texte brut"),
    // Dialogs
    ("dialog.save", "Enregistrer"),
    ("dialog.dont_save", "Ne pas enregistrer"),
    ("dialog.cancel", "Annuler"),
    ("dialog.ok", "OK"),
    ("dialog.yes", "Oui"),
    ("dialog.no", "Non"),
    ("dialog.close", "Fermer"),
    ("dialog.reload", "Recharger"),
    ("dialog.keep", "Garder"),
    ("dialog.unsaved_title", "Modifications non enregistr\u{00E9}es"),
    ("dialog.unsaved_message", "Voulez-vous enregistrer les modifications avant de fermer\u{00A0}?"),
    ("dialog.file_changed_title", "Fichier modifi\u{00E9}"),
    ("dialog.file_changed_message", "Ce fichier a \u{00E9}t\u{00E9} modifi\u{00E9} en externe. Recharger\u{00A0}?"),
    // Search panel
    ("search.find", "Rechercher\u{00A0}:"),
    ("search.replace_with", "Remplacer\u{00A0}:"),
    ("search.previous", "Pr\u{00E9}c\u{00E9}dent"),
    ("search.next", "Suivant"),
    ("search.replace", "Remplacer"),
    ("search.replace_all", "Tout remplacer"),
    ("search.case_sensitive", "Respect. casse"),
    ("search.whole_word", "Mot entier"),
    ("search.regex", "Regex"),
    // Command palette
    ("command_palette.hint", "Saisissez une commande\u{2026}"),
    // Settings: Change Language command
    ("cmd.settings.change_language", "Param\u{00E8}tres\u{00A0}: Changer la langue"),
];

// ── Spanish translations (partial — menus, dialogs) ─────────────────────

static SPANISH: &[(&str, &str)] = &[
    // Menu bar
    ("menu.file", "Archivo"),
    ("menu.edit", "Editar"),
    ("menu.view", "Ver"),
    ("menu.selection", "Selecci\u{00F3}n"),
    ("menu.go", "Ir"),
    ("menu.terminal", "Terminal"),
    ("menu.settings", "Configuraci\u{00F3}n"),
    ("menu.help", "Ayuda"),
    // File menu
    ("file.new", "Nuevo archivo"),
    ("file.open", "Abrir archivo"),
    ("file.open_folder", "Abrir carpeta"),
    ("file.save", "Guardar"),
    ("file.save_as", "Guardar como"),
    ("file.close_tab", "Cerrar pesta\u{00F1}a"),
    ("file.exit", "Salir"),
    // Edit menu
    ("edit.undo", "Deshacer"),
    ("edit.redo", "Rehacer"),
    ("edit.cut", "Cortar"),
    ("edit.copy", "Copiar"),
    ("edit.paste", "Pegar"),
    ("edit.find", "Buscar"),
    ("edit.find_in_files", "Buscar en archivos"),
    ("edit.replace", "Reemplazar"),
    ("edit.select_all", "Seleccionar todo"),
    ("edit.toggle_comment", "Alternar comentario"),
    // View menu
    ("view.command_palette", "Paleta de comandos"),
    ("view.toggle_sidebar", "Alternar barra lateral"),
    ("view.toggle_terminal", "Alternar terminal"),
    ("view.minimap", "Minimapa"),
    ("view.line_numbers", "N\u{00FA}meros de l\u{00ED}nea"),
    ("view.word_wrap", "Ajuste de l\u{00ED}nea"),
    ("view.markdown_preview", "Vista previa Markdown"),
    ("view.zen_mode", "Modo Zen"),
    ("view.zoom_in", "Acercar"),
    ("view.zoom_out", "Alejar"),
    // Selection menu
    ("selection.add_cursor_above", "A\u{00F1}adir cursor arriba"),
    ("selection.add_cursor_below", "A\u{00F1}adir cursor abajo"),
    ("selection.select_all_occurrences", "Seleccionar todas las ocurrencias"),
    ("selection.add_next_occurrence", "A\u{00F1}adir siguiente ocurrencia"),
    // Go menu
    ("go.go_to_line", "Ir a l\u{00ED}nea"),
    ("go.go_to_file", "Ir a archivo"),
    ("go.go_to_definition", "Ir a definici\u{00F3}n"),
    ("go.go_to_references", "Ir a referencias"),
    ("go.rename_symbol", "Renombrar s\u{00ED}mbolo"),
    // Terminal menu
    ("terminal.new", "Nuevo terminal"),
    ("terminal.send_selection", "Enviar selecci\u{00F3}n al terminal"),
    // Settings menu
    ("settings.theme", "Tema"),
    ("settings.vim_mode", "Modo Vim"),
    ("settings.font_size", "Tama\u{00F1}o de fuente:"),
    ("settings.tab_size", "Tama\u{00F1}o de tabulaci\u{00F3}n"),
    ("settings.auto_save", "Guardado autom\u{00E1}tico"),
    ("settings.language", "Idioma"),
    ("settings.open_themes_folder", "Abrir carpeta de temas\u{2026}"),
    ("settings.create_theme", "Crear tema desde el actual\u{2026}"),
    ("settings.reload_themes", "Recargar temas"),
    // Help menu
    ("help.about", "Acerca de OpenEdit"),
    ("help.shortcuts", "Atajos de teclado"),
    // Dialogs
    ("dialog.save", "Guardar"),
    ("dialog.dont_save", "No guardar"),
    ("dialog.cancel", "Cancelar"),
    ("dialog.ok", "Aceptar"),
    ("dialog.yes", "S\u{00ED}"),
    ("dialog.no", "No"),
    ("dialog.close", "Cerrar"),
    ("dialog.reload", "Recargar"),
    ("dialog.keep", "Mantener"),
    ("dialog.unsaved_title", "Cambios sin guardar"),
    ("dialog.unsaved_message", "\u{00BF}Desea guardar los cambios antes de cerrar?"),
    ("dialog.file_changed_title", "Archivo modificado"),
    ("dialog.file_changed_message", "Este archivo ha sido modificado externamente. \u{00BF}Recargar?"),
    // Search panel
    ("search.find", "Buscar:"),
    ("search.replace_with", "Reemplazar:"),
    ("search.previous", "Anterior"),
    ("search.next", "Siguiente"),
    ("search.replace", "Reemplazar"),
    ("search.replace_all", "Reemplazar todo"),
    ("search.case_sensitive", "May\u{00FA}sculas"),
    ("search.whole_word", "Palabra completa"),
    ("search.regex", "Regex"),
    // Command palette
    ("command_palette.hint", "Escriba un comando\u{2026}"),
    // Settings: Change Language command
    ("cmd.settings.change_language", "Configuraci\u{00F3}n: Cambiar idioma"),
];

// ── Japanese translations (partial — menus, dialogs) ────────────────────

static JAPANESE: &[(&str, &str)] = &[
    // Menu bar
    ("menu.file", "\u{30D5}\u{30A1}\u{30A4}\u{30EB}"),
    ("menu.edit", "\u{7DE8}\u{96C6}"),
    ("menu.view", "\u{8868}\u{793A}"),
    ("menu.selection", "\u{9078}\u{629E}"),
    ("menu.go", "\u{79FB}\u{52D5}"),
    ("menu.terminal", "\u{30BF}\u{30FC}\u{30DF}\u{30CA}\u{30EB}"),
    ("menu.settings", "\u{8A2D}\u{5B9A}"),
    ("menu.help", "\u{30D8}\u{30EB}\u{30D7}"),
    // File menu
    ("file.new", "\u{65B0}\u{898F}\u{30D5}\u{30A1}\u{30A4}\u{30EB}"),
    ("file.open", "\u{30D5}\u{30A1}\u{30A4}\u{30EB}\u{3092}\u{958B}\u{304F}"),
    ("file.open_folder", "\u{30D5}\u{30A9}\u{30EB}\u{30C0}\u{30FC}\u{3092}\u{958B}\u{304F}"),
    ("file.save", "\u{4FDD}\u{5B58}"),
    ("file.save_as", "\u{540D}\u{524D}\u{3092}\u{4ED8}\u{3051}\u{3066}\u{4FDD}\u{5B58}"),
    ("file.close_tab", "\u{30BF}\u{30D6}\u{3092}\u{9589}\u{3058}\u{308B}"),
    ("file.exit", "\u{7D42}\u{4E86}"),
    // Edit menu
    ("edit.undo", "\u{5143}\u{306B}\u{623B}\u{3059}"),
    ("edit.redo", "\u{3084}\u{308A}\u{76F4}\u{3057}"),
    ("edit.cut", "\u{5207}\u{308A}\u{53D6}\u{308A}"),
    ("edit.copy", "\u{30B3}\u{30D4}\u{30FC}"),
    ("edit.paste", "\u{8CBC}\u{308A}\u{4ED8}\u{3051}"),
    ("edit.find", "\u{691C}\u{7D22}"),
    ("edit.find_in_files", "\u{30D5}\u{30A1}\u{30A4}\u{30EB}\u{5185}\u{691C}\u{7D22}"),
    ("edit.replace", "\u{7F6E}\u{63DB}"),
    ("edit.select_all", "\u{3059}\u{3079}\u{3066}\u{9078}\u{629E}"),
    ("edit.toggle_comment", "\u{30B3}\u{30E1}\u{30F3}\u{30C8}\u{5207}\u{308A}\u{66FF}\u{3048}"),
    // View menu
    ("view.command_palette", "\u{30B3}\u{30DE}\u{30F3}\u{30C9}\u{30D1}\u{30EC}\u{30C3}\u{30C8}"),
    ("view.toggle_sidebar", "\u{30B5}\u{30A4}\u{30C9}\u{30D0}\u{30FC}\u{5207}\u{308A}\u{66FF}\u{3048}"),
    ("view.toggle_terminal", "\u{30BF}\u{30FC}\u{30DF}\u{30CA}\u{30EB}\u{5207}\u{308A}\u{66FF}\u{3048}"),
    ("view.minimap", "\u{30DF}\u{30CB}\u{30DE}\u{30C3}\u{30D7}"),
    ("view.line_numbers", "\u{884C}\u{756A}\u{53F7}"),
    ("view.word_wrap", "\u{6298}\u{308A}\u{8FD4}\u{3057}"),
    ("view.markdown_preview", "Markdown\u{30D7}\u{30EC}\u{30D3}\u{30E5}\u{30FC}"),
    ("view.zen_mode", "Zen\u{30E2}\u{30FC}\u{30C9}"),
    ("view.zoom_in", "\u{62E1}\u{5927}"),
    ("view.zoom_out", "\u{7E2E}\u{5C0F}"),
    // Settings menu
    ("settings.theme", "\u{30C6}\u{30FC}\u{30DE}"),
    ("settings.vim_mode", "Vim\u{30E2}\u{30FC}\u{30C9}"),
    ("settings.font_size", "\u{30D5}\u{30A9}\u{30F3}\u{30C8}\u{30B5}\u{30A4}\u{30BA}:"),
    ("settings.tab_size", "\u{30BF}\u{30D6}\u{30B5}\u{30A4}\u{30BA}"),
    ("settings.auto_save", "\u{81EA}\u{52D5}\u{4FDD}\u{5B58}"),
    ("settings.language", "\u{8A00}\u{8A9E}"),
    // Help menu
    ("help.about", "OpenEdit\u{306B}\u{3064}\u{3044}\u{3066}"),
    ("help.shortcuts", "\u{30AD}\u{30FC}\u{30DC}\u{30FC}\u{30C9}\u{30B7}\u{30E7}\u{30FC}\u{30C8}\u{30AB}\u{30C3}\u{30C8}"),
    // Dialogs
    ("dialog.save", "\u{4FDD}\u{5B58}"),
    ("dialog.dont_save", "\u{4FDD}\u{5B58}\u{3057}\u{306A}\u{3044}"),
    ("dialog.cancel", "\u{30AD}\u{30E3}\u{30F3}\u{30BB}\u{30EB}"),
    ("dialog.ok", "OK"),
    ("dialog.yes", "\u{306F}\u{3044}"),
    ("dialog.no", "\u{3044}\u{3044}\u{3048}"),
    ("dialog.close", "\u{9589}\u{3058}\u{308B}"),
    ("dialog.reload", "\u{518D}\u{8AAD}\u{307F}\u{8FBC}\u{307F}"),
    ("dialog.keep", "\u{4FDD}\u{6301}"),
    ("dialog.unsaved_title", "\u{672A}\u{4FDD}\u{5B58}\u{306E}\u{5909}\u{66F4}"),
    ("dialog.unsaved_message", "\u{9589}\u{3058}\u{308B}\u{524D}\u{306B}\u{5909}\u{66F4}\u{3092}\u{4FDD}\u{5B58}\u{3057}\u{307E}\u{3059}\u{304B}\u{FF1F}"),
    // Search panel
    ("search.find", "\u{691C}\u{7D22}:"),
    ("search.replace_with", "\u{7F6E}\u{63DB}:"),
    ("search.replace", "\u{7F6E}\u{63DB}"),
    ("search.replace_all", "\u{3059}\u{3079}\u{3066}\u{7F6E}\u{63DB}"),
    // Command palette
    ("command_palette.hint", "\u{30B3}\u{30DE}\u{30F3}\u{30C9}\u{3092}\u{5165}\u{529B}\u{2026}"),
    // Settings: Change Language command
    ("cmd.settings.change_language", "\u{8A2D}\u{5B9A}: \u{8A00}\u{8A9E}\u{3092}\u{5909}\u{66F4}"),
];

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_lookup() {
        set_locale(Locale::English);
        assert_eq!(t("menu.file"), "File");
        assert_eq!(t("menu.edit"), "Edit");
        assert_eq!(t("file.save"), "Save");
        assert_eq!(t("dialog.cancel"), "Cancel");
    }

    #[test]
    fn german_lookup() {
        set_locale(Locale::German);
        assert_eq!(t("menu.file"), "Datei");
        assert_eq!(t("menu.edit"), "Bearbeiten");
        assert_eq!(t("file.save"), "Speichern");
        assert_eq!(t("dialog.cancel"), "Abbrechen");
    }

    #[test]
    fn french_lookup() {
        set_locale(Locale::French);
        assert_eq!(t("menu.file"), "Fichier");
        assert_eq!(t("file.save"), "Enregistrer");
        assert_eq!(t("dialog.cancel"), "Annuler");
    }

    #[test]
    fn spanish_lookup() {
        set_locale(Locale::Spanish);
        assert_eq!(t("menu.file"), "Archivo");
        assert_eq!(t("file.save"), "Guardar");
        assert_eq!(t("dialog.cancel"), "Cancelar");
    }

    #[test]
    fn japanese_lookup() {
        set_locale(Locale::Japanese);
        assert_eq!(t("menu.file"), "\u{30D5}\u{30A1}\u{30A4}\u{30EB}");
        assert_eq!(t("file.save"), "\u{4FDD}\u{5B58}");
    }

    #[test]
    fn fallback_to_english_for_missing_key() {
        set_locale(Locale::Japanese);
        // Japanese doesn't have this key, should fall back to English
        assert_eq!(t("status.read_only"), "READ-ONLY");
    }

    #[test]
    fn fallback_returns_key_for_unknown() {
        set_locale(Locale::English);
        // Completely unknown key — returns the key string itself
        assert_eq!(t("nonexistent.key.xyz"), "nonexistent.key.xyz");
    }

    #[test]
    fn locale_from_id_roundtrip() {
        for locale in Locale::all() {
            let id = locale.id();
            let parsed = Locale::from_id(id).expect("should parse");
            assert_eq!(*locale, parsed);
        }
    }

    #[test]
    fn locale_from_id_unknown() {
        assert!(Locale::from_id("xx").is_none());
        assert!(Locale::from_id("").is_none());
    }

    #[test]
    fn locale_display_name_nonempty() {
        for locale in Locale::all() {
            assert!(!locale.display_name().is_empty());
        }
    }

    #[test]
    fn locale_default_is_english() {
        assert_eq!(Locale::default(), Locale::English);
    }

    #[test]
    fn all_english_keys_present() {
        // Verify every key used in other locales also exists in English
        let en_table = TranslationTable::new(ENGLISH);
        for &(key, _) in GERMAN {
            assert!(
                en_table.get(key).is_some(),
                "German key '{}' missing from English",
                key
            );
        }
        for &(key, _) in FRENCH {
            assert!(
                en_table.get(key).is_some(),
                "French key '{}' missing from English",
                key
            );
        }
        for &(key, _) in SPANISH {
            assert!(
                en_table.get(key).is_some(),
                "Spanish key '{}' missing from English",
                key
            );
        }
        for &(key, _) in JAPANESE {
            assert!(
                en_table.get(key).is_some(),
                "Japanese key '{}' missing from English",
                key
            );
        }
    }

    #[test]
    fn set_and_get_locale() {
        set_locale(Locale::French);
        assert_eq!(get_locale(), Locale::French);
        set_locale(Locale::English);
        assert_eq!(get_locale(), Locale::English);
    }

    #[test]
    fn unsupported_locale_falls_back_to_english() {
        // Korean has no translation table entries, so everything falls back
        set_locale(Locale::Korean);
        assert_eq!(t("menu.file"), "File");
        assert_eq!(t("file.save"), "Save");
    }
}
