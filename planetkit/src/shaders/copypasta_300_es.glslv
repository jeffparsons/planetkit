#version 300 es

in vec3 a_pos;
in vec2 a_tex_coord;
in vec3 a_color;
out vec2 v_tex_coord;
out vec4 v_color;
uniform mat4 u_model_view_proj;

void main() {
    v_tex_coord = a_tex_coord;
    v_color = vec4(a_color, 1.0);
    gl_Position = u_model_view_proj * vec4(a_pos, 1.0);
}
