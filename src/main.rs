use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::layout::{Flex, Rect};
use ratatui::prelude::Stylize;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::block::Position;
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout},
    text::{Line, Text},
    widgets::{block::Title, Block, Borders, Table, Clear},
    DefaultTerminal, Frame,
};
use std::{fmt::Display, io};
use tui_textarea::{Input, Key, TextArea};

use ratatui::widgets::{Cell, Paragraph, Row, TableState};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
enum Status {
    NotStarted,
    InProgress,
    Complete,
    Overdue,
}

#[derive(Debug, Clone, PartialEq)]
enum State {
    Home,
    New,
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Status::NotStarted => "Not Started",
            Status::InProgress => "In Progress",
            Status::Complete => "Complete",
            Status::Overdue => "Overdue",
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Task {
    created: chrono::DateTime<chrono::Local>,
    duration: std::time::Duration,
    title: String,
    description: String,
    status: Status,
}

fn is_number(textarea: &mut TextArea) -> bool {
    match textarea.lines()[0].parse::<i32>() {
        Ok(_) => {
            textarea.set_style(Style::default().fg(Color::LightGreen));
            textarea.set_block(
                Block::default()
                    .border_style(Color::LightGreen)
                    .borders(Borders::ALL)
                    .title("Span (hrs)"),
            );
            true
        }
        Err(err) => {
            textarea.set_style(Style::default().fg(Color::LightRed));
            textarea.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Color::LightRed)
                    .title(format!("ERROR: {}", err)),
            );
            false
        }
    }
}

fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

impl<'a> From<&Task> for Row<'a> {
    fn from(val: &Task) -> Self {
        Row::new(vec![
            Cell::new(Text::from(format!("{}", val.status))),
            Cell::new(val.title.clone()),
            Cell::new(Text::from(format!("{}", val.created + val.duration))),
        ])
    }
}

struct App<'a> {
    file: String,
    tasks: Vec<Task>,
    exit: bool,
    state: State,
    date_input: TextArea<'a>,
    title_input: TextArea<'a>,
    description_input: TextArea<'a>,
    show_description: bool,
    focus: usize,
    table_state: TableState,
}

impl Drop for App<'_> {
    fn drop(&mut self) {
        let serialized = serde_json::to_string(&self.tasks).unwrap();
        std::fs::write(&self.file, serialized).unwrap();
    }
}

impl App<'_> {
    fn render(&mut self, area: Rect, frame: &mut Frame) {
        match self.state {
            State::Home => {
                let block = Block::default()
                    .title(Title::from("Tasks").alignment(Alignment::Center))
                    .borders(Borders::ALL);
                let header = Row::new(vec![
                    Cell::new(Text::from("Status")),
                    Cell::new(Text::from("Title")),
                    Cell::new(Text::from("Due Date")),
                ]);

                let constraints = [
                    Constraint::Fill(0),
                    Constraint::Percentage(75),
                    Constraint::Fill(0),
                ];

                let selected_style = Style::default()
                    .add_modifier(Modifier::REVERSED)
                    .fg(Color::Blue);

                let table = Table::new(&self.tasks, constraints)
                    .column_spacing(1)
                    .highlight_style(selected_style)
                    .header(header)
                    .block(block);
                frame.render_stateful_widget(table, area, &mut self.table_state);

                if self.show_description {
                    let popup = popup_area(area, 80, 80);

                    let block = Block::default()
                        .title(Title::from("Description").alignment(Alignment::Center))
                        .borders(Borders::ALL);

                    let task = match self.table_state.selected() {
                        Some(i) => &self.tasks[i],
                        None => return,
                    };

                    let paragraph = Paragraph::new(task.description.clone())
                        .block(block);

                    frame.render_widget(Clear, popup);
                    frame.render_widget(paragraph, popup);
                }
            }
            State::New => {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(0)])
                    .split(area);

                let inner_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .split(main_layout[0]);

                frame.render_widget(&self.title_input, inner_layout[0]);
                frame.render_widget(&self.date_input, inner_layout[1]);
                frame.render_widget(&self.description_input, main_layout[1]);
            }
        }
    }

    fn new(file: &str) -> Self {
        let tasks = match std::fs::read_to_string(file) {
            Ok(content) => serde_json::from_str(&content).unwrap(),
            Err(_) => Vec::new(),
        };

        let bordered = Block::default().borders(Borders::ALL);

        let mut date_input = TextArea::default();
        let mut title_input = TextArea::default();
        let mut description_input = TextArea::default();

        let instruction = Title::from(Line::from(vec![
            " Exit ".into(),
            "<Esc>".blue().bold(),
            " Next ".into(),
            "<Tab>".blue().bold(),
            " Save ".into(),
            "<Enter>".blue().bold(),
        ]));

        date_input.set_block(bordered.clone().title(Title::from("Span (hrs)")));
        title_input.set_block(bordered.clone().title(Title::from("Title")));
        description_input.set_block(
            bordered.clone().title(Title::from("Description")).title(
                instruction
                    .alignment(Alignment::Center)
                    .position(Position::Bottom),
            ),
        );

        App {
            file: file.to_owned(),
            tasks,
            exit: false,
            state: State::Home,
            date_input,
            title_input,
            description_input,
            show_description: false,
            focus: 0,
            table_state: TableState::default(),
        }
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn next(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= self.tasks.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.tasks.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    fn draw(&mut self, frame: &mut Frame) {
        self.render(frame.area(), frame);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(ev) if ev.kind == KeyEventKind::Press => self.handle_key_event(ev),
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') if self.state == State::Home => self.exit = true,
            KeyCode::Char('n') if self.state == State::Home => self.state = State::New,
            KeyCode::Esc if self.state == State::New => self.state = State::Home,
            KeyCode::Char(c) if self.state == State::New => {
                match self.focus {
                    0 => self.title_input.input(Input {
                        key: Key::Char(c),
                        ..Input::default()
                    }),
                    1 => self.date_input.input(Input {
                        key: Key::Char(c),
                        ..Input::default()
                    }),
                    2 => self.description_input.input(Input {
                        key: Key::Char(c),
                        ..Input::default()
                    }),
                    _ => unreachable!(),
                };
            }
            KeyCode::Backspace if self.state == State::New => {
                match self.focus {
                    0 => self.title_input.input(Input {
                        key: Key::Backspace,
                        ..Input::default()
                    }),
                    1 => self.date_input.input(Input {
                        key: Key::Backspace,
                        ..Input::default()
                    }),
                    2 => self.description_input.input(Input {
                        key: Key::Backspace,
                        ..Input::default()
                    }),
                    _ => unreachable!(),
                };
            }
            KeyCode::Tab if self.state == State::New => self.focus = (self.focus + 1) % 3,
            KeyCode::Enter if self.state == State::New => {
                if is_number(&mut self.date_input) {
                    self.tasks.push(Task {
                        created: chrono::Local::now(),
                        duration: std::time::Duration::from_secs(
                            self.date_input.lines()[0].parse::<u64>().unwrap()*3600,
                        ),
                        title: self.title_input.lines()[0].clone(),
                        description: self.description_input.lines().join("\n"),
                        status: Status::NotStarted,
                    });
                    self.state = State::Home;
                }
            }
            KeyCode::Down if self.state == State::Home => self.next(),
            KeyCode::Up if self.state == State::Home => self.previous(),
            KeyCode::Enter if self.state == State::Home => {
                let i = self.table_state.selected().unwrap();
                self.tasks[i].status = match self.tasks[i].status {
                    Status::NotStarted => Status::InProgress,
                    Status::InProgress => Status::Complete,
                    Status::Complete => Status::Overdue,
                    Status::Overdue => Status::NotStarted,
                };
            }
            KeyCode::Char(' ') if self.state == State::Home => {
                self.show_description = !self.show_description
            }

            _ => {}
        };
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::new("tasks.json");
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}
