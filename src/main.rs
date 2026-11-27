//! Zettui: local notes, Vim editing, wiki-link completion, Zotero, and themes.

mod app;
mod bibliography;
mod buffer_switcher;
mod buffers;
mod cli;
mod command_palette;
mod commands;
mod config;
mod document_export;
mod editor;
mod editor_search;
mod file_ops;
mod focus;
mod fuzzy;
mod heading_search;
mod layout;
mod links;
mod markdown;
mod outline;
mod search;
mod session;
mod snippets;
mod statistics_cli;
mod system_theme;
mod theme;

fn main() -> std::io::Result<()> {
    if statistics_cli::run()? {
        return Ok(());
    }
    app::start()
}
