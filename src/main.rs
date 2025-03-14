//! # [Ratatui] Table example
//!
//! The latest version of this example is available in the [examples] folder in the repository.
//!
//! Please note that the examples are designed to be run against the `main` branch of the Github
//! repository. This means that you may not be able to compile with the latest release version on
//! crates.io, or the one that you have installed locally.
//!
//! See the [examples readme] for more information on finding examples that match the version of the
//! library you are using.
//!
//! [Ratatui]: https://github.com/ratatui/ratatui
//! [examples]: https://github.com/ratatui/ratatui/blob/main/examples
//! [examples readme]: https://github.com/ratatui/ratatui/blob/main/examples/README.md

use color_eyre::Result;
use crossterm::event::KeyModifiers;
use itertools::Itertools;
use ratatui::{
    DefaultTerminal, Frame,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Flex, Layout, Margin, Position, Rect},
    style::{self, Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Cell, Clear, HighlightSpacing, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
};
use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: [&str; 2] = [
    "(Esc) quit | (↑) move up | (↓) move down | (←) move left | (→) move right",
    "(Shift + →) next color | (Shift + ←) previous color",
];

const ITEM_HEIGHT: usize = 4;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();
    let app_result = App::new().run(terminal);
    ratatui::restore();
    app_result
}
struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_row_style_fg: Color,
    selected_column_style_fg: Color,
    selected_cell_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_row_style_fg: color.c400,
            selected_column_style_fg: color.c400,
            selected_cell_style_fg: color.c600,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

struct Data {
    name: String,
    address: String,
    email: String,
    col2: String,
    col3: String,
}

impl Data {
    const fn ref_array(&self) -> [&String; 5] {
        [
            &self.name,
            &self.address,
            &self.email,
            &self.col2,
            &self.col3,
        ]
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn address(&self) -> &str {
        &self.address
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn col2(&self) -> &str {
        &self.col2
    }

    fn col3(&self) -> &str {
        &self.col3
    }
}

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

struct App {
    state: TableState,
    items: Vec<Data>,
    longest_item_lens: (u16, u16, u16, u16, u16), // order is (name, address, email)
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
    show_popup: bool,
    input: String,
    input_mode: InputMode,
    character_index: usize,
}

/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn popup_area(area: Rect, percent_x: u16, percent_y: u16) -> Rect {
    let vertical = Layout::vertical([Constraint::Percentage(percent_y)]).flex(Flex::Center);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)]).flex(Flex::Center);
    let [area] = vertical.areas(area);
    let [area] = horizontal.areas(area);
    area
}

impl App {
    fn new() -> Self {
        let data_vec = generate_fake_names();
        Self {
            state: TableState::default().with_selected(0),
            longest_item_lens: constraint_len_calculator(&data_vec),
            scroll_state: ScrollbarState::new((data_vec.len() - 1) * ITEM_HEIGHT),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            items: data_vec,
            show_popup: false,
            input: String::new(),
            input_mode: InputMode::Normal,
            character_index: 0,
        }
    }
    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub fn edit_cell(&mut self) {
        if let Some(content) = self.get_current_cell_content() {
            self.input = content.to_owned();
        }

        self.show_popup = !self.show_popup;
        self.input_mode = InputMode::Editing;
    }

    pub fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    fn submit_message(&mut self) {
        // self.messages.push(self.input.clone());
        self.reset_cursor();
        self.input_mode = InputMode::Normal;
        self.show_popup = false;

        // 获取当前选中的行和列
        if let Some(selected_row) = self.state.selected() {
            let selected_column = self.state.selected_column().unwrap_or(0); // 列索引

            // 获取对应的 Data 对象
            // let data = self.items.get(selected_row.unwrap());

            if let Some(data) = self.items.get_mut(selected_row) {
                // 根据列索引获取对应的字段内容
                match selected_column {
                    0 => data.name = self.input.clone(),
                    1 => data.address = self.input.clone(),
                    2 => data.email = self.input.clone(),
                    3 => data.col2 = self.input.clone(),
                    4 => data.col3 = self.input.clone(),
                    _ => {}
                };
            }
        }
        self.input.clear();
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn byte_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn delete_char(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            // Method "remove" is not used on the saved text for deleting the selected char.
            // Reason: Using remove on String works on bytes instead of the chars.
            // Using remove would require special care because of char boundaries.

            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            // Getting all characters before the selected character.
            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            // Getting all characters after selected character.
            let after_char_to_delete = self.input.chars().skip(current_index);

            // Put all characters together except the selected one.
            // By leaving the selected one out, it is forgotten and therefore deleted.
            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }

    fn clamp_cursor(&self, new_cursor_pos: usize) -> usize {
        new_cursor_pos.clamp(0, self.input.chars().count())
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn enter_char(&mut self, new_char: char) {
        let index = self.byte_index();
        self.input.insert(index, new_char);
        self.move_cursor_right();
    }

    // 获取当前光标所在 CELL 的内容
    fn get_current_cell_content(&self) -> Option<&str> {
        // 获取当前选中的行和列
        let selected_row = self.state.selected()?; // 行索引
        let selected_column = self.state.selected_column().unwrap_or(0); // 列索引

        // 获取对应的 Data 对象
        let data = self.items.get(selected_row)?;

        // 根据列索引获取对应的字段内容
        match selected_column {
            0 => Some(data.name()),
            1 => Some(data.address()),
            2 => Some(data.email()),
            3 => Some(data.col2()),
            4 => Some(data.col3()),
            _ => None,
        }
    }

    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        loop {
            terminal.draw(|frame| self.draw(frame))?;

            if let Event::Key(key) = event::read()? {
                // if key.kind == KeyEventKind::Press {
                //     let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);
                //     match key.code {
                //         KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                //         KeyCode::Char('j') | KeyCode::Down => self.next_row(),
                //         KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
                //         KeyCode::Char('l') | KeyCode::Right if shift_pressed => self.next_color(),
                //         KeyCode::Char('h') | KeyCode::Left if shift_pressed => {
                //             self.previous_color();
                //         }
                //         KeyCode::Char('l') | KeyCode::Right => self.next_column(),
                //         KeyCode::Char('h') | KeyCode::Left => self.previous_column(),
                //         KeyCode::Char('e') | KeyCode::Enter => self.edit_cell(),
                //         _ => {}
                //     }
                // }

                let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);

                match self.input_mode {
                    InputMode::Normal => match key.code {
                        // KeyCode::Char('e') => {
                        //     self.input_mode = InputMode::Editing;
                        // }
                        // KeyCode::Char('q') => {
                        //     return Ok(());
                        // }
                        // _ => {}
                        KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                        KeyCode::Char('j') | KeyCode::Down => self.next_row(),
                        KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
                        KeyCode::Char('l') | KeyCode::Right if shift_pressed => self.next_color(),
                        // KeyCode::Char('h') | KeyCode::Left if shift_pressed => {
                        //     self.previous_color();
                        // }
                        KeyCode::Left if shift_pressed => {
                            self.previous_color();
                        }
                        KeyCode::Char('l') | KeyCode::Right => self.next_column(),
                        // KeyCode::Char('h') | KeyCode::Left => self.previous_column(),
                        KeyCode::Left => self.previous_column(),
                        KeyCode::Char('e') | KeyCode::Enter => self.edit_cell(),
                        _ => {}
                    },
                    InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                        KeyCode::Enter => self.submit_message(),
                        KeyCode::Char(to_insert) => self.enter_char(to_insert),
                        KeyCode::Backspace => self.delete_char(),
                        KeyCode::Left => self.move_cursor_left(),
                        KeyCode::Right => self.move_cursor_right(),
                        KeyCode::Esc => self.input_mode = InputMode::Normal,
                        _ => {}
                    },
                    _ => {}
                }
            }
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
        let rects = vertical.split(frame.area());

        // let vertical2 = &Layout::vertical([
        //     Constraint::Length(1),
        //     Constraint::Length(3),
        //     Constraint::Min(1),
        // ]);
        // let [input_area] = vertical.areas(frame.area());

        self.set_colors();

        self.render_table(frame, rects[0]);
        self.render_scrollbar(frame, rects[0]);
        self.render_footer(frame, rects[1]);

        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));

        let area = frame.area();
        if self.show_popup {
            // frame.render_widget(input, input_area);

            // let block = Block::bordered().title("Popup");
            let area = popup_area(area, 40, 20);
            frame.render_widget(Clear, area); //this clears out the background
            // frame.render_widget(block, area);
            frame.render_widget(input, area);

            match self.input_mode {
                // Hide the cursor. `Frame` does this by default, so we don't need to do anything here
                InputMode::Normal => {}

                // Make the cursor visible and ask ratatui to put it at the specified coordinates after
                // rendering
                #[allow(clippy::cast_possible_truncation)]
                InputMode::Editing => frame.set_cursor_position(Position::new(
                    // Draw the cursor at the current position in the input field.
                    // This position is can be controlled via the left and right arrow key
                    area.x + self.character_index as u16 + 1,
                    // Move one line down, from the border to the input line
                    area.y + 1,
                )),
            }
        }
    }

    fn render_table(&mut self, frame: &mut Frame, area: Rect) {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = ["Name", "Address", "Email", "col2", "col3"]
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(4)
        });
        let bar = " █ ";
        let t = Table::new(
            rows,
            [
                // + 1 is for padding.
                Constraint::Length(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.1 + 1),
                Constraint::Min(self.longest_item_lens.2),
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .column_highlight_style(selected_col_style)
        .cell_highlight_style(selected_cell_style)
        .highlight_symbol(Text::from(vec![
            "".into(),
            bar.into(),
            bar.into(),
            "".into(),
        ]))
        .bg(self.colors.buffer_bg)
        .highlight_spacing(HighlightSpacing::Always);
        frame.render_stateful_widget(t, area, &mut self.state);
    }

    fn render_scrollbar(&mut self, frame: &mut Frame, area: Rect) {
        frame.render_stateful_widget(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            &mut self.scroll_state,
        );
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            );
        frame.render_widget(info_footer, area);
    }
}

fn generate_fake_names() -> Vec<Data> {
    use fakeit::{address, contact, job, name};

    (0..20)
        .map(|_| {
            let name = name::full();
            let address = format!(
                "{}\n{}, {} {}",
                address::street(),
                address::city(),
                address::state(),
                address::zip()
            );
            let email = contact::email();
            let col2 = job::descriptor();
            let col3 = job::level();

            Data {
                name,
                address,
                email,
                col2,
                col3,
            }
        })
        .sorted_by(|a, b| a.name.cmp(&b.name))
        .collect()
}

fn constraint_len_calculator(items: &[Data]) -> (u16, u16, u16, u16, u16) {
    let name_len = items
        .iter()
        .map(Data::name)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let address_len = items
        .iter()
        .map(Data::address)
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let email_len = items
        .iter()
        .map(Data::email)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let col2_len = items
        .iter()
        .map(Data::col2)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let col3_len = items
        .iter()
        .map(Data::col3)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (
        name_len as u16,
        address_len as u16,
        email_len as u16,
        col2_len as u16,
        col3_len as u16,
    )
}

// #[cfg(test)]
// mod tests {
//     use crate::Data;

//     #[test]
//     fn constraint_len_calculator() {
//         let test_data = vec![
//             Data {
//                 name: "Emirhan Tala".to_string(),
//                 address: "Cambridgelaan 6XX\n3584 XX Utrecht".to_string(),
//                 email: "tala.emirhan@gmail.com".to_string(),
//             },
//             Data {
//                 name: "thistextis26characterslong".to_string(),
//                 address: "this line is 31 characters long\nbottom line is 33 characters long"
//                     .to_string(),
//                 email: "thisemailis40caharacterslong@ratatui.com".to_string(),
//             },
//         ];
//         let (longest_name_len, longest_address_len, longest_email_len) =
//             crate::constraint_len_calculator(&test_data);

//         assert_eq!(26, longest_name_len);
//         assert_eq!(33, longest_address_len);
//         assert_eq!(40, longest_email_len);
//     }
// }
