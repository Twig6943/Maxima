const vec2 verts[6] = vec2[6](
    vec2(-1.0, -1.0),
    vec2( 1.0, -1.0),
    vec2( 1.0,  1.0),
    vec2(-1.0, -1.0),
    vec2( 1.0,  1.0),
    vec2(-1.0,  1.0)
);
const vec4 colors[6] = vec4[6](
    vec4(0.0, 0.0, 0.0, 1.0),
    vec4(1.0, 0.0, 0.0, 1.0),
    vec4(1.0, 1.0, 0.0, 1.0),
    vec4(0.0, 0.0, 0.0, 1.0),
    vec4(1.0, 1.0, 0.0, 1.0),
    vec4(0.0, 1.0, 0.0, 1.0)
);
//the extra miniscule vram hit is likely better than doing a bunch of weird flipping and scaling every frame
const vec2 uvs[6] = vec2[6](
    vec2(0.0,1.0),
    vec2(1.0,1.0),
    vec2(1.0,0.0),
    vec2(0.0,1.0),
    vec2(1.0,0.0),
    vec2(0.0,0.0)
);
out vec4 v_color;
out vec2 og_pos;
out vec2 tex_coords;
uniform vec2 u_dimensions;
uniform vec2 u_img_dimensions;
void main() {
    v_color = colors[gl_VertexID];
    vec2 vpos = verts[gl_VertexID];
    og_pos = verts[gl_VertexID];
    tex_coords = uvs[gl_VertexID];
    float src_aspect = u_dimensions.x / u_dimensions.y;
    float dst_aspect = u_img_dimensions.x / u_img_dimensions.y;
    
    if ( (src_aspect / dst_aspect) < 1.0 ) {        // Taller
        vpos.x *= ((u_dimensions.y/u_img_dimensions.y) * u_img_dimensions.x) / u_dimensions.x;
    } else if ( (src_aspect / dst_aspect) > 1.0 ) { // Wider
        vpos.y *= ((u_dimensions.x/u_img_dimensions.x) * u_img_dimensions.y) / u_dimensions.y;
    }

    gl_Position = vec4(vpos, 0.0, 1.0);
    
}