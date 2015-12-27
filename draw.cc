#include <cairo/cairo.h>
#include <cmath>
#include <iostream>
#include <fstream>
#include <vector>
#include <algorithm>

// 300 is 4000 × 4000 pixels
const int METERS_PER_PIXEL = 100;

// Bounding box of France in the Lambert93 projection
const int XMIN = 100 * 1000;
const int XMAX = 1300 * 1000;
const int YMIN = 6000 * 1000; // Fun fact, the 0 is in Algeria
const int YMAX = 7200 * 1000;

float to_pixel(float val) {
  return val / METERS_PER_PIXEL;
}

struct edge {
  float x1, y1, x2, y2, count;
};

float width(size_t count) {
  return 2 * ::log10(count) - 1;
}

float darkness(float width, float max_width) {
  return (max_width - width)/(1.5 * max_width);
}

void draw(const std::vector<edge> &edges, size_t size, int cut_off, std::string filename) {
  float max_width = width(size);
  cairo_surface_t *surface;
  cairo_t *cr;

  surface = cairo_image_surface_create (CAIRO_FORMAT_RGB24, to_pixel(XMAX - XMIN), to_pixel(YMAX - YMIN));
  cr = cairo_create (surface);

  cairo_new_path (cr);
  cairo_rectangle(cr, 0, 0, to_pixel(XMAX - XMIN), to_pixel(YMAX - YMIN));
  cairo_set_source_rgb (cr, 1, 1, 1);
  cairo_fill(cr);

  cairo_set_line_cap  (cr, CAIRO_LINE_CAP_ROUND);
  for(auto &e : edges) {
    if(e.count > cut_off) {
      float x1 = to_pixel(e.x1 - XMIN);
      float y1 = to_pixel(YMAX - e.y1);
      float x2 = to_pixel(e.x2 - XMIN);
      float y2 = to_pixel(YMAX - e.y2);

      float w = width(e.count);
      float d = darkness(w, max_width);

      cairo_set_source_rgb(cr, d, d, d);
      cairo_set_line_width (cr, w);
      cairo_move_to(cr, x1, y1);
      cairo_line_to(cr, x2, y2);
      cairo_stroke(cr);
    }
  }
  cairo_surface_write_to_png(surface, filename.c_str());
}

int main() {
  std::ifstream file("edges_dump");
  
  size_t size;
  file.read(reinterpret_cast<char *>(&size), sizeof(size));

  std::vector<edge> edges(size);
  file.read(reinterpret_cast<char *>(edges.data()), sizeof(edge) * size);
  // We want that the least important edges are drawn earlier
  std::sort(edges.begin(), edges.end(), [](edge a, edge b) { return a.count < b.count;});
   
  draw(edges, size, 10, "routes_from_nd.png");
}
