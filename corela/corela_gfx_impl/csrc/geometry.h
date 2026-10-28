#ifndef CORELA_GEOMETRY_H
#define CORELA_GEOMETRY_H

#define CELL_MARKER_OUTER_RADIUS 15.75f
#define CELL_MARKER_INNER_RADIUS 14.25f
#define CHEVRON_SIZE 36.0f
#define HEX_RADIUS 4
#define HEX_SIZE 32.0f
// pixels shaved off each tile's radius so neighbouring tiles show a gap
#define HEX_TILE_INSET 2.0f
#define HEX_CORNER_COUNT 6
// flat-top hexagon: the top and bottom edges span +-radius/2 around the center
#define HEX_HALF_EDGE_TO_RADIUS_RATIO 0.5f
// sqrt(3) / 2
#define HEX_HALF_HEIGHT_TO_RADIUS_RATIO 0.8660254037844386f

// Mirrored by `corela_gfx::Geometry`; keep the field order in sync.
typedef struct {
  float cell_marker_outer_radius;
  float cell_marker_inner_radius;
  float chevron_size;
  int hex_radius;
  float hex_size;
  float hex_tile_inset;
} gfx_geometry;

#endif
