use fakeit::address::info;
use lazy_static::lazy_static;
use log::info;
use rust_decimal::Decimal;
use sqlx::{MySql, MySqlPool, Pool};
use std::{env, fmt::format, path::Path, str::FromStr, sync::Mutex, time::Duration};
use tokio::sync::mpsc;

use color_eyre::Result;
use crossterm::event::{KeyEvent, KeyModifiers};
use itertools::Itertools;
use ratatui::{
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Flex, Layout, Margin, Position, Rect},
    style::{self, Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, BorderType, Cell, Clear, HighlightSpacing, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState,
    },
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};
use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

use std::sync::Arc;
use tokio::{sync::RwLock, time::sleep};

mod db;
use db::dbutils::{Database, MmVolumeTask};
// use db::dbutils::Database::get_all_users;
use futures::{executor::block_on, future::ok};
use log4rs;

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

// 定义全局变量为一个包含 MyStruct 的数组
lazy_static! {
    static ref GLOBAL_ARRAY: Mutex<Vec<MmVolumeTask>> = Mutex::new(vec![]);
}

// static GLOBAL_DATA = new vec();

#[derive(Serialize, Deserialize, Debug)]
struct Config {
    db_server: String,
    db_port: u16,
    db_user: String,
    db_password: String,
    db_name: String,
    enable_logging: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            db_server: "http://localhost".to_string(),
            db_port: 3306,
            db_user: "root".to_string(),
            db_password: "password".to_string(),
            db_name: "db".to_string(),
            enable_logging: true,
        }
    }
}

fn load_config() -> Config {
    confy::load_path("/opt/xtool/config.toml").expect("Failed to load config")
}

fn is_numeric(s: &str) -> bool {
    Decimal::from_str(s).is_ok()
}

#[tokio::main]
async fn main() -> Result<()> {
    // read config file
    env::set_var("RUST_BACKTRACE", "full");
    // env::set_var("RUST_LOG", "info");
    // env_logger::init();

    log4rs::init_file("/opt/xtool/log4rs.yaml", Default::default()).unwrap();

    let config_file_path = "/opt/xtool/config.toml";

    // check if config file exists
    if !Path::new(&config_file_path).exists() {
        eprintln!(
            "Error: Configuration file does not exist at {:?}",
            config_file_path
        );
        std::process::exit(1);
    }

    let config: Config = load_config();

    let db_url = format!(
        "mysql://{}:{}@{}:{}/{}",
        config.db_user, config.db_password, config.db_server, config.db_port, config.db_name
    );
    let db = Database::new(&db_url).await.unwrap();

    let mm_volumes: &Vec<MmVolumeTask> = &db.get_all_mm_volume_task().await.unwrap();
    {
        let mut _datas: Vec<MmVolumeTask> = vec![];
        let mut array = GLOBAL_ARRAY.lock().unwrap();
        for _temp in mm_volumes {
            // let _data = Data {
            //     key: _user.accountId.to_string(),
            //     address: _user.userName.clone(),
            //     email: _user.realName.clone(),
            //     col2: _user.password.clone(),
            //     col3: _user.phoneNumber.clone(),
            // };
            array.push(_temp.clone());
        }
    }

    color_eyre::install()?;
    let mut terminal = ratatui::init();
    let mut app = App::new();

    let (tx, mut rx) = mpsc::channel(32);

    tokio::spawn(async move {
        loop {
            match db.get_all_mm_volume_task().await {
                Ok(tasks) => {
                    let mut datas = Vec::new();
                    for _temp in tasks {
                        // let data = Data {
                        //     key: user.accountId.to_string(),
                        //     address: user.userName.clone(),
                        //     email: user.realName.clone(),
                        //     col2: user.password.clone(),
                        //     col3: user.phoneNumber.clone(),
                        // };
                        datas.push(_temp);
                    }

                    {
                        let mut array = GLOBAL_ARRAY.lock().unwrap();
                        *array = datas.clone();
                    }

                    if tx.send(datas).await.is_err() {
                        break;
                    }
                }
                Err(e) => eprintln!("Failed to fetch data: {:?}", e),
            }

            sleep(Duration::from_secs(10)).await;
        }
    });

    loop {
        tokio::select! {
            Some(data) = rx.recv() => {
                app.refresh_data(data);
            }

            result = async {
                if event::poll(Duration::from_millis(100)).map_err(|e| e as std::io::Error)? {
                    if let Event::Key(key) = event::read().map_err(|e| e as std::io::Error)? {
                        return Ok(Some(key));
                    }
                }
                Ok::<Option<KeyEvent>, std::io::Error>(None)
            } => {
                if let Ok(Some(key)) = result {

                    let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);

                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                            KeyCode::Char('j') | KeyCode::Down => app.next_row(),
                            KeyCode::Char('k') | KeyCode::Up => app.previous_row(),
                            KeyCode::Char('l') | KeyCode::Right if shift_pressed => app.next_color(),
                            KeyCode::Left if shift_pressed => {
                                app.previous_color();
                            }
                            KeyCode::Char('l') | KeyCode::Right => app.next_column(),
                            // KeyCode::Char('h') | KeyCode::Left => self.previous_column(),
                            KeyCode::Left => app.previous_column(),
                            KeyCode::Char('e') | KeyCode::Enter => app.edit_cell(),
                            _ => {}
                        },
                        InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
                            KeyCode::Enter => app.submit_message(),
                            KeyCode::Char(to_insert) => app.enter_char(to_insert),
                            KeyCode::Backspace => app.delete_char(),
                            KeyCode::Left => app.move_cursor_left(),
                            KeyCode::Right => app.move_cursor_right(),
                            KeyCode::Esc => app.cancel_edit(),
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }
        }

        // 渲染 UI
        terminal.draw(|frame| app.draw(frame))?;
    }

    ratatui::restore();
    Ok(())
    // app_result
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

// #[derive(Clone, Debug)]
// struct Data {
//     key: String,
//     address: String,
//     email: String,
//     col2: String,
//     col3: String,
// }

#[derive(Clone, Debug, PartialEq)]
struct SelectedCell {
    key_name: String,
    key_value: String,
    cell_name: String,
    cell_value: String,
}

impl MmVolumeTask {
    const fn ref_array(&self) -> [&String; 14] {
        [
            &self.id,
            &self.launch_id,
            &self.token_add,
            &self.target_volume,
            &self.do_status,
            &self.use_wallet_type,
            &self.remark,
            &self.buy_rate,
            &self.buy_per_low,
            &self.buy_per_high,
            &self.sell_percent,
            &self.frequent_low,
            &self.frequent_high,
            &self.real_sol,
        ]
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn launch_id(&self) -> &str {
        &self.launch_id
    }

    fn token_add(&self) -> &str {
        &self.token_add
    }

    fn target_volume(&self) -> &str {
        &self.target_volume
    }

    fn do_status(&self) -> &str {
        &self.do_status
    }

    fn use_wallet_type(&self) -> &str {
        &self.use_wallet_type
    }

    fn remark(&self) -> &str {
        &self.remark
    }

    fn buy_rate(&self) -> &str {
        &self.buy_rate
    }

    fn buy_per_low(&self) -> &str {
        &self.buy_per_low
    }

    fn buy_per_high(&self) -> &str {
        &self.buy_per_high
    }

    fn sell_percent(&self) -> &str {
        &self.sell_percent
    }

    fn frequent_low(&self) -> &str {
        &self.frequent_low
    }

    fn frequent_high(&self) -> &str {
        &self.frequent_high
    }

    fn real_sol(&self) -> &str {
        &self.real_sol
    }
}

#[derive(PartialEq)]
enum InputMode {
    Normal,
    Editing,
}

struct App {
    state: TableState,
    items: Vec<MmVolumeTask>,
    longest_item_lens: (
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
        u16,
    ), // order is (name, address, email)
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
    show_popup: bool,
    input: String,
    input_mode: InputMode,
    character_index: usize,
    editing_key: String,
    editing_key_value: String,
    editing_column: String,
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
        let array: std::sync::MutexGuard<'_, Vec<MmVolumeTask>> = GLOBAL_ARRAY.lock().unwrap();
        let data_vec = array.clone();

        info!("data_vec {:?} ", data_vec);

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
            editing_key: String::new(),
            editing_key_value: String::new(),
            editing_column: String::new(),
        }
    }

    pub fn refresh_data(&mut self, data: Vec<MmVolumeTask>) {
        self.items = data;
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
        let mut can_edit = true;

        if let Some(content) = self.get_current_cell_content() {
            info!("{:?}", content);
            self.input = content.cell_value.to_owned();
            self.editing_key = content.key_name;
            self.editing_key_value = content.key_value;
            self.editing_column = content.cell_name.clone();

            if content.cell_name.eq("id") {
                can_edit = false;
            }
        }

        info!("can edit :  {}", can_edit);

        if can_edit {
            self.show_popup = !self.show_popup;
            self.input_mode = InputMode::Editing;
        }
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

    pub fn submit_message(&mut self) {
        // self.messages.push(self.input.clone());

        let is_num = is_numeric(&self.input.clone());

        info!("{} is_num {}", &self.input.clone(), is_num);

        let mut data_valid = true;

        // currently use manual check
        if self.editing_column.clone().eq("launch_id") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("target_volume") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("do_status") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("use_wallet_type") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("buy_rate") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("buy_per_low") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("buy_per_high") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("sell_percent") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("frequent_low") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("frequent_high") && !is_num {
            data_valid = false;
        }

        if self.editing_column.clone().eq("real_sol") && !is_num {
            data_valid = false;
        }

        if data_valid {
            self.reset_cursor();
            self.input_mode = InputMode::Normal;
            self.show_popup = false;

            // ------- ugly code start
            block_on(async {
                let config: Config = load_config();

                let db_url = format!(
                    "mysql://{}:{}@{}:{}/{}",
                    config.db_user,
                    config.db_password,
                    config.db_server,
                    config.db_port,
                    config.db_name
                );
                let db1 = Database::new(&db_url).await.unwrap();
                info!(" self.editing_key : {} ", self.editing_key);
                db1.update_record(
                    &self.editing_key,
                    &self.editing_key_value,
                    &self.editing_column,
                    &self.input.clone(),
                )
                .await
                .unwrap();

                let mm_tasks = db1.get_all_mm_volume_task().await.unwrap();
                let mut _datas: Vec<MmVolumeTask> = vec![];
                // let mut array = GLOBAL_ARRAY.lock().unwrap();
                for _temp in mm_tasks {
                    _datas.push(_temp);
                }

                self.items = _datas;
            });

            self.input.clear();
        }
    }

    pub fn cancel_edit(&mut self) {
        // self.messages.push(self.input.clone());
        self.reset_cursor();
        self.input_mode = InputMode::Normal;
        self.show_popup = false;
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

    fn get_current_cell_content(&self) -> Option<SelectedCell> {
        let selected_row = self.state.selected()?;
        let selected_column = self.state.selected_column().unwrap_or(0);

        let data = self.items.get(selected_row)?;

        match selected_column {
            0 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "id".to_string(),
                cell_value: data.id().to_string(),
            }),
            1 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "launch_id".to_string(),
                cell_value: data.launch_id().to_string(),
            }),
            2 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "token_add".to_string(),
                cell_value: data.token_add().to_string(),
            }),
            3 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "target_volume".to_string(),
                cell_value: data.target_volume().to_string(),
            }),
            4 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "do_status".to_string(),
                cell_value: data.do_status().to_string(),
            }),
            5 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "use_wallet_type".to_string(),
                cell_value: data.use_wallet_type().to_string(),
            }),
            6 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "remark".to_string(),
                cell_value: data.remark().to_string(),
            }),
            7 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "buy_rate".to_string(),
                cell_value: data.buy_rate().to_string(),
            }),
            8 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "buy_per_low".to_string(),
                cell_value: data.buy_per_low().to_string(),
            }),
            9 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "buy_per_high".to_string(),
                cell_value: data.buy_per_high().to_string(),
            }),
            10 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "sell_percent".to_string(),
                cell_value: data.sell_percent().to_string(),
            }),
            11 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "frequent_low".to_string(),
                cell_value: data.frequent_low().to_string(),
            }),
            12 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "frequent_high".to_string(),
                cell_value: data.frequent_high().to_string(),
            }),
            13 => Some(SelectedCell {
                key_name: "id".to_string(),
                key_value: data.id().to_string(),
                cell_name: "real_sol".to_string(),
                cell_value: data.real_sol().to_string(),
            }),

            _ => None,
        }
    }

    // fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
    //     let (tx, mut rx) = mpsc::channel(32);

    //     tokio::spawn(async move {
    //         loop {
    //             // // ugly code
    //             let config: Config = load_config();

    //             let db_url = format!(
    //                 "mysql://{}:{}@{}:{}/{}",
    //                 config.db_user,
    //                 config.db_password,
    //                 config.db_server,
    //                 config.db_port,
    //                 config.db_name
    //             );
    //             let db1 = Database::new(&db_url).await.unwrap();
    //             // ugly code

    //             let users = db1.get_all_users().await.unwrap();
    //             let mut _datas: Vec<Data> = vec![];
    //             for _user in users {
    //                 let _data = Data {
    //                     key: _user.accountId.to_string(),
    //                     address: _user.userName.clone(),
    //                     email: _user.realName.clone(),
    //                     col2: _user.password.clone(),
    //                     col3: _user.phoneNumber.clone(),
    //                 };
    //                 _datas.push(_data);
    //             }

    //             let _ = tx.send(_datas).await.unwrap();

    //             sleep(Duration::from_secs(10)).await;
    //         }
    //     });

    //     loop {
    //         terminal.draw(|frame| self.draw(frame))?;

    //         if let Event::Key(key) = event::read()? {
    //             let shift_pressed = key.modifiers.contains(KeyModifiers::SHIFT);

    //             match self.input_mode {
    //                 InputMode::Normal => match key.code {
    //                     KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
    //                     KeyCode::Char('j') | KeyCode::Down => self.next_row(),
    //                     KeyCode::Char('k') | KeyCode::Up => self.previous_row(),
    //                     KeyCode::Char('l') | KeyCode::Right if shift_pressed => self.next_color(),
    //                     // KeyCode::Char('h') | KeyCode::Left if shift_pressed => {
    //                     //     self.previous_color();
    //                     // }
    //                     KeyCode::Left if shift_pressed => {
    //                         self.previous_color();
    //                     }
    //                     KeyCode::Char('l') | KeyCode::Right => self.next_column(),
    //                     // KeyCode::Char('h') | KeyCode::Left => self.previous_column(),
    //                     KeyCode::Left => self.previous_column(),
    //                     KeyCode::Char('e') | KeyCode::Enter => self.edit_cell(),
    //                     _ => {}
    //                 },
    //                 InputMode::Editing if key.kind == KeyEventKind::Press => match key.code {
    //                     KeyCode::Enter => self.submit_message(),
    //                     KeyCode::Char(to_insert) => self.enter_char(to_insert),
    //                     KeyCode::Backspace => self.delete_char(),
    //                     KeyCode::Left => self.move_cursor_left(),
    //                     KeyCode::Right => self.move_cursor_right(),
    //                     KeyCode::Esc => self.input_mode = InputMode::Normal,
    //                     _ => {}
    //                 },
    //                 _ => {}
    //             }
    //         }
    //     }
    // }

    fn draw(&mut self, frame: &mut Frame) {
        let vertical = &Layout::vertical([Constraint::Length(4), Constraint::Min(8), Constraint::Length(4)]);
        let rects = vertical.split(frame.area());

        self.set_colors();

        self.render_header(frame, rects[0]);
        self.render_table(frame, rects[1]);
        self.render_scrollbar(frame, rects[1]);
        self.render_footer(frame, rects[2]);

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

        let header = [
            "id",
            "launch_id",
            "token_add",
            "target_volume",
            "do_status",
            "use_wallet_type",
            "remark",
            "buy_rate",
            "buy_per_low",
            "buy_per_high",
            "sell_percent",
            "frequent_low",
            "frequent_high",
            "real_sol",
        ]
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
                .height(3)
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
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
                Constraint::Min(self.longest_item_lens.0 + 1),
            ],
        )
        .header(header)
        .row_highlight_style(selected_row_style)
        .column_highlight_style(selected_col_style)
        .cell_highlight_style(selected_cell_style)
        .highlight_symbol(Text::from(vec![
            // "".into(),
            bar.into(),
            bar.into(),
            bar.into(),
            // bar.into(),
            // "".into(),
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

    fn render_header(&self, frame: &mut Frame, area: Rect) {
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

fn constraint_len_calculator(
    items: &[MmVolumeTask],
) -> (
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
    u16,
) {
    let id_len = items
        .iter()
        .map(MmVolumeTask::id)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let launch_id_len = items
        .iter()
        .map(MmVolumeTask::launch_id)
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let token_add_len = items
        .iter()
        .map(MmVolumeTask::token_add)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let target_volume_len = items
        .iter()
        .map(MmVolumeTask::target_volume)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let do_status_len = items
        .iter()
        .map(MmVolumeTask::do_status)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let use_wallet_type_len = items
        .iter()
        .map(MmVolumeTask::use_wallet_type)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let remark_len = items
        .iter()
        .map(MmVolumeTask::remark)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let buy_rate_len = items
        .iter()
        .map(MmVolumeTask::buy_rate)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let buy_per_low_len = items
        .iter()
        .map(MmVolumeTask::buy_per_low)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let buy_per_high_len = items
        .iter()
        .map(MmVolumeTask::buy_per_high)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let sell_percent_len = items
        .iter()
        .map(MmVolumeTask::sell_percent)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let frequent_low_len = items
        .iter()
        .map(MmVolumeTask::frequent_low)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let frequent_high_len = items
        .iter()
        .map(MmVolumeTask::frequent_high)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let real_sol_len = items
        .iter()
        .map(MmVolumeTask::real_sol)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (
        id_len as u16,
        launch_id_len as u16,
        token_add_len as u16,
        target_volume_len as u16,
        do_status_len as u16,
        use_wallet_type_len as u16,
        remark_len as u16,
        buy_rate_len as u16,
        buy_per_low_len as u16,
        buy_per_high_len as u16,
        sell_percent_len as u16,
        frequent_low_len as u16,
        frequent_high_len as u16,
        real_sol_len as u16,
    )
}
