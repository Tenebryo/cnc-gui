#version 450

layout(location = 0) in vec4 f_color;
layout(location = 1) in float time_interp;

layout(location = 0) out vec4 color;

void main() {

  // float mod_time = mod(time_interp, time_interval);

  // if (mod_time < 0.2 * time_interval) {
  //   Target0 = f_color * 0.2;
  // } else {
  // }
    color = f_color;
}
