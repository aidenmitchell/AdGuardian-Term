use std::{
  io::stdout,
  sync::Arc,
  time::Duration,
};
use crossterm::{
  event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
  execute,
  terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use tui::{
  backend::CrosstermBackend,
  layout::{Constraint, Direction, Layout},
  style::Color,
  Terminal,
};

use crate::fetch::fetch_stats::StatsResponse;
use crate::fetch::fetch_query_log::Query;
use crate::fetch::fetch_status::StatusResponse;
use crate::fetch::fetch_filters::{AdGuardFilteringStatus, Filter};

use crate::widgets::gauge::make_gauge;
use crate::widgets::table::make_query_table;
use crate::widgets::chart::{make_history_chart, prepare_chart_data};
use crate::widgets::status::render_status_paragraph;
use crate::widgets::filters::make_filters_list;
use crate::widgets::list::make_list;

pub async fn draw_ui(
    mut data_rx: tokio::sync::mpsc::Receiver<Vec<Query>>,
    mut stats_rx: tokio::sync::mpsc::Receiver<StatsResponse>,
    mut status_rx: tokio::sync::mpsc::Receiver<StatusResponse>,
    filters: AdGuardFilteringStatus,
    shutdown: Arc<tokio::sync::Notify>
) -> Result<(), anyhow::Error> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // Ensure cleanup happens even on panic or early return
    let cleanup = scopeguard::guard((), |_| {
        let _ = disable_raw_mode();
        let _ = execute!(
            std::io::stdout(),
            LeaveAlternateScreen,
            DisableMouseCapture
        );
    });

    // Initialize state with default/empty data
    let mut data: Option<Vec<Query>> = None;
    let mut stats: Option<StatsResponse> = None;
    let mut status: Option<StatusResponse> = None;

    loop {
        // Collect updates from all channels before redrawing
        let mut received_count = 0;

        // Wait for all three channels to send data
        while received_count < 3 {
            tokio::select! {
                Some(new_data) = data_rx.recv() => {
                    data = Some(new_data);
                    received_count += 1;
                }
                Some(new_stats) = stats_rx.recv() => {
                    stats = Some(new_stats);
                    received_count += 1;
                }
                Some(new_status) = status_rx.recv() => {
                    status = Some(new_status);
                    received_count += 1;
                }
                else => break, // All channels closed
            }
        }

        // Only render if we have at least some data
        if data.is_none() || stats.is_none() || status.is_none() {
            continue;
        }

        // Prepare the data for the chart
        let mut stats_clone = stats.clone().unwrap();
        prepare_chart_data(&mut stats_clone);

        terminal.draw(|f| {
            let size = f.size();

            // Make the charts
            let gauge = make_gauge(&stats_clone);
            let table = make_query_table(data.as_ref().unwrap(), size.width);
            let graph = make_history_chart(&stats_clone);
            let paragraph = render_status_paragraph(status.as_ref().unwrap(), &stats_clone);
            let filter_items: &[Filter] = filters
                .filters
                .as_deref()
                .unwrap_or(&[]);
            let filters_list = make_filters_list(filter_items, size.width);
            let top_queried_domains = make_list("Top Queried Domains", &stats_clone.top_queried_domains, Color::Green, size.width);
            let top_blocked_domains = make_list("Top Blocked Domains", &stats_clone.top_blocked_domains, Color::Red, size.width);
            let top_clients = make_list("Top Clients", &stats_clone.top_clients, Color::Cyan, size.width);

            let constraints = if size.height > 42 {
                vec![
                    Constraint::Percentage(30),
                    Constraint::Min(1),
                    Constraint::Percentage(20)
                ]
            } else {
                vec![
                    Constraint::Percentage(30),
                    Constraint::Min(1),
                    Constraint::Percentage(0)
                ]
            };

            let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(&*constraints)
            .split(size);

            // Split the top part (charts + gauge) into left (gauge + block) and right (line chart)
            let top_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(
                [
                    Constraint::Percentage(30), 
                    Constraint::Percentage(70), 
                ]
                .as_ref(),
            )
            .split(chunks[0]);

            // Split the left part of top (gauge + block) into top (gauge) and bottom (block)
            let left_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Min(0),
                        Constraint::Length(3),
                    ]
                    .as_ref(),
                )
                .split(top_chunks[0]);

            let bottom_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(25), 
                        Constraint::Percentage(25), 
                        Constraint::Percentage(25), 
                        Constraint::Percentage(25), 
                    ]
                    .as_ref(),
                )
                .split(chunks[2]);

            // Render the widgets to the UI
            f.render_widget(paragraph, left_chunks[0]);
            f.render_widget(gauge, left_chunks[1]);
            f.render_widget(graph, top_chunks[1]);
            f.render_widget(table, chunks[1]);
            if size.height > 42 {
                f.render_widget(filters_list, bottom_chunks[0]);
                f.render_widget(top_queried_domains, bottom_chunks[1]);
                f.render_widget(top_blocked_domains, bottom_chunks[2]);
                f.render_widget(top_clients, bottom_chunks[3]);
            }
        })?;

        // Check for user input events (non-blocking)
        if poll(Duration::from_millis(0))? {
            match read()? {
                Event::Key(KeyEvent {
                    code: KeyCode::Char('q'),
                    ..
                }) => {
                    shutdown.notify_waiters();
                    break;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('Q'),
                    ..
                }) => {
                    shutdown.notify_waiters();
                    break;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    modifiers: KeyModifiers::CONTROL,
                }) => {
                    shutdown.notify_waiters();
                    break;
                }
                Event::Resize(_, _) => {}, // Handle resize event
                _ => {}
            }
        }

    }

    // Show cursor before cleanup guard runs
    terminal.show_cursor()?;

    // Explicit cleanup is handled by the scopeguard
    drop(cleanup);
    Ok(())
}

