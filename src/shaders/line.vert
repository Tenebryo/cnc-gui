#version 450

layout(push_constant) uniform PushConstants {
  mat4 matrix;
  vec2 viewport;
};

layout(location = 0) in vec3 pos;
layout(location = 1) in vec4 col;
layout(location = 2) in float time;

layout(location = 0) out vec4 f_color;
layout(location = 1) out float time_interp;
layout(location = 2) out vec2 center;

// Built-in:
// vec4 gl_Position

void main() {
  f_color = col;
  time_interp = time;

  vec4 tpos = matrix * vec4(pos.xyz, 1);

  gl_Position = tpos;

  vec2 vp = viewport;
  center = 0.5 * (tpos.xy + vec2(1)) * vp;
}
