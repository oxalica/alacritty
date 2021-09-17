#version 330 core

// Cell properties.
layout (location = 0) in vec2 gridCoords;

// Background color.
layout (location = 4) in vec4 backgroundColor;

flat out vec4 color;

// Terminal properties
uniform vec2 cellDim;
uniform vec4 projection;

void main()
{
    color = backgroundColor / 255.0;

    vec2 projectionOffset = projection.xy;
    vec2 projectionScale = projection.zw;

    // Compute vertex corner position
    vec2 position;
    position.x = (gl_VertexID == 0 || gl_VertexID == 1) ? 1. : 0.;
    position.y = (gl_VertexID == 0 || gl_VertexID == 3) ? 0. : 1.;

    // Position of cell from top-left
    vec2 cellPosition = cellDim * gridCoords;

    // Final position.
    vec2 finalPosition = cellPosition + cellDim * position;
    gl_Position =
        vec4(projectionOffset + projectionScale * finalPosition, 0.0, 1.0);
}
