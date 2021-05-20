#version 400

uniform float time_interval;
uniform vec4 tint;

in vec4 f_color;
in float time_interp;

out vec4 Target0;

void main() {

  // float mod_time = mod(time_interp, time_interval);

  // if (mod_time < 0.2 * time_interval) {
  //   Target0 = f_color * 0.2;
  // } else {
  // }
    Target0 = f_color * tint;
}
