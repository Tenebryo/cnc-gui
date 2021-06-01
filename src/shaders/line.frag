#version 450

layout(location = 0) in vec4 f_color;
layout(location = 1) in float time_interp;
layout(location = 2) in vec2 center;

layout(location = 0) out vec4 color;

void main() {

  color = f_color;
}
