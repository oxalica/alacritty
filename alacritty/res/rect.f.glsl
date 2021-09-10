#version 330 core

flat in vec4 color;
flat in float dashPeriod;
flat in vec2 startPos;
in vec2 relativePos;

out vec4 FragColor;

void main()
{
    float dist = (relativePos.x - startPos.x) / dashPeriod;
    if (fract(dist) > 0.5)
        discard;

    FragColor = color;
}
