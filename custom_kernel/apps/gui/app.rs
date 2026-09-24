pub trait App {
    fn update(&mut self);
    fn draw(&mut self, buffer: &mut [u32], width: usize, height: usize);
    fn on_mouse_event(&mut self, x: i32, y: i32, buttons: u8);
    fn on_key_event(&mut self, c: char);
}

pub enum AppType {
    Terminal(crate::gui::terminal_app::TerminalApp),
    Notepad(crate::gui::notepad_app::NotepadApp),
    Snake(crate::gui::snake_app::SnakeApp),
    Calculator(crate::gui::calculator_app::CalculatorApp),
    Minesweeper(crate::gui::minesweeper_app::MinesweeperApp),
    Clock(crate::gui::clock_app::ClockApp),
    Calendar(crate::gui::calendar_app::CalendarApp),
    Tetris(crate::gui::tetris_app::TetrisApp),
    Pong(crate::gui::pong_app::PongApp),
    Game2048(crate::gui::game2048_app::Game2048App),
    Chess(crate::gui::chess_app::ChessApp),
    Sudoku(crate::gui::sudoku_app::SudokuApp),
    FileManager(crate::gui::file_manager_app::FileManagerApp),
    Settings(crate::gui::settings_app::SettingsApp),
    TaskManager(crate::gui::task_manager_app::TaskManagerApp),
    SysMon(crate::gui::sysmon_app::SysMonApp),
    Paint(crate::gui::paint_app::PaintApp),
    Browser(crate::gui::browser_app::BrowserApp),
    None,
}

impl AppType {
    pub fn update(&mut self) {
        match self {
            AppType::Terminal(app)    => app.update(),
            AppType::Notepad(app)     => app.update(),
            AppType::Snake(app)       => app.update(),
            AppType::Calculator(app)  => app.update(),
            AppType::Minesweeper(app) => app.update(),
            AppType::Clock(app)       => app.update(),
            AppType::Calendar(app)    => app.update(),
            AppType::Tetris(app)      => app.update(),
            AppType::Pong(app)        => app.update(),
            AppType::Game2048(app)    => app.update(),
            AppType::Chess(app)       => app.update(),
            AppType::Sudoku(app)       => app.update(),
            AppType::FileManager(app)  => app.update(),
            AppType::Settings(app)     => app.update(),
            AppType::TaskManager(app)  => app.update(),
            AppType::SysMon(app)       => app.update(),
            AppType::Paint(app)        => app.update(),
            AppType::Browser(app)      => app.update(),
            AppType::None              => {},
        }
    }

    pub fn draw(&mut self, buffer: &mut [u32], width: usize, height: usize) {
        match self {
            AppType::Terminal(app)    => app.draw(buffer, width, height),
            AppType::Notepad(app)     => app.draw(buffer, width, height),
            AppType::Snake(app)       => app.draw(buffer, width, height),
            AppType::Calculator(app)  => app.draw(buffer, width, height),
            AppType::Minesweeper(app) => app.draw(buffer, width, height),
            AppType::Clock(app)       => app.draw(buffer, width, height),
            AppType::Calendar(app)    => app.draw(buffer, width, height),
            AppType::Tetris(app)      => app.draw(buffer, width, height),
            AppType::Pong(app)        => app.draw(buffer, width, height),
            AppType::Game2048(app)    => app.draw(buffer, width, height),
            AppType::Chess(app)       => app.draw(buffer, width, height),
            AppType::Sudoku(app)       => app.draw(buffer, width, height),
            AppType::FileManager(app)  => app.draw(buffer, width, height),
            AppType::Settings(app)     => app.draw(buffer, width, height),
            AppType::TaskManager(app)  => app.draw(buffer, width, height),
            AppType::SysMon(app)       => app.draw(buffer, width, height),
            AppType::Paint(app)        => app.draw(buffer, width, height),
            AppType::Browser(app)      => app.draw(buffer, width, height),
            AppType::None              => {},
        }
    }

    pub fn on_mouse_event(&mut self, x: i32, y: i32, buttons: u8) {
        match self {
            AppType::Terminal(app)    => app.on_mouse_event(x, y, buttons),
            AppType::Notepad(app)     => app.on_mouse_event(x, y, buttons),
            AppType::Snake(app)       => app.on_mouse_event(x, y, buttons),
            AppType::Calculator(app)  => app.on_mouse_event(x, y, buttons),
            AppType::Minesweeper(app) => app.on_mouse_event(x, y, buttons),
            AppType::Clock(app)       => app.on_mouse_event(x, y, buttons),
            AppType::Calendar(app)    => app.on_mouse_event(x, y, buttons),
            AppType::Tetris(app)      => app.on_mouse_event(x, y, buttons),
            AppType::Pong(app)        => app.on_mouse_event(x, y, buttons),
            AppType::Game2048(app)    => app.on_mouse_event(x, y, buttons),
            AppType::Chess(app)       => app.on_mouse_event(x, y, buttons),
            AppType::Sudoku(app)       => app.on_mouse_event(x, y, buttons),
            AppType::FileManager(app)  => app.on_mouse_event(x, y, buttons),
            AppType::Settings(app)     => app.on_mouse_event(x, y, buttons),
            AppType::TaskManager(app)  => app.on_mouse_event(x, y, buttons),
            AppType::SysMon(app)       => app.on_mouse_event(x, y, buttons),
            AppType::Paint(app)        => app.on_mouse_event(x, y, buttons),
            AppType::Browser(app)      => app.on_mouse_event(x, y, buttons),
            AppType::None              => {},
        }
    }

    pub fn on_key_event(&mut self, c: char) {
        match self {
            AppType::Terminal(app)    => app.on_key_event(c),
            AppType::Notepad(app)     => app.on_key_event(c),
            AppType::Snake(app)       => app.on_key_event(c),
            AppType::Calculator(app)  => app.on_key_event(c),
            AppType::Minesweeper(app) => app.on_key_event(c),
            AppType::Clock(app)       => app.on_key_event(c),
            AppType::Calendar(app)    => app.on_key_event(c),
            AppType::Tetris(app)      => app.on_key_event(c),
            AppType::Pong(app)        => app.on_key_event(c),
            AppType::Game2048(app)    => app.on_key_event(c),
            AppType::Chess(app)       => app.on_key_event(c),
            AppType::Sudoku(app)       => app.on_key_event(c),
            AppType::FileManager(app)  => app.on_key_event(c),
            AppType::Settings(app)     => app.on_key_event