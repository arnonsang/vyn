use comfy_table::{Attribute, Cell, CellAlignment, Color, ContentArrangement, Table};
use console::{Style, Term};
use indicatif::{ProgressBar, ProgressStyle};

/// Style
fn s_green_bold() -> Style { Style::new().green().bold() }
fn s_yellow_bold() -> Style { Style::new().yellow().bold() }
fn s_red_bold() -> Style { Style::new().red().bold() }
fn s_cyan_bold() -> Style { Style::new().cyan().bold() }
fn s_dim() -> Style { Style::new().dim() }
fn s_bold() -> Style { Style::new().bold() }

/// Banner with a cyan line and the command name in bold
pub fn print_banner(command: &str) {
    let term_width = Term::stdout().size().1 as usize;
    let line = "─".repeat(term_width.min(60));
    println!("{}", s_dim().apply_to(&line));
    println!(
        "  {} {}",
        s_cyan_bold().apply_to("vyn"),
        s_bold().apply_to(command)
    );
    println!("{}", s_dim().apply_to(&line));
}

// st command
pub fn print_status_added(path: &str) {
    println!("  {} {}", s_green_bold().apply_to("+ added   "), path);
}

pub fn print_status_modified(path: &str) {
    println!("  {} {}", s_yellow_bold().apply_to("~ modified"), path);
}

pub fn print_status_deleted(path: &str) {
    println!("  {} {}", s_red_bold().apply_to("- deleted "), path);
}

pub fn print_status_clean() {
    println!(
        "\n  {} nothing to push\n",
        s_green_bold().apply_to("✔")
    );
}

pub fn print_binary_modified(path: &str, old_size: usize, new_size: usize) {
    println!(
        "  {} {} {} → {} bytes",
        s_yellow_bold().apply_to("~ binary "),
        path,
        s_dim().apply_to(format!("{old_size}")),
        s_dim().apply_to(format!("{new_size}")),
    );
}

/// Diff output
pub fn print_diff_header(path: &str) {
    println!(
        "\n{} {}",
        s_cyan_bold().apply_to("━━ diff:"),
        s_bold().apply_to(path)
    );
}

pub fn print_diff_text(diff: &str) {
    let add = Style::new().green();
    let del = Style::new().red();
    let meta = Style::new().cyan();
    let hunk = Style::new().cyan().dim();
    let dim = Style::new().dim();

    for line in diff.lines() {
        if line.starts_with("+++") || line.starts_with("---") {
            println!("{}", meta.apply_to(line));
        } else if line.starts_with("@@") {
            println!("{}", hunk.apply_to(line));
        } else if line.starts_with('+') {
            println!("{}", add.apply_to(line));
        } else if line.starts_with('-') {
            println!("{}", del.apply_to(line));
        } else {
            println!("{}", dim.apply_to(line));
        }
    }
}

/// Progress bars
pub fn new_spinner(msg: &str) -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(std::time::Duration::from_millis(80));
    pb
}

pub fn new_progress_bar(total: u64, msg: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "  {msg:20} [{bar:30.cyan/dim}] {pos}/{len} {elapsed_precise}",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏ "),
    );
    pb.set_message(msg.to_string());
    pb
}

pub fn finish_progress(pb: &ProgressBar, msg: &str) {
    pb.finish_with_message(format!("{} {}", s_green_bold().apply_to("✔"), msg));
}

pub fn fail_progress(pb: &ProgressBar, msg: &str) {
    pb.finish_with_message(format!("{} {}", s_red_bold().apply_to("✗"), msg));
}

/// Summary line
pub fn print_success(msg: &str) {
    println!("\n  {} {}\n", s_green_bold().apply_to("✔"), msg);
}

pub fn print_warning(msg: &str) {
    println!("  {} {}", s_yellow_bold().apply_to("⚠"), msg);
}

pub fn print_error(msg: &str) {
    println!("  {} {}", s_red_bold().apply_to("✗"), msg);
}

pub fn print_info(label: &str, value: &str) {
    println!("  {:<18} {}", s_dim().apply_to(label), value);
}


/// Styled table
pub fn make_table(headers: &[&str]) -> Table {
    let mut table = Table::new();
    table.set_content_arrangement(ContentArrangement::Dynamic);
    table.load_preset(comfy_table::presets::UTF8_BORDERS_ONLY);
    table.set_header(
        headers
            .iter()
            .map(|h| {
                Cell::new(h)
                    .fg(Color::Cyan)
                    .add_attribute(Attribute::Bold)
                    .set_alignment(CellAlignment::Left)
            })
            .collect::<Vec<_>>(),
    );
    table
}

pub fn print_table(table: &Table) {
    for line in table.to_string().lines() {
        println!("  {line}");
    }
    println!();
}

/// Docker check rows
pub fn print_check_row(name: &str, ok: bool, detail: &str) {
    let (icon, style) = if ok {
        ("✔", s_green_bold())
    } else {
        ("✗", s_red_bold())
    };
    println!(
        "  {}  {:<22} {}",
        style.apply_to(icon),
        s_bold().apply_to(name),
        s_dim().apply_to(detail)
    );
}

