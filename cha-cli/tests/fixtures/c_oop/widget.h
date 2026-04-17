// widget.h — struct with cross-file methods (should NOT be flagged as lazy_class)
typedef struct {
    int x;
    int y;
} Widget;

void widget_init(Widget *w);
void widget_draw(Widget *w);
