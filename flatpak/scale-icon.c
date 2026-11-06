/* Build-time scaling of the project's existing icon for Flatpak's 512px limit. */
#include <gdk-pixbuf/gdk-pixbuf.h>
#include <stdio.h>

int main(int argc, char **argv) {
    if (argc != 3) {
        fprintf(stderr, "Usage: scale-icon INPUT OUTPUT\n");
        return 2;
    }
    GError *error = NULL;
    GdkPixbuf *icon = gdk_pixbuf_new_from_file_at_scale(argv[1], 512, 512, TRUE, &error);
    if (!icon) {
        fprintf(stderr, "Could not load icon: %s\n", error->message);
        g_error_free(error);
        return 1;
    }
    gboolean saved = gdk_pixbuf_save(icon, argv[2], "png", &error, NULL);
    g_object_unref(icon);
    if (!saved) {
        fprintf(stderr, "Could not save icon: %s\n", error->message);
        g_error_free(error);
        return 1;
    }
    return 0;
}
