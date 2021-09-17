#version 330 core

flat in vec4 fg;
flat in vec4 bg;
smooth in vec2 cellRelativePosition;

layout(location = 0, index = 0) out vec4 FragColor;
layout(location = 0, index = 1) out vec4 FragAlphaMask;

uniform vec4 projection;
uniform vec2 cellDim;

// Metrics
uniform float decent;
uniform float strikeoutPosition;
uniform float strikeoutThickness;
uniform float underlinePosition;
uniform float underlineThickness;

#define STRIKEOUT 4
#define UNDERLINE 8
#define DOUBLE_UNDERLINE 16

void main()
{
    int flags = int(fg.a);

    float strikeoutBottom = -decent + strikeoutPosition - 0.5 * strikeoutThickness;
    bool inStrikeout =
        (flags & STRIKEOUT) != 0 &&
        strikeoutBottom <= cellRelativePosition.y &&
        cellRelativePosition.y <= strikeoutBottom + strikeoutThickness;

    float underlineBottom = -decent + underlinePosition - 0.5 * underlineThickness;
    bool inUnderline =
        (flags & UNDERLINE) != 0 &&
        underlineBottom <= cellRelativePosition.y &&
        cellRelativePosition.y <= underlineBottom + underlineThickness;

    // Position underlines so each one has 50% of descent available.
    float doubleUnderlineBottom1 = -decent + decent * 0.25 - 0.5 * underlineThickness;
    float doubleUnderlineBottom2 = -decent + decent * 0.75 - 0.5 * underlineThickness;
    bool inDoubleUnderline =
        (flags & DOUBLE_UNDERLINE) != 0 &&
        ((doubleUnderlineBottom1 <= cellRelativePosition.y &&
        cellRelativePosition.y <= doubleUnderlineBottom1 + underlineThickness) ||
        (doubleUnderlineBottom2 <= cellRelativePosition.y &&
        cellRelativePosition.y <= doubleUnderlineBottom2 + underlineThickness));

    bool shouldUseFg = inUnderline || inStrikeout || inDoubleUnderline;

    FragColor = vec4(mix(bg.rgb, fg.rgb, float(shouldUseFg)), 1.0);
    FragAlphaMask = vec4(1.0);
}
