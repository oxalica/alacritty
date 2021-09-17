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

#define UNDERLINE_MASK   070
#define SINGLE_UNDERLINE 010
#define DOUBLE_UNDERLINE 020
#define DOTTED_UNDERLINE 030
#define DASHED_UNDERLINE 040
#define CURLY_UNDERLINE  050

#define TAU 6.28318530717958647692528676655900577

void main()
{
    int flags = int(fg.a);
    int underlineFlag = flags & UNDERLINE_MASK;

    float strikeoutBottom = -decent + strikeoutPosition - 0.5 * strikeoutThickness;
    bool inStrikeout =
        strikeoutBottom <= cellRelativePosition.y &&
        cellRelativePosition.y <= strikeoutBottom + strikeoutThickness;

    float underlineBottom = -decent + underlinePosition - 0.5 * underlineThickness;
    bool inSingleUnderline =
        underlineBottom <= cellRelativePosition.y &&
        cellRelativePosition.y <= underlineBottom + underlineThickness;

    // Position underlines so each one has 50% of descent available.
    float doubleUnderlineBottom1 = -decent + decent * 0.75 - 0.5 * underlineThickness;
    float doubleUnderlineBottom2 = -decent + decent * 0.25 - 0.5 * underlineThickness;
    bool inDoubleUnderline =
        ((doubleUnderlineBottom1 <= cellRelativePosition.y &&
        cellRelativePosition.y <= doubleUnderlineBottom1 + underlineThickness) ||
        (doubleUnderlineBottom2 <= cellRelativePosition.y &&
        cellRelativePosition.y <= doubleUnderlineBottom2 + underlineThickness));

    bool inDottedUnderline =
        inSingleUnderline &&
        fract(cellRelativePosition.x / (underlineThickness * 2.)) <= 0.5;

    // Dash starts from 1/4 period for symmetry.
    float dashPosition = fract(cellRelativePosition.x / cellDim.x);
    bool inDashedUnderline =
        inSingleUnderline &&
        (dashPosition <= 0.25 || dashPosition >= 0.75);

    float curlyAmplify = (doubleUnderlineBottom2 - doubleUnderlineBottom1) / 2.;
    float curlyY = doubleUnderlineBottom1 + curlyAmplify * (sin(cellRelativePosition.x / cellDim.x * TAU) + 1.);
    bool inCurlyUnderline =
        curlyY <= cellRelativePosition.y &&
        cellRelativePosition.y <= curlyY + underlineThickness;

    bool shouldUseFg =
        (flags & STRIKEOUT) != 0 && inStrikeout ||
        underlineFlag == SINGLE_UNDERLINE && inSingleUnderline ||
        underlineFlag == DOUBLE_UNDERLINE && inDoubleUnderline ||
        underlineFlag == DOTTED_UNDERLINE && inDottedUnderline ||
        underlineFlag == DASHED_UNDERLINE && inDashedUnderline ||
        underlineFlag == CURLY_UNDERLINE  && inCurlyUnderline;

    FragColor = vec4(mix(bg.rgb, fg.rgb, float(shouldUseFg)), 1.0);
    FragAlphaMask = vec4(1.0);
}
