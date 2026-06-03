use anyhow::Result;
use chrono::Local;
use clap::{Parser, Subcommand};
use saga_core::model::{Echo, EchoContent, PlainData, Section};
use saga_storage_sqlite::Storage;

#[derive(Parser)]
#[command(name = "saga")]
#[command(about = "A privacy-first journaling app", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new section
    Section {
        #[command(subcommand)]
        action: SectionAction,
    },
    /// Manage echoes
    Echo {
        #[command(subcommand)]
        action: EchoAction,
    },
}

#[derive(Subcommand)]
enum SectionAction {
    /// Create a new section
    #[command(name = "new")]
    Create {
        /// Section name
        name: String,
    },
    /// List all sections
    List,
}

#[derive(Subcommand)]
enum EchoAction {
    /// Create a new echo
    #[command(name = "new")]
    Create {
        /// Section name
        #[arg(short, long)]
        section: String,
        /// Echo title
        title: String,
        /// Echo content
        text: String,
    },
    /// List today's echoes
    Today,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let db_path = dirs::home_dir()
        .expect("Could not find home directory")
        .join(".saga")
        .join("saga.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    {
        let storage = Storage::new(&db_path)?;
        match cli.command {
            Commands::Section { action } => match action {
                SectionAction::Create { name } => {
                    let sort_order = storage.get_next_sort_order()?;
                    let new_section = Section::new(name, sort_order);
                    storage.save_section(&new_section)?;
                    println!("Creating section: {:?}", new_section);
                }
                SectionAction::List => {
                    match storage.get_all_sections() {
                        Ok(sections) if sections.is_empty() => println!("No sections found."),
                        Ok(sections) => {
                            println!("Sections:");
                            for section in &sections {
                                println!("  - {} (id: {}, order: {})", section.name, section.id, section.sort_order);
                            }
                        }
                        Err(e) => return Err(e.into()),
                    }
                }
            },
            Commands::Echo { action } => match action {
                EchoAction::Create { section, title, text } => {
                    let found_section = find_section_by_name(&storage, &section)?;
                    let new_echo = Echo::new(
                        Local::now().date_naive(),
                        found_section.id,
                        title,
                        EchoContent::PlainEcho(PlainData { markdown: text }),
                    );
                    storage.save_echo(&new_echo).expect("Failed to save Echo");
                    println!("Created echo in section '{}': {}", section, new_echo);
                }
                EchoAction::Today => {
                    let today = Local::now().date_naive();
                    let echoes = storage.get_echoes_for_day(today)?;
                    if echoes.is_empty() {
                        println!("No echoes for today.");
                        return Ok(());
                    }
                    let sections = storage.get_all_sections()?;
                    println!("\nToday's Echoes ({}):\n", today.format("%B %e, %Y"));
                    for echo in echoes {
                        let section_name = sections.iter()
                            .find(|s| s.id == echo.section_id)
                            .map(|s| s.name.as_str())
                            .unwrap_or("Unknown");
                        let preview = match &echo.content {
                            EchoContent::PlainEcho(data) => data.markdown.as_str(),
                            EchoContent::MeditationEcho(data) => data.markdown.as_deref().unwrap_or("(no notes)"),
                            EchoContent::TaskEcho(data) => data.title.as_str(),
                            EchoContent::WorkoutEcho(data) => data.notes.as_deref().unwrap_or("(no notes)"),
                        };
                        println!("[{}] {} — {}", section_name, echo.content_type_name(), preview);
                        println!("  Created: {}\n", echo.created_at.format("%l:%M %p"));
                    }
                }
            },
        }
    }
    Ok(())
}

fn find_section_by_name(storage: &Storage, section_name: &str) -> Result<Section> {
    storage.get_all_sections()?.into_iter()
        .find(|s| s.name.eq_ignore_ascii_case(section_name))
        .ok_or_else(|| anyhow::anyhow!(
            "Section '{}' not found. Create it first with: saga section new \"{}\"",
            section_name, section_name
        ))
}
