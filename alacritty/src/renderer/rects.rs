use std::mem;

use crossfont::Metrics;
use enum_map::{Enum, EnumMap};
use memoffset::offset_of;

use alacritty_terminal::index::Point;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::color::Rgb;
use alacritty_terminal::term::SizeInfo;

use crate::display::content::RenderableCell;
use crate::gl;
use crate::gl::types::*;
use crate::renderer;

// Should this be configurable?
// 2 periods per cell. Dash&gap width is (at most) 1/4 cell width.
// Also it is at least 2px to keep visable.
const DASH_PERIODS_PER_CELL: f32 = 2.;

#[derive(Debug, Copy, Clone)]
pub struct RenderRect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: Rgb,
    pub alpha: f32,
    pub style: RectStyle,
}

#[derive(Debug, Copy, Clone)]
pub enum RectStyle {
    /// Solid rectangle.
    Solid,
    /// Dashed rectangle (line).
    /// `period` for the length of dash period (a dash and a gap).
    Dashed { period: f32 },
}

impl RenderRect {
    pub fn new(x: f32, y: f32, width: f32, height: f32, color: Rgb, alpha: f32) -> Self {
        RenderRect { x, y, width, height, color, alpha, style: RectStyle::Solid }
    }

    pub fn new_with_style(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: Rgb,
        alpha: f32,
        style: RectStyle,
    ) -> Self {
        RenderRect { x, y, width, height, color, alpha, style }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct RenderLine {
    start: Point<usize>,
    end: Point<usize>,
    color: Rgb,
}

impl RenderLine {
    /// Create a line's rect at a position relative to the baseline.
    fn rect(
        &self,
        size: &SizeInfo,
        descent: f32,
        position: f32,
        mut thickness: f32,
        style: RectStyle,
    ) -> RenderRect {
        let Self { start, end, color } = *self;
        let start_x = start.column.0 as f32 * size.cell_width();
        let end_x = (end.column.0 + 1) as f32 * size.cell_width();
        let width = end_x - start_x;

        // Make sure lines are always visible.
        thickness = thickness.max(1.);

        let line_bottom = (start.line as f32 + 1.) * size.cell_height();
        let baseline = line_bottom + descent;

        let mut y = (baseline - position - thickness / 2.).ceil();
        let max_y = line_bottom - thickness;
        if y > max_y {
            y = max_y;
        }

        RenderRect::new_with_style(
            start_x + size.padding_x(),
            y + size.padding_y(),
            width,
            thickness,
            color,
            1.,
            style,
        )
    }
}

/// Lines for underline and strikeout.
#[derive(Default)]
pub struct RenderLines {
    inner: EnumMap<LineKind, Vec<RenderLine>>,
}

#[derive(Debug, Clone, Copy, Enum)]
enum LineKind {
    StrikeOut,
    SingleUnderline,
    DoubleUnderline,
    DottedUnderline,
    DashedUnderline,
}

impl RenderLines {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    // Calculate positions of all lines and turn them into rects.
    #[inline]
    pub fn rects(&self, metrics: &Metrics, size: &SizeInfo) -> Vec<RenderRect> {
        let mut rects = Vec::new();

        // Strike out line.
        for line in &self.inner[LineKind::StrikeOut] {
            rects.push(line.rect(
                size,
                metrics.descent,
                metrics.strikeout_position,
                metrics.strikeout_thickness,
                RectStyle::Solid,
            ));
        }

        // Single underline.
        for line in &self.inner[LineKind::SingleUnderline] {
            rects.push(line.rect(
                size,
                metrics.descent,
                metrics.underline_position,
                metrics.underline_thickness,
                RectStyle::Solid,
            ));
        }

        // Double underline.
        {
            // Position underlines so each one has 50% of descent available.
            let top_pos = 0.25 * metrics.descent;
            let bottom_pos = 0.75 * metrics.descent;
            for line in &self.inner[LineKind::DoubleUnderline] {
                rects.push(line.rect(
                    size,
                    metrics.descent,
                    top_pos,
                    metrics.underline_thickness,
                    RectStyle::Solid,
                ));
                rects.push(line.rect(
                    size,
                    metrics.descent,
                    bottom_pos,
                    metrics.underline_thickness,
                    RectStyle::Solid,
                ));
            }
        }

        // Dotted underline.
        {
            // The dash width (half period) must be at least 1px and should be multiple
            // of px, or it will be visually inconsistent.
            let period = metrics.underline_thickness.round().max(1.) * 2.;
            for line in &self.inner[LineKind::DottedUnderline] {
                rects.push(line.rect(
                    size,
                    metrics.descent,
                    metrics.underline_position,
                    metrics.underline_thickness,
                    RectStyle::Dashed { period },
                ));
            }
        }

        // Dashed underline.
        {
            let dash_width = size.cell_width() / (DASH_PERIODS_PER_CELL * 2.);
            // Same reason as DOTTED_UNDERLINE. But this should at least longer than it.
            let period = dash_width.round().max(2.) * 2.;
            for line in &self.inner[LineKind::DashedUnderline] {
                rects.push(line.rect(
                    size,
                    metrics.descent,
                    metrics.underline_position,
                    metrics.underline_thickness,
                    RectStyle::Dashed { period },
                ));
            }
        }

        rects
    }

    /// Update the stored lines with the next cell info.
    #[inline]
    pub fn update(&mut self, cell: &RenderableCell) {
        self.update_kind(cell, LineKind::StrikeOut, Flags::STRIKEOUT);
        self.update_kind(cell, LineKind::SingleUnderline, Flags::UNDERLINE);
        self.update_kind(cell, LineKind::DoubleUnderline, Flags::DOUBLE_UNDERLINE);
        self.update_kind(cell, LineKind::DottedUnderline, Flags::DOTTED_UNDERLINE);
        self.update_kind(cell, LineKind::DashedUnderline, Flags::DASHED_UNDERLINE);
    }

    /// Update the lines for a specific kind.
    fn update_kind(&mut self, cell: &RenderableCell, kind: LineKind, flag: Flags) {
        if !cell.flags.contains(flag) {
            return;
        }

        // Include wide char spacer if the current cell is a wide char.
        let mut end = cell.point;
        if cell.flags.contains(Flags::WIDE_CHAR) {
            end.column += 1;
        }

        let lines = &mut self.inner[kind];

        // Check if there's an active line.
        if let Some(line) = lines.last_mut() {
            if cell.fg == line.color
                && cell.point.column == line.end.column + 1
                && cell.point.line == line.end.line
            {
                // Update the length of the line.
                line.end = end;
                return;
            }
        }

        // Start new line if there currently is none.
        lines.push(RenderLine { start: cell.point, end, color: cell.fg });
    }
}

/// Shader sources for rect rendering program.
static RECT_SHADER_F: &str = include_str!("../../res/rect.f.glsl");
static RECT_SHADER_V: &str = include_str!("../../res/rect.v.glsl");

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct Vertex {
    // Normalized screen coordinates.
    x: f32,
    y: f32,

    // Color.
    r: u8,
    g: u8,
    b: u8,
    a: u8,

    // The period of dashes.
    // For solid rect, this is set to a large enough value.
    dash_period: f32,
}

#[derive(Debug)]
pub struct RectRenderer {
    // GL buffer objects.
    vao: GLuint,
    vbo: GLuint,

    program: RectShaderProgram,

    vertices: Vec<Vertex>,
}

impl RectRenderer {
    pub fn new() -> Result<Self, renderer::Error> {
        let mut vao: GLuint = 0;
        let mut vbo: GLuint = 0;
        let program = RectShaderProgram::new()?;

        unsafe {
            // Allocate buffers.
            gl::GenVertexArrays(1, &mut vao);
            gl::GenBuffers(1, &mut vbo);

            gl::BindVertexArray(vao);

            // VBO binding is not part of VAO itself, but VBO binding is stored in attributes.
            gl::BindBuffer(gl::ARRAY_BUFFER, vbo);

            // Position.
            // [x, y], r, g, b, a, dash_period
            gl::VertexAttribPointer(
                0,
                2,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, x) as *const _,
            );
            gl::EnableVertexAttribArray(0);

            // Color.
            // x, y, [r, g, b, a], dash_period
            gl::VertexAttribPointer(
                1,
                4,
                gl::UNSIGNED_BYTE,
                gl::TRUE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, r) as *const _,
            );
            gl::EnableVertexAttribArray(1);

            // Dash period.
            // x, y, r, g, b, a, [dash_period]
            gl::VertexAttribPointer(
                2,
                1,
                gl::FLOAT,
                gl::FALSE,
                mem::size_of::<Vertex>() as i32,
                offset_of!(Vertex, dash_period) as *const _,
            );
            gl::EnableVertexAttribArray(2);

            // Reset buffer bindings.
            gl::BindVertexArray(0);
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
        }

        Ok(Self { vao, vbo, program, vertices: Vec::new() })
    }

    pub fn draw(&mut self, size_info: &SizeInfo, rects: Vec<RenderRect>) {
        unsafe {
            // Bind VAO to enable vertex attribute slots.
            gl::BindVertexArray(self.vao);

            // Bind VBO only once for buffer data upload only.
            gl::BindBuffer(gl::ARRAY_BUFFER, self.vbo);

            gl::UseProgram(self.program.id);
        }

        let half_width = size_info.width() / 2.;
        let half_height = size_info.height() / 2.;

        // Build rect vertices vector.
        self.vertices.clear();
        for rect in &rects {
            self.add_rect(half_width, half_height, rect);
        }

        unsafe {
            // Upload accumulated vertices.
            gl::BufferData(
                gl::ARRAY_BUFFER,
                (self.vertices.len() * mem::size_of::<Vertex>()) as isize,
                self.vertices.as_ptr() as *const _,
                gl::STREAM_DRAW,
            );

            // Draw all vertices as list of triangles.
            gl::DrawArrays(gl::TRIANGLES, 0, self.vertices.len() as i32);

            // Disable program.
            gl::UseProgram(0);

            // Reset buffer bindings to nothing.
            gl::BindBuffer(gl::ARRAY_BUFFER, 0);
            gl::BindVertexArray(0);
        }
    }

    fn add_rect(&mut self, half_width: f32, half_height: f32, rect: &RenderRect) {
        // Calculate rectangle vertices positions in normalized device coordinates.
        // NDC range from -1 to +1, with Y pointing up.
        let x = rect.x / half_width - 1.0;
        let y = -rect.y / half_height + 1.0;
        let width = rect.width / half_width;
        let height = rect.height / half_height;
        let Rgb { r, g, b } = rect.color;
        let a = (rect.alpha * 255.) as u8;
        let dash_period = match rect.style {
            // To make it all solid in viewport, simply set period to a number greater than
            // 2 times the viewport size 2.0 (from -1 to +1),
            RectStyle::Solid => 2. * 2.,
            RectStyle::Dashed { period } => period / half_width,
        };

        // Make quad vertices.
        // ^y   0 - 2
        // |    | / |
        // ->x  1 - 3
        let quad = [
            Vertex { x, y, r, g, b, a, dash_period },
            Vertex { x, y: y - height, r, g, b, a, dash_period },
            Vertex { x: x + width, y, r, g, b, a, dash_period },
            Vertex { x: x + width, y: y - height, r, g, b, a, dash_period },
        ];

        // Append the vertices to form two triangles.
        // The order matters! Dotted line shader relies on a specific provoking vertex (the last
        // one by default).
        self.vertices.push(quad[2]);
        self.vertices.push(quad[0]);
        self.vertices.push(quad[1]);
        self.vertices.push(quad[3]);
        self.vertices.push(quad[2]);
        self.vertices.push(quad[1]);
    }
}

impl Drop for RectRenderer {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteBuffers(1, &self.vbo);
            gl::DeleteVertexArrays(1, &self.vao);
        }
    }
}

/// Rectangle drawing program.
#[derive(Debug)]
pub struct RectShaderProgram {
    /// Program id.
    id: GLuint,
}

impl RectShaderProgram {
    pub fn new() -> Result<Self, renderer::ShaderCreationError> {
        let vertex_shader = renderer::create_shader(gl::VERTEX_SHADER, RECT_SHADER_V)?;
        let fragment_shader = renderer::create_shader(gl::FRAGMENT_SHADER, RECT_SHADER_F)?;
        let program = renderer::create_program(vertex_shader, fragment_shader)?;

        unsafe {
            gl::DeleteShader(fragment_shader);
            gl::DeleteShader(vertex_shader);
            gl::UseProgram(program);
        }

        let shader = Self { id: program };

        unsafe { gl::UseProgram(0) }

        Ok(shader)
    }
}

impl Drop for RectShaderProgram {
    fn drop(&mut self) {
        unsafe {
            gl::DeleteProgram(self.id);
        }
    }
}
