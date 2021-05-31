#version 450

layout(push_constant) uniform PushConstants {
  uniform mat4 matrix;
};

layout(location=0) in vec3 pos;
layout(location=1) in vec4 col;
layout(location=2) in float time;

layout(location=0) out vec4 f_color;
layout(location=1) out float time_interp;

// Built-in:
// vec4 gl_Position

void main() {
  f_color = col;
  time_interp = time;
  gl_Position = matrix * vec4(pos.xyz, 1);
}
