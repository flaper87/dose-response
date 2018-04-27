#version 150 core

uniform sampler2D tex;

uniform vec2 texture_size_px;

// Tile-based index into the tilemap
in vec2 v_tile_pos_px;
in vec4 v_color;

out vec4 color;

void main() {
  // TODO: pick magic values that can't be set normally
  if (v_tile_pos_px == vec2(0.0, 5.0)) {
	color = v_color;
  } else {
    color = texture(tex, v_tile_pos_px / texture_size_px) * v_color;
  }
}
