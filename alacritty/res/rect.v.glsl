#version 330 core
layout (location = 0) in vec2 aPos;
layout (location = 1) in vec4 aColor;
layout (location = 2) in float aDashPeriod;

flat out vec4 color;
flat out float dashPeriod;
flat out vec2 startPos;
out vec2 relativePos;

void main()
{
    color = aColor;
    dashPeriod = aDashPeriod;
    startPos = aPos.xy;
    relativePos = aPos.xy;
    gl_Position = vec4(aPos.xy, 0.0, 1.0);
}
