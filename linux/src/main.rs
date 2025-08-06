mod config;
mod input_widget;
mod list_widget;

use std::{
    time::Duration,
};

use color_eyre::Result;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers, poll},
    layout::{Constraint, Layout},
    DefaultTerminal,
    Frame,
};
use smol::{
    future::FutureExt, 
    io::AsyncWriteExt, 
    stream::StreamExt, 
    Timer,
    net::TcpStream,
};

use config::Config;
use input_widget::InputWidget;
use list_widget::ListWidgets;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}


#[derive(Debug)]
struct App {
    input_widget: InputWidget,
    widgets: ListWidgets<300>,

    should_exit: bool,
    input: String,
}

impl App {
    /// The duration between each tick.
    const TICK_RATE: Duration = Duration::from_millis(20);

    /// Create a new instance of the app.
    fn new() -> Self {
        Self {
            input_widget: InputWidget::new(),
            widgets: ListWidgets::new(),
            should_exit: false,
            input: String::new(),
        }
    }

    /// Run the app until the user exits.
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        let toml_str = std::fs::read_to_string("config.toml")
            .expect("Failed to read config.toml file");
        let config: Config = toml::from_str(&toml_str)
            .expect("Failed to deserialize Cargo.toml");

        smol::block_on(async {
            let mut stream = TcpStream::connect(&config.ip).await?;

            while !self.should_exit {
                FutureExt::or(
                    async {
                        let _ = smol::io::copy(&mut stream, &mut self.widgets).await;
                    },
                    async {
                        Timer::interval(Self::TICK_RATE).next().await;
                    },
                )
                .await;

                let _ = self.handle_events();
                if self.input.len() > 0 {
                    let cmd = format!("<= {}", self.input.as_str());
                    self.widgets.cmd().add_item(cmd);
                    self.input.push('\n');
                    stream.write_all(self.input.as_bytes()).await?;
                    self.input.clear();
                }
                terminal.draw(|frame| self.render(frame))?;
            }
            Ok(())
        })
    }

    /// Handle events from the terminal.
    fn handle_events(&mut self) -> Result<()> {
        if poll(Duration::from_micros(1))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.should_exit = false;
                    match key.code {
                        KeyCode::Enter => self.input = self.input_widget.get_message(),
                        KeyCode::Char('c') => if key.modifiers == KeyModifiers::CONTROL {
                            self.should_exit = true;
                        } else {
                            self.input_widget.handle_key_input(key.code);
                        }
                        _ => self.input_widget.handle_key_input(key.code),
                    };
                }
            }
        }
        Ok(())
    }

    fn render(&mut self, frame: &mut Frame) {
        let outer_layout = Layout::horizontal([
            Constraint::Length(40), 
            Constraint::Fill(1)]
        ).split(frame.area());
        let right_layout = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Length(3)
        ]).split(outer_layout[1]);

        let can_rf_area = outer_layout[0];
        let command_area = right_layout[0];
        let input_area = right_layout[1];

        self.input_widget.render(frame, &input_area);
        self.widgets.render(frame, &can_rf_area, &command_area);
    }
}
