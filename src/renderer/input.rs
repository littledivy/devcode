use crate::renderer::rectangle::{Rectangle, Region};
use crate::renderer::Dimensions;
use unicode_segmentation::UnicodeSegmentation;
use wgpu_glyph::ab_glyph::{Font, FontArc};
use wgpu_glyph::{GlyphPositioner, Layout, SectionGeometry, Text};
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::VirtualKeyCode;

#[derive(Debug)]
pub struct Cursor {
  pub rect: Rectangle,
  pub row: usize,
  pub column: usize,
  pub x_offset: f32,
}

impl Cursor {
  pub fn new(
    device: &wgpu::Device,
    screen_size: PhysicalSize<f32>,
    dimensions: Dimensions,
    color: [f32; 3],
    region: Option<Region>,
  ) -> Self {
    Self {
      rect: Rectangle::new(device, screen_size, dimensions, color, region),
      row: 0,
      column: 0,
      x_offset: 0.0,
    }
  }
}

pub trait TextInput {
  fn input_special(
    &mut self,
    screen_size: PhysicalSize<f32>,
    key: VirtualKeyCode,
  );
  fn input_char(&mut self, screen_size: PhysicalSize<f32>, ch: char);
}

pub struct TextArea {
  cursor: Cursor,
  font: FontArc,
  font_height: f32,
  max_line_length: f32,
  text: Vec<String>,
  _multiline: Option<f32>,
}

impl TextArea {
  pub fn _new(
    font: FontArc,
    text: String,
    font_height: f32,
    device: &wgpu::Device,
    screen_size: PhysicalSize<f32>,
    multiline: Option<f32>,
  ) -> Self {
    let mut split_text =
      text.lines().map(|s| s.to_string()).collect::<Vec<String>>();
    if multiline.is_some() && text.ends_with('\n') {
      split_text.push(String::from(""));
    }

    if multiline.is_none() {
      assert_eq!(split_text.len(), 1);
    }

    // TODO: bounding rect

    let cursor = Cursor::new(
      device,
      screen_size,
      Dimensions {
        x: 0.0,
        y: screen_size.height as f32 - font_height,
        width: 1.0,
        height: font_height,
      },
      [0.7, 0.0, 0.0],
      Some(Region {
        x: 0,
        y: 0,
        width: screen_size.width as u32,
        height: screen_size.height as u32,
      }),
    );

    let max_line_length =
      max_line_length(&split_text, font.clone(), font_height);

    Self {
      text: split_text,
      cursor,
      font,
      font_height,
      max_line_length,
      _multiline: multiline,
    }
  }
}

impl super::RenderElement for TextArea {
  fn get_rects(&self) -> Vec<&Rectangle> {
    vec![&self.cursor.rect]
  }

  fn get_elements(&mut self) -> Vec<&mut dyn super::RenderElement> {
    vec![]
  }

  fn get_dimensions(&self) -> Dimensions {
    todo!()
  }
}

impl TextInput for TextArea {
  fn input_special(
    &mut self,
    screen_size: PhysicalSize<f32>,
    key: VirtualKeyCode,
  ) {
    input_special(
      screen_size,
      key,
      &mut self.text,
      &mut self.cursor,
      self.font.clone(),
      self.font_height,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalPosition { x: 0.0, y: 0.0 },
    );
  }

  fn input_char(&mut self, screen_size: PhysicalSize<f32>, ch: char) {
    self.max_line_length = input_char(
      screen_size,
      ch,
      &mut self.text,
      &mut self.cursor,
      self.font.clone(),
      self.font_height,
      PhysicalPosition { x: 0.0, y: 0.0 },
      PhysicalPosition { x: 0.0, y: 0.0 },
    );
  }
}

pub fn line_length(line: &str, font: FontArc, font_height: f32) -> f32 {
  let layout = Layout::default_wrap();
  let text = Text::new(line).with_scale(font_height);
  let section_glyphs = layout.calculate_glyphs(
    &[font.clone()],
    &SectionGeometry {
      ..Default::default()
    },
    &[text],
  );

  if let Some(section_glyph) = section_glyphs.last() {
    section_glyph.glyph.position.x
      + font.glyph_bounds(&section_glyph.glyph).width()
  } else {
    0.0
  }
}

pub fn max_line_length(
  lines: &[String],
  font: FontArc,
  font_height: f32,
) -> f32 {
  let mut max_line_width = 0.0;
  for line in lines {
    let width = line_length(line, font.clone(), font_height);

    if width > max_line_width {
      max_line_width = width;
    }
  }

  max_line_width
}

pub fn cursor_x_position(
  row: usize,
  column: usize,
  text: &[String],
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
) -> Option<f32> {
  let text = Text::new(&text[row]).with_scale(font_height);
  let layout = Layout::default_wrap();

  let section_glyphs = layout.calculate_glyphs(
    &[font.clone()],
    &SectionGeometry {
      screen_position: (offset.x, offset.y),
      ..Default::default()
    },
    &[text],
  );

  if let Some(section_glyph) = section_glyphs.get(column) {
    Some(section_glyph.glyph.position.x)
  } else if column != 0 {
    section_glyphs.get(column - 1).map(|section_glyph| {
      section_glyph.glyph.position.x
        + font.glyph_bounds(&section_glyph.glyph).width()
    })
  } else {
    None
  }
}

#[allow(clippy::too_many_arguments)]
pub fn input_special(
  screen_size: PhysicalSize<f32>,
  key: VirtualKeyCode,
  text: &mut Vec<String>,
  cursor: &mut Cursor,
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
  scroll_offset: PhysicalPosition<f32>,
) {
  let cursor_x_position2 = |row: usize, column: usize| {
    cursor_x_position(
      row,
      column,
      text,
      font.clone(),
      font_height,
      scroll_offset,
    )
  };

  match key {
    VirtualKeyCode::Up => {
      if cursor.row != 0 {
        cursor.row -= 1;
        if let Some(offset) = cursor_x_position2(cursor.row, cursor.column) {
          cursor.x_offset = offset;
        } else {
          cursor.column = text[cursor.row].len();
          cursor.x_offset =
            cursor_x_position2(cursor.row, cursor.column).unwrap_or(0.0);
        }
      } else {
        cursor.x_offset = 0.0;
        cursor.column = 0;
      }
    }
    VirtualKeyCode::Left => {
      if cursor.column != 0 {
        cursor.column -= 1;
        cursor.x_offset =
          cursor_x_position2(cursor.row, cursor.column).unwrap();
      } else if cursor.row != 0 {
        cursor.row -= 1;
        cursor.column = text[cursor.row].len();
        cursor.x_offset =
          cursor_x_position2(cursor.row, cursor.column).unwrap_or(0.0);
      }
    }
    VirtualKeyCode::Down => {
      if cursor.row != (text.len() - 1) {
        cursor.row += 1;
        if let Some(offset) = cursor_x_position2(cursor.row, cursor.column) {
          cursor.x_offset = offset;
        } else {
          cursor.column = text[cursor.row].len();
          cursor.x_offset =
            cursor_x_position2(cursor.row, cursor.column).unwrap_or(0.0);
        }
      } else {
        cursor.column = text[cursor.row].len();
        cursor.x_offset =
          cursor_x_position2(cursor.row, cursor.column).unwrap_or(0.0);
      }
    }
    VirtualKeyCode::Right => {
      if cursor.row != (text.len() - 1) {
        if let Some(offset) = cursor_x_position2(cursor.row, cursor.column + 1)
        {
          cursor.column += 1;
          cursor.x_offset = offset;
        } else {
          cursor.x_offset = 0.0;
          cursor.column = 0;
          cursor.row += 1;
        }
      } else if let Some(offset) =
        cursor_x_position2(cursor.row, cursor.column + 1)
      {
        cursor.column += 1;
        cursor.x_offset = offset;
      }
    }
    _ => return,
  }

  cursor.rect.resize(
    screen_size,
    Dimensions {
      x: offset.x + scroll_offset.x + cursor.x_offset,
      y: scroll_offset.y + font_height + (cursor.row as f32 * font_height),
      ..cursor.rect.dimensions
    },
  );
}

#[allow(clippy::too_many_arguments)]
pub fn input_char(
  screen_size: PhysicalSize<f32>,
  ch: char,
  text: &mut Vec<String>,
  cursor: &mut Cursor,
  font: FontArc,
  font_height: f32,
  offset: PhysicalPosition<f32>,
  scroll_offset: PhysicalPosition<f32>,
) -> f32 {
  let input_spc =
    |key: VirtualKeyCode, text: &mut Vec<String>, cursor: &mut Cursor| {
      input_special(
        screen_size,
        key,
        text,
        cursor,
        font.clone(),
        font_height,
        offset,
        scroll_offset,
      );
    };

  match ch {
    // backspace
    '\u{7f}' => {
      if cursor.column != 0 {
        let mut graphemes_indices = text[cursor.row].grapheme_indices(true);
        let index = graphemes_indices.nth(cursor.column - 1).unwrap().0;
        text[cursor.row].remove(index);
        input_spc(VirtualKeyCode::Left, text, cursor);
      } else if cursor.row != 0 {
        let removed = text.remove(cursor.row);
        cursor.row -= 1;
        cursor.column = text[cursor.row].len() + 1;
        text[cursor.row] += &removed;
        input_spc(VirtualKeyCode::Left, text, cursor);
      }
    }
    // enter
    '\r' => {
      let mut graphemes_indices = text[cursor.row].grapheme_indices(true);
      let index = graphemes_indices
        .nth(cursor.column)
        .map(|(i, _)| i)
        .unwrap_or_else(|| text[cursor.row].len());
      let after_enter = text[cursor.row].split_off(index);
      text.insert(cursor.row + 1, after_enter);
      input_spc(VirtualKeyCode::Right, text, cursor);
    }
    _ => {
      let mut graphemes_indices = text[cursor.row].grapheme_indices(true);
      let index = graphemes_indices
        .nth(cursor.column)
        .map(|(i, _)| i)
        .unwrap_or_else(|| text[cursor.row].len());
      text[cursor.row].insert(index, ch);
      input_spc(VirtualKeyCode::Right, text, cursor);
    }
  }

  max_line_length(&text, font, font_height)
}
