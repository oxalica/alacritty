#version 330 core

flat in vec4 color;

layout(location = 0, index = 0) out vec4 FragColor;
layout(location = 0, index = 1) out vec4 FragAlphaMask;

void main()
{
    FragAlphaMask = vec4(1.0);
    FragColor = vec4(color.rgb, 1.0);
}
