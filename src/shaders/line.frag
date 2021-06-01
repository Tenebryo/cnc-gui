#version 450


layout(location = 0) in vec4 f_color;
layout(location = 1) in float time_interp;
layout(location = 2) in vec2 center;

layout(location = 0) out vec4 color;

void main() {

  float line_width = 3.0;
  float blend_factor = 5.0;

  vec4 col = f_color;        
  double d = length(center-gl_FragCoord.xy);
  double w = line_width;
  if (d>w) {
    discard;
  } else {
    col.w *= pow(float((w-d)/w), blend_factor);
  }
  color = col;
  // color = vec4(center.x, center.y, gl_FragCoord.x, gl_FragCoord.y);
}
