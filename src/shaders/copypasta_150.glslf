#version 150 core
in vec2 v_tex_coord;
out vec4 o_color;
uniform sampler2D t_color;
void main() {
    vec4 tex = texture(t_color, v_tex_coord);
    float blend = dot(v_tex_coord-vec2(0.5,0.5), v_tex_coord-vec2(0.5,0.5));
    o_color = mix(tex, vec4(0.0,0.0,0.0,0.0), blend*1.0);
}
