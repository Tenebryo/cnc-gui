#version 400

uniform mat4 matrix;

in vec3 pos;
in vec4 col;
in float time;

out vec4 f_color;
out float time_interp;

// Built-in:
// vec4 gl_Position

void main() {
  f_color = col;
  time_interp = time;
  gl_Position = matrix * vec4(pos.xyz, 1);
}
