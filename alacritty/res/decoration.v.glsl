#version 330 core

// Cell properties.
layout (location = 0) in vec2 gridCoords;

// Text foreground rgb packed together with cell flags. textColor.a
// are the bitflags; consult RenderingGlyphFlags in renderer/mod.rs
// for the possible values.
layout(location = 3) in vec4 textColor;

// Background color.
layout (location = 4) in vec4 backgroundColor;

flat out vec4 fg;
flat out vec4 bg;
// The position relative to the grid cell left-bottom corner in pixel.
smooth out vec2 cellRelativePosition;

// Terminal properties
uniform vec2 cellDim;
uniform vec4 projection;

#define WIDE_CHAR 1

void main()
{
    fg = vec4(textColor.rgb / 255.0, textColor.a);
    bg = backgroundColor / 255.0;

    vec2 projectionOffset = projection.xy;
    vec2 projectionScale = projection.zw;

    // Compute vertex corner position
    vec2 position;
    position.x = (gl_VertexID == 0 || gl_VertexID == 1) ? 1. : 0.;
    position.y = (gl_VertexID == 0 || gl_VertexID == 3) ? 0. : 1.;

    // Double the width for wide char.
    position.x *= 1.0 + float((int(textColor.a) & WIDE_CHAR) != 0);

    // Position of cell from top-left
    vec2 cellPosition = cellDim * gridCoords;

    // Final position.
    vec2 finalPosition = cellPosition + cellDim * position;
    gl_Position =
        vec4(projectionOffset + projectionScale * finalPosition, 0.0, 1.0);

    cellRelativePosition = cellDim * vec2(position.x, 1. - position.y); // Change origion to left-bottom.
}
