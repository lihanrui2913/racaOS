use core::{cmp::min, fmt};

use super::ansi::{Attr, Handler, Performer};
use super::ansi::{LineClearMode, ScreenClearMode};
use super::buffer::TerminalBuffer;
use super::cell::{Cell, Flags};
use super::graphic::{DrawTarget, TextOnGraphic};

#[derive(Debug, Default, Clone, Copy)]
struct Cursor {
    row: usize,
    column: usize,
}

pub struct Console<D: DrawTarget> {
    parser: vte::Parser,
    inner: ConsoleInner<D>,
}

pub struct ConsoleInner<D: DrawTarget> {
    cursor: Cursor,
    saved_cursor: Cursor,
    attribute_template: Cell,
    buffer: TerminalBuffer<D>,
}

impl<D: DrawTarget> Console<D> {
    pub fn new(buffer: D) -> Self {
        let (width, height) = buffer.size();
        let mut graphic = TextOnGraphic::new(buffer, width, height);
        graphic.clear(Cell::default());

        Console {
            parser: vte::Parser::new(),
            inner: ConsoleInner {
                cursor: Cursor::default(),
                saved_cursor: Cursor::default(),
                attribute_template: Cell::default(),
                buffer: TerminalBuffer::new(graphic),
            },
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        self.parser
            .advance(&mut Performer::new(&mut self.inner), byte);
    }
}

impl<D: DrawTarget> fmt::Write for Console<D> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

impl<D: DrawTarget> Handler for ConsoleInner<D> {
    fn input(&mut self, content: char) {
        if self.cursor.column >= self.buffer.width() {
            self.cursor.column = 0;
            self.linefeed();
        }
        let template = self.attribute_template.with_content(content);
        self.buffer
            .write(self.cursor.row, self.cursor.column, template);
        self.cursor.column += 1;
    }

    fn goto(&mut self, row: usize, col: usize) {
        self.cursor.row = min(row, self.buffer.height());
        self.cursor.column = min(col, self.buffer.width());
    }

    fn goto_row(&mut self, row: usize) {
        self.goto(row, self.cursor.column)
    }

    fn goto_column(&mut self, col: usize) {
        self.goto(self.cursor.row, col)
    }

    fn move_up(&mut self, rows: usize) {
        self.goto(self.cursor.row.saturating_sub(rows), self.cursor.column)
    }

    fn move_down(&mut self, rows: usize) {
        let goto_row = min(self.cursor.row + rows, self.buffer.height() - 1) as _;
        self.goto(goto_row, self.cursor.column)
    }

    fn move_forward(&mut self, cols: usize) {
        self.cursor.column = min(self.cursor.column + cols, self.buffer.width() - 1);
    }

    fn move_backward(&mut self, cols: usize) {
        self.cursor.column = self.cursor.column.saturating_sub(cols);
    }

    fn move_up_and_cr(&mut self, rows: usize) {
        self.goto(self.cursor.row.saturating_sub(rows), 0)
    }

    fn move_down_and_cr(&mut self, rows: usize) {
        let goto_row = min(self.cursor.row + rows, self.buffer.height() - 1) as _;
        self.goto(goto_row, 0)
    }

    fn put_tab(&mut self) {
        let tab_stop = self.cursor.column.div_ceil(8) * 8;
        let end_column = tab_stop.min(self.buffer.width());
        let template = self.attribute_template.reset();

        while self.cursor.column < end_column {
            self.buffer
                .write(self.cursor.row, self.cursor.column, template);
            self.cursor.column += 1;
        }
    }

    fn backspace(&mut self) {
        self.cursor.column = self.cursor.column.saturating_sub(1);
    }

    fn carriage_return(&mut self) {
        self.cursor.column = 0;
    }

    fn linefeed(&mut self) {
        self.cursor.column = 0;
        if self.cursor.row < self.buffer.height() - 1 {
            self.cursor.row += 1;
        } else {
            self.buffer.new_line(self.attribute_template);
        }
    }

    fn erase_chars(&mut self, count: usize) {
        let start = self.cursor.column;
        let end = min(start + count, self.buffer.width());

        let template = self.attribute_template.reset();
        for column in start..end {
            self.buffer.write(self.cursor.row, column, template);
        }
    }

    fn delete_chars(&mut self, count: usize) {
        let (row, columns) = (self.cursor.row, self.buffer.width());
        let count = min(count, columns - self.cursor.column - 1);

        let template = self.attribute_template.reset();
        for column in (self.cursor.column + count)..columns {
            self.buffer
                .write(row, column - count, self.buffer.read(row, column));
            self.buffer.write(row, column, template);
        }
    }

    fn save_cursor_position(&mut self) {
        self.saved_cursor = self.cursor;
    }

    fn restore_cursor_position(&mut self) {
        self.cursor = self.saved_cursor;
    }

    fn clear_line(&mut self, mode: LineClearMode) {
        let (start, end) = match mode {
            LineClearMode::Right => (self.cursor.column, self.buffer.width()),
            LineClearMode::Left => (0, self.cursor.column + 1),
            LineClearMode::All => (0, self.buffer.width()),
        };
        let template = self.attribute_template.reset();
        for column in start..end {
            self.buffer.write(self.cursor.row, column, template);
        }
    }

    fn clear_screen(&mut self, mode: ScreenClearMode) {
        let template = self.attribute_template.reset();
        match mode {
            ScreenClearMode::Above => {
                for row in 0..self.cursor.row {
                    for column in 0..self.buffer.width() {
                        self.buffer.write(row, column, template);
                    }
                }
                for column in 0..self.cursor.column {
                    self.buffer.write(self.cursor.row, column, template);
                }
            }
            ScreenClearMode::Below => {
                for column in self.cursor.column..self.buffer.width() {
                    self.buffer.write(self.cursor.row, column, template);
                }
                for row in self.cursor.row + 1..self.buffer.height() {
                    for column in 0..self.buffer.width() {
                        self.buffer.write(row, column, template);
                    }
                }
            }
            ScreenClearMode::All => {
                self.buffer.clear(template);
                self.cursor = Cursor::default();
            }
            _ => {}
        }
    }

    fn terminal_attribute(&mut self, attr: Attr) {
        match attr {
            Attr::Foreground(color) => self.attribute_template.foreground = color,
            Attr::Background(color) => self.attribute_template.background = color,
            Attr::Reset => self.attribute_template = Cell::default(),
            Attr::Reverse => self.attribute_template.flags |= Flags::INVERSE,
            Attr::CancelReverse => self.attribute_template.flags.remove(Flags::INVERSE),
            Attr::Hidden => self.attribute_template.flags.insert(Flags::HIDDEN),
            Attr::CancelHidden => self.attribute_template.flags.remove(Flags::HIDDEN),
        }
    }
}
