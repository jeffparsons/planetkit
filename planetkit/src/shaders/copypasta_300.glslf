#version 300 es

precision mediump float;

in vec2 v_tex_coord;
in vec4 v_color;
out vec4 o_color;
uniform sampler2D t_color;

void main() {
    o_color = v_color;
}
