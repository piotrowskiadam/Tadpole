/* Build-time scaling of the existing icon without a nested image-loader sandbox. */
#include <cairo.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "Usage: scale-icon INPUT OUTPUT\n");
        return 2;
    }
    cairo_surface_t *input = cairo_image_surface_create_from_png(argv[1]);
    cairo_status_t status = cairo_surface_status(input);
    if (status != CAIRO_STATUS_SUCCESS) {
        fprintf(stderr, "Could not load icon: %s\n", cairo_status_to_string(status));
        cairo_surface_destroy(input);
        return 1;
    }
    const int width = cairo_image_surface_get_width(input);
    const int height = cairo_image_surface_get_height(input);
    if (width <= 0 || height <= 0) {
        cairo_surface_destroy(input);
        return 1;
    }
    cairo_surface_t *output = cairo_image_surface_create(CAIRO_FORMAT_ARGB32, 512, 512);
    cairo_t *context = cairo_create(output);
    const double scale = 512.0 / (width > height ? width : height);
    cairo_translate(context, (512.0 - width * scale) / 2.0, (512.0 - height * scale) / 2.0);
    cairo_scale(context, scale, scale);
    cairo_set_source_surface(context, input, 0, 0);
    cairo_pattern_set_filter(cairo_get_source(context), CAIRO_FILTER_BEST);
    cairo_paint(context);
    status = cairo_surface_write_to_png(output, argv[2]);
    cairo_destroy(context);
    cairo_surface_destroy(output);
    cairo_surface_destroy(input);
    if (status != CAIRO_STATUS_SUCCESS) {
        fprintf(stderr, "Could not save icon: %s\n", cairo_status_to_string(status));
        return 1;
    }
    return 0;
}
